use cartridge::mappers::cnrom::SingleBankedChrChip;
use cartridge::mappers::{ChrData, SingleBankedPrgChip};
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
    info!("Creating ColorDreams mapper for cartridge {:?}", header);
    (
        Box::new(SingleBankedPrgChip::new(
            prg_rom,
            header.prg_rom_16kb_units as usize / 2,
            0b11,
            0,
        )),
        Box::new(SingleBankedChrChip::new(
            ChrData::from(chr_rom),
            header.mirroring,
            0b1111_0000,
            4,
        )),
        header,
    )
}
