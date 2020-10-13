use cartridge::mappers::ChrData;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::MirroringMode;
use cartridge::PpuCartridgeAddressBus;
use log::{debug, info};

pub(crate) struct MapperZeroPrgChip {
    prg_rom: Vec<u8>,
    prg_ram: [u8; 0x2000],
}

pub(crate) struct MapperZeroChrChip {
    chr_data: ChrData,
    ppu_vram: [u8; 0x1000],
    mirroring_mode: MirroringMode,
}

impl MapperZeroChrChip {
    fn new(chr_rom: Option<Vec<u8>>, mirroring_mode: MirroringMode) -> Self {
        match chr_rom {
            Some(rom) => MapperZeroChrChip {
                chr_data: ChrData::Rom(rom),
                ppu_vram: [0; 0x1000],
                mirroring_mode,
            },
            None => MapperZeroChrChip {
                chr_data: ChrData::Ram([0; 0x2000]),
                ppu_vram: [0; 0x1000],
                mirroring_mode,
            },
        }
    }
}

impl CpuCartridgeAddressBus for MapperZeroPrgChip {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x6000..=0x7FFF => self.prg_ram[(address - 0x6000) as usize], // TODO - Family basic model only
            0x8000..=0xFFFF => self.prg_rom[(address - 0x8000) as usize],
            _ => todo!("Not yet mapped addresses in zero mapper {:04X}", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        match address {
            0x6000..=0x7FFF => self.prg_ram[(address - 0x6000) as usize] = value, // TODO - Family basic model only
            _ => (),
        }
    }
}

impl PpuCartridgeAddressBus for MapperZeroChrChip {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x1FFF => match &self.chr_data {
                ChrData::Rom(rom) => rom[address as usize],
                ChrData::Ram(ram) => ram[address as usize],
            },
            0x2000..=0x3EFF => {
                let mirrored_address = self.mirroring_mode.get_mirrored_address(address);
                self.ppu_vram[mirrored_address as usize]
            }
            0x3000..=0x3EFF => self.ppu_vram[(address - 0x3000) as usize],
            _ => todo!("Not yet mapped addresses in zero mapper {:04X}", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        debug!("Writing to CHR address bus {:04X}={:02X}", address, value);

        match address {
            0x0000..=0x1FFF => match &mut self.chr_data {
                ChrData::Rom(_) => (),
                ChrData::Ram(ram) => ram[address as usize] = value,
            },
            0x2000..=0x2FFF => self.ppu_vram[(address - 0x2000) as usize] = value,
            0x3000..=0x3EFF => self.ppu_vram[(address - 0x3000) as usize] = value,
            0x3F00..=0x3FFF => panic!(
                "Shouldn't be writing to palette registers through the cartridge address bus"
            ),
            _ => panic!(
                "Write to {:04X} ({:02X}) invalid for CHR address bus",
                address, value
            ),
        }
    }

    fn cpu_write_byte(&mut self, _: u16, _: u8, _: u32) {}
}

pub(crate) fn from_header(
    prg_rom: Vec<u8>,
    chr_rom: Option<Vec<u8>>,
    header: CartridgeHeader,
) -> (
    Box<dyn CpuCartridgeAddressBus>,
    Box<dyn PpuCartridgeAddressBus>,
    CartridgeHeader,
) {
    // NROM either has 16KB or 32KB of ROM, to make lookups faster we pre-
    // mirror the ROM data into the second bank here
    let full_prg_rom = match prg_rom.len() {
        0x4000 => {
            let mut full = prg_rom.clone();
            full.extend(prg_rom);

            full
        }
        _ => prg_rom,
    };

    info!("Creating NROM mapper for cartridge");
    (
        Box::new(MapperZeroPrgChip {
            prg_rom: full_prg_rom,
            prg_ram: [0; 0x2000],
        }),
        Box::new(MapperZeroChrChip::new(chr_rom, header.mirroring)),
        header,
    )
}
