use cartridge::mappers::{ChrBaseData, ChrData, PrgBaseData};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use cpu::CpuCycle;
use log::{debug, info};
use ppu::PpuCycle;

struct Mmc2PrgChip {
    base: PrgBaseData,
}

impl Mmc2PrgChip {
    fn new(prg_rom: Vec<u8>, total_banks: usize) -> Self {
        debug_assert!(total_banks >= 4);

        Mmc2PrgChip {
            base: PrgBaseData {
                prg_rom,
                prg_ram: None,
                total_banks,
                bank_size: 0x2000,
                banks: vec![0, total_banks - 3, total_banks - 2, total_banks - 1],
                bank_offsets: vec![
                    0,
                    (total_banks - 3) * 0x2000,
                    (total_banks - 2) * 0x2000,
                    (total_banks - 1) * 0x2000,
                ],
            },
        }
    }
}

impl CpuCartridgeAddressBus for Mmc2PrgChip {
    fn read_byte(&self, address: u16) -> u8 {
        self.base.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        self.base.write_byte(address, value);

        // MMC2 has four banks, switched by 0xA000-0xFFFF where only the first is switchable
        // and the remainder are fixed to the last 3 available banks
        if let 0xA000..=0xAFFF = address {
            self.base.banks[0] = (value as usize & 0b1111) % self.base.total_banks;
            self.base.bank_offsets[0] = self.base.banks[0] as usize * 0x2000;

            info!(
                "MMC2 PRG Bank switch {:?} -> {:?}",
                self.base.banks, self.base.bank_offsets
            );
        }
    }
}

pub(crate) struct Mmc2Mmc4ChrChip {
    base: ChrBaseData,
    chr_banks: [[usize; 2]; 2],
    chr_bank_offsets: [[usize; 2]; 2],
    latches: [usize; 2],
    // TODO - Depending on whether there's other mappers with the same latching trick this might not be adequate
    is_mmc_4: bool,
}

impl Mmc2Mmc4ChrChip {
    pub(super) fn new(chr_data: ChrData, mirroring_mode: MirroringMode, is_mmc_4: bool) -> Self {
        Mmc2Mmc4ChrChip {
            base: ChrBaseData::new(mirroring_mode, chr_data, 0x1000, vec![0, 1], vec![0, 0x1000]),
            chr_banks: [[0; 2]; 2],
            chr_bank_offsets: [[0, 0x1000]; 2],
            latches: [0; 2],
            is_mmc_4,
        }
    }
}

impl PpuCartridgeAddressBus for Mmc2Mmc4ChrChip {
    fn check_trigger_irq(&mut self, _: bool) -> bool {
        false
    }

    fn update_vram_address(&mut self, _: u16, _: PpuCycle) {}

    fn read_byte(&mut self, address: u16, _: PpuCycle) -> u8 {
        let value = self.base.read_byte(address);

        // Set latches based on certain reads - I _think_ that this is done on
        // read not on latched address but not 100% sure
        if let Some((bank, latch, latch_value)) = match (address, self.is_mmc_4) {
            (0x0FD8, false) | (0x0FD8..=0x0FDF, true) => Some((0, 0, 0)),
            (0x0FE8, false) | (0x0FE8..=0x0FEF, true) => Some((0, 0, 1)),
            (0x1FD8..=0x1FDF, _) => Some((1, 1, 0)),
            (0x1FE8..=0x1FEF, _) => Some((1, 1, 1)),
            _ => None,
        } {
            self.base.banks[bank] = self.chr_banks[latch_value][bank];
            self.base.bank_offsets[bank] = self.chr_bank_offsets[latch_value][bank];
            self.latches[latch] = latch_value;
            debug!(
                "MMC2 bank switch caused by PPU read {:?} {:?} {:?}",
                self.latches, self.base.banks, self.base.bank_offsets
            );
        }

        value
    }

    fn write_byte(&mut self, address: u16, value: u8, _: PpuCycle) {
        self.base.write_byte(address, value);
    }

    fn cpu_write_byte(&mut self, address: u16, value: u8, cycles: CpuCycle) {
        debug!(
            "CPU write to CHR bus {:04X}={:02X} at {:} cycles",
            address, value, cycles
        );

        // MMC2 has two 4KB CHR ROM banks out of a 128KB capacity and switchable mirroring.
        // The banks themselves are switched based on a latch which is set on vram address
        if let Some((latch, latch_value, bank)) = match address {
            0xB000..=0xBFFF => Some((0, 0, 0)),
            0xC000..=0xCFFF => Some((0, 1, 0)),
            0xD000..=0xDFFF => Some((1, 0, 1)),
            0xE000..=0xEFFF => Some((1, 1, 1)),
            0xF000..=0xFFFF => {
                self.base.mirroring_mode = if value & 0b1 == 0b1 {
                    MirroringMode::Horizontal
                } else {
                    MirroringMode::Vertical
                };

                info!("Changing mirroring MMC2/MMC4 {:?}", self.base.mirroring_mode);

                None
            }
            _ => None,
        } {
            self.chr_banks[latch_value][bank] = (value as usize & 0b1_1111) % self.base.total_banks;
            self.chr_bank_offsets[latch_value][bank] = self.chr_banks[latch_value][bank] as usize * 0x1000;

            if latch_value == self.latches[latch] {
                self.base.banks[bank] = self.chr_banks[latch_value][bank];
                self.base.bank_offsets[bank] = self.chr_bank_offsets[latch_value][bank];
                info!("Updating currently latched bank");
            }

            info!(
                "MMC2 bank switch caused by CPU write {:04X}={:02X} {:?} {:?} {:?} {:?} {:?} {}",
                address,
                value,
                self.latches,
                self.base.banks,
                self.base.bank_offsets,
                self.chr_banks,
                self.chr_bank_offsets,
                self.base.total_banks
            );
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
    info!("Creating MMC2 mapper for cartridge {:?}", header);

    (
        Box::new(Mmc2PrgChip::new(prg_rom, header.prg_rom_16kb_units as usize * 2)),
        Box::new(Mmc2Mmc4ChrChip::new(
            ChrData::from(chr_rom),
            MirroringMode::Vertical,
            false,
        )),
        header,
    )
}
