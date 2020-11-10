use cartridge::mappers::{BankedPrgChip, ChrData};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use cpu::CpuCycle;
use log::{debug, info};
use ppu::PpuCycle;

/// MMC2 has four banks, switched by 0xA000-0xFFFF where only the first is switchable
/// and the remainder are fixed to the last 3 available banks
fn mmc2rom_update_prg_banks(
    address: u16,
    value: u8,
    total_banks: u8,
    banks: &mut [u8; 4],
    bank_offsets: &mut [usize; 4],
) {
    if let 0xA000..=0xAFFF = address {
        banks[0] = (value & 0b1111) % total_banks;
        bank_offsets[0] = banks[0] as usize * 0x2000;

        info!("MMC2 PRG Bank switch {:?} -> {:?}", banks, bank_offsets);
    }
}

pub(crate) struct Mmc2ChrChip {
    chr_data: ChrData,
    ppu_vram: [u8; 0x1000],
    total_chr_banks: u8,
    chr_banks: [[u8; 2]; 2],
    chr_bank_offsets: [[usize; 2]; 2],
    latch_0: usize,
    latch_1: usize,
    mirroring_mode: MirroringMode,
}

impl Mmc2ChrChip {
    pub(super) fn new(chr_rom: Option<Vec<u8>>, mirroring_mode: MirroringMode, total_chr_banks: u8) -> Self {
        match chr_rom {
            Some(rom) => Mmc2ChrChip {
                chr_data: ChrData::Rom(rom),
                ppu_vram: [0; 0x1000],
                total_chr_banks,
                chr_banks: [[0; 2]; 2],
                chr_bank_offsets: [[0, 0x1000]; 2],
                latch_0: 0,
                latch_1: 1,
                mirroring_mode,
            },
            None => Mmc2ChrChip {
                chr_data: ChrData::Ram(Box::new([0; 0x2000])),
                ppu_vram: [0; 0x1000],
                total_chr_banks,
                chr_banks: [[0; 2]; 2],
                chr_bank_offsets: [[0, 0x1000]; 2],
                latch_0: 0,
                latch_1: 1,
                mirroring_mode,
            },
        }
    }
}

impl PpuCartridgeAddressBus for Mmc2ChrChip {
    fn check_trigger_irq(&mut self, _: bool) -> bool {
        false
    }

    fn update_vram_address(&mut self, _: u16, _: PpuCycle) {}

    fn read_byte(&mut self, address: u16, _: PpuCycle) -> u8 {
        let value = match address {
            0x0000..=0x0FFF => match &self.chr_data {
                ChrData::Rom(rom) => rom[address as usize + self.chr_bank_offsets[self.latch_0][0]],
                ChrData::Ram(ram) => ram[address as usize],
            },
            0x1000..=0x1FFF => match &self.chr_data {
                ChrData::Rom(rom) => rom[address as usize - 0x1000 + self.chr_bank_offsets[self.latch_1][1]],
                ChrData::Ram(ram) => ram[address as usize],
            },
            0x2000..=0x3EFF => {
                let mirrored_address = self.mirroring_mode.get_mirrored_address(address);
                debug!("Read {:04X} mirrored to {:04X}", address, mirrored_address);

                self.ppu_vram[mirrored_address as usize]
            }
            0x3F00..=0x3FFF => panic!("Shouldn't be reading from palette RAM through cartridge bus"),
            _ => panic!("Reading from {:04X} invalid for CHR address bus", address),
        };

        // Set latches based on certain reads - I _think_ that this is done on
        // read not on latched address but not 100% sure
        match address {
            0x0FD8 => self.latch_0 = 0,
            0x0FE8 => self.latch_0 = 1,
            0x1FD8..=0x1FDF => self.latch_1 = 0,
            0x1FE8..=0x1FEF => self.latch_1 = 1,
            _ => {}
        }

        value
    }

    fn write_byte(&mut self, address: u16, value: u8, _: PpuCycle) {
        debug!("MMC2 CHR write {:04X}={:02X}", address, value);
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

    fn cpu_write_byte(&mut self, address: u16, value: u8, cycles: CpuCycle) {
        debug!(
            "CPU write to CHR bus {:04X}={:02X} at {:} cycles",
            address, value, cycles
        );

        // MMC2 has two 4KB CHR ROM banks out of a 128KB capacity and switchable mirroring.
        // The banks themselves are switched based on a latch which is set on vram address
        match address {
            0xB000..=0xBFFF => {
                self.chr_banks[0][0] = (value & 0b1_1111) % self.total_chr_banks;
                self.chr_bank_offsets[0][0] = self.chr_banks[0][0] as usize * 0x1000;
                info!("Updated MMC1 banks {:?} {:?}", self.chr_banks, self.chr_bank_offsets);
            }
            0xC000..=0xCFFF => {
                self.chr_banks[1][0] = (value & 0b1_1111) % self.total_chr_banks;
                self.chr_bank_offsets[1][0] = self.chr_banks[1][0] as usize * 0x1000;
                info!("Updated MMC1 banks {:?} {:?}", self.chr_banks, self.chr_bank_offsets);
            }
            0xD000..=0xDFFF => {
                self.chr_banks[0][1] = (value & 0b1_1111) % self.total_chr_banks;
                self.chr_bank_offsets[0][1] = self.chr_banks[0][1] as usize * 0x1000;
                info!("Updated MMC1 banks {:?} {:?}", self.chr_banks, self.chr_bank_offsets);
            }
            0xE000..=0xEFFF => {
                self.chr_banks[1][1] = (value & 0b1_1111) % self.total_chr_banks;
                self.chr_bank_offsets[1][1] = self.chr_banks[1][1] as usize * 0x1000;
                info!("Updated MMC1 banks {:?} {:?}", self.chr_banks, self.chr_bank_offsets);
            }
            0xF000..=0xFFFF => {
                self.mirroring_mode = if value & 0b1 == 0b1 {
                    MirroringMode::Horizontal
                } else {
                    MirroringMode::Vertical
                };
            }
            _ => {}
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

    let total_banks = header.prg_rom_16kb_units * 2;
    debug_assert!(total_banks >= 4);

    (
        Box::new(BankedPrgChip::new(
            prg_rom,
            None,
            total_banks,
            [0, total_banks - 3, total_banks - 2, total_banks - 1],
            [
                0,
                (total_banks as usize - 3) * 0x2000,
                (total_banks as usize - 2) * 0x2000,
                (total_banks as usize - 1) * 0x2000,
            ],
            mmc2rom_update_prg_banks,
        )),
        Box::new(Mmc2ChrChip::new(
            chr_rom,
            MirroringMode::Vertical,
            header.chr_rom_8kb_units * 2,
        )),
        header,
    )
}
