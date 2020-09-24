use cartridge::mappers::Cartridge;
use cartridge::CartridgeHeader;

pub(crate) struct MapperZero {
    pub(crate) header: CartridgeHeader,
    pub(crate) prg_rom: Vec<u8>,
    pub(crate) chr_rom: Vec<u8>,
}
impl Cartridge for MapperZero {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x8000..=0xBFFF => self.prg_rom[(address - 0x8000) as usize],
            0xC000..=0xFFFF => self.prg_rom[(address - 0xC000) as usize], // TODO! - Not true for NROM-256
            _ => todo!("Not yet mapped addresses in zero mapper {:04X}", address),
        }
    }

    fn write_byte(&self, _address: u16, _value: u8) {}
}
