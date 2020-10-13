use cartridge::mappers::ChrData;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::MirroringMode;
use cartridge::PpuCartridgeAddressBus;
use log::{debug, info};

#[derive(Debug, PartialEq)]
enum PRGBankMode {
    Switch32KB,
    FixFirst16KB,
    FixLast16KB,
}

#[derive(Debug)]
enum CHRBankMode {
    Switch8KB,
    Switch4KB,
}

struct LoadRegister {
    shift_writes: u8,
    value: u8,
    last_write_cycle: u32,
}

impl LoadRegister {
    fn new() -> Self {
        LoadRegister {
            last_write_cycle: 0,
            value: 0,
            shift_writes: 0,
        }
    }
}

pub(crate) struct MMC1PrgChip {
    prg_rom: Vec<u8>,
    prg_banks: u8,
    prg_ram: [u8; 0x2000],
    prg_ram_enabled: bool,
    prg_bank_mode: PRGBankMode,
    prg_bank: u8,
    prg_bank_offsets: [u32; 2],
    load_register: LoadRegister,
}

impl MMC1PrgChip {
    fn new(prg_rom: Vec<u8>, prg_banks: u8) -> Self {
        debug_assert!(prg_rom.len() >= 0x4000);

        let mut chip = MMC1PrgChip {
            prg_rom,
            prg_banks,
            prg_ram: [0; 0x2000],
            prg_ram_enabled: true,
            prg_bank_mode: PRGBankMode::FixLast16KB,
            prg_bank: 0,
            prg_bank_offsets: [0; 2],
            load_register: LoadRegister::new(),
        };

        chip.update_bank_offsets();

        chip
    }

    fn update_control_register(&mut self, value: u8) {
        self.prg_bank_mode = match (value >> 2) & 0b11 {
            0b00 | 0b01 => PRGBankMode::Switch32KB,
            0b10 => PRGBankMode::FixFirst16KB,
            0b11 => PRGBankMode::FixLast16KB,
            _ => panic!(),
        };

        debug!(
            "MMC1 Control register updated PRG bank mode : {:?}",
            self.prg_bank_mode
        );

        self.update_bank_offsets();
    }

    fn update_prg_bank(&mut self, value: u8) {
        self.prg_ram_enabled = value & 0b1_0000 == 0;

        self.prg_bank = match self.prg_bank_mode {
            PRGBankMode::Switch32KB => (value & 0b1110) >> 1,
            _ => value & 0b1111,
        } % self.prg_banks;

        info!(
            "PRG Bank updated to {:02X}/{:02X}",
            self.prg_bank, self.prg_banks
        );

        self.update_bank_offsets();
    }

    fn update_bank_offsets(&mut self) {
        match self.prg_bank_mode {
            PRGBankMode::FixFirst16KB => {
                self.prg_bank_offsets[0] = 0;
                self.prg_bank_offsets[1] = self.prg_bank as u32 * 0x4000;
            }
            PRGBankMode::FixLast16KB => {
                self.prg_bank_offsets[0] = self.prg_bank as u32 * 0x4000;
                self.prg_bank_offsets[1] = self.prg_rom.len() as u32 - 0x4000;
            }
            PRGBankMode::Switch32KB => {
                self.prg_bank_offsets[0] = self.prg_bank as u32 * 0x8000;
                self.prg_bank_offsets[1] = self.prg_bank_offsets[0] + 0x4000;
            }
        };

        info!(
            "Bank offsets updated: {:04X} {:04X}",
            self.prg_bank_offsets[0], self.prg_bank_offsets[1]
        );
    }
}

impl CpuCartridgeAddressBus for MMC1PrgChip {
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
            _ => todo!("Not yet mapped addresses in MMC1 {:04X}", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8, cycles: u32) {
        // Skip writes on consecutive cycles
        if cycles == self.load_register.last_write_cycle + 1 {
            return;
        }
        self.load_register.last_write_cycle = cycles;

        match address {
            0x6000..=0x7FFF => {
                if self.prg_ram_enabled {
                    // TODO - some variants of MMC1 always have RAM enabled
                    self.prg_ram[(address - 0x6000) as usize] = value;
                }
            }
            0x8000..=0xFFFF => {
                if value & 0b1000_0000 != 0 {
                    self.load_register.value = 0;
                    self.load_register.shift_writes = 0;
                    self.update_control_register(0x0C);
                } else {
                    self.load_register.value |= (value & 1) << self.load_register.shift_writes;
                    self.load_register.shift_writes += 1;

                    if self.load_register.shift_writes == 5 {
                        match address {
                            0x8000..=0x9FFF => {
                                self.update_control_register(self.load_register.value)
                            }
                            0xA000..=0xBFFF => (), // Rust ownership...this write is handled by the CHR bus //self.update_chr_bank(self.load_register, 0),
                            0xC000..=0xDFFF => (), // Rust ownership...this write is handled by the CHR bus self.update_chr_bank(self.load_register, 1),
                            0xE000..=0xFFFF => self.update_prg_bank(self.load_register.value),
                            _ => panic!("Invalid MMC1 address {:04X}={:02X}", address, value),
                        }

                        self.load_register.value = 0;
                        self.load_register.shift_writes = 0;
                    }
                }
            }
            _ => (), // TODO - Do writes to anywhere else do anything?
        }
    }
}

pub(crate) struct MMC1ChrChip {
    chr_data: ChrData,
    chr_banks: u8,
    ppu_vram: [u8; 0x1000],
    chr_bank: [u8; 2],
    chr_bank_offsets: [u16; 2],
    load_register: LoadRegister,
    mirroring_mode: MirroringMode,
    chr_bank_mode: CHRBankMode,
}

impl MMC1ChrChip {
    fn new(chr_rom: Option<Vec<u8>>, banks: u8) -> Self {
        match chr_rom {
            Some(rom) => MMC1ChrChip {
                chr_data: ChrData::Rom(rom),
                chr_banks: if banks == 0 { 1 } else { banks },
                ppu_vram: [0; 0x1000],
                chr_bank: [0; 2],
                chr_bank_offsets: [0; 2],
                load_register: LoadRegister::new(),
                mirroring_mode: MirroringMode::OneScreenLowerBank,
                chr_bank_mode: CHRBankMode::Switch8KB,
            },
            None => MMC1ChrChip {
                chr_data: ChrData::Ram([0; 0x2000]),
                chr_banks: if banks == 0 { 1 } else { banks },
                ppu_vram: [0; 0x1000],
                chr_bank: [0; 2],
                chr_bank_offsets: [0; 2],
                load_register: LoadRegister::new(),
                mirroring_mode: MirroringMode::OneScreenLowerBank,
                chr_bank_mode: CHRBankMode::Switch8KB,
            },
        }
    }

    fn update_control_register(&mut self, value: u8) {
        self.mirroring_mode = match value & 0b11 {
            0b00 => MirroringMode::OneScreenLowerBank,
            0b01 => MirroringMode::OneScreenUpperBank,
            0b10 => MirroringMode::Vertical,
            0b11 => MirroringMode::Horizontal,
            _ => panic!(),
        };

        self.chr_bank_mode = match (value >> 4) & 0b1 {
            0b0 => CHRBankMode::Switch8KB,
            0b1 => CHRBankMode::Switch4KB,
            _ => panic!(),
        };

        debug!(
            "MMC1 Control register updated mirroring mode: {:?}, chr bank mode {:?}",
            self.mirroring_mode, self.chr_bank_mode
        );

        self.update_bank_offsets();
    }

    fn update_chr_bank(&mut self, value: u8, bank: usize) {
        debug_assert!(bank <= 1);

        self.chr_bank[bank] = match self.chr_bank_mode {
            CHRBankMode::Switch4KB => (value & 0b1_1111) % self.chr_banks,
            CHRBankMode::Switch8KB => (value & 0b1_1110) % self.chr_banks,
        };

        debug!(
            "CHR banks updated to {:02X}, {:02X}",
            self.chr_bank[0], self.chr_bank[1]
        );

        self.update_bank_offsets();
    }

    fn update_bank_offsets(&mut self) {
        match self.chr_bank_mode {
            CHRBankMode::Switch4KB => {
                self.chr_bank_offsets[0] = self.chr_bank[0] as u16 * 0x1000;
                self.chr_bank_offsets[1] = self.chr_bank[1] as u16 * 0x1000;
            }
            CHRBankMode::Switch8KB => {
                self.chr_bank_offsets[0] = (self.chr_bank[0] >> 1) as u16 * 0x1000;
                self.chr_bank_offsets[1] = self.chr_bank_offsets[0] + 0x1000;
            }
        }
    }
}

impl PpuCartridgeAddressBus for MMC1ChrChip {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x1FFF => match &self.chr_data {
                ChrData::Rom(rom) => {
                    let adjusted_address = if address & 0x1000 == 0 {
                        address + self.chr_bank_offsets[0]
                    } else {
                        address + self.chr_bank_offsets[1]
                    };

                    rom[adjusted_address as usize]
                }
                ChrData::Ram(ram) => ram[address as usize],
            },
            0x2000..=0x3EFF => {
                let mirrored_address = self.mirroring_mode.get_mirrored_address(address);
                debug!("Read {:04X} mirrored to {:04X}", address, mirrored_address);

                self.ppu_vram[mirrored_address as usize]
            }
            0x3F00..=0x3FFF => {
                panic!("Shouldn't be reading from palette RAM through cartridge bus")
            }
            _ => panic!("Reading from {:04X} invalid for CHR address bus", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        info!("MMC1 CHR write {:04X}={:02X}", address, value);
        match address {
            0x0000..=0x1FFF => match &mut self.chr_data {
                ChrData::Rom(_) => (),
                ChrData::Ram(ram) => ram[address as usize] = value,
            },
            0x2000..=0x3EFF => {
                let mirrored_address = self.mirroring_mode.get_mirrored_address(address);

                self.ppu_vram[mirrored_address as usize] = value;
            }
            0x3F00..=0x3FFF => panic!(
                "Shouldn't be writing to palette registers through the cartridge address bus"
            ),
            _ => panic!(
                "Write to {:04X} ({:02X}) invalid for CHR address bus",
                address, value
            ),
        }
    }

    fn cpu_write_byte(&mut self, address: u16, value: u8, cycles: u32) {
        // Skip writes on consecutive cycles
        if cycles == self.load_register.last_write_cycle + 1 {
            return;
        }
        self.load_register.last_write_cycle = cycles;

        match address {
            0x8000..=0xFFFF => {
                if value & 0b1000_0000 != 0 {
                    self.load_register.value = 0;
                    self.load_register.shift_writes = 0;
                    self.update_control_register(0x0C);
                } else {
                    self.load_register.value |= (value & 1) << self.load_register.shift_writes;
                    self.load_register.shift_writes += 1;

                    if self.load_register.shift_writes == 5 {
                        match address {
                            0x8000..=0x9FFF => {
                                self.update_control_register(self.load_register.value)
                            }
                            0xA000..=0xBFFF => self.update_chr_bank(self.load_register.value, 0),
                            0xC000..=0xDFFF => self.update_chr_bank(self.load_register.value, 1),
                            0xE000..=0xFFFF => (), // Rust ownership...this write is handled by the PRG bus
                            _ => panic!("Invalid MMC1 address {:04X}={:02X}", address, value),
                        }

                        self.load_register.value = 0;
                        self.load_register.shift_writes = 0;
                    }
                }
            }
            _ => (), // TODO - Do writes to anywhere else do anything?
        }
    }
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
    (
        Box::new(MMC1PrgChip::new(prg_rom, header.prg_rom_16kb_units)),
        Box::new(MMC1ChrChip::new(chr_rom, header.chr_rom_8kb_units)),
        header,
    )
}

#[cfg(test)]
mod mmc1_tests {
    use super::{MMC1PrgChip, PRGBankMode};
    use cartridge::CpuCartridgeAddressBus;

    #[test]
    fn test_change_bank() {
        let mut mmc1 = MMC1PrgChip::new(vec![0; 0x4000 * 16], 16);
        mmc1.write_byte(0xE000, 0b0001, 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        assert_eq!(mmc1.prg_bank, 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        assert_eq!(mmc1.prg_bank, 1);
    }

    #[test]
    fn test_change_bank_needs_wrap() {
        let mut mmc1 = MMC1PrgChip::new(vec![0; 0x4000 * 2], 2);
        mmc1.write_byte(0xE000, 0b0011, 0);
        mmc1.write_byte(0xE000, 0b0001, 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        assert_eq!(mmc1.prg_bank, 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        assert_eq!(mmc1.prg_bank, 1);
    }

    #[test]
    fn test_ignore_sequential_writes() {
        let mut mmc1 = MMC1PrgChip::new(vec![0; 0x4000 * 16], 16);
        mmc1.write_byte(0xE000, 0b0001, 0);
        mmc1.write_byte(0xE000, 0b0000, 2);
        mmc1.write_byte(0xE000, 0b0000, 4);
        mmc1.write_byte(0xE000, 0b0000, 6);
        assert_eq!(mmc1.prg_bank, 0);
        mmc1.write_byte(0xE000, 0b0000, 7); // This write is ignored because it happens on the next cycle
        assert_eq!(mmc1.prg_bank, 0);
        mmc1.write_byte(0xE000, 0b0000, 9);
        assert_eq!(mmc1.prg_bank, 1);
    }

    #[test]
    fn test_set_control_register() {
        let value = 0b1111;
        let mut mmc1 = MMC1PrgChip::new(vec![0; 0x4000 * 16], 16);
        mmc1.write_byte(0x8000, 0, 0);
        mmc1.write_byte(0x8000, 0, 2);
        mmc1.write_byte(0x8000, 0, 4);
        mmc1.write_byte(0x8000, 0, 6);
        mmc1.write_byte(0x8000, 0, 8);
        assert_eq!(mmc1.prg_bank_mode, PRGBankMode::Switch32KB);
        mmc1.write_byte(0x8000, value, 0);
        mmc1.write_byte(0x8000, value >> 1, 2);
        mmc1.write_byte(0x8000, value >> 2, 4);
        mmc1.write_byte(0x8000, value >> 3, 6);
        mmc1.write_byte(0x8000, value >> 4, 8);
        assert_eq!(mmc1.prg_bank_mode, PRGBankMode::FixLast16KB);
    }
}
