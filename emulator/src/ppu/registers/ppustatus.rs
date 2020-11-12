/// https://wiki.nesdev.com/w/index.php/PPU_registers#PPUSTATUS
#[derive(Debug)]
pub(crate) struct PpuStatus {
    pub(crate) sprite_overflow: bool,
    /// Set when a nonzero pixel of sprite 0 overlaps a nonzero background pixel; cleared at dot 1 of the line 261
    pub(crate) sprite_zero_hit: bool,
    /// Set at dot 1 of line 241, cleared on read of PPUSTATUS and at dot 1 of line 261
    pub(crate) vblank_started: bool,
}

impl PpuStatus {
    pub(crate) fn new() -> Self {
        PpuStatus {
            sprite_overflow: false,
            sprite_zero_hit: false,
            vblank_started: false,
        }
    }

    pub(crate) fn read(&mut self, last_written_byte: u8) -> u8 {
        let mut result = last_written_byte & 0b0001_1111;
        if self.sprite_overflow {
            result |= 0b0010_0000
        };
        if self.sprite_zero_hit {
            result |= 0b0100_0000
        };
        if self.vblank_started {
            result |= 0b1000_0000
        };

        self.vblank_started = false;

        result
    }
}
