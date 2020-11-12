use cartridge::mappers::{ChrBaseData, ChrData, NoBankChrChip, SingleBankedPrgChip};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

#[inline]
fn bxrom_address_is_control(address: u16) -> bool {
    address >= 0x8000
}

#[inline]
fn nina_001_address_is_prg_control(address: u16) -> bool {
    address == 0x7FFD
}

/// NINA-001 has 2 4KB banks switched on 2 registers
struct Nina001ChrChip {
    base: ChrBaseData,
}

impl Nina001ChrChip {
    pub(super) fn new(chr_data: ChrData) -> Self {
        Nina001ChrChip {
            base: ChrBaseData::new(MirroringMode::Horizontal, chr_data, 0x1000, vec![0, 1], vec![0, 0x1000]),
        }
    }
}

impl PpuCartridgeAddressBus for Nina001ChrChip {
    fn check_trigger_irq(&mut self, _: bool) -> bool {
        false
    }

    fn update_vram_address(&mut self, _: u16, _: u32) {}

    fn read_byte(&mut self, address: u16, _: u32) -> u8 {
        self.base.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        self.base.write_byte(address, value);
    }

    fn cpu_write_byte(&mut self, address: u16, value: u8, _: u32) {
        match address {
            0x7FFE => {
                self.base.banks[0] = value as usize & 0b1111;
                self.base.bank_offsets[0] = self.base.banks[0] * self.base.bank_size;
            }
            0x7FFF => {
                self.base.banks[1] = value as usize & 0b1111;
                self.base.bank_offsets[1] = self.base.banks[1] * self.base.bank_size;
            }
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
    // We can distinguish between BxROM and NINA-001 based on the number of CHR units
    match header.chr_rom_8kb_units {
        // BxRom
        0..=1 => {
            info!("Creating BxROM mapper for cartridge {:?}", header);
            (
                Box::new(SingleBankedPrgChip::new(
                    prg_rom,
                    None,
                    header.prg_rom_16kb_units as usize / 2,
                    0b11,
                    0,
                    bxrom_address_is_control,
                )),
                Box::new(NoBankChrChip::new(ChrData::from(chr_rom), header.mirroring)),
                header,
            )
        }
        // NINA-001
        _ => {
            info!("Creating NINA-001 mapper for cartridge {:?}", header);
            (
                Box::new(SingleBankedPrgChip::new(
                    prg_rom,
                    Some([0; 0x2000]),
                    header.prg_rom_16kb_units as usize / 2,
                    0b1,
                    0,
                    nina_001_address_is_prg_control,
                )),
                Box::new(Nina001ChrChip::new(ChrData::from(chr_rom))),
                header,
            )
        }
    }
}
