use cartridge::mappers::{BankedChrChip, BankedPrgChip};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

fn cnrom_update_prg_banks(_: u16, _: u8, _: u8, _: &mut [u8; 2], _: &mut [usize; 2]) {}

fn cnrom_chr_cpu_write_fn(
    address: u16,
    value: u8,
    total_banks: u8,
    bank: &mut u8,
    bank_offset: &mut usize,
    _: &mut MirroringMode,
) {
    if let 0x8000..=0xFFFF = address {
        *bank = value % total_banks;
        *bank_offset = *bank as usize * 0x2000;
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
        Box::new(BankedPrgChip::new(
            prg_rom,
            None,
            header.prg_rom_16kb_units,
            [0, 1],
            [0, 0x4000],
            cnrom_update_prg_banks,
        )),
        Box::new(BankedChrChip::new(
            chr_rom,
            header.mirroring,
            header.chr_rom_8kb_units,
            cnrom_chr_cpu_write_fn,
        )),
        header,
    )
}
