use cpu::status_flags::StatusFlags;

#[derive(Debug)]
pub(crate) struct Registers {
    // Accumulator
    pub(crate) a: u8,

    // X, Y - index registers
    pub(crate) x: u8,
    pub(crate) y: u8,

    pub(crate) stack_pointer: u8,
    pub(crate) program_counter: u16,
    pub(crate) status_register: StatusFlags,
}

impl Default for Registers {
    fn default() -> Self {
        Registers {
            a: 0x0,
            x: 0x0,
            y: 0x0,
            stack_pointer: 0xFD,
            status_register: StatusFlags::INTERRUPT_DISABLE_FLAG,
            program_counter: 0xC000,
        }
    }
}
