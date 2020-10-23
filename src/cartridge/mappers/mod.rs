use cartridge::mirroring::MirroringMode;
use cartridge::{CpuCartridgeAddressBus, PpuCartridgeAddressBus};
use log::debug;

pub(super) mod cnrom; // Mapper 3
pub(super) mod mmc1; // Mapper 1
pub(super) mod mmc3; // Mapper 4
pub(super) mod nrom; // Mapper 0
pub(super) mod uxrom; // Mapper 2, 94, 180

pub(crate) enum ChrData {
    Rom(Vec<u8>),
    Ram(Box<[u8; 0x2000]>),
}

pub(super) struct BankedPrgChip {
    prg_rom: Vec<u8>,
    prg_ram: Option<[u8; 0x2000]>,
    total_banks: u8,
    prg_rom_banks: [u8; 2],
    prg_rom_bank_offsets: [usize; 2],
    write_byte_function: fn(u16, u8, u8, &mut [u8; 2], &mut [usize; 2]) -> (),
}

impl BankedPrgChip {
    pub(super) fn new(
        prg_rom: Vec<u8>,
        prg_ram: Option<[u8; 0x2000]>,
        total_banks: u8,
        prg_rom_banks: [u8; 2],
        prg_rom_bank_offsets: [usize; 2],
        write_byte_function: fn(u16, u8, u8, &mut [u8; 2], &mut [usize; 2]) -> (),
    ) -> Self {
        let full_prg_rom = match prg_rom.len() {
            0x4000 => {
                let mut full = prg_rom.clone();
                full.extend(prg_rom);

                full
            }
            _ => prg_rom,
        };

        BankedPrgChip {
            prg_rom: full_prg_rom,
            prg_ram,
            total_banks,
            prg_rom_banks,
            prg_rom_bank_offsets,
            write_byte_function,
        }
    }
}

impl CpuCartridgeAddressBus for BankedPrgChip {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x6000..=0x7FFF => match self.prg_ram {
                None => 0x0,
                Some(ram) => ram[(address - 0x6000) as usize],
            },
            0x8000..=0xBFFF => self.prg_rom[self.prg_rom_bank_offsets[0] + (address as usize - 0x8000)],
            0xC000..=0xFFFF => self.prg_rom[self.prg_rom_bank_offsets[1] + (address as usize - 0xC000)],
            _ => 0x0, // TODO - Not 100% sure on this, but mapper tests do check it.
        }
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        debug!("Mapper write {:04X}={:02X}", address, value);

        match address {
            0x6000..=0x7FFF => match self.prg_ram {
                None => (),
                Some(mut ram) => ram[(address - 0x6000) as usize] = value,
            },
            0x8000..=0xFFFF => (self.write_byte_function)(
                address,
                value,
                self.total_banks,
                &mut self.prg_rom_banks,
                &mut self.prg_rom_bank_offsets,
            ),
            _ => (), // TODO - Not 100% sure on this, but mapper tests do check it.
        }
    }
}

pub(crate) struct BankedChrChip {
    chr_data: ChrData,
    ppu_vram: [u8; 0x1000],
    total_chr_banks: u8,
    chr_bank: u8,
    chr_bank_offset: usize,
    mirroring_mode: MirroringMode,
}

impl BankedChrChip {
    pub(super) fn new(chr_rom: Option<Vec<u8>>, mirroring_mode: MirroringMode, total_chr_banks: u8) -> Self {
        match chr_rom {
            Some(rom) => BankedChrChip {
                chr_data: ChrData::Rom(rom),
                ppu_vram: [0; 0x1000],
                total_chr_banks,
                chr_bank: 0,
                chr_bank_offset: 0,
                mirroring_mode,
            },
            None => BankedChrChip {
                chr_data: ChrData::Ram(Box::new([0; 0x2000])),
                ppu_vram: [0; 0x1000],
                total_chr_banks,
                chr_bank: 0,
                chr_bank_offset: 0,
                mirroring_mode,
            },
        }
    }
}

impl PpuCartridgeAddressBus for BankedChrChip {
    fn check_trigger_irq(&mut self) -> bool {
        false
    }

    fn read_byte(&mut self, address: u16, _: u32) -> u8 {
        match address {
            0x0000..=0x1FFF => match &self.chr_data {
                ChrData::Rom(rom) => rom[address as usize + self.chr_bank_offset],
                ChrData::Ram(ram) => ram[address as usize],
            },
            0x2000..=0x3EFF => {
                let mirrored_address = self.mirroring_mode.get_mirrored_address(address);
                debug!("Read {:04X} mirrored to {:04X}", address, mirrored_address);

                self.ppu_vram[mirrored_address as usize]
            }
            0x3F00..=0x3FFF => panic!("Shouldn't be reading from palette RAM through cartridge bus"),
            _ => panic!("Reading from {:04X} invalid for CHR address bus", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        debug!("MMC1 CHR write {:04X}={:02X}", address, value);
        match address {
            0x0000..=0x1FFF => match &mut self.chr_data {
                ChrData::Rom(_) => (),
                ChrData::Ram(ram) => ram[address as usize] = value,
            },
            0x2000..=0x3EFF => {
                let mirrored_address = self.mirroring_mode.get_mirrored_address(address);

                self.ppu_vram[mirrored_address as usize] = value;
            }
            0x3F00..=0x3FFF => panic!("Shouldn't be writing to palette registers through the cartridge address bus"),
            _ => panic!("Write to {:04X} ({:02X}) invalid for CHR address bus", address, value),
        }
    }

    fn cpu_write_byte(&mut self, address: u16, value: u8, cycles: u32) {
        debug!(
            "CPU write to CHR bus {:04X}={:02X} at {:} cycles",
            address, value, cycles
        );

        if let 0x8000..=0xFFFF = address {
            self.chr_bank = value % self.total_chr_banks;
            self.chr_bank_offset = self.chr_bank as usize * 0x2000;
        }
    }
}
