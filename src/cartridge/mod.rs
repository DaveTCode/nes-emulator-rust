pub(crate) mod mappers;

use cartridge::mappers::mapper_zero::MapperZero;
use cartridge::mappers::Cartridge;
use log::info;
use std::error::Error;
use std::fmt;
use std::io;

/// Represents any error which occurs during loading a cartridge
#[derive(Debug)]
pub(crate) struct CartridgeError {
    pub(crate) message: String,
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
        }
    }
}

pub(crate) struct CartridgeHeader {
    pub(crate) prg_rom_16kb_units: u8,
    pub(crate) chr_rom_8kb_units: u8,
    pub(crate) mapper: u8,
    // TODO - Lots more flags and possible options
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

pub(crate) fn from_file(file_path: &str) -> Result<Box<dyn Cartridge>, CartridgeError> {
    let bytes = std::fs::read(file_path)?;

    if bytes.len() < 0x10 {
        return Err(CartridgeError {
            message: format!("Invalid cartridge file {}, header < 16 bytes", file_path),
        });
    }

    let header = CartridgeHeader {
        prg_rom_16kb_units: bytes[4],
        chr_rom_8kb_units: bytes[5],
        mapper: (bytes[6] >> 4) | (bytes[7] & 0b1111_0000),
    };

    info!("{}", header);

    let prg_rom_start = 0x10 as usize;
    let prg_rom_end = prg_rom_start + (header.prg_rom_16kb_units as usize * 0x4000);
    let chr_rom_end = prg_rom_end + (header.chr_rom_8kb_units as usize * 0x2000);

    if bytes.len() < chr_rom_end {
        return Err(CartridgeError {
          message: format!("Invalid cartridge file {}, header specified {:x} prg rom units and {:x} chr rom units but total length was {:x}", file_path, header.prg_rom_16kb_units, header.chr_rom_8kb_units, bytes.len())
        });
    }

    match header.mapper {
        0 => Ok(Box::new(MapperZero {
            header,
            prg_rom: bytes[16..prg_rom_end].to_vec(),
            chr_rom: bytes[prg_rom_end..chr_rom_end].to_vec(),
        })),
        _ => Err(CartridgeError {
            message: format!("Mapper {:x} not yet implemented", header.mapper),
        }),
    }
}
