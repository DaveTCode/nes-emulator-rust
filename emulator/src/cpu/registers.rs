use cpu::status_flags::StatusFlags;

#[derive(Debug)]
pub(super) struct Registers {
    // Accumulator
    pub(super) a: u8,

    // X, Y - index registers
    pub(super) x: u8,
    pub(super) y: u8,

    pub(super) stack_pointer: u8,
    pub(super) program_counter: u16,
    pub(super) status_register: StatusFlags,
}

impl Registers {
    pub(super) fn new(pc: u16) -> Self {
        Registers {
            a: 0x0,
            x: 0x0,
            y: 0x0,
            stack_pointer: 0xFD,
            status_register: StatusFlags::INTERRUPT_DISABLE_FLAG,
            program_counter: pc,
        }
    }
}
