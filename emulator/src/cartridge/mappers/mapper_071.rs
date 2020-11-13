use cartridge::mappers::{ChrBaseData, ChrData, PrgBaseData};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

struct Mapper71PrgChip {
    base: PrgBaseData,
}

impl Mapper71PrgChip {
    fn new(prg_rom: Vec<u8>, total_banks: usize) -> Self {
        Mapper71PrgChip {
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

impl CpuCartridgeAddressBus for Mapper71PrgChip {
    fn read_byte(&self, address: u16) -> u8 {
        self.base.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        self.base.write_byte(address, value);

        if let 0xC000..=0xFFFF = address {
            self.base.banks[0] = (value & 0b1111) as usize % self.base.total_banks;
            self.base.bank_offsets[0] = self.base.banks[0] * self.base.bank_size;
            info!(
                "Mapper 71 bank switch {:?} => {:?}",
                self.base.banks, self.base.bank_offsets
            );
        }
    }
}

struct Mapper71ChrChip {
    base: ChrBaseData,
}

impl Mapper71ChrChip {
    fn new(chr_data: ChrData, mirroring: MirroringMode) -> Self {
        Mapper71ChrChip {
            base: ChrBaseData::new(mirroring, chr_data, 0x2000, vec![0], vec![0]),
        }
    }
}

impl PpuCartridgeAddressBus for Mapper71ChrChip {
    fn check_trigger_irq(&mut self, _: bool) -> bool {
        false
    }

    fn update_vram_address(&mut self, _: u16, _: u32) {}

    fn read_byte(&mut self, address: u16, _: u32) -> u8 {
        self.base.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        self.base.write_byte(address, value);
    }

    fn cpu_write_byte(&mut self, address: u16, value: u8, _: u32) {
        // This (8000..9FFF) register is only actually present on a specific submapper of mapper 71,
        // By moving it to 9000 instead we support both formats without needing to resort to trusting submappers
        // in rom dumps
        if let 0x9000..=0x9FFF = address {
            self.base.mirroring_mode = if (value & 0b1_0000) == 0 {
                MirroringMode::OneScreenLowerBank
            } else {
                MirroringMode::OneScreenUpperBank
            };
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
    info!("Creating Mapper 71 for cartridge {:?}", header);
    (
        Box::new(Mapper71PrgChip::new(prg_rom, header.prg_rom_16kb_units as usize)),
        Box::new(Mapper71ChrChip::new(ChrData::from(chr_rom), header.mirroring)),
        header,
    )
}
