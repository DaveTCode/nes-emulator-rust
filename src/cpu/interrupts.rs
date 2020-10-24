#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone)]
pub(crate) enum Interrupt {
    NMI(u32),
    IRQ(u32),
    IRQ_BRK(u32),
    RESET(u32),
}

impl Interrupt {
    pub(super) fn offset(&self) -> u16 {
        match self {
            Interrupt::NMI(_) => 0xFFFA,
            Interrupt::IRQ(_) => 0xFFFE,
            Interrupt::IRQ_BRK(_) => 0xFFFE,
            Interrupt::RESET(_) => 0xFFFC,
        }
    }
}
