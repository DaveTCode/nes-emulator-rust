use log::info;

#[derive(Debug)]
pub(super) struct Envelope {
    constant_volume: u8,
    loop_envelope: bool,
    use_envelope: bool,
    start_flag: bool,
    decay_level: u8,
    divider: u8,
}

impl Envelope {
    pub(super) fn new() -> Self {
        Envelope {
            constant_volume: 0,
            loop_envelope: false,
            use_envelope: false,
            start_flag: false,
            divider: 0,
            decay_level: 0,
        }
    }

    pub(super) fn clock(&mut self) {
        if self.start_flag {
            self.start_flag = false;
            self.decay_level = 15;
            self.divider = self.constant_volume; // TODO - Maybe + 1? Depends on when clocked I think?
        } else if self.divider == 0 {
            self.divider = self.constant_volume;
            if self.decay_level == 0 {
                if self.loop_envelope {
                    self.decay_level = 15;
                }
            } else {
                self.decay_level -= 1;
            }
        } else {
            self.divider -= 1;
        }
    }

    /// Handles writes to 4000 | 4004 | 400C
    pub(super) fn register_write(&mut self, value: u8) {
        self.loop_envelope = value & 0b0010_0000 != 0;
        self.use_envelope = value & 0b0001_0000 != 0;
        self.constant_volume = value & 0b1111;

        info!("Envelope updated {:?}", &self);
    }

    /// Used on writes from 4003 | 4007 | 400F
    pub(crate) fn set_start_flag(&mut self) {
        self.start_flag = true;
    }

    /// Returns the current output level of the envelope
    pub(super) fn volume(&self) -> u8 {
        if self.use_envelope {
            self.decay_level
        } else {
            self.constant_volume
        }
    }
}
