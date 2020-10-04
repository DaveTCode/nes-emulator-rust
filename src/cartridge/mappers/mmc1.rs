use cartridge::mappers::ChrData;
use cartridge::CartridgeAddressBus;
use cartridge::CartridgeHeader;
use log::{debug, info};

enum MirroringMode {
    OneScreenLowerBank,
    OneScreenUpperBank,
    Vertical,
    Horizontal,
}

enum PRGBankMode {
    Switch32KB,
    FixFirst16KB,
    FixLast16KB,
}

enum CHRBankMode {
    Switch8KB,
    Switch4KB,
}

struct ControlRegister {
    mirroring_mode: MirroringMode,
    prg_bank_mode: PRGBankMode,
    chr_bank_mode: CHRBankMode,
}

pub(crate) struct MMC1PrgChip {
    prg_rom: Vec<u8>,
    prg_ram: [u8; 0x2000],
    prg_ram_enabled: bool,
    last_write_cycle: u32,
    load_register: u8,
    shift_writes: u8,
    prg_bank: u8,
    prg_bank_offsets: [u16; 2],
    chr_bank: [u8; 2],
    control_register: ControlRegister,
}

impl MMC1PrgChip {
    fn new(prg_rom: Vec<u8>) -> Self {
        let mut chip = MMC1PrgChip {
            prg_rom,
            prg_ram: [0; 0x2000],
            prg_ram_enabled: true,
            last_write_cycle: 0,
            load_register: 0,
            shift_writes: 0,
            prg_bank: 0,
            prg_bank_offsets: [0; 2],
            chr_bank: [0; 2],
            control_register: ControlRegister {
                mirroring_mode: MirroringMode::OneScreenLowerBank,
                prg_bank_mode: PRGBankMode::FixLast16KB,
                chr_bank_mode: CHRBankMode::Switch8KB,
            },
        };

        chip.update_bank_offsets();

        chip
    }

    fn update_control_register(&mut self, value: u8) {
        self.control_register.mirroring_mode = match value & 0b11 {
            0b00 => MirroringMode::OneScreenLowerBank,
            0b01 => MirroringMode::OneScreenUpperBank,
            0b10 => MirroringMode::Vertical,
            0b11 => MirroringMode::Horizontal,
            _ => panic!(),
        };

        self.control_register.prg_bank_mode = match (value >> 2) & 0b11 {
            0b00 | 0b01 => PRGBankMode::Switch32KB,
            0b10 => PRGBankMode::FixFirst16KB,
            0b11 => PRGBankMode::FixLast16KB,
            _ => panic!(),
        };

        self.control_register.chr_bank_mode = match (value >> 4) & 0b1 {
            0b0 => CHRBankMode::Switch8KB,
            0b1 => CHRBankMode::Switch4KB,
            _ => panic!(),
        };

        debug!("MMC1 Control register updated with value: {:02X}", value);

        self.update_bank_offsets();
    }

    fn update_chr_bank(&mut self, value: u8, bank: usize) {
        debug_assert!(bank <= 1);

        self.chr_bank[bank] = match self.control_register.chr_bank_mode {
            CHRBankMode::Switch4KB => value & 0b1_1111,
            CHRBankMode::Switch8KB => (value >> 1) & 0b1111,
        }
    }

    fn update_prg_bank(&mut self, value: u8) {
        self.prg_ram_enabled = value & 0b1_0000 == 0;

        self.prg_bank = match self.control_register.prg_bank_mode {
            PRGBankMode::Switch32KB => (value >> 1) & 0b111,
            _ => value & 0b1111,
        };

        self.update_bank_offsets();
    }

    fn update_bank_offsets(&mut self) {
        match self.control_register.prg_bank_mode {
            PRGBankMode::FixFirst16KB => {
                let base = ((self.prg_bank as u16 & 0xF) >> 1) * 0x4000;
                self.prg_bank_offsets[0] = base;
                self.prg_bank_offsets[1] = base + 0x4000;
            }
            PRGBankMode::FixLast16KB => {
                self.prg_bank_offsets[0] = (self.prg_bank as u16 & 0xF) * 0x4000;
                self.prg_bank_offsets[1] = self.prg_rom.len() as u16 - 0x4000;
            }
            PRGBankMode::Switch32KB => {
                self.prg_bank_offsets[0] = 0;
                self.prg_bank_offsets[1] = (self.prg_bank as u16 & 0xF) * 0x4000;
            }
        };

        info!("Bank offsets updated: {:04X} {:04X}", self.prg_bank_offsets[0], self.prg_bank_offsets[1]);
    }
}

impl CartridgeAddressBus for MMC1PrgChip {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x6000..=0x7FFF => {
                if self.prg_ram_enabled {
                    self.prg_ram[(address - 0x6000) as usize]
                } else {
                    0x0
                }
            }
            0x8000..=0xBFFF => {
                let adj_addr = address as usize - 0x8000;

                self.prg_rom[adj_addr + self.prg_bank_offsets[0] as usize]
            }
            0xC000..=0xFFFF => {
                let adj_addr = address as usize - 0xC000;

                self.prg_rom[adj_addr + self.prg_bank_offsets[1] as usize]
            }
            _ => todo!("Not yet mapped addresses in zero mapper {:04X}", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8, cycles: u32) {
        // Skip writes on consectutive cycles
        if cycles == self.last_write_cycle + 1 {
            return;
        }
        self.last_write_cycle = cycles;

        match address {
            0x6000..=0x7FFF => {
                if self.prg_ram_enabled {
                    // TODO - some variants of MMC1 always have RAM enabled
                    self.prg_ram[(address - 0x6000) as usize] = value;
                }
            }
            0x8000..=0xFFFF => {
                if value & 0b1000_0000 != 0 {
                    self.load_register = 0;
                    self.shift_writes = 0;
                    self.update_control_register(0x0C);
                } else {
                    self.load_register |= (value & 1) << self.shift_writes;
                    self.shift_writes += 1;

                    if self.shift_writes == 5 {
                        match address {
                            0x8000..=0x9FFF => self.update_control_register(value),
                            0xA000..=0xBFFF => self.update_chr_bank(value, 0),
                            0xC000..=0xDFFF => self.update_chr_bank(value, 1),
                            0xE000..=0xFFFF => self.update_prg_bank(value),
                            _ => panic!("Invalid MMC1 address {:04X}={:02X}", address, value),
                        }

                        self.load_register = 0;
                        self.shift_writes = 0;
                    }
                }
            }
            _ => (), // TODO - Do writes to anywhere else do anything?
        }
    }
}

pub(crate) struct MMC1ChrChip {
    chr_data: ChrData,
    ppu_vram: [u8; 0x1000],
}

impl MMC1ChrChip {
    fn new(chr_rom: Option<Vec<u8>>) -> Self {
        match chr_rom {
            Some(rom) => MMC1ChrChip {
                chr_data: ChrData::Rom(rom),
                ppu_vram: [0; 0x1000],
            },
            None => MMC1ChrChip {
                chr_data: ChrData::Ram([0; 0x2000]),
                ppu_vram: [0; 0x1000],
            }
        }
    }
}

impl CartridgeAddressBus for MMC1ChrChip {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x1FFF => match &self.chr_data {
                ChrData::Rom(rom) => rom[address as usize],
                ChrData::Ram(ram) => ram[address as usize],
            },
            0x2000..=0x2FFF => self.ppu_vram[(address - 0x2000) as usize],
            0x3000..=0x3EFF => self.ppu_vram[(address - 0x3000) as usize],
            _ => todo!("Not yet mapped addresses in zero mapper {:04X}", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        info!("MMC1 write {:04X}={:02X}", address, value);
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
}

pub(crate) fn from_header(
    prg_rom: Vec<u8>,
    chr_rom: Option<Vec<u8>>,
    header: CartridgeHeader,
) -> (
    Box<dyn CartridgeAddressBus>,
    Box<dyn CartridgeAddressBus>,
    CartridgeHeader,
) {
    (
        Box::new(MMC1PrgChip::new(prg_rom)),
        Box::new(MMC1ChrChip::new(chr_rom)),
        header,
    )
}
