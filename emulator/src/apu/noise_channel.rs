use apu::envelope::Envelope;
use apu::length_counter::LengthCounter;
use log::{debug, error, info};

const TIMER_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

pub(super) struct NoiseChannel {
    enabled: bool,
    length_counter: LengthCounter,
    lsfr_use_bit_6: bool,
    period: u16,
    timer: u16,
    /// 15 bit wide shift register for the LSFR
    shift_register: u16,
    envelope: Envelope,
}

impl NoiseChannel {
    pub(super) fn new() -> Self {
        NoiseChannel {
            enabled: false,
            length_counter: LengthCounter::new(),
            lsfr_use_bit_6: false,
            period: 0,
            timer: 0,
            shift_register: 1,
            envelope: Envelope::new(),
        }
    }

    pub(super) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !self.enabled {
            self.length_counter.disable();
        }
    }

    /// Corresponds to writes to 0x400C
    pub(super) fn write_length_halt_envelope_register(&mut self, value: u8) {
        self.length_counter.set_halt(value & 0b0010_0000 != 0);
        self.envelope.register_write(value);
    }

    /// Corresponds to write to 400E
    pub(super) fn set_mode_and_period(&mut self, value: u8) {
        self.lsfr_use_bit_6 = value & 0b1000_0000 == 0b1000_0000;
        self.period = TIMER_PERIOD_TABLE[value as usize & 0b0000_1111];
    }

    /// Corresponds to writes to 0x400F
    pub(super) fn load_length_counter(&mut self, value: u8) {
        if self.enabled {
            self.length_counter.set(value);
            info!(
                "Loaded length counter for noise channel: {:02X} -> {:?}",
                value, self.length_counter
            );
        }
        self.envelope.set_start_flag();
    }

    pub(crate) fn non_zero_length_counter(&self) -> bool {
        self.length_counter.is_non_zero()
    }

    pub(super) fn clock_length_counter(&mut self) {
        info!("Clocking length counter for triangle channel {:?}", self.length_counter);
        self.length_counter.clock();
    }

    pub(super) fn clock_envelope(&mut self) {
        self.envelope.clock();
    }

    /// Noise channel is clocked on every APU cycle
    pub(super) fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.period;

            // Step the LSFR
            debug!("Updating LSFR {:015b}", self.shift_register);
            let feedback = self.shift_register & 0b1
                ^ if self.lsfr_use_bit_6 {
                    (self.shift_register & 0b0100_0000) >> 6
                } else {
                    (self.shift_register & 0b10) >> 1
                };

            self.shift_register >>= 1;
            self.shift_register |= feedback << 14;
        } else {
            self.timer -= 1;
        }
    }

    /// The output volume for the channel
    pub(super) fn mixer_value(&self) -> u8 {
        if self.length_counter.is_non_zero() && self.shift_register & 0b1 == 0 {
            self.envelope.volume()
        } else {
            0
        }
    }
}
