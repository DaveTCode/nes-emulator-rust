use apu::dmc_channel::DmcChannel;
use apu::noise_channel::NoiseChannel;
use apu::pulse_channel::PulseChannel;
use apu::triangle_channel::TriangleChannel;
use log::info;

mod dmc_channel;
mod noise_channel;
mod pulse_channel;
mod triangle_channel;

#[derive(Debug)]
enum FrameCounterMode {
    FourStep,
    FiveStep,
}

#[derive(Debug)]
struct FrameCounter {
    inhibit_interrupts: bool,
    mode: FrameCounterMode,
    step: u8,
}

impl FrameCounter {
    fn set(&mut self, value: u8) {
        if value & 0b1000_0000 == 0 {
            self.mode = FrameCounterMode::FourStep
        } else {
            self.mode = FrameCounterMode::FiveStep
        }
        self.inhibit_interrupts = value & 0b0100_0000 == 0b0100_0000;
    }
}

pub(crate) struct Apu {
    pulse_channel_1: PulseChannel,
    pulse_channel_2: PulseChannel,
    triangle_channel: TriangleChannel,
    noise_channel: NoiseChannel,
    dmc_channel: DmcChannel,
    frame_counter: FrameCounter,
}

impl Apu {
    pub(crate) fn new() -> Self {
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
            },
        }
    }

    fn write_status_register(&mut self, value: u8) {
        if value & 0b1 == 0 {
            self.pulse_channel_1.disable();
        }
        if value & 0b10 == 0 {
            self.pulse_channel_2.disable();
        }
        if value & 0b100 == 0 {
            self.triangle_channel.disable();
        }
        if value & 0b1000 == 0 {
            self.noise_channel.disable();
        }
        if value & 0b1_0000 == 0 {
            self.dmc_channel.disable();
        }
    }

    fn read_status_register(&self) -> u8 {
        let mut mask = 0u8;
        if self.pulse_channel_1.length_counter > 0 {
            mask |= 0b1
        };
        if self.pulse_channel_2.length_counter > 0 {
            mask |= 0b10
        };
        // TODO - Read length from other channels

        info!("Reading APU status register as {:02X}", mask);
        mask
    }

    pub(crate) fn read_byte(&self, address: u16) -> u8 {
        info!("Reading byte from APU registers {:04X}", address);
        match address {
            0x4000..=0x4014 => 0x0, // TODO
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
            0x4008..=0x4014 => {} // TODO
            0x4015 => self.write_status_register(value),
            0x4017 => {
                // TODO - Various side effects happen here e.g.: clocking components if mode is set to 5 step etc
                self.frame_counter.set(value);
            }
            _ => panic!("Address invalid for APU {:04X}", address),
        }
    }
}

impl Iterator for Apu {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        match self.frame_counter.mode {
            FrameCounterMode::FourStep => {
                if self.frame_counter.step & 1 == 1 {
                    self.pulse_channel_1.clock_length_counter();
                    self.pulse_channel_2.clock_length_counter();
                    self.pulse_channel_1.clock_sweep_unit();
                    self.pulse_channel_2.clock_sweep_unit();

                    if self.frame_counter.step == 3 {
                        // TODO - Check for interrupts that need setting
                    }
                }

                // TODO - Step envelope and linear counter

                self.frame_counter.step = (self.frame_counter.step + 1) & 3;
            }
            FrameCounterMode::FiveStep => {
                if self.frame_counter.step == 1 || self.frame_counter.step == 4 {
                    self.pulse_channel_1.clock_length_counter();
                    self.pulse_channel_2.clock_length_counter();
                    // TODO - Step length counter and sweep unit
                }

                if self.frame_counter.step != 3 {
                    // TODO - Step envelope and linear counter
                }

                self.frame_counter.step = (self.frame_counter.step + 1) % 5;
            }
        }

        self.pulse_channel_1.clock_timer();
        self.pulse_channel_2.clock_timer();

        // Apu never stops clocking
        None
    }
}
