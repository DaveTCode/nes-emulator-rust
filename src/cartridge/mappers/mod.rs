pub(super) mod mapper_zero;

pub(crate) trait Cartridge {
    fn read_byte(&self, address: u16) -> u8;
    fn write_byte(&self, address: u16, value: u8);
}
