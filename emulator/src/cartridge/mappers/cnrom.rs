use cartridge::mappers::{ChrData, NoBankPrgChip, SingleBankedChrChip};
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

#[inline]
fn cnrom_address_is_control(address: u16) -> bool {
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
    info!("Creating CNROM mapper for cartridge {:?}", header);
    (
        Box::new(NoBankPrgChip::new(prg_rom)),
        Box::new(SingleBankedChrChip::new(
            ChrData::from(chr_rom),
            header.mirroring,
            0xFF,
            0,
            cnrom_address_is_control,
        )),
        header,
    )
}
