use cartridge::mappers::{BankedChrChip, BankedPrgChip};
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

fn cnrom_update_banks(_: u16, _: u8, _: u8, _: &mut [u8; 2], _: &mut [usize; 2]) {}

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
        Box::new(BankedPrgChip::new(
            prg_rom,
            None,
            header.prg_rom_16kb_units,
            [0, 1],
            [0, 0x4000],
            cnrom_update_banks,
        )),
        Box::new(BankedChrChip::new(chr_rom, header.mirroring, header.chr_rom_8kb_units)),
        header,
    )
}
