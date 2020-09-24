use cartridge::mappers::Cartridge;
use log::debug;

pub(crate) struct Mmu<'a> {
    ram: [u8; 0x800],
    cartridge: &'a Box<dyn Cartridge>,
}

impl<'a> Mmu<'a> {
    pub fn new(cartridge: &'a Box<dyn Cartridge>) -> Self {
        Mmu {
            ram: [0; 0x800],
            cartridge,
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x07FF => self.ram[address as usize],
            0x0800..=0x1FFF => self.ram[(address % 0x0800) as usize], // Mirrors of ram space
            0x2000..=0x2007 => 0x00, // TODO - Implement PPU registers
            0x2008..=0x3FFF => 0x00, // TODO - Mirrors of PPU registers
            0x4000..=0x4017 => 0x00, // TODO - APU & IO registers
            0x4018..=0x401F => 0x00, // TODO - Unused APU & IO registers
            0x4020..=0xFFFF => self.cartridge.read_byte(address),
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        debug!("{:04X} = {:02X}", address, value);

        match address {
            0x0000..=0x07FF => self.ram[address as usize] = value,
            0x0800..=0x1FFF => self.ram[(address % 0x0800) as usize] = value, // Mirrors of ram space
            0x2000..=0x2007 => (), // TODO - Implement PPU registers
            0x2008..=0x3FFF => (), // TODO - Mirrors of PPU registers
            0x4000..=0x4017 => (), // TODO - APU & IO registers
            0x4018..=0x401F => (), // TODO - Unused APU & IO registers
            0x4020..=0xFFFF => self.cartridge.write_byte(address, value),
        }
    }
}

// #[cfg(test)]
// mod test {
//     use super::Mmu;

//     #[test]
//     fn test_read_write_ram() {
//         let mut m = Mmu { ram: [0; 0x800] };
//         m.write_byte(0x0, 0x1);
//         assert_eq!(0x1, m.read_byte(0x0));
//         assert_eq!(0x1, m.read_byte(0x800));
//         assert_eq!(0x1, m.read_byte(0x1000));
//         assert_eq!(0x1, m.read_byte(0x1800));
//         assert_eq!(0x0, m.read_byte(0x1801));
//     }
// }
