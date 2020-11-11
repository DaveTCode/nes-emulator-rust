use cartridge::mappers::{ChrData, NoBankChrChip, NoBankPrgChip};
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

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
