use cartridge::mappers::nrom::NoBankPrgChip;
use cartridge::mappers::{ChrBaseData, ChrData, PrgBaseData};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

struct ColorDreamsPrgChip {
    base: PrgBaseData,
}

impl ColorDreamsPrgChip {
    fn new(prg_rom: Vec<u8>, total_banks: usize) -> Self {
        ColorDreamsPrgChip {
            base: PrgBaseData {
                prg_rom,
                prg_ram: None,
                total_banks,
                bank_size: 0x8000,
                banks: vec![0],
                bank_offsets: vec![0],
            },
        }
    }
}

impl CpuCartridgeAddressBus for ColorDreamsPrgChip {
    fn read_byte(&self, address: u16) -> u8 {
        self.base.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        self.base.write_byte(address, value);

        // ColorDreams has a single 32KB switchable bank driven by PRG 0-1
        if let 0x8000..=0xFFFF = address {
            self.base.banks[0] = (value as usize & 0b11) % self.base.total_banks;
            self.base.bank_offsets[0] = self.base.banks[0] as usize * 0x8000;
            info!(
                "ColorDreams PRG Bank switch {:?} -> {:?}",
                self.base.banks, self.base.bank_offsets
            );
        }
    }
}

/// Straightforward CHR banked chip with one bank switched on 0x8000..0xFFFF
/// Used for unofficial ColorDreams cartridges
struct ColorDreamsChrChip {
    base: ChrBaseData,
}

impl ColorDreamsChrChip {
    fn new(chr_data: ChrData, mirroring_mode: MirroringMode) -> Self {
        ColorDreamsChrChip {
            base: ChrBaseData::new(mirroring_mode, chr_data, 0x2000, vec![0], vec![0]),
        }
    }
}

impl PpuCartridgeAddressBus for ColorDreamsChrChip {
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
        if let 0x8000..=0xFFFF = address {
            self.base.banks[0] = (value as usize >> 4) % self.base.total_banks;
            self.base.bank_offsets[0] = self.base.banks[0] as usize * 0x2000;
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
    info!("Creating ColorDreams mapper for cartridge {:?}", header);
    (
        Box::new(ColorDreamsPrgChip::new(prg_rom, header.prg_rom_16kb_units as usize / 2)),
        Box::new(ColorDreamsChrChip::new(ChrData::from(chr_rom), header.mirroring)),
        header,
    )
}
