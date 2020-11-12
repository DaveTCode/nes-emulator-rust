use cartridge::mappers::{ChrBaseData, ChrData, PrgBaseData};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use cpu::CpuCycle;
use log::{debug, info};
use ppu::PpuCycle;

#[derive(Debug, PartialEq)]
enum PRGBankMode {
    Switch32KB,
    FixFirst16KB,
    FixLast16KB,
}

#[derive(Debug, PartialEq)]
enum CHRBankMode {
    Switch8KB,
    Switch4KB,
}

#[derive(Debug, PartialEq)]
enum MMC1Variant {
    MMC1,
    /// MMC1A allocated to mapper 155 is identical to MMC1 except that RAM is _always_ enabled
    MMC1A,
}

struct LoadRegister {
    shift_writes: u8,
    value: u8,
    last_write_cycle: PpuCycle,
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
    base: PrgBaseData,
    prg_ram_enabled: bool,
    prg_bank_mode: PRGBankMode,
    load_register: LoadRegister,
    variant: MMC1Variant,
}

impl MMC1PrgChip {
    fn new(prg_rom: Vec<u8>, total_banks: usize, variant: MMC1Variant) -> Self {
        debug_assert!(prg_rom.len() >= 0x4000);

        let mut chip = MMC1PrgChip {
            base: PrgBaseData::new(
                prg_rom,
                Some([0; 0x2000]), // TODO - I think this should be optional
                total_banks,
                0x4000,
                vec![0, total_banks - 1],
                vec![0, (total_banks - 1) * 0x4000],
            ),
            prg_ram_enabled: true,
            prg_bank_mode: PRGBankMode::FixLast16KB,
            load_register: LoadRegister::new(),
            variant,
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

        debug!("MMC1 Control register updated PRG bank mode : {:?}", self.prg_bank_mode);

        self.update_bank_offsets();
    }

    fn update_prg_bank(&mut self, value: u8) {
        self.prg_ram_enabled = value & 0b1_0000 == 0;

        self.base.banks[0] = match self.prg_bank_mode {
            PRGBankMode::Switch32KB => (value as usize & 0b1110) >> 1,
            _ => value as usize & 0b1111,
        } % self.base.total_banks;

        info!("PRG Banks updated to {:?}", self.base.banks);

        self.update_bank_offsets();
    }

    fn update_bank_offsets(&mut self) {
        match self.prg_bank_mode {
            PRGBankMode::FixFirst16KB => {
                self.base.bank_offsets[0] = 0;
                self.base.bank_offsets[1] = self.base.banks[0] * 0x4000;
            }
            PRGBankMode::FixLast16KB => {
                self.base.bank_offsets[0] = self.base.banks[0] * 0x4000;
                self.base.bank_offsets[1] = self.base.prg_rom.len() as usize - 0x4000;
            }
            PRGBankMode::Switch32KB => {
                self.base.bank_offsets[0] = self.base.banks[0] as usize * 0x8000;
                self.base.bank_offsets[1] = self.base.bank_offsets[0] + 0x4000;
            }
        };

        info!("Bank offsets updated: {:?}", self.base.bank_offsets);
    }
}

impl CpuCartridgeAddressBus for MMC1PrgChip {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x6000..=0x7FFF => match self.base.prg_ram {
                Some(ram) => {
                    if self.prg_ram_enabled || self.variant == MMC1Variant::MMC1A {
                        ram[(address - 0x6000) as usize]
                    } else {
                        0x0
                    }
                }
                None => 0x0,
            },
            0x8000..=0xBFFF => {
                let adj_addr = address as usize - 0x8000;

                self.base.prg_rom[adj_addr + self.base.bank_offsets[0] as usize]
            }
            0xC000..=0xFFFF => {
                let adj_addr = address as usize - 0xC000;

                self.base.prg_rom[adj_addr + self.base.bank_offsets[1] as usize]
            }
            _ => 0x0,
        }
    }

    fn write_byte(&mut self, address: u16, value: u8, cycles: PpuCycle) {
        // Skip writes on consecutive cycles
        if cycles == self.load_register.last_write_cycle + 1 {
            return;
        }
        self.load_register.last_write_cycle = cycles;

        match address {
            0x6000..=0x7FFF => match &mut self.base.prg_ram {
                Some(ram) => {
                    if self.prg_ram_enabled || self.variant == MMC1Variant::MMC1A {
                        ram[(address - 0x6000) as usize] = value;
                    }
                }
                None => {}
            },
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
                            0x8000..=0x9FFF => self.update_control_register(self.load_register.value),
                            0xA000..=0xBFFF => (),
                            0xC000..=0xDFFF => (),
                            0xE000..=0xFFFF => self.update_prg_bank(self.load_register.value),
                            _ => panic!("Invalid MMC1 address {:04X}={:02X}", address, value),
                        }

                        self.load_register.value = 0;
                        self.load_register.shift_writes = 0;
                    }
                }
            }
            _ => (),
        }
    }
}

pub(crate) struct MMC1ChrChip {
    base: ChrBaseData,
    load_register: LoadRegister,
    chr_bank_mode: CHRBankMode,
}

impl MMC1ChrChip {
    fn new(chr_data: ChrData) -> Self {
        MMC1ChrChip {
            base: ChrBaseData::new(
                MirroringMode::OneScreenLowerBank,
                chr_data,
                0x1000,
                vec![0, 1],
                vec![0, 0x1000],
            ),
            load_register: LoadRegister::new(),
            chr_bank_mode: CHRBankMode::Switch4KB,
        }
    }

    fn update_control_register(&mut self, value: u8) {
        self.base.mirroring_mode = match value & 0b11 {
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

        info!(
            "MMC1 Control register updated mirroring mode: {:?}, chr bank mode {:?}",
            self.base.mirroring_mode, self.chr_bank_mode
        );

        self.update_bank_offsets();
    }

    fn update_chr_bank(&mut self, value: u8, bank: usize) {
        debug_assert!(bank <= 1);

        self.base.banks[bank] = match self.chr_bank_mode {
            CHRBankMode::Switch4KB => (value as usize & 0b1_1111) % self.base.total_banks,
            CHRBankMode::Switch8KB => (value as usize & 0b1_1110) % self.base.total_banks,
        };

        self.update_bank_offsets();

        info!(
            "CHR banks updated to {:?}, offsets to {:?} - Mode {:?} from value {:02X} on bank {:02X}",
            self.base.banks, self.base.bank_offsets, self.chr_bank_mode, value, bank
        );
    }

    fn update_bank_offsets(&mut self) {
        match self.chr_bank_mode {
            CHRBankMode::Switch4KB => {
                self.base.bank_offsets[0] = self.base.banks[0] as usize * 0x1000;
                self.base.bank_offsets[1] = self.base.banks[1] as usize * 0x1000;
            }
            CHRBankMode::Switch8KB => {
                self.base.bank_offsets[0] = self.base.banks[0] as usize * 0x1000;
                self.base.bank_offsets[1] = self.base.bank_offsets[0] + 0x1000;
            }
        }
    }
}

impl PpuCartridgeAddressBus for MMC1ChrChip {
    fn check_trigger_irq(&mut self, _: bool) -> bool {
        false
    }

    fn update_vram_address(&mut self, _: u16, _: PpuCycle) {}

    fn read_byte(&mut self, address: u16, _: PpuCycle) -> u8 {
        self.base.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8, _: PpuCycle) {
        self.base.write_byte(address, value);
    }

    fn cpu_write_byte(&mut self, address: u16, value: u8, cycles: CpuCycle) {
        debug!(
            "CPU write to MMC1 CHR bus {:04X}={:02X} at {:} cycles",
            address, value, cycles
        );
        // Skip writes on consecutive cycles
        if cycles == self.load_register.last_write_cycle + 1 {
            return;
        }
        self.load_register.last_write_cycle = cycles;

        if let 0x8000..=0xFFFF = address {
            if value & 0b1000_0000 != 0 {
                self.load_register.value = 0;
                self.load_register.shift_writes = 0;
                self.update_control_register(0x0C);
            } else {
                self.load_register.value |= (value & 1) << self.load_register.shift_writes;
                self.load_register.shift_writes += 1;

                if self.load_register.shift_writes == 5 {
                    match address {
                        0x8000..=0x9FFF => self.update_control_register(self.load_register.value),
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
        Box::new(MMC1PrgChip::new(
            prg_rom,
            header.prg_rom_16kb_units as usize,
            match header.mapper {
                1 => MMC1Variant::MMC1,
                155 => MMC1Variant::MMC1A,
                _ => panic!("Mapper {} isn't mapped to MMC1", header.mapper),
            },
        )),
        Box::new(MMC1ChrChip::new(ChrData::from(chr_rom))),
        header,
    )
}

#[cfg(test)]
mod mmc1_tests {
    use super::{MMC1PrgChip, PRGBankMode};
    use cartridge::mappers::mmc1::MMC1Variant;
    use cartridge::CpuCartridgeAddressBus;

    #[test]
    fn test_change_bank() {
        let mut mmc1 = MMC1PrgChip::new(vec![0; 0x4000 * 16], 16, MMC1Variant::MMC1);
        mmc1.write_byte(0xE000, 0b0001, 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        assert_eq!(mmc1.base.banks[0], 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        assert_eq!(mmc1.base.banks[0], 1);
    }

    #[test]
    fn test_change_bank_needs_wrap() {
        let mut mmc1 = MMC1PrgChip::new(vec![0; 0x4000 * 2], 2, MMC1Variant::MMC1);
        mmc1.write_byte(0xE000, 0b0011, 0);
        mmc1.write_byte(0xE000, 0b0001, 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        assert_eq!(mmc1.base.banks[0], 0);
        mmc1.write_byte(0xE000, 0b0000, 0);
        assert_eq!(mmc1.base.banks[0], 1);
    }

    #[test]
    fn test_ignore_sequential_writes() {
        let mut mmc1 = MMC1PrgChip::new(vec![0; 0x4000 * 16], 16, MMC1Variant::MMC1);
        mmc1.write_byte(0xE000, 0b0001, 0);
        mmc1.write_byte(0xE000, 0b0000, 2);
        mmc1.write_byte(0xE000, 0b0000, 4);
        mmc1.write_byte(0xE000, 0b0000, 6);
        assert_eq!(mmc1.base.banks[0], 0);
        mmc1.write_byte(0xE000, 0b0000, 7); // This write is ignored because it happens on the next cycle
        assert_eq!(mmc1.base.banks[0], 0);
        mmc1.write_byte(0xE000, 0b0000, 9);
        assert_eq!(mmc1.base.banks[0], 1);
    }

    #[test]
    fn test_set_control_register() {
        let value = 0b1111;
        let mut mmc1 = MMC1PrgChip::new(vec![0; 0x4000 * 16], 16, MMC1Variant::MMC1);
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
