#[derive(Debug, Copy, Clone)]
pub(super) enum Interrupt {
    NMI,
    IRQ,
    RESET,
}

impl Interrupt {
    pub(super) fn offset(&self) -> u16 {
        match self {
            Interrupt::NMI => 0xFFFA,
            Interrupt::IRQ => 0xFFFE,
            Interrupt::RESET => 0xFFFC,
        }
    }
}
