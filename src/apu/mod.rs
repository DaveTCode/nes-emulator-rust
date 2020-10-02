pub(crate) struct Apu {}

impl Apu {
    pub(crate) fn new() -> Self {
        Apu {}
    }

    pub(crate) fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x4000..=0x4015 => 0x0, // TODO
            _ => panic!("Address invalid for APU {:04X}", address),
        }
    }

    pub(crate) fn write_byte(&self, address: u16, value: u8) {
        match address {
            0x4000..=0x4015 => {} // TODO
            0x4017 => {}          // TODO - Frame counter (only accepts write)
            _ => panic!("Address invalid for APU {:04X}", address),
        }
    }
}
