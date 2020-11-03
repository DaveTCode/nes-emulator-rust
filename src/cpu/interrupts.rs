use ppu::PpuCycle;

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone)]
pub(crate) enum Interrupt {
    NMI(PpuCycle),
    IRQ(PpuCycle),
    IRQ_BRK(PpuCycle),
    RESET(PpuCycle),
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
