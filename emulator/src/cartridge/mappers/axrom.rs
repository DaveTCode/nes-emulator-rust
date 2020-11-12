use cartridge::mappers::{ChrBaseData, ChrData, SingleBankedPrgChip};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

/// AxROM doesn't bank it's CHRROM/RAM but it is possible to switch mirroring
/// mode through PRG 4
struct AxRomChrChip {
    base: ChrBaseData,
}

impl AxRomChrChip {
    pub(super) fn new(chr_data: ChrData, mirroring_mode: MirroringMode) -> Self {
        AxRomChrChip {
            base: ChrBaseData::new(mirroring_mode, chr_data, 0x2000, vec![0], vec![0]),
        }
    }
}

impl PpuCartridgeAddressBus for AxRomChrChip {
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
        if let 0x8000..=0xFFFF = address {
            self.base.mirroring_mode = if value & 0b1_0000 == 0 {
                MirroringMode::OneScreenLowerBank
            } else {
                MirroringMode::OneScreenUpperBank
            };
        }
    }
}

#[inline]
fn axrom_address_is_control(address: u16) -> bool {
    address >= 0x8000
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
    info!("Creating AxROM mapper for cartridge {:?}", header);
    (
        Box::new(SingleBankedPrgChip::new(
            prg_rom,
            header.prg_rom_16kb_units as usize / 2,
            0b111,
            0,
            axrom_address_is_control,
        )),
        Box::new(AxRomChrChip::new(
            ChrData::from(chr_rom),
            MirroringMode::OneScreenLowerBank,
        )),
        header,
    )
}
