use log::{debug, info};

const LENGTH_COUNTER_MAP: [u8; 0x20] = [
    0x0A, 0xFE, 0x14, 0x02, 0x28, 0x04, 0x50, 0x06, 0xA0, 0x08, 0x3C, 0x0A, 0x0E, 0x0C, 0x1A, 0x0E, 0x0C, 0x10, 0x18,
    0x12, 0x30, 0x14, 0x60, 0x16, 0xC0, 0x18, 0x48, 0x1A, 0x10, 0x1C, 0x20, 0x1E,
];

const EIGHTH_DUTY_CYCLE: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
const QUARTER_DUTY_CYCLE: [u8; 8] = [0, 0, 0, 0, 0, 0, 1, 1];
const HALF_DUTY_CYCLE: [u8; 8] = [0, 0, 0, 0, 1, 1, 1, 1];
const NEGATIVE_QUARTER_DUTY_CYCLE: [u8; 8] = [1, 1, 1, 1, 1, 1, 0, 0];

struct SweepUnit {
    enabled: bool,
    divider_period: u8,
    is_negate: bool,
    shift_count: u8,
}

impl SweepUnit {
    fn new() -> Self {
        SweepUnit {
            enabled: false,
            divider_period: 0,
            is_negate: false,
            shift_count: 0,
        }
    }

    fn update(&mut self, value: u8) {
        self.enabled = value & 0b1000_0000 == 0b1000_0000;
        self.divider_period = (value & 0b0111_0000) >> 4;
        self.is_negate = value & 0b0000_1000 == 0b0000_1000;
        self.shift_count = value & 0b0000_0111;
    }
}

pub(super) struct PulseChannel {
    name: String,
    enabled: bool,
    pub(super) length_counter: u8,
    length_counter_halt: bool,
    duty_cycle: [u8; 8],
    sequence: usize,
    timer_load: u16,
    timer: u16,
    sweep_unit: SweepUnit,
}

impl PulseChannel {
    pub(super) fn new(name: String) -> Self {
        PulseChannel {
            name,
            enabled: false,
            length_counter: 0,
            length_counter_halt: false,
            duty_cycle: EIGHTH_DUTY_CYCLE,
            sequence: 0,
            timer_load: 0,
            timer: 0,
            sweep_unit: SweepUnit::new(),
        }
    }

    pub(super) fn disable(&self) {
        self.enabled = false;
        self.length_counter = 0;
    }

    /// Corresponds to writes to 0x4000 (pulse 1) & 0x4004 (pulse 2)
    pub(super) fn write_duty_length_halt_envelope_register(&mut self, value: u8) {
        self.duty_cycle = match value >> 6 {
            0b00 => EIGHTH_DUTY_CYCLE,
            0b01 => QUARTER_DUTY_CYCLE,
            0b10 => HALF_DUTY_CYCLE,
            0b11 => NEGATIVE_QUARTER_DUTY_CYCLE,
            _ => panic!(),
        };
        self.length_counter_halt = value & 0b0010_0000 != 0;
        // TODO - Envelope and constant volume flags
    }

    /// Corresponds to writes to 0x4002 (pulse 1) & 0x4006 (pulse 2)
    pub(super) fn load_timer_low(&mut self, value: u8) {
        info!("Loading timer low for {} with {:02X}", self.name, value);
        self.timer_load = (self.timer_load & 0b0111_0000_0000) | value as u16;
    }

    /// Corresponds to writes to 0x4003 (pulse 1) & 0x4007 (pulse 2)
    pub(super) fn load_length_timer_high(&mut self, value: u8) {
        if self.enabled {
            self.length_counter = LENGTH_COUNTER_MAP[(((value as usize) & 0b1111_1000) >> 3)];
            info!(
                "Loaded length counter for {}: {:02X} -> {:0X}",
                self.name, value, self.length_counter
            );
        }
        self.timer_load = (self.timer_load & 0b1111_1111) | ((value as u16 & 0b111) << 8);
        self.timer = self.timer_load;
        self.sequence = 0;
        // TODO - Restart envelope
    }

    /// Corresponds to writes to 0x4001 (pulse 1) & 0x4005 (pulse 2)
    pub(super) fn load_sweep_register(&mut self, value: u8) {
        info!("Loading sweep unit for {} with {:02X}", self.name, value);
        self.sweep_unit.update(value);
    }

    pub(super) fn clock_length_counter(&mut self) {
        if !self.length_counter_halt {
            debug!("Clocking length counter for {} {}", self.name, self.length_counter);
            self.length_counter = self.length_counter.saturating_sub(1);
            // TODO - Disable output on length 0 or just track it as length 0?}
        }
    }

    pub(super) fn clock_sweep_unit(&mut self) {
        // TODO
    }

    /// Called once per APU clock (once every two CPU clocks) and steps the timer
    pub(super) fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_load;

            // Go to next step in waveform duty sequence
            self.sequence = (self.sequence + 1) & 7;
            debug!(
                "Clocking wave duty waveform {:?} to step {}",
                self.duty_cycle, self.sequence
            );
        } else {
            self.timer -= 1;
        }
    }
}
