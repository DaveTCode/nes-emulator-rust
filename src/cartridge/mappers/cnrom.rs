use cartridge::mappers::nrom::NoBankPrgChip;
use cartridge::mappers::{ChrBaseData, ChrData};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

/// Straightforward CHR banked chip with one bank switched on 0x8000..0xFFFF
/// Used in at least Cnrom & Uxrom variants
pub(super) struct SingleBankedChrChip {
    base: ChrBaseData,
}

impl SingleBankedChrChip {
    pub(super) fn new(chr_data: ChrData, mirroring_mode: MirroringMode) -> Self {
        SingleBankedChrChip {
            base: ChrBaseData::new(mirroring_mode, chr_data, 0x2000, vec![0], vec![0]),
        }
    }
}

impl PpuCartridgeAddressBus for SingleBankedChrChip {
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
            self.base.banks[0] = value as usize % self.base.total_banks;
            self.base.bank_offsets[0] = self.base.banks[0] as usize * 0x2000;
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
    info!("Creating CNROM mapper for cartridge {:?}", header);
    (
        Box::new(NoBankPrgChip::new(prg_rom)),
        Box::new(SingleBankedChrChip::new(ChrData::from(chr_rom), header.mirroring)),
        header,
    )
}
