use apu::length_counter::LengthCounter;
use log::{debug, info};

const TRIANGLE_SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];

pub(super) struct TriangleChannel {
    enabled: bool,
    timer_load: u16,
    timer: u16,
    sequence: u8,
    length_counter: LengthCounter,
    control_flag: bool,
    linear_counter_reload_flag: bool,
    linear_counter_reload: u8,
    linear_counter: u8,
}

impl TriangleChannel {
    pub(super) fn new() -> Self {
        TriangleChannel {
            enabled: false,
            timer_load: 0,
            timer: 0,
            sequence: 0,
            length_counter: LengthCounter::new(),
            control_flag: false,
            linear_counter_reload_flag: false,
            linear_counter_reload: 0,
            linear_counter: 0,
        }
    }

    pub(super) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !self.enabled {
            self.length_counter.disable();
        }
    }

    /// Corresponds to writes to 0x4008
    pub(super) fn load_linear_counter(&mut self, value: u8) {
        self.linear_counter_reload = value & 0b0111_1111;
        self.control_flag = value & 0b1000_0000 == 0b1000_0000;
        self.length_counter.set_halt(self.control_flag);
    }

    /// Corresponds to writes to 0x400A
    pub(super) fn load_timer_low(&mut self, value: u8) {
        info!("Loading timer low for triangle channel with {:02X}", value);
        self.timer_load = (self.timer_load & 0b0111_0000_0000) | value as u16;
    }

    /// Corresponds to writes to 0x400B
    pub(super) fn load_length_timer_high(&mut self, value: u8) {
        if self.enabled {
            self.length_counter.set(value);
            info!(
                "Loaded length counter for triangle channel: {:02X} -> {:?}",
                value, self.length_counter
            );
        }
        self.timer_load = (self.timer_load & 0b1111_1111) | ((value as u16 & 0b111) << 8);
        self.timer = self.timer_load;
        self.linear_counter_reload_flag = true;
    }

    pub(crate) fn non_zero_length_counter(&self) -> bool {
        self.length_counter.is_non_zero()
    }

    pub(super) fn clock_length_counter(&mut self) {
        info!("Clocking length counter for triangle channel {:?}", self.length_counter);
        self.length_counter.clock();
    }

    pub(super) fn clock_linear_counter(&mut self) {
        info!("Clocking linear counter for triangle channel {:?}", self.linear_counter);
        if self.linear_counter_reload_flag {
            self.linear_counter = self.linear_counter_reload;
        } else if self.linear_counter > 0 {
            self.linear_counter = self.linear_counter.saturating_sub(1);
        }

        if !self.control_flag {
            self.linear_counter_reload_flag = false;
        }
    }

    /// Called once per CPU clock (note not once per APU cycle)
    pub(super) fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_load;

            // Go to next step in triangle sequence
            if self.linear_counter > 0 && self.length_counter.is_non_zero() {
                self.sequence = (self.sequence + 1) & 31;
                debug!("Clocking triangle waveform to step {}", self.sequence);
            }
        } else {
            self.timer -= 1;
        }
    }

    /// The output volume for the channel
    pub(super) fn mixer_value(&self) -> u8 {
        if self.linear_counter > 0 && self.length_counter.is_non_zero() {
            TRIANGLE_SEQUENCE[self.sequence as usize]
        } else {
            0
        }
    }
}
