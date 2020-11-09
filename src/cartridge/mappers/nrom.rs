use cartridge::mappers::{BankedChrChip, BankedPrgChip};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

fn nrom_write_byte_function(_: u16, _: u8, _: u8, _: &mut [u8; 2], _: &mut [usize; 2]) {}

fn nrom_chr_cpu_write_fn(_: u16, _: u8, _: u8, _: &mut u8, _: &mut usize, _: &mut MirroringMode) {}

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
        Box::new(BankedPrgChip::new(
            prg_rom,
            Some([0; 0x2000]),
            2,
            [0, 1],
            [0, 0x4000],
            nrom_write_byte_function,
        )),
        Box::new(BankedChrChip::new(chr_rom, header.mirroring, 1, nrom_chr_cpu_write_fn)),
        header,
    )
}
