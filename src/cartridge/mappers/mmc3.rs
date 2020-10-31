use cartridge::mappers::ChrData;
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::{debug, error, info};

#[derive(Debug)]
enum PRGBankMode {
    /// 8000-9FFF swappable bank, C000-DFFF fixed to second last bank
    LowBankSwappable,
    /// C000-DFFF swappable bank, 8000-9FFF fixed to second last bank
    HighBankSwappable,
}

pub(crate) struct MMC3PrgChip {
    prg_rom: Vec<u8>,
    total_prg_banks: u8,
    prg_ram: Option<[u8; 0x2000]>,
    prg_ram_readonly: bool,
    prg_ram_disabled: bool,
    prg_banks: [u8; 4],
    prg_bank_offsets: [usize; 4],
    bank_mode: PRGBankMode,
    /// 0b000-0b111 -> The register to be written to on next write to BankData
    bank_select: u8,
}

impl MMC3PrgChip {
    fn new(prg_rom: Vec<u8>, total_prg_banks: u8, prg_ram: Option<[u8; 0x2000]>) -> Self {
        debug_assert!(prg_rom.len() >= 0x4000);

        MMC3PrgChip {
            prg_rom,
            total_prg_banks,
            prg_ram,
            prg_ram_readonly: false,
            prg_ram_disabled: false,
            prg_banks: [0, 1, total_prg_banks - 2, total_prg_banks - 1],
            prg_bank_offsets: [
                0x0000,
                0x2000,
                (total_prg_banks as usize - 2) * 0x2000,
                (total_prg_banks as usize - 1) * 0x2000,
            ],
            bank_mode: PRGBankMode::LowBankSwappable,
            bank_select: 0, // TODO - Does this initial value matter?
        }
    }

    fn update_bank_offsets(&mut self) {
        match self.bank_mode {
            PRGBankMode::LowBankSwappable => {
                self.prg_bank_offsets[0] = self.prg_banks[0] as usize * 0x2000;
                self.prg_bank_offsets[2] = self.prg_banks[2] as usize * 0x2000;
            }
            PRGBankMode::HighBankSwappable => {
                self.prg_bank_offsets[2] = self.prg_banks[0] as usize * 0x2000;
                self.prg_bank_offsets[0] = self.prg_banks[2] as usize * 0x2000;
            }
        };

        self.prg_bank_offsets[1] = self.prg_banks[1] as usize * 0x2000;

        info!(
            "MMC3 PRG bank offsets updated {:?} -> {:?}",
            self.prg_banks, self.prg_bank_offsets
        );
    }
}

impl CpuCartridgeAddressBus for MMC3PrgChip {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x6000..=0x7FFF => match self.prg_ram {
                Some(ram) => {
                    if self.prg_ram_disabled {
                        0x0 // TODO - Should be open bus
                    } else {
                        ram[(address - 0x6000) as usize]
                    }
                }
                None => 0x0,
            },
            // PRG Bank 0 - Switchable or fixed to second to last bank
            0x8000..=0x9FFF => {
                let adj_addr = address as usize - 0x8000;
                self.prg_rom[adj_addr + self.prg_bank_offsets[0] as usize]
            }
            // PRG Bank 1 - Switchable
            0xA000..=0xBFFF => {
                let adj_addr = address as usize - 0xA000;
                self.prg_rom[adj_addr + self.prg_bank_offsets[1] as usize]
            }
            // PRG Bank 2 - Switchable or fixed to second to last bank (swaps with bank 0)
            0xC000..=0xDFFF => {
                let adj_addr = address as usize - 0xC000;
                self.prg_rom[adj_addr + self.prg_bank_offsets[2] as usize]
            }
            // PRG Bank 3 - Fixed to last bank
            0xE000..=0xFFFF => {
                let adj_addr = address as usize - 0xE000;
                self.prg_rom[adj_addr + self.prg_bank_offsets[3] as usize]
            }
            _ => 0x0, // TODO - Would like to understand what reads of e.g. 0x4025 do here.
        }
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        info!("CPU write to MMC3 PRG bus {:04X}={:02X}", address, value);

        match address {
            0x6000..=0x7FFF => {
                if let Some(ram) = &mut self.prg_ram {
                    if !self.prg_ram_disabled && !self.prg_ram_readonly {
                        ram[(address - 0x6000) as usize] = value
                    }
                }
            }
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
                        0b110 => self.prg_banks[0] = value % self.total_prg_banks,
                        0b111 => self.prg_banks[1] = value % self.total_prg_banks,
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
    chr_data: ChrData,
    total_chr_banks: u8,
    ppu_vram: [u8; 0x1000],
    chr_banks: [u8; 8],
    chr_bank_offsets: [usize; 8],
    mirroring_mode: MirroringMode,
    bank_mode: CHRBankMode,
    /// 0b000-0b111 -> The register to be written to on next write to BankData
    bank_select: u8,
    /// Track the cycle on which we last noticed an A12 change to low
    /// It takes 6 cycles at low voltage before a high voltage causes a counter decrement
    /// This is set to 0 whenever we see A12 high, if it was >=6 then we trigger a count
    a12_cycles_at_last_low: Option<u32>,
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
    fn new(chr_data: ChrData, total_chr_banks: u8, mirroring_mode: MirroringMode) -> Self {
        MMC3ChrChip {
            chr_data,
            total_chr_banks,
            ppu_vram: [0; 0x1000],
            chr_banks: [0, 1, 2, 3, 4, 5, 6, 7],
            chr_bank_offsets: [0x0000, 0x0400, 0x0800, 0x0C00, 0x1000, 0x1400, 0x1800, 0x1C00],
            mirroring_mode,
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
                    self.chr_bank_offsets[i] = self.chr_banks[i] as usize * 0x400;
                }
            }
            CHRBankMode::HighBank2KB => {
                for i in 0..8 {
                    self.chr_bank_offsets[(i + 4) % 8] = self.chr_banks[i] as usize * 0x400;
                }
            }
        };

        info!(
            "MMC3 CHR bank offsets updated {:?} -> {:?}",
            self.chr_banks, self.chr_bank_offsets
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
    fn check_trigger_irq(&mut self) -> bool {
        let val = self.irq_triggered;

        self.irq_triggered = false;

        val
    }

    fn update_vram_address(&mut self, address: u16, ppu_cycles: u32) {
        let cycle_diff = match self.a12_cycles_at_last_low {
            None => None,
            Some(c) => Some(ppu_cycles - c),
        };

        info!(
            "MMC3 notified of PPU ADDR change {:04X} at cycle {}",
            address, ppu_cycles
        );

        self.a12_cycles_at_last_low = match (address & 0x1000 == 0x1000, cycle_diff) {
            (false, _) => Some(ppu_cycles),
            (true, Some(6..=u32::MAX)) => {
                self.clock_irq_counter();
                None
            }
            (true, _) => self.a12_cycles_at_last_low,
        };
    }

    fn read_byte(&mut self, address: u16, _: u32) -> u8 {
        match (address, &self.chr_data) {
            (0x0000..=0x03FF, ChrData::Ram(ram)) => ram[address as usize - 0x0000 + self.chr_bank_offsets[0]],
            (0x0400..=0x07FF, ChrData::Ram(ram)) => ram[address as usize - 0x0400 + self.chr_bank_offsets[1]],
            (0x0800..=0x0BFF, ChrData::Ram(ram)) => ram[address as usize - 0x0800 + self.chr_bank_offsets[2]],
            (0x0C00..=0x0FFF, ChrData::Ram(ram)) => ram[address as usize - 0x0C00 + self.chr_bank_offsets[3]],
            (0x1000..=0x13FF, ChrData::Ram(ram)) => ram[address as usize - 0x1000 + self.chr_bank_offsets[4]],
            (0x1400..=0x17FF, ChrData::Ram(ram)) => ram[address as usize - 0x1400 + self.chr_bank_offsets[5]],
            (0x1800..=0x1BFF, ChrData::Ram(ram)) => ram[address as usize - 0x1800 + self.chr_bank_offsets[6]],
            (0x1C00..=0x1FFF, ChrData::Ram(ram)) => ram[address as usize - 0x1C00 + self.chr_bank_offsets[7]],
            (0x0000..=0x03FF, ChrData::Rom(rom)) => rom[address as usize - 0x0000 + self.chr_bank_offsets[0]],
            (0x0400..=0x07FF, ChrData::Rom(rom)) => rom[address as usize - 0x0400 + self.chr_bank_offsets[1]],
            (0x0800..=0x0BFF, ChrData::Rom(rom)) => rom[address as usize - 0x0800 + self.chr_bank_offsets[2]],
            (0x0C00..=0x0FFF, ChrData::Rom(rom)) => rom[address as usize - 0x0C00 + self.chr_bank_offsets[3]],
            (0x1000..=0x13FF, ChrData::Rom(rom)) => rom[address as usize - 0x1000 + self.chr_bank_offsets[4]],
            (0x1400..=0x17FF, ChrData::Rom(rom)) => rom[address as usize - 0x1400 + self.chr_bank_offsets[5]],
            (0x1800..=0x1BFF, ChrData::Rom(rom)) => rom[address as usize - 0x1800 + self.chr_bank_offsets[6]],
            (0x1C00..=0x1FFF, ChrData::Rom(rom)) => rom[address as usize - 0x1C00 + self.chr_bank_offsets[7]],
            (0x2000..=0x3EFF, _) => {
                let mirrored_address = self.mirroring_mode.get_mirrored_address(address);
                debug!("Read {:04X} mirrored to {:04X}", address, mirrored_address);

                self.ppu_vram[mirrored_address as usize]
            }
            (0x3F00..=0x3FFF, _) => panic!("Shouldn't be reading from palette RAM through cartridge bus"),
            _ => panic!("Reading from {:04X} invalid for CHR address bus", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        debug!("MMC3 CHR write {:04X}={:02X}", address, value);
        match address {
            0x0000..=0x1FFF => {
                if let ChrData::Ram(ram) = &mut self.chr_data {
                    match address {
                        0x0000..=0x03FF => ram[address as usize - 0x0000 + self.chr_bank_offsets[0]] = value,
                        0x0400..=0x07FF => ram[address as usize - 0x0400 + self.chr_bank_offsets[1]] = value,
                        0x0800..=0x0BFF => ram[address as usize - 0x0800 + self.chr_bank_offsets[2]] = value,
                        0x0C00..=0x0FFF => ram[address as usize - 0x0C00 + self.chr_bank_offsets[3]] = value,
                        0x1000..=0x13FF => ram[address as usize - 0x1000 + self.chr_bank_offsets[4]] = value,
                        0x1400..=0x17FF => ram[address as usize - 0x1400 + self.chr_bank_offsets[5]] = value,
                        0x1800..=0x1BFF => ram[address as usize - 0x1800 + self.chr_bank_offsets[6]] = value,
                        0x1C00..=0x1FFF => ram[address as usize - 0x1C00 + self.chr_bank_offsets[7]] = value,
                        _ => panic!(),
                    }
                }
            }
            0x2000..=0x3EFF => {
                let mirrored_address = self.mirroring_mode.get_mirrored_address(address);

                self.ppu_vram[mirrored_address as usize] = value;
            }
            0x3F00..=0x3FFF => panic!("Shouldn't be writing to palette registers through the cartridge address bus"),
            _ => panic!("Write to {:04X} ({:02X}) invalid for CHR address bus", address, value),
        }
    }

    fn cpu_write_byte(&mut self, address: u16, value: u8, _: u32) {
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
                            self.chr_banks[0] = (value & 0b1111_1110) % self.total_chr_banks;
                            self.chr_banks[1] = self.chr_banks[0] + 1;
                        }
                        0b001 => {
                            self.chr_banks[2] = (value & 0b1111_1110) % self.total_chr_banks;
                            self.chr_banks[3] = self.chr_banks[2] + 1;
                        }
                        0b010..=0b101 => self.chr_banks[self.bank_select as usize + 2] = value % self.total_chr_banks,
                        _ => (), // Do nothing with PRG banks here
                    };

                    self.update_bank_offsets();
                }
                _ => panic!(),
            },
            // Mirroring & PRG RAM Protect registers - PRG RAM handled by PRG cartridge
            0xA000..=0xBFFF => {
                if address & 1 == 0 && self.mirroring_mode != MirroringMode::FourScreen {
                    self.mirroring_mode = if value & 1 == 0 {
                        MirroringMode::Vertical
                    } else {
                        MirroringMode::Horizontal
                    };

                    info!("MMC3 mirroring mode change {:?}", self.mirroring_mode);
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
        Box::new(MMC3PrgChip::new(
            prg_rom,
            header.prg_rom_16kb_units * 2,
            Some([0; 0x2000]),
        )),
        Box::new(match chr_rom {
            None => MMC3ChrChip::new(ChrData::Ram(Box::new([0; 0x2000])), 8, header.mirroring),
            Some(rom) => MMC3ChrChip::new(ChrData::Rom(rom), header.chr_rom_8kb_units * 4, header.mirroring),
        }),
        header,
    )
}
