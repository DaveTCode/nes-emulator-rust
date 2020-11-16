use apu::envelope::Envelope;
use apu::length_counter::LengthCounter;
use log::{debug, info};

const EIGHTH_DUTY_CYCLE: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 1];
const QUARTER_DUTY_CYCLE: [u8; 8] = [0, 0, 0, 0, 0, 0, 1, 1];
const HALF_DUTY_CYCLE: [u8; 8] = [0, 0, 0, 0, 1, 1, 1, 1];
const NEGATIVE_QUARTER_DUTY_CYCLE: [u8; 8] = [1, 1, 1, 1, 1, 1, 0, 0];

#[derive(Debug)]
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

#[derive(Debug)]
pub(super) struct PulseChannel {
    name: String,
    enabled: bool,
    length_counter: LengthCounter,
    duty_cycle: [u8; 8],
    sequence: usize,
    timer_load: u16,
    timer: u16,
    sweep_unit: SweepUnit,
    envelope: Envelope,
}

impl PulseChannel {
    pub(super) fn new(name: String) -> Self {
        PulseChannel {
            name,
            enabled: false,
            length_counter: LengthCounter::new(),
            duty_cycle: EIGHTH_DUTY_CYCLE,
            sequence: 0,
            timer_load: 0,
            timer: 0,
            sweep_unit: SweepUnit::new(),
            envelope: Envelope::new(),
        }
    }

    pub(super) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter.disable();
        }
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
        self.length_counter.set_halt(value & 0b0010_0000 != 0);
        self.envelope.register_write(value);
    }

    /// Corresponds to writes to 0x4002 (pulse 1) & 0x4006 (pulse 2)
    pub(super) fn load_timer_low(&mut self, value: u8) {
        info!("Loading timer low for {} with {:02X}", self.name, value);
        self.timer_load = (self.timer_load & 0b0111_0000_0000) | value as u16;
    }

    /// Corresponds to writes to 0x4003 (pulse 1) & 0x4007 (pulse 2)
    pub(super) fn load_length_timer_high(&mut self, value: u8) {
        if self.enabled {
            self.length_counter.set(value);
            info!(
                "Loaded length counter for {}: {:02X} -> {:?}",
                self.name, value, self.length_counter
            );
        }
        self.timer_load = (self.timer_load & 0b1111_1111) | ((value as u16 & 0b111) << 8);
        self.timer = self.timer_load;
        self.sequence = 0;
        self.envelope.set_start_flag();
    }

    /// Corresponds to writes to 0x4001 (pulse 1) & 0x4005 (pulse 2)
    pub(super) fn load_sweep_register(&mut self, value: u8) {
        info!("Loading sweep unit for {} with {:02X}", self.name, value);
        self.sweep_unit.update(value);
    }

    pub(crate) fn non_zero_length_counter(&self) -> bool {
        self.length_counter.is_non_zero()
    }

    pub(super) fn clock_length_counter(&mut self) {
        info!("Clocking length counter for {} {:?}", self.name, self.length_counter);
        self.length_counter.clock();
    }

    pub(super) fn clock_sweep_unit(&mut self) {
        // TODO
    }

    pub(super) fn clock_envelope(&mut self) {
        self.envelope.clock();
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

    pub(super) fn mixer_value(&self) -> u8 {
        self.envelope.volume()
    }
}
