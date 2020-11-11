use cartridge::mappers::{ChrBaseData, ChrData, PrgBaseData};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use cpu::CpuCycle;
use log::{debug, info};
use ppu::PpuCycle;

#[derive(Debug)]
enum PRGBankMode {
    /// 8000-9FFF swappable bank, C000-DFFF fixed to second last bank
    LowBankSwappable,
    /// C000-DFFF swappable bank, 8000-9FFF fixed to second last bank
    HighBankSwappable,
}

pub(crate) struct MMC3PrgChip {
    base: PrgBaseData,
    prg_ram_readonly: bool,
    prg_ram_disabled: bool,
    bank_mode: PRGBankMode,
    /// 0b000-0b111 -> The register to be written to on next write to BankData
    bank_select: u8,
}

impl MMC3PrgChip {
    fn new(prg_rom: Vec<u8>, total_banks: usize) -> Self {
        MMC3PrgChip {
            base: PrgBaseData::new(
                prg_rom,
                Some([0; 0x2000]),
                total_banks,
                0x2000,
                vec![0, 1, total_banks - 2, total_banks - 1],
                vec![0, 0x2000, (total_banks - 2) * 0x2000, (total_banks - 1) * 0x2000],
            ),
            prg_ram_readonly: false,
            prg_ram_disabled: false,
            bank_mode: PRGBankMode::LowBankSwappable,
            bank_select: 0, // TODO - Does this initial value matter?
        }
    }

    fn update_bank_offsets(&mut self) {
        match self.bank_mode {
            PRGBankMode::LowBankSwappable => {
                self.base.bank_offsets[0] = self.base.banks[0] * 0x2000;
                self.base.bank_offsets[2] = self.base.banks[2] * 0x2000;
            }
            PRGBankMode::HighBankSwappable => {
                self.base.bank_offsets[2] = self.base.banks[0] * 0x2000;
                self.base.bank_offsets[0] = self.base.banks[2] * 0x2000;
            }
        };

        self.base.bank_offsets[1] = self.base.banks[1] * 0x2000;

        info!(
            "MMC3 PRG bank offsets updated {:?} -> {:?}",
            self.base.banks, self.base.bank_offsets
        );
    }
}

impl CpuCartridgeAddressBus for MMC3PrgChip {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x6000..=0x7FFF => match &self.base.prg_ram {
                Some(ram) => {
                    if self.prg_ram_disabled {
                        0x0 // TODO - Should be open bus
                    } else {
                        ram[(address - 0x6000) as usize]
                    }
                }
                None => 0x0,
            },
            0x8000..=0xFFFF => self.base.read_byte(address),
            _ => 0x0,
        }
    }

    fn write_byte(&mut self, address: u16, value: u8, _: PpuCycle) {
        info!("CPU write to MMC3 PRG bus {:04X}={:02X}", address, value);

        match address {
            0x6000..=0x7FFF => match &mut self.base.prg_ram {
                Some(ram) => {
                    if !self.prg_ram_disabled && !self.prg_ram_readonly {
                        ram[(address - 0x6000) as usize] = value
                    }
                }
                None => {}
            },
            // Bank select and Bank data registers
            0x8000..=0x9FFF => match address & 1 {
                // Even addresses => Bank select register
                0 => {
                    self.bank_select = value & 0b0000_0111;
                    self.bank_mode = if value & 0b0100_0000 == 0 {
                        PRGBankMode::LowBankSwappable
                    } else {
                        PRGBankMode::HighBankSwappable
                    };
                }
                // Odd addresses => Bank data register
                1 => {
                    match self.bank_select {
                        0b110 => self.base.banks[0] = value as usize % self.base.total_banks,
                        0b111 => self.base.banks[1] = value as usize % self.base.total_banks,
                        _ => (), // Do nothing with CHR registers here
                    };

                    self.update_bank_offsets();
                }
                _ => panic!(),
            },
            // Mirroring & PRG RAM Protect registers
            0xA000..=0xBFFF => match address & 1 {
                // Even addresses - Nametable mirroring handled by CHR bus
                0 => {}
                1 => {
                    // Odd addresses - RAM disable/enable/readonly
                    self.prg_ram_disabled = value & 0b1000_0000 == 0b1000_0000;
                    self.prg_ram_readonly = value & 0b0100_0000 == 0b0100_0000;
                }
                _ => panic!(),
            },
            // IRQ registers - Handled by CHR bus
            0xC000..=0xFFFF => {}
            _ => (),
        }
    }
}

#[derive(Debug)]
enum CHRBankMode {
    /// Two 2KB banks at 0000-0FFF and four 1KB banks at 1000-1FFF  
    LowBank2KB,
    /// Two 2KB banks at 1000-1FFF and four 1KB banks at 0000-0FFF
    HighBank2KB,
}

pub(crate) struct MMC3ChrChip {
    base: ChrBaseData,
    bank_mode: CHRBankMode,
    /// 0b000-0b111 -> The register to be written to on next write to BankData
    bank_select: u8,
    /// Track the cycle on which we last noticed an A12 change to low
    /// It takes 6 cycles at low voltage before a high voltage causes a counter decrement
    /// This is set to 0 whenever we see A12 high, if it was >=6 then we trigger a count
    a12_cycles_at_last_low: Option<PpuCycle>,
    /// IRQ register holding the value to load into the counter on the next reload
    irq_latch: u8,
    /// Set on reload to note that on the next rising edge the counter will get reloaded with the IRQ latch
    reload_irq_next_rising_edge: bool,
    /// Current IRQ counter value
    irq_counter: u8,
    /// Set via C000/C001 register pair to determine whether IRQ counter getting to zero triggers an IRQ or not
    irq_enabled: bool,
    /// Internal bookkeeping to tell the CPU whether it needs to process an IRQ
    irq_triggered: bool,
}

impl MMC3ChrChip {
    fn new(chr_data: ChrData, mirroring_mode: MirroringMode) -> Self {
        MMC3ChrChip {
            base: ChrBaseData::new(
                mirroring_mode,
                chr_data,
                0x400,
                vec![0, 1, 2, 3, 4, 5, 6, 7],
                vec![0x0000, 0x0400, 0x0800, 0x0C00, 0x1000, 0x1400, 0x1800, 0x1C00],
            ),
            bank_mode: CHRBankMode::LowBank2KB,
            bank_select: 0,
            a12_cycles_at_last_low: None,
            irq_latch: 0,
            reload_irq_next_rising_edge: false,
            irq_counter: 0,
            irq_enabled: false,
            irq_triggered: false,
        }
    }

    fn update_bank_offsets(&mut self) {
        match self.bank_mode {
            CHRBankMode::LowBank2KB => {
                for i in 0..8 {
                    self.base.bank_offsets[i] = self.base.banks[i] * 0x400;
                }
            }
            CHRBankMode::HighBank2KB => {
                for i in 0..8 {
                    self.base.bank_offsets[(i + 4) % 8] = self.base.banks[i] * 0x400;
                }
            }
        };

        info!(
            "MMC3 CHR bank offsets updated {:?} -> {:?}",
            self.base.banks, self.base.bank_offsets
        );
    }

    fn clock_irq_counter(&mut self) {
        info!("Clocking IRQ counter {:02X}", self.irq_counter);
        if self.reload_irq_next_rising_edge || self.irq_counter == 0 {
            info!(
                "MMC3 - Reloading IRQ counter (current {:02X}) {:02X}",
                self.irq_counter, self.irq_latch
            );
            self.irq_counter = self.irq_latch;
            self.reload_irq_next_rising_edge = false;
        } else {
            self.irq_counter -= 1;
        }

        if self.irq_counter == 0 && self.irq_enabled {
            info!("Triggering MMC3 IRQ by counter hitting 0");
            self.irq_triggered = true;
        }
    }
}

impl PpuCartridgeAddressBus for MMC3ChrChip {
    fn check_trigger_irq(&mut self, clear: bool) -> bool {
        let val = self.irq_triggered;

        if clear {
            self.irq_triggered = false;
        }

        val
    }

    fn update_vram_address(&mut self, address: u16, cycles: PpuCycle) {
        if address < 0x2000 {
            let cycle_diff = match self.a12_cycles_at_last_low {
                None => None,
                Some(c) => Some(cycles - c),
            };

            info!("MMC3 notified of PPU ADDR change {:04X} at cycle {}", address, cycles);

            self.a12_cycles_at_last_low = match (address & 0x1000 == 0x1000, cycle_diff) {
                (false, _) => Some(cycles),
                (true, Some(6..=PpuCycle::MAX)) => {
                    self.clock_irq_counter();
                    None
                }
                (true, _) => self.a12_cycles_at_last_low,
            };
        }
    }

    fn read_byte(&mut self, address: u16, _: PpuCycle) -> u8 {
        self.base.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8, _: PpuCycle) {
        self.base.write_byte(address, value);
    }

    fn cpu_write_byte(&mut self, address: u16, value: u8, _: CpuCycle) {
        debug!("CPU write to MMC3 CHR bus {:04X}={:02X}", address, value);

        match address {
            // Bank select and Bank data registers
            0x8000..=0x9FFF => match address & 1 {
                // Even addresses => Bank select register
                0 => {
                    self.bank_select = value & 0b0000_0111;
                    self.bank_mode = if value & 0b1000_0000 == 0 {
                        CHRBankMode::LowBank2KB
                    } else {
                        CHRBankMode::HighBank2KB
                    };
                }
                // Odd addresses => Bank data register
                1 => {
                    match self.bank_select {
                        0b000 => {
                            self.base.banks[0] = (value as usize & 0b1111_1110) % self.base.total_banks;
                            self.base.banks[1] = self.base.banks[0] + 1;
                        }
                        0b001 => {
                            self.base.banks[2] = (value as usize & 0b1111_1110) % self.base.total_banks;
                            self.base.banks[3] = self.base.banks[2] + 1;
                        }
                        0b010..=0b101 => {
                            self.base.banks[self.bank_select as usize + 2] = value as usize % self.base.total_banks
                        }
                        _ => (), // Do nothing with PRG banks here
                    };

                    self.update_bank_offsets();
                }
                _ => panic!(),
            },
            // Mirroring & PRG RAM Protect registers - PRG RAM handled by PRG cartridge
            0xA000..=0xBFFF => {
                if address & 1 == 0 && self.base.mirroring_mode != MirroringMode::FourScreen {
                    self.base.mirroring_mode = if value & 1 == 0 {
                        MirroringMode::Vertical
                    } else {
                        MirroringMode::Horizontal
                    };

                    info!("MMC3 mirroring mode change {:?}", self.base.mirroring_mode);
                }
            }
            // IRQ Latch & IRQ Reload registers
            0xC000..=0xDFFF => {
                if address & 1 == 0 {
                    self.irq_latch = value;
                    info!("Setting IRQ latch value to {:02X}", value);
                } else {
                    self.irq_counter = 0;
                    self.irq_triggered = false;
                    self.reload_irq_next_rising_edge = true;
                    info!("Triggering manual reload of IRQ counter");
                }
            }
            // IRQ Disable/Enable registers
            0xE000..=0xFFFF => match address & 1 {
                0 => {
                    self.irq_enabled = false;
                    self.irq_triggered = false;
                }
                1 => self.irq_enabled = true,
                _ => panic!(),
            },
            _ => (),
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
        Box::new(MMC3PrgChip::new(prg_rom, header.prg_rom_16kb_units as usize * 2)),
        Box::new(match chr_rom {
            None => MMC3ChrChip::new(ChrData::Ram(Box::new([0; 0x2000])), header.mirroring),
            Some(rom) => MMC3ChrChip::new(ChrData::Rom(rom), header.mirroring),
        }),
        header,
    )
}
