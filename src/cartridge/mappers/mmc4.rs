use cartridge::mappers::mmc2::Mmc2Mmc4ChrChip;
use cartridge::mappers::{ChrData, PrgBaseData};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

struct Mmc4PrgChip {
    base: PrgBaseData,
}

impl Mmc4PrgChip {
    fn new(prg_rom: Vec<u8>, total_banks: usize) -> Self {
        Mmc4PrgChip {
            base: PrgBaseData::new(
                prg_rom,
                None,
                total_banks,
                0x4000,
                vec![0, total_banks - 1],
                vec![0, (total_banks - 1) * 0x4000],
            ),
        }
    }
}

impl CpuCartridgeAddressBus for Mmc4PrgChip {
    fn read_byte(&self, address: u16) -> u8 {
        self.base.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        self.base.write_byte(address, value);

        // MMC4 has two banks, switched by 0xA000-0xFFFF where only the first is switchable
        // and the last is fixed to the last bank
        if let 0xA000..=0xAFFF = address {
            self.base.banks[0] = (value as usize & 0b1111) % self.base.total_banks;
            self.base.bank_offsets[0] = self.base.banks[0] as usize * 0x4000;

            info!(
                "MMC4 PRG Bank switch {:?} -> {:?}",
                self.base.banks, self.base.bank_offsets
            );
        }
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
    info!("Creating MMC4 mapper for cartridge {:?}", header);
    (
        Box::new(Mmc4PrgChip::new(prg_rom, header.prg_rom_16kb_units as usize)),
        Box::new(Mmc2Mmc4ChrChip::new(
            ChrData::from(chr_rom),
            MirroringMode::Vertical,
            true,
        )),
        header,
    )
}
