use cartridge::mirroring::MirroringMode;
use log::debug;

pub(super) mod axrom; // Mapper 7
pub(super) mod cnrom; // Mapper 3
pub(super) mod mmc1; // Mapper 1
pub(super) mod mmc2; // Mapper 9
pub(super) mod mmc3; // Mapper 4
pub(super) mod mmc4; // Mapper 10
pub(super) mod nrom; // Mapper 0
pub(super) mod uxrom; // Mapper 2, 94, 180

#[derive(Debug)]
pub(crate) enum ChrData {
    Rom(Vec<u8>),
    Ram(Box<[u8; 0x2000]>),
}

impl From<Option<Vec<u8>>> for ChrData {
    fn from(chr_rom: Option<Vec<u8>>) -> Self {
        match chr_rom {
            Some(rom) => ChrData::Rom(rom),
            None => ChrData::Ram(Box::new([0; 0x2000])),
        }
    }
}

/// This structure contains common information used by all CHR units on all mappers
#[derive(Debug)]
pub(crate) struct ChrBaseData {
    mirroring_mode: MirroringMode,
    chr_data: ChrData,
    ppu_vram: [u8; 0x1000],
    bank_size: usize,
    total_banks: usize,
    banks: Vec<usize>,
    bank_offsets: Vec<usize>,
}

impl ChrBaseData {
    fn new(
        mirroring_mode: MirroringMode,
        chr_data: ChrData,
        bank_size: usize,
        banks: Vec<usize>,
        bank_offsets: Vec<usize>,
    ) -> Self {
        debug_assert!(banks.len() == bank_offsets.len());

        let total_banks = match &chr_data {
            ChrData::Ram(_) => 0x2000 / bank_size,
            ChrData::Rom(rom) => rom.len() / bank_size,
        };

        ChrBaseData {
            mirroring_mode,
            chr_data,
            total_banks: if total_banks == 0 { 1 } else { total_banks },
            bank_size,
            banks,
            bank_offsets,
            ppu_vram: [0; 0x1000],
        }
    }

    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x1FFF => {
                let bank = address as usize / self.bank_size;
                let offset = bank * self.bank_size;

                match &self.chr_data {
                    ChrData::Rom(rom) => rom[address as usize - offset + self.bank_offsets[bank]],
                    ChrData::Ram(ram) => ram[address as usize - offset + self.bank_offsets[bank]],
                }
            }
            0x2000..=0x3EFF => {
                let mirrored_address = self.mirroring_mode.get_mirrored_address(address);
                debug!("Read {:04X} mirrored to {:04X}", address, mirrored_address);

                self.ppu_vram[mirrored_address as usize]
            }
            0x3F00..=0x3FFF => panic!("Shouldn't be reading from palette RAM through cartridge bus"),
            _ => panic!("Reading from {:04X} invalid for CHR address bus", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        debug!("CHR write {:04X}={:02X}", address, value);

        match address {
            0x0000..=0x1FFF => match &mut self.chr_data {
                ChrData::Rom(_) => (),
                ChrData::Ram(ram) => {
                    let bank = address as usize / self.bank_size;
                    let offset = bank * self.bank_size;
                    ram[address as usize - offset + self.bank_offsets[bank]] = value
                }
            },
            0x2000..=0x3EFF => {
                let mirrored_address = self.mirroring_mode.get_mirrored_address(address);

                self.ppu_vram[mirrored_address as usize] = value;
            }
            0x3F00..=0x3FFF => panic!("Shouldn't be writing to palette registers through the cartridge address bus"),
            _ => panic!("Write to {:04X} ({:02X}) invalid for CHR address bus", address, value),
        }
    }
}

pub(crate) struct PrgBaseData {
    prg_rom: Vec<u8>,
    prg_ram: Option<[u8; 0x2000]>,
    total_banks: usize,
    bank_size: usize,
    banks: Vec<usize>,
    bank_offsets: Vec<usize>,
}

impl PrgBaseData {
    pub(super) fn new(
        prg_rom: Vec<u8>,
        prg_ram: Option<[u8; 0x2000]>,
        total_banks: usize,
        bank_size: usize,
        banks: Vec<usize>,
        bank_offsets: Vec<usize>,
    ) -> Self {
        let full_prg_rom = match prg_rom.len() {
            0x4000 => {
                let mut full = prg_rom.clone();
                full.extend(prg_rom);

                full
            }
            _ => prg_rom,
        };

        debug_assert!(banks.len() == bank_offsets.len());
        debug_assert!(
            total_banks * bank_size == full_prg_rom.len(),
            "{} * {} != {}",
            total_banks,
            bank_size,
            full_prg_rom.len()
        );

        PrgBaseData {
            prg_rom: full_prg_rom,
            prg_ram,
            total_banks,
            bank_size,
            banks,
            bank_offsets,
        }
    }

    pub(crate) fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x6000..=0x7FFF => match self.prg_ram {
                None => 0x0,
                Some(ram) => ram[(address - 0x6000) as usize],
            },
            0x8000..=0xFFFF => {
                let bank = (address as usize - 0x8000) / self.bank_size;
                let offset = bank * self.bank_size;

                self.prg_rom[self.bank_offsets[bank] + (address as usize) - offset - 0x8000]
            }
            _ => 0x0,
        }
    }

    pub(crate) fn write_byte(&mut self, address: u16, value: u8) {
        debug!("Mapper write {:04X}={:02X}", address, value);

        if let 0x6000..=0x7FFF = address {
            match self.prg_ram {
                None => (),
                Some(mut ram) => ram[(address - 0x6000) as usize] = value,
            }
        }
    }
}
