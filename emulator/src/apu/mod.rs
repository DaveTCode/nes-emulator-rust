use apu::dmc_channel::DmcChannel;
use apu::noise_channel::NoiseChannel;
use apu::pulse_channel::PulseChannel;
use apu::triangle_channel::TriangleChannel;
use log::info;

mod dmc_channel;
mod length_counter;
mod noise_channel;
mod pulse_channel;
mod triangle_channel;

/// This type is used to represent an APU cycle to make it clearer when
/// we're talking about cycles which type (PPU, CPU, APU) we mean.
/// An APU cycle occurs once for every two CPU cycles.
type ApuCycle = u32;

#[derive(Debug, PartialEq)]
enum FrameCounterMode {
    FourStep,
    FiveStep,
}

impl FrameCounterMode {
    fn wrapping_number(&self) -> u32 {
        match self {
            FrameCounterMode::FourStep => 14915,
            FrameCounterMode::FiveStep => 18641,
        }
    }
}

#[derive(Debug)]
struct FrameCounter {
    inhibit_interrupts: bool,
    mode: FrameCounterMode,
    step: u8,
    sequence_cycles: ApuCycle,
    timer_reset_countdown: u8,
}

impl FrameCounter {
    fn set(&mut self, value: u8, is_apu_cycle: bool) {
        if value & 0b1000_0000 == 0 {
            self.mode = FrameCounterMode::FourStep
        } else {
            self.mode = FrameCounterMode::FiveStep
        }
        self.inhibit_interrupts = value & 0b0100_0000 == 0b0100_0000;
        self.timer_reset_countdown = if is_apu_cycle { 3 } else { 4 };
    }
}

pub struct Apu {
    pulse_channel_1: PulseChannel,
    pulse_channel_2: PulseChannel,
    triangle_channel: TriangleChannel,
    noise_channel: NoiseChannel,
    dmc_channel: DmcChannel,
    frame_counter: FrameCounter,
    total_apu_cycles: ApuCycle,
    is_apu_cycle: bool,
    interrupt_triggered_cycles: Option<ApuCycle>,
}

impl Apu {
    pub fn new() -> Self {
        Apu {
            pulse_channel_1: PulseChannel::new("Pulse 1".to_string()),
            pulse_channel_2: PulseChannel::new("Pulse 2".to_string()),
            triangle_channel: TriangleChannel::new(),
            noise_channel: NoiseChannel::new(),
            dmc_channel: DmcChannel::new(),
            frame_counter: FrameCounter {
                inhibit_interrupts: false,
                mode: FrameCounterMode::FourStep,
                step: 0,
                sequence_cycles: 4,
                timer_reset_countdown: 0,
            },
            total_apu_cycles: 4, // TODO - What's the total number of APU cycles that occur during startup? 8/2?
            is_apu_cycle: false, // TODO - Guesswork, does the APU clock on cpu cycle 0 or 1?
            interrupt_triggered_cycles: None,
        }
    }

    fn write_status_register(&mut self, value: u8) {
        self.pulse_channel_1.set_enabled(value & 0b1 != 0);
        self.pulse_channel_2.set_enabled(value & 0b10 != 0);
        self.triangle_channel.set_enabled(value & 0b100 != 0);
        self.noise_channel.set_enabled(value & 0b1000 != 0);
        self.dmc_channel.set_enabled(value & 0b1_0000 != 0);
    }

    fn read_status_register(&mut self) -> u8 {
        let mut mask = 0u8;
        if self.pulse_channel_1.non_zero_length_counter() {
            mask |= 0b1
        };
        if self.pulse_channel_2.non_zero_length_counter() {
            mask |= 0b10
        };
        if self.triangle_channel.non_zero_length_counter() {
            mask |= 0b100
        };
        if self.noise_channel.non_zero_length_counter() {
            mask |= 0b1000
        };
        // TODO - Read active flag from DMC channel

        // TODO - Set DMC interrupt flag
        if let Some(c) = self.interrupt_triggered_cycles {
            mask |= 0b0100_0000;

            // Don't clear the flag if it was only just set
            if self.total_apu_cycles - c > 1 {
                self.interrupt_triggered_cycles = None;
            }
        }

        info!("Reading APU status register as {:02X}", mask);
        mask
    }

    pub(crate) fn check_trigger_irq(&mut self) -> bool {
        if let Some(c) = self.interrupt_triggered_cycles {
            self.total_apu_cycles - c > 4
        } else {
            false
        }
    }

    pub(crate) fn read_byte(&mut self, address: u16) -> u8 {
        info!("Reading byte from APU registers {:04X}", address);
        match address {
            0x4000..=0x4014 => 0x0, // TODO - what does this return? Open bus or 0?
            0x4015 => self.read_status_register(),
            _ => panic!("Address invalid for APU {:04X}", address),
        }
    }

    pub(crate) fn write_byte(&mut self, address: u16, value: u8) {
        info!("Writing byte to APU registers {:04X}={:02X}", address, value);
        match address {
            0x4000 => self.pulse_channel_1.write_duty_length_halt_envelope_register(value),
            0x4001 => self.pulse_channel_1.load_sweep_register(value),
            0x4002 => self.pulse_channel_1.load_timer_low(value),
            0x4003 => self.pulse_channel_1.load_length_timer_high(value),
            0x4004 => self.pulse_channel_2.write_duty_length_halt_envelope_register(value),
            0x4005 => self.pulse_channel_2.load_sweep_register(value),
            0x4006 => self.pulse_channel_2.load_timer_low(value),
            0x4007 => self.pulse_channel_2.load_length_timer_high(value),
            0x4008 => self.triangle_channel.load_linear_counter(value),
            0x4009 => {} // Unused
            0x400A => self.triangle_channel.load_timer_low(value),
            0x400B => self.triangle_channel.load_length_timer_high(value),
            0x400C => self.noise_channel.write_length_halt_envelope_register(value),
            0x400D => {} // Unused
            0x400E => self.noise_channel.set_mode_and_period(value),
            0x400F => self.noise_channel.load_length_counter(value),
            0x4010 => self.dmc_channel.write_flag_and_rate(value),
            0x4011 => self.dmc_channel.direct_load(value),
            0x4012 => self.dmc_channel.set_sample_address(value),
            0x4013 => self.dmc_channel.set_sample_length(value),
            0x4014 => panic!("4014 isn't mapped to the APU"),
            0x4015 => self.write_status_register(value),
            0x4017 => {
                self.frame_counter.set(value, self.is_apu_cycle);
                if self.frame_counter.inhibit_interrupts {
                    self.interrupt_triggered_cycles = None;
                }

                if self.frame_counter.mode == FrameCounterMode::FiveStep {
                    self.half_frame();
                    self.quarter_frame();
                }
            }
            _ => panic!("Address invalid for APU {:04X}", address),
        }
    }

    fn quarter_frame(&mut self) {
        info!("Running quarter frame update: apu_cycles={}", self.total_apu_cycles);
        self.pulse_channel_1.clock_envelope();
        self.pulse_channel_2.clock_envelope();
        // TODO - Update envelopes on other channels
        self.triangle_channel.clock_linear_counter();
    }

    fn half_frame(&mut self) {
        info!("Running half frame update: apu_cycles={}", self.total_apu_cycles);
        self.quarter_frame();
        self.pulse_channel_1.clock_length_counter();
        self.pulse_channel_2.clock_length_counter();
        self.triangle_channel.clock_length_counter();
        self.noise_channel.clock_length_counter();

        self.pulse_channel_1.clock_sweep_unit();
        self.pulse_channel_2.clock_sweep_unit();
    }
}

impl Iterator for Apu {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        if self.frame_counter.timer_reset_countdown > 0 {
            self.frame_counter.timer_reset_countdown -= 1;
            if self.frame_counter.timer_reset_countdown == 0 {
                self.frame_counter.sequence_cycles = 0;
            }
        }

        if self.is_apu_cycle {
            self.frame_counter.sequence_cycles =
                (self.frame_counter.sequence_cycles + 1) % self.frame_counter.mode.wrapping_number();

            // Note that the timers are not clocked by the frame counter but on every apu cycle
            self.pulse_channel_1.clock_timer();
            self.pulse_channel_2.clock_timer();
            self.noise_channel.clock_timer();

            if !self.frame_counter.inhibit_interrupts
                && self.frame_counter.sequence_cycles == 0
                && self.frame_counter.mode == FrameCounterMode::FourStep
            {
                info!("Triggering APU IRQ at apu cycle {}", self.total_apu_cycles);
                self.interrupt_triggered_cycles = Some(self.total_apu_cycles);
            }

            self.total_apu_cycles = self.total_apu_cycles.wrapping_add(1);
        } else {
            // Note that the clocking here actually occurs on the NON APU cycle deliberately
            match self.frame_counter.sequence_cycles {
                3729 => self.quarter_frame(),
                7457 => self.half_frame(),
                11186 => self.quarter_frame(),
                0 => self.half_frame(),
                _ => (),
            };
        }

        // Note this is clocked on all CPU cycles
        self.triangle_channel.clock_timer();

        // Every other cycle is an APU cycle (as clocked by the CPU)
        self.is_apu_cycle = !self.is_apu_cycle;

        // Apu never stops clocking
        None
    }
}
