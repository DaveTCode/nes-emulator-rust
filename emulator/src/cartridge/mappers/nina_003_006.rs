use cartridge::mappers::{ChrData, SingleBankedChrChip, SingleBankedPrgChip};
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

#[inline]
fn nina_003_006_control_register_check(address: u16) -> bool {
    (address & 0b1110_0001_0000_0000) == 0b0100_0001_0000_0000
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
    info!("Creating NINA-003-006 mapper for cartridge {:?}", header);
    (
        Box::new(SingleBankedPrgChip::new(
            prg_rom,
            None,
            header.prg_rom_16kb_units as usize / 2,
            0b1000,
            3,
            nina_003_006_control_register_check,
        )),
        Box::new(SingleBankedChrChip::new(
            ChrData::from(chr_rom),
            header.mirroring,
            0b111,
            0,
            nina_003_006_control_register_check,
        )),
        header,
    )
}

#[cfg(test)]
mod nina_003_006_tests {
    use cartridge::mappers::nina_003_006::nina_003_006_control_register_check;

    #[test]
    fn test_check_control_register() {
        for i in 0..=u16::MAX {
            let expected = matches!(
                ((i >> 8) & 0b1, (i >> 13) & 0b1, (i >> 14) & 0b1, (i >> 15) & 0b1),
                (1, 0, 1, 0)
            );
            assert_eq!(nina_003_006_control_register_check(i), expected);
        }
    }
}
