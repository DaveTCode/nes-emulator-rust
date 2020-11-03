pub(crate) const LENGTH_COUNTER_MAP: [u8; 0x20] = [
    0x0A, 0xFE, 0x14, 0x02, 0x28, 0x04, 0x50, 0x06, 0xA0, 0x08, 0x3C, 0x0A, 0x0E, 0x0C, 0x1A, 0x0E, 0x0C, 0x10, 0x18,
    0x12, 0x30, 0x14, 0x60, 0x16, 0xC0, 0x18, 0x48, 0x1A, 0x10, 0x1C, 0x20, 0x1E,
];

#[derive(Debug)]
pub(crate) struct LengthCounter {
    length_counter: u8,
    length_counter_halt: bool,
}

impl LengthCounter {
    pub(crate) fn new() -> Self {
        LengthCounter {
            length_counter: 0,
            length_counter_halt: false,
        }
    }

    pub(crate) fn clock(&mut self) {
        if !self.length_counter_halt {
            self.length_counter = self.length_counter.saturating_sub(1);
        }
    }

    pub(crate) fn disable(&mut self) {
        self.length_counter = 0;
    }

    pub(crate) fn set(&mut self, value: u8) {
        self.length_counter = LENGTH_COUNTER_MAP[(((value as usize) & 0b1111_1000) >> 3)];
    }

    pub(crate) fn set_halt(&mut self, halt: bool) {
        self.length_counter_halt = halt;
    }

    pub(crate) fn is_non_zero(&self) -> bool {
        self.length_counter > 0
    }
}
