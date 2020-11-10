use cartridge::mappers::{BankedChrChip, BankedPrgChip};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

fn uxrom_update_prg_banks(
    address: u16,
    value: u8,
    total_banks: u8,
    banks: &mut [u8; 4],
    bank_offsets: &mut [usize; 4],
) {
    if let 0x8000..=0xFFFF = address {
        banks[0] = value % total_banks;
        banks[1] = banks[0] + 1;
        bank_offsets[0] = banks[0] as usize * 0x4000;
        bank_offsets[1] = bank_offsets[0] + 0x2000;
        info!("Bank switch {:?} -> {:?}", banks, bank_offsets);
    }
}

fn uxrom_chr_cpu_write_fn(
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
    };
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
    info!("Creating UxROM mapper for cartridge {:?}", header);
    (
        Box::new(BankedPrgChip::new(
            prg_rom,
            None,
            header.prg_rom_16kb_units,
            [0, 1, header.prg_rom_16kb_units - 1, header.prg_rom_16kb_units],
            [
                0,
                0x2000,
                (header.prg_rom_16kb_units as usize - 1) * 0x4000,
                (header.prg_rom_16kb_units as usize - 1) * 0x4000 + 0x2000,
            ],
            uxrom_update_prg_banks,
        )),
        Box::new(BankedChrChip::new(chr_rom, header.mirroring, 1, uxrom_chr_cpu_write_fn)),
        header,
    )
}
