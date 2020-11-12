use cartridge::mappers::{ChrData, SingleBankedChrChip, SingleBankedPrgChip};
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

#[inline]
fn gxrom_address_is_control(address: u16) -> bool {
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
    info!("Creating GxROM mapper for cartridge {:?}", header);
    (
        Box::new(SingleBankedPrgChip::new(
            prg_rom,
            header.prg_rom_16kb_units as usize / 2,
            0b11_0000,
            4,
            gxrom_address_is_control,
        )),
        Box::new(SingleBankedChrChip::new(
            ChrData::from(chr_rom),
            header.mirroring,
            0b11,
            0,
            gxrom_address_is_control,
        )),
        header,
    )
}
