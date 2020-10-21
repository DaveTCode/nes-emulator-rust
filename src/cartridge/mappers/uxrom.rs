use cartridge::mappers::nrom::FixedMirroringChrChip;
use cartridge::CartridgeHeader;
use cartridge::CpuCartridgeAddressBus;
use cartridge::PpuCartridgeAddressBus;
use log::{debug, info};

pub(crate) struct UxRomPrgChip {
    prg_rom: Vec<u8>,
    total_banks: u8,
    prg_rom_banks: [u8; 2],
    prg_rom_bank_offsets: [usize; 2],
}

impl CpuCartridgeAddressBus for UxRomPrgChip {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x6000..=0x7FFF => 0x0, // TODO - No RAM on UxROM chip, does it return 0 for this area?
            0x8000..=0xBFFF => self.prg_rom[self.prg_rom_bank_offsets[0] + (address as usize - 0x8000)],
            0xC000..=0xFFFF => self.prg_rom[self.prg_rom_bank_offsets[1] + (address as usize - 0xC000)],
            _ => panic!("Unmapped addresses in UxROM {:04X}", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8, _: u32) {
        debug!("UXRom write {:04X}={:02X}", address, value);

        if let 0x8000..=0xFFFF = address {
            self.prg_rom_banks[0] = value % self.total_banks;
            self.prg_rom_bank_offsets[0] = self.prg_rom_banks[0] as usize * 0x4000;
            info!(
                "UXRom bank switch {:?} -> {:?}",
                self.prg_rom_banks, self.prg_rom_bank_offsets
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
        Box::new(UxRomPrgChip {
            prg_rom,
            total_banks: header.prg_rom_16kb_units,
            prg_rom_banks: [0, header.prg_rom_16kb_units - 1],
            prg_rom_bank_offsets: [0, (header.prg_rom_16kb_units as usize - 1) * 0x4000],
        }),
        Box::new(FixedMirroringChrChip::new(chr_rom, header.mirroring)),
        header,
    )
}
