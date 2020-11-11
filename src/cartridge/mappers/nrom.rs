use cartridge::mappers::{ChrBaseData, ChrData, PrgBaseData};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

pub(crate) struct NoBankPrgChip {
    base: PrgBaseData,
}

impl NoBankPrgChip {
    pub(super) fn new(prg_rom: Vec<u8>) -> Self {
        NoBankPrgChip {
            base: PrgBaseData::new(prg_rom, Some([0; 0x2000]), 1, 0x8000, vec![0], vec![0]),
        }
    }
}

impl CpuCartridgeAddressBus for NoBankPrgChip {
    fn read_byte(&self, address: u16) -> u8 {
        self.base.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        self.base.write_byte(address, value)
    }
}

/// NRom is a chip with no CHR banking and fixed soldered mirroring mode from the cartridge itself
pub(crate) struct NoBankChrChip {
    base: ChrBaseData,
}

impl NoBankChrChip {
    pub(super) fn new(chr_data: ChrData, mirroring_mode: MirroringMode) -> Self {
        NoBankChrChip {
            base: ChrBaseData::new(mirroring_mode, chr_data, 0x2000, vec![0], vec![0]),
        }
    }
}

impl PpuCartridgeAddressBus for NoBankChrChip {
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

    fn cpu_write_byte(&mut self, _: u16, _: u8, _: u32) {}
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
    info!("Creating NROM mapper for cartridge");
    (
        Box::new(NoBankPrgChip::new(prg_rom)),
        Box::new(NoBankChrChip::new(ChrData::from(chr_rom), header.mirroring)),
        header,
    )
}
