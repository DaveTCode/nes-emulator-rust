use cartridge::mappers::{BankedChrChip, BankedPrgChip};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

/// AxROM has a single 32KB switchable bank driven by PRG 0-2
fn axrom_update_prg_banks(
    address: u16,
    value: u8,
    total_banks: u8,
    banks: &mut [u8; 4],
    bank_offsets: &mut [usize; 4],
) {
    if let 0x8000..=0xFFFF = address {
        banks[0] = (value & 0b111) % total_banks;
        bank_offsets[0] = banks[0] as usize * 0x8000;
        bank_offsets[1] = bank_offsets[0] + 0x2000;
        bank_offsets[2] = bank_offsets[1] + 0x2000;
        bank_offsets[3] = bank_offsets[2] + 0x2000;
        info!("AxROM PRG Bank switch {:?} -> {:?}", banks, bank_offsets);
    }
}

/// AxROM doesn't bank it's CHRROM/RAM but it is possible to switch mirroring
/// mode through PRG 4
fn axrom_chr_cpu_write_fn(
    address: u16,
    value: u8,
    _: u8,
    _: &mut u8,
    _: &mut usize,
    mirroring_mode: &mut MirroringMode,
) {
    if let 0x8000..=0xFFFF = address {
        *mirroring_mode = if value & 0b1_0000 == 0 {
            MirroringMode::OneScreenLowerBank
        } else {
            MirroringMode::OneScreenUpperBank
        };
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
    info!("Creating AxROM mapper for cartridge {:?}", header);
    (
        Box::new(BankedPrgChip::new(
            prg_rom,
            None,
            header.prg_rom_16kb_units / 2,
            [0, 1, 2, 3],
            [0, 0x2000, 0x4000, 0x6000],
            axrom_update_prg_banks,
        )),
        Box::new(BankedChrChip::new(
            chr_rom,
            MirroringMode::OneScreenLowerBank,
            header.chr_rom_8kb_units,
            axrom_chr_cpu_write_fn,
        )),
        header,
    )
}
