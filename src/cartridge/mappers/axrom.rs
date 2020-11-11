use cartridge::mappers::{ChrBaseData, ChrData, PrgBaseData};
use cartridge::mirroring::MirroringMode;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::info;

struct AxRomPrgChip {
    base: PrgBaseData,
}

impl AxRomPrgChip {
    fn new(prg_rom: Vec<u8>, total_banks: usize) -> Self {
        AxRomPrgChip {
            base: PrgBaseData::new(prg_rom, None, total_banks, 0x8000, vec![0], vec![0]),
        }
    }
}

impl CpuCartridgeAddressBus for AxRomPrgChip {
    fn read_byte(&self, address: u16) -> u8 {
        self.base.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        self.base.write_byte(address, value);

        // AxROM has a single 32KB switchable bank driven by PRG 0-2
        if let 0x8000..=0xFFFF = address {
            self.base.banks[0] = (value as usize & 0b111) % self.base.total_banks;
            self.base.bank_offsets[0] = self.base.banks[0] as usize * 0x8000;
            info!(
                "AxROM PRG Bank switch {:?} -> {:?}",
                self.base.banks, self.base.bank_offsets
            );
        }
    }
}

/// AxROM doesn't bank it's CHRROM/RAM but it is possible to switch mirroring
/// mode through PRG 4
struct AxRomChrChip {
    base: ChrBaseData,
}

impl AxRomChrChip {
    pub(super) fn new(chr_data: ChrData, mirroring_mode: MirroringMode) -> Self {
        AxRomChrChip {
            base: ChrBaseData::new(mirroring_mode, chr_data, 0x2000, vec![0], vec![0]),
        }
    }
}

impl PpuCartridgeAddressBus for AxRomChrChip {
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
            self.base.mirroring_mode = if value & 0b1_0000 == 0 {
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
    info!("Creating AxROM mapper for cartridge {:?}", header);
    (
        Box::new(AxRomPrgChip::new(prg_rom, header.prg_rom_16kb_units as usize / 2)),
        Box::new(AxRomChrChip::new(
            ChrData::from(chr_rom),
            MirroringMode::OneScreenLowerBank,
        )),
        header,
    )
}
