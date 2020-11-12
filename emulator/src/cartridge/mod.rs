mod mappers;
mod mirroring;

use cartridge::mirroring::MirroringMode;
use cpu::CpuCycle;
use log::info;
use ppu::PpuCycle;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;
use zip::result::ZipError;
use zip::ZipArchive;
use Cartridge;

/// Represents any error which occurs during loading a cartridge
#[derive(Debug)]
pub struct CartridgeError {
    pub message: String,
    pub mapper: Option<u8>,
}
impl Error for CartridgeError {}
impl fmt::Display for CartridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error loading the cartridge")
    }
}
impl From<io::Error> for CartridgeError {
    fn from(error: io::Error) -> Self {
        CartridgeError {
            message: error.to_string(),
            mapper: None,
        }
    }
}
impl From<ZipError> for CartridgeError {
    fn from(error: ZipError) -> Self {
        CartridgeError {
            message: error.to_string(),
            mapper: None,
        }
    }
}

/// A trait representing the CPU address bus into the cartridge
pub trait CpuCartridgeAddressBus {
    /// Read from the 16 bit CPU address bus
    fn read_byte(&self, address: u16) -> u8;
    /// Write to the 16 bit CPU address bus
    fn write_byte(&mut self, address: u16, value: u8, cycles: PpuCycle);
}

/// A trait representing the PPU address bus into the cartridge
pub trait PpuCartridgeAddressBus {
    /// Certain mappers can trigger an IRQ based on scanline counting (MMC3)
    /// This function allows the CPU to poll and request state on whether an IRQ is ready to fire.
    fn check_trigger_irq(&mut self, clear: bool) -> bool;
    /// Certain mappers can trigger an IRQ based on scanline counting (MMC3)
    /// This function allows the mapper to listen on address bus changes
    fn update_vram_address(&mut self, address: u16, cycles: PpuCycle);
    /// Read from the 14 bit PPU address bus
    fn read_byte(&mut self, address: u16, cycles: PpuCycle) -> u8;
    /// Write to the 14 bit PPU address bus
    fn write_byte(&mut self, address: u16, value: u8, cycles: PpuCycle);
    /// Write to the 16 bit CPU address bus, required to set mapper registers
    fn cpu_write_byte(&mut self, address: u16, value: u8, cycles: CpuCycle);
}

/// Represents flags/details about the rom from the header
/// c.f. http://wiki.nesdev.com/w/index.php/INES for details
#[derive(Debug)]
pub struct CartridgeHeader {
    pub prg_rom_16kb_units: u8,
    pub chr_rom_8kb_units: u8,
    pub mapper: u8,
    pub mirroring: MirroringMode,
    pub ram_is_battery_backed: bool,
    // TODO - Lots more flags and possible options
}

impl CartridgeHeader {
    fn new(prg_rom_16kb_units: u8, chr_rom_8kb_units: u8, flags_6: u8, flags_7: u8) -> Self {
        CartridgeHeader {
            prg_rom_16kb_units,
            chr_rom_8kb_units,
            mapper: (flags_6 >> 4) | (flags_7 & 0b1111_0000),
            mirroring: match (flags_6 & 1 == 0, flags_6 & 0b1000 == 0) {
                (true, true) => MirroringMode::Horizontal,
                (false, true) => MirroringMode::Vertical,
                (_, false) => MirroringMode::FourScreen,
            },
            ram_is_battery_backed: flags_6 & 0b10 == 0b10,
        }
    }
}

impl fmt::Display for CartridgeHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PRG Units {}, CHR Units {}, Mapper {}",
            self.prg_rom_16kb_units, self.chr_rom_8kb_units, self.mapper
        )
    }
}

pub(crate) fn from_file(file_path: &str) -> Result<Cartridge, CartridgeError> {
    let file_extension = Path::new(file_path).extension().and_then(OsStr::to_str);
    let file = File::open(file_path)?;

    let mut bytes = Vec::<u8>::new();
    match file_extension {
        Some("zip") => {
            let mut zip = ZipArchive::new(file)?;

            let nes_files = (0..zip.len())
                .filter_map(|ix| {
                    let zfile = zip.by_index(ix).unwrap();
                    let extension = Path::new(zfile.name()).extension().and_then(OsStr::to_str);

                    match extension {
                        Some("nes") => Some(ix),
                        _ => None,
                    }
                })
                .collect::<Vec<_>>();

            match nes_files.first() {
                None => {
                    return Err(CartridgeError {
                        message: "The zip file must contain only one file with the .nes extension".to_string(),
                        mapper: None,
                    });
                }
                Some(zip_file_index) => {
                    let mut zfile = zip.by_index(*zip_file_index).unwrap();
                    zfile.read_to_end(&mut bytes)?;
                }
            }
        }
        _ => bytes = std::fs::read(file_path)?,
    };

    if bytes.len() < 0x10 {
        return Err(CartridgeError {
            message: format!("Invalid cartridge file {}, header < 16 bytes", file_path),
            mapper: None,
        });
    }

    let header = CartridgeHeader::new(bytes[4], bytes[5], bytes[6], bytes[7]);

    info!("{}: {:08b} {:08b}", header, bytes[6], bytes[7]);

    let prg_rom_start = 0x10 as usize;
    let prg_rom_end = prg_rom_start + (header.prg_rom_16kb_units as usize * 0x4000);
    let chr_rom_end = prg_rom_end + (header.chr_rom_8kb_units as usize * 0x2000);

    if bytes.len() < chr_rom_end {
        return Err(CartridgeError {
          message: format!("Invalid cartridge file {}, header specified {:x} prg rom units and {:x} chr rom units but total length was {:x}",
                           file_path,
                           header.prg_rom_16kb_units,
                           header.chr_rom_8kb_units,
                           bytes.len()),
          mapper: None,
        });
    }

    let prg_rom = bytes[16..prg_rom_end].to_vec();
    let chr_rom = match header.chr_rom_8kb_units {
        0 => None,
        _ => Some(bytes[prg_rom_end..chr_rom_end].to_vec()),
    };

    match header.mapper {
        0 => Ok(mappers::nrom::from_header(prg_rom, chr_rom, header)),
        1 | 155 => Ok(mappers::mmc1::from_header(prg_rom, chr_rom, header)),
        2 | 94 | 180 => Ok(mappers::uxrom::from_header(prg_rom, chr_rom, header)),
        3 => Ok(mappers::cnrom::from_header(prg_rom, chr_rom, header)),
        4 => Ok(mappers::mmc3::from_header(prg_rom, chr_rom, header)),
        7 => Ok(mappers::axrom::from_header(prg_rom, chr_rom, header)),
        9 => Ok(mappers::mmc2::from_header(prg_rom, chr_rom, header)),
        10 => Ok(mappers::mmc4::from_header(prg_rom, chr_rom, header)),
        11 => Ok(mappers::color_dreams::from_header(prg_rom, chr_rom, header)),
        66 => Ok(mappers::gxrom::from_header(prg_rom, chr_rom, header)),
        _ => Err(CartridgeError {
            message: format!("Mapper {} not yet implemented", header.mapper),
            mapper: Some(header.mapper),
        }),
    }
}
