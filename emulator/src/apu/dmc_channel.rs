const RATE_TABLE: [u16; 0x10] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

#[derive(Debug)]
struct DmcOutputUnit {
    shift_register: u8,
    bits_remaining_counter: u8,
    output_level: u8,
    silence_flag: bool,
}

#[derive(Debug)]
pub(super) struct DmcChannel {
    enabled: bool,
    /// The rate determines for how many CPU cycles happen between changes in the output level
    /// during automatic delta-encoded sample playback. For example, on NTSC (1.789773 MHz),
    /// a rate of 428 gives a frequency of 1789773/428 Hz = 4181.71 Hz. These periods are all
    /// even numbers because there are 2 CPU cycles in an APU cycle.
    /// A rate of 428 means the output level changes every 214 APU cycles.
    rate: u16,
    timer_countdown: u16,
    /// Whether an IRQ is triggered when there are 0 bytes remaining and the DMC is not looping
    irq_enabled_flag: bool,
    /// Set when an IRQ is triggered to track
    irq_flag: bool,
    /// Indicates whether the DMC will loop through samples or play them once
    loop_flag: bool,
    output_unit: DmcOutputUnit,
    /// The address in memory where the samples will be read from
    sample_address: u16,
    /// The number of bytes read from memory
    sample_length: u16,
}

impl DmcChannel {
    pub(super) fn new() -> Self {
        DmcChannel {
            enabled: false,
            rate: RATE_TABLE[0],
            timer_countdown: RATE_TABLE[0],
            irq_enabled_flag: false,
            irq_flag: false,
            loop_flag: false,
            output_unit: DmcOutputUnit {
                shift_register: 0,
                bits_remaining_counter: 8,
                output_level: 0,
                silence_flag: true,
            },
            sample_address: 0xC000,
            sample_length: 1,
        }
    }

    /// Corresponds to 0x4010 on CPU address bus
    pub(super) fn write_flag_and_rate(&mut self, value: u8) {
        self.irq_enabled_flag = value & 0b1000_0000 == 0b1000_0000;
        if !self.irq_enabled_flag {
            self.irq_flag = false;
        }
        self.loop_flag = value & 0b0100_0000 == 0b0100_0000;
        self.rate = RATE_TABLE[value as usize & 0b1111];
    }

    /// Corresponds to 0x4011 on CPU address bus
    /// The DMC output level is set to an unsigned value. If the timer is outputting a clock at the same time,
    /// the output level is occasionally not changed properly (c.f. http://forums.nesdev.com/viewtopic.php?p=104491#p104491)
    pub(super) fn direct_load(&mut self, value: u8) {
        self.output_unit.output_level = value & 0b0111_1111;
    }

    pub(super) fn set_sample_address(&mut self, value: u8) {
        self.sample_address = value as u16 * 64 + 0xC000;
    }

    pub(super) fn set_sample_length(&mut self, value: u8) {
        self.sample_length = value as u16 * 16 + 1;
    }

    pub(super) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub(super) fn clock_timer(&mut self) {
        // TODO
    }

    pub(super) fn mixer_value(&self) -> u8 {
        // TODO
        0
    }
}
