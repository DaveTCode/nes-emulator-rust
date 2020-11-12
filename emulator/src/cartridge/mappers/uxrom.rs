use cartridge::mappers::{ChrData, NoBankChrChip, PrgBaseData};
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

/// UxRom board comes in a variety of variants which subtly change how
/// banking is achieved
#[derive(Debug, PartialEq)]
enum UxRomVariant {
    Unrom,        // Mapper 002
    UnromReverse, // Mapper 180
    HvcUn1Rom,    // Mapper 094
}

struct UxRom {
    base: PrgBaseData,
    variant: UxRomVariant,
}

impl UxRom {
    fn new(prg_rom: Vec<u8>, total_banks: usize, variant: UxRomVariant) -> Self {
        UxRom {
            variant,
            base: PrgBaseData {
                prg_rom,
                prg_ram: None,
                bank_size: 0x4000,
                total_banks,
                banks: vec![0, total_banks - 1],
                bank_offsets: vec![0, (total_banks - 1) * 0x4000],
            },
        }
    }
}

impl CpuCartridgeAddressBus for UxRom {
    fn read_byte(&self, address: u16) -> u8 {
        self.base.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        self.base.write_byte(address, value);

        if let 0x8000..=0xFFFF = address {
            // TODO - According to https://wiki.nesdev.com/w/index.php/UxROM UOROM uses 4 bits to describe the bank and UNROM uses 3 bits, I mask here with 4 bits because I'm not sure how to tell the two apart.
            let (switchable_bank, value) = match self.variant {
                UxRomVariant::Unrom => (0, (value as usize & 0b1111) % self.base.total_banks),
                UxRomVariant::UnromReverse => (1, (value as usize & 0b1111) % self.base.total_banks),
                UxRomVariant::HvcUn1Rom => (0, ((value as usize & 0b1_1100) >> 2) % self.base.total_banks),
            };

            self.base.banks[switchable_bank] = value;
            self.base.bank_offsets[switchable_bank] = self.base.banks[switchable_bank] * 0x4000;
            info!(
                "UxROM ({:?}) bank switch {:?} => {:?}",
                self.variant, self.base.banks, self.base.bank_offsets
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
    info!("Creating UxROM mapper for cartridge {:?}", header);
    (
        Box::new(UxRom::new(
            prg_rom,
            header.prg_rom_16kb_units as usize,
            match header.mapper {
                2 => UxRomVariant::Unrom,
                94 => UxRomVariant::HvcUn1Rom,
                180 => UxRomVariant::UnromReverse,
                _ => panic!("Can't create UxROM from mapper {}", header.mapper),
            },
        )),
        Box::new(NoBankChrChip::new(ChrData::from(chr_rom), header.mirroring)),
        header,
    )
}
