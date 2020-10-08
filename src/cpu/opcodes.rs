use cpu::interrupts::Interrupt;
use cpu::status_flags::StatusFlags;
use cpu::Cpu;
use cpu::CpuState;
use cpu::InterruptState;
use cpu::State;
use log::error;

#[derive(Debug)]
pub(super) struct Opcode {
    pub(super) opcode: u8,
    pub(super) operation: Operation,
    pub(super) address_mode: AddressingMode,
    is_illegal: bool,
}

impl Opcode {
    pub(super) fn nes_test_log(&self, pc_1: u8, pc_2: u8) -> String {
        match self.address_mode.instruction_length() {
            InstructionLength::OneByte => format!(
                "{:02X}       {:}{:?} {:027}",
                self.opcode,
                if self.is_illegal { "*" } else { " " },
                self.operation,
                " "
            ),
            InstructionLength::TwoByte => format!(
                "{:02X} {:02X}    {:}{:?} {:027}",
                self.opcode,
                pc_1,
                if self.is_illegal { "*" } else { " " },
                self.operation,
                " "
            ),
            InstructionLength::ThreeByte => format!(
                "{:02X} {:02X} {:02X} {:}{:?} {:027}",
                self.opcode,
                pc_1,
                pc_2,
                if self.is_illegal { "*" } else { " " },
                self.operation,
                " "
            ),
        }
    }

    pub(super) fn execute(
        &self,
        cpu: &mut Cpu,
        operand: Option<u8>,
        address: Option<u16>,
    ) -> State {
        match self.operation {
            Operation::ADC => {
                cpu.adc(operand.unwrap());
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::AHX => {
                todo!();
            }
            Operation::ALR => {
                todo!();
            }
            Operation::ANC => {
                todo!();
            }
            Operation::AND => {
                cpu.registers.a &= operand.unwrap();
                cpu.set_negative_zero_flags(cpu.registers.a);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::ARR => {
                todo!();
            }
            Operation::ASL => {
                let result = operand.unwrap() << 1;
                cpu.registers
                    .status_register
                    .set(StatusFlags::CARRY_FLAG, operand.unwrap() & 0b1000_0000 != 0);
                cpu.set_negative_zero_flags(result);

                match self.address_mode {
                    AddressingMode::Accumulator => {
                        cpu.registers.a = result;
                        State::CpuState(CpuState::FetchOpcode)
                    }
                    _ => State::CpuState(CpuState::WritingResult {
                        address: address.unwrap(),
                        value: result,
                        dummy: true,
                    }),
                }
            }
            Operation::AXS => {
                todo!();
            }
            Operation::BCC
            | Operation::BCS
            | Operation::BEQ
            | Operation::BMI
            | Operation::BNE
            | Operation::BPL
            | Operation::BVC
            | Operation::BVS => State::CpuState(CpuState::SetProgramCounter {
                address: address.unwrap(),
            }),
            Operation::BIT => {
                let result = cpu.registers.a & operand.unwrap();
                cpu.registers
                    .status_register
                    .set(StatusFlags::ZERO_FLAG, result == 0);
                cpu.registers.status_register.set(
                    StatusFlags::OVERFLOW_FLAG,
                    operand.unwrap() & 0b0100_0000 != 0,
                );
                cpu.registers.status_register.set(
                    StatusFlags::NEGATIVE_FLAG,
                    operand.unwrap() & 0b1000_0000 != 0,
                );
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::BRK => State::InterruptState(InterruptState::PushPCH(Interrupt::IRQ_BRK)),
            Operation::CLC => {
                cpu.registers
                    .status_register
                    .remove(StatusFlags::CARRY_FLAG);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::CLD => {
                cpu.registers
                    .status_register
                    .remove(StatusFlags::DECIMAL_FLAG);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::CLI => {
                cpu.registers
                    .status_register
                    .remove(StatusFlags::INTERRUPT_DISABLE_FLAG);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::CLV => {
                cpu.registers
                    .status_register
                    .remove(StatusFlags::OVERFLOW_FLAG);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::CMP => {
                cpu.compare(operand.unwrap(), cpu.registers.a);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::CPX => {
                cpu.compare(operand.unwrap(), cpu.registers.x);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::CPY => {
                cpu.compare(operand.unwrap(), cpu.registers.y);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::DCP => {
                let result = cpu.decrement(operand.unwrap());
                cpu.compare(result, cpu.registers.a);
                State::CpuState(CpuState::WritingResult {
                    value: result,
                    address: address.unwrap(),
                    dummy: true,
                })
            }
            Operation::DEC => {
                let result = cpu.decrement(operand.unwrap());

                match self.address_mode {
                    AddressingMode::Accumulator => {
                        cpu.registers.a = result;
                        State::CpuState(CpuState::FetchOpcode)
                    }
                    _ => State::CpuState(CpuState::WritingResult {
                        address: address.unwrap(),
                        value: result,
                        dummy: true,
                    }),
                }
            }
            Operation::DEX => {
                cpu.registers.x = cpu.decrement(cpu.registers.x);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::DEY => {
                cpu.registers.y = cpu.decrement(cpu.registers.y);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::EOR => {
                cpu.registers.a ^= operand.unwrap();
                cpu.set_negative_zero_flags(cpu.registers.a);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::INC => {
                let result = cpu.increment(operand.unwrap());

                match self.address_mode {
                    AddressingMode::Accumulator => {
                        cpu.registers.a = result;
                        State::CpuState(CpuState::FetchOpcode)
                    }
                    _ => State::CpuState(CpuState::WritingResult {
                        address: address.unwrap(),
                        value: result,
                        dummy: true,
                    }),
                }
            }
            Operation::INX => {
                cpu.registers.x = cpu.increment(cpu.registers.x);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::INY => {
                cpu.registers.y = cpu.increment(cpu.registers.y);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::ISB => {
                let result = cpu.increment(operand.unwrap());
                cpu.adc(!result);

                State::CpuState(CpuState::WritingResult {
                    value: result,
                    address: address.unwrap(),
                    dummy: true,
                })
            }
            Operation::JMP => {
                cpu.registers.program_counter = address.unwrap();
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::JSR => State::CpuState(CpuState::WritePCHToStack {
                address: address.unwrap(),
            }),
            Operation::KIL => {
                // Illegal opcode - KIL
                error!("KIL opcode");
                panic!();
            }
            Operation::LAS => {
                todo!();
            }
            Operation::LAX => {
                cpu.registers.a = operand.unwrap();
                cpu.registers.x = operand.unwrap();
                cpu.set_negative_zero_flags(cpu.registers.a);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::LDA => {
                cpu.registers.a = operand.unwrap();
                cpu.set_negative_zero_flags(cpu.registers.a);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::LDX => {
                cpu.registers.x = operand.unwrap();
                cpu.set_negative_zero_flags(cpu.registers.x);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::LDY => {
                cpu.registers.y = operand.unwrap();
                cpu.set_negative_zero_flags(cpu.registers.y);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::LSR => {
                let result = operand.unwrap() >> 1;
                cpu.registers
                    .status_register
                    .set(StatusFlags::CARRY_FLAG, operand.unwrap() & 1 == 1);
                cpu.set_negative_zero_flags(result);

                match self.address_mode {
                    AddressingMode::Accumulator => {
                        cpu.registers.a = result;
                        State::CpuState(CpuState::FetchOpcode)
                    }
                    _ => State::CpuState(CpuState::WritingResult {
                        address: address.unwrap(),
                        value: result,
                        dummy: true,
                    }),
                }
            }
            Operation::NOP => State::CpuState(CpuState::FetchOpcode),
            Operation::ORA => {
                cpu.registers.a |= operand.unwrap();
                cpu.set_negative_zero_flags(cpu.registers.a);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::PHA => State::CpuState(CpuState::PushRegisterOnStack {
                value: cpu.registers.a,
            }),
            Operation::PHP => {
                // Note that we mask bits 4, 5 here as this is called from an instruction
                State::CpuState(CpuState::PushRegisterOnStack {
                    value: cpu.registers.status_register.bits() | 0b0011_0000,
                })
            }
            Operation::PLA => State::CpuState(CpuState::PreIncrementStackPointer {
                operation: self.operation,
            }),
            Operation::PLP => State::CpuState(CpuState::PreIncrementStackPointer {
                operation: self.operation,
            }),
            Operation::RLA => {
                let mut result = operand.unwrap() << 1;
                if cpu
                    .registers
                    .status_register
                    .contains(StatusFlags::CARRY_FLAG)
                {
                    result |= 1;
                }
                cpu.registers
                    .status_register
                    .set(StatusFlags::CARRY_FLAG, operand.unwrap() & 0b1000_0000 != 0);
                cpu.registers.a &= result;
                cpu.set_negative_zero_flags(cpu.registers.a);

                match self.address_mode {
                    AddressingMode::Accumulator => {
                        cpu.registers.a = result;
                        State::CpuState(CpuState::FetchOpcode)
                    }
                    _ => State::CpuState(CpuState::WritingResult {
                        address: address.unwrap(),
                        value: result,
                        dummy: true,
                    }),
                }
            }
            Operation::ROL => {
                let mut result = operand.unwrap() << 1;
                if cpu
                    .registers
                    .status_register
                    .contains(StatusFlags::CARRY_FLAG)
                {
                    result |= 1;
                }
                cpu.registers
                    .status_register
                    .set(StatusFlags::CARRY_FLAG, operand.unwrap() & 0b1000_0000 != 0);
                cpu.set_negative_zero_flags(result);

                match self.address_mode {
                    AddressingMode::Accumulator => {
                        cpu.registers.a = result;
                        State::CpuState(CpuState::FetchOpcode)
                    }
                    _ => State::CpuState(CpuState::WritingResult {
                        address: address.unwrap(),
                        value: result,
                        dummy: true,
                    }),
                }
            }
            Operation::ROR => {
                let mut result = operand.unwrap() >> 1;
                if cpu
                    .registers
                    .status_register
                    .contains(StatusFlags::CARRY_FLAG)
                {
                    result |= 0b1000_0000;
                }
                cpu.registers
                    .status_register
                    .set(StatusFlags::CARRY_FLAG, operand.unwrap() & 1 == 1);
                cpu.set_negative_zero_flags(result);

                match self.address_mode {
                    AddressingMode::Accumulator => {
                        cpu.registers.a = result;
                        State::CpuState(CpuState::FetchOpcode)
                    }
                    _ => State::CpuState(CpuState::WritingResult {
                        address: address.unwrap(),
                        value: result,
                        dummy: true,
                    }),
                }
            }
            Operation::RRA => {
                let mut result = operand.unwrap() >> 1;
                if cpu
                    .registers
                    .status_register
                    .contains(StatusFlags::CARRY_FLAG)
                {
                    result |= 0b1000_0000;
                }
                cpu.registers
                    .status_register
                    .set(StatusFlags::CARRY_FLAG, operand.unwrap() & 1 == 1);
                cpu.adc(result);

                match self.address_mode {
                    AddressingMode::Accumulator => {
                        cpu.registers.a = result;
                        State::CpuState(CpuState::FetchOpcode)
                    }
                    _ => State::CpuState(CpuState::WritingResult {
                        address: address.unwrap(),
                        value: result,
                        dummy: true,
                    }),
                }
            }
            Operation::RTI => State::CpuState(CpuState::PreIncrementStackPointer {
                operation: self.operation,
            }),
            Operation::RTS => State::CpuState(CpuState::PreIncrementStackPointer {
                operation: self.operation,
            }),
            Operation::SAX => State::CpuState(CpuState::WritingResult {
                value: cpu.registers.a & cpu.registers.x,
                address: address.unwrap(),
                dummy: false,
            }),
            Operation::SBC => {
                cpu.adc(!operand.unwrap());
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::SEC => {
                cpu.registers
                    .status_register
                    .insert(StatusFlags::CARRY_FLAG);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::SED => {
                cpu.registers
                    .status_register
                    .insert(StatusFlags::DECIMAL_FLAG);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::SEI => {
                cpu.registers
                    .status_register
                    .insert(StatusFlags::INTERRUPT_DISABLE_FLAG);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::SHX => {
                todo!();
            }
            Operation::SHY => {
                todo!();
            }
            Operation::SLO => {
                let result = operand.unwrap() << 1;
                cpu.registers
                    .status_register
                    .set(StatusFlags::CARRY_FLAG, operand.unwrap() & 0b1000_0000 != 0);
                cpu.registers.a |= result;
                cpu.set_negative_zero_flags(cpu.registers.a);

                State::CpuState(CpuState::WritingResult {
                    value: result,
                    address: address.unwrap(),
                    dummy: true,
                })
            }
            Operation::SRE => {
                let result = operand.unwrap() >> 1;
                cpu.registers
                    .status_register
                    .set(StatusFlags::CARRY_FLAG, operand.unwrap() & 1 == 1);
                cpu.registers.a ^= result;
                cpu.set_negative_zero_flags(cpu.registers.a);

                State::CpuState(CpuState::WritingResult {
                    address: address.unwrap(),
                    value: result,
                    dummy: true,
                })
            }
            Operation::STA => State::CpuState(CpuState::WritingResult {
                value: cpu.registers.a,
                address: address.unwrap(),
                dummy: false,
            }),
            Operation::STX => State::CpuState(CpuState::WritingResult {
                value: cpu.registers.x,
                address: address.unwrap(),
                dummy: false,
            }),
            Operation::STY => State::CpuState(CpuState::WritingResult {
                value: cpu.registers.y,
                address: address.unwrap(),
                dummy: false,
            }),
            Operation::TAS => {
                todo!();
            }
            Operation::TAX => {
                cpu.registers.x = cpu.registers.a;
                cpu.set_negative_zero_flags(cpu.registers.x);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::TAY => {
                cpu.registers.y = cpu.registers.a;
                cpu.set_negative_zero_flags(cpu.registers.y);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::TSX => {
                cpu.registers.x = cpu.registers.stack_pointer;
                cpu.set_negative_zero_flags(cpu.registers.x);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::TXA => {
                cpu.registers.a = cpu.registers.x;
                cpu.set_negative_zero_flags(cpu.registers.a);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::TXS => {
                cpu.registers.stack_pointer = cpu.registers.x;
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::TYA => {
                cpu.registers.a = cpu.registers.y;
                cpu.set_negative_zero_flags(cpu.registers.a);
                State::CpuState(CpuState::FetchOpcode)
            }
            Operation::XAA => {
                todo!();
            }
        }
    }
}

#[derive(Debug)]
pub(super) enum InstructionType {
    Read,
    ReadModifyWrite,
    Write,
    Branch,
    Jump,
    Stack,
}

#[derive(Debug)]
pub(super) enum InstructionLength {
    OneByte,
    TwoByte,
    ThreeByte,
}

#[derive(Debug, Copy, Clone)]
pub(super) enum AddressingMode {
    Accumulator,
    Absolute,
    AbsoluteXIndexed,
    AbsoluteYIndexed,
    Immediate,
    Implied,
    Indirect,
    IndirectXIndexed,
    IndirectYIndexed,
    Relative,
    ZeroPage,
    ZeroPageXIndexed,
    ZeroPageYIndexed,
}

impl AddressingMode {
    pub(super) fn instruction_length(&self) -> InstructionLength {
        match self {
            AddressingMode::Accumulator => InstructionLength::OneByte,
            AddressingMode::Absolute => InstructionLength::ThreeByte,
            AddressingMode::AbsoluteXIndexed => InstructionLength::ThreeByte,
            AddressingMode::AbsoluteYIndexed => InstructionLength::ThreeByte,
            AddressingMode::Immediate => InstructionLength::TwoByte,
            AddressingMode::Implied => InstructionLength::OneByte,
            AddressingMode::Indirect => InstructionLength::ThreeByte,
            AddressingMode::IndirectXIndexed => InstructionLength::TwoByte,
            AddressingMode::IndirectYIndexed => InstructionLength::TwoByte,
            AddressingMode::Relative => InstructionLength::TwoByte,
            AddressingMode::ZeroPage => InstructionLength::TwoByte,
            AddressingMode::ZeroPageXIndexed => InstructionLength::TwoByte,
            AddressingMode::ZeroPageYIndexed => InstructionLength::TwoByte,
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub(super) enum Operation {
    ADC,
    AHX,
    ALR,
    ANC,
    AND,
    ARR,
    ASL,
    AXS,
    BCC,
    BCS,
    BEQ,
    BIT,
    BMI,
    BNE,
    BPL,
    BRK,
    BVC,
    BVS,
    CLC,
    CLD,
    CLI,
    CLV,
    CMP,
    CPX,
    CPY,
    DCP,
    DEC,
    DEX,
    DEY,
    EOR,
    INC,
    INX,
    INY,
    ISB,
    JMP,
    JSR,
    KIL,
    LAS,
    LAX,
    LDA,
    LDX,
    LDY,
    LSR,
    NOP,
    ORA,
    PHA,
    PHP,
    PLA,
    PLP,
    RLA,
    ROL,
    ROR,
    RRA,
    RTI,
    RTS,
    SAX,
    SBC,
    SEC,
    SED,
    SEI,
    SHX,
    SHY,
    SLO,
    SRE,
    STA,
    STX,
    STY,
    TAS,
    TAX,
    TAY,
    TSX,
    TXA,
    TXS,
    TYA,
    XAA,
}

impl Operation {
    pub(super) fn instruction_type(&self) -> InstructionType {
        match self {
            Operation::JMP | Operation::JSR => InstructionType::Jump,
            Operation::STA | Operation::STX | Operation::STY | Operation::SAX => {
                InstructionType::Write
            }
            Operation::ASL
            | Operation::LSR
            | Operation::ROL
            | Operation::ROR
            | Operation::INC
            | Operation::DEC
            | Operation::SLO
            | Operation::SRE
            | Operation::RLA
            | Operation::RRA
            | Operation::ISB
            | Operation::DCP => InstructionType::ReadModifyWrite,
            Operation::LDA
            | Operation::LDX
            | Operation::LDY
            | Operation::EOR
            | Operation::AND
            | Operation::ORA
            | Operation::ADC
            | Operation::SBC
            | Operation::CMP
            | Operation::CPX
            | Operation::CPY
            | Operation::BIT
            | Operation::LAX
            | Operation::NOP => InstructionType::Read,
            Operation::BCC
            | Operation::BCS
            | Operation::BNE
            | Operation::BEQ
            | Operation::BPL
            | Operation::BMI
            | Operation::BVC
            | Operation::BVS => InstructionType::Branch,
            Operation::BRK
            | Operation::RTI
            | Operation::RTS
            | Operation::PHA
            | Operation::PHP
            | Operation::PLA
            | Operation::PLP => InstructionType::Stack,
            _ => panic!("Have not yet determined instruction type for {:?}", self),
        }
    }
}

pub(super) const OPCODE_TABLE: [Opcode; 0x100] = [
    // 0x00-0x0F
    Opcode {
        opcode: 0x00,
        operation: Operation::BRK,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x01,
        operation: Operation::ORA,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x02,
        operation: Operation::KIL,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x03,
        operation: Operation::SLO,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x04,
        operation: Operation::NOP,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x05,
        operation: Operation::ORA,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x06,
        operation: Operation::ASL,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x07,
        operation: Operation::SLO,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x08,
        operation: Operation::PHP,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x09,
        operation: Operation::ORA,
        address_mode: AddressingMode::Immediate,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x0A,
        operation: Operation::ASL,
        address_mode: AddressingMode::Accumulator,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x0B,
        operation: Operation::ANC,
        address_mode: AddressingMode::Immediate,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x0C,
        operation: Operation::NOP,
        address_mode: AddressingMode::Absolute,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x0D,
        operation: Operation::ORA,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x0E,
        operation: Operation::ASL,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x0F,
        operation: Operation::SLO,
        address_mode: AddressingMode::Absolute,
        is_illegal: true,
    },
    // 0x10-0x1F
    Opcode {
        opcode: 0x10,
        operation: Operation::BPL,
        address_mode: AddressingMode::Relative,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x11,
        operation: Operation::ORA,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x12,
        operation: Operation::KIL,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x13,
        operation: Operation::SLO,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x14,
        operation: Operation::NOP,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x15,
        operation: Operation::ORA,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x16,
        operation: Operation::ASL,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x17,
        operation: Operation::SLO,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x18,
        operation: Operation::CLC,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x19,
        operation: Operation::ORA,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x1A,
        operation: Operation::NOP,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x1B,
        operation: Operation::SLO,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x1C,
        operation: Operation::NOP,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x1D,
        operation: Operation::ORA,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x1E,
        operation: Operation::ASL,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x1F,
        operation: Operation::SLO,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: true,
    },
    // 0x20-0x2F
    Opcode {
        opcode: 0x20,
        operation: Operation::JSR,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x21,
        operation: Operation::AND,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x22,
        operation: Operation::KIL,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x23,
        operation: Operation::RLA,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x24,
        operation: Operation::BIT,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x25,
        operation: Operation::AND,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x26,
        operation: Operation::ROL,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x27,
        operation: Operation::RLA,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x28,
        operation: Operation::PLP,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x29,
        operation: Operation::AND,
        address_mode: AddressingMode::Immediate,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x2A,
        operation: Operation::ROL,
        address_mode: AddressingMode::Accumulator,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x2B,
        operation: Operation::ANC,
        address_mode: AddressingMode::Immediate,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x2C,
        operation: Operation::BIT,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x2D,
        operation: Operation::AND,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x2E,
        operation: Operation::ROL,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x2F,
        operation: Operation::RLA,
        address_mode: AddressingMode::Absolute,
        is_illegal: true,
    },
    // 0x30-0x3F
    Opcode {
        opcode: 0x30,
        operation: Operation::BMI,
        address_mode: AddressingMode::Relative,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x31,
        operation: Operation::AND,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x32,
        operation: Operation::KIL,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x33,
        operation: Operation::RLA,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x34,
        operation: Operation::NOP,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x35,
        operation: Operation::AND,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x36,
        operation: Operation::ROL,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x37,
        operation: Operation::RLA,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x38,
        operation: Operation::SEC,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x39,
        operation: Operation::AND,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x3A,
        operation: Operation::NOP,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x3B,
        operation: Operation::RLA,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x3C,
        operation: Operation::NOP,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x3D,
        operation: Operation::AND,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x3E,
        operation: Operation::ROL,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x3F,
        operation: Operation::RLA,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: true,
    },
    // 0x40-0x4F
    Opcode {
        opcode: 0x40,
        operation: Operation::RTI,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x41,
        operation: Operation::EOR,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x42,
        operation: Operation::KIL,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x43,
        operation: Operation::SRE,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x44,
        operation: Operation::NOP,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x45,
        operation: Operation::EOR,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x46,
        operation: Operation::LSR,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x47,
        operation: Operation::SRE,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x48,
        operation: Operation::PHA,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x49,
        operation: Operation::EOR,
        address_mode: AddressingMode::Immediate,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x4A,
        operation: Operation::LSR,
        address_mode: AddressingMode::Accumulator,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x4B,
        operation: Operation::ALR,
        address_mode: AddressingMode::Immediate,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x4C,
        operation: Operation::JMP,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x4D,
        operation: Operation::EOR,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x4E,
        operation: Operation::LSR,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x4F,
        operation: Operation::SRE,
        address_mode: AddressingMode::Absolute,
        is_illegal: true,
    },
    // 0x50-0x5F
    Opcode {
        opcode: 0x50,
        operation: Operation::BVC,
        address_mode: AddressingMode::Relative,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x51,
        operation: Operation::EOR,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x52,
        operation: Operation::KIL,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x53,
        operation: Operation::SRE,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x54,
        operation: Operation::NOP,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x55,
        operation: Operation::EOR,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x56,
        operation: Operation::LSR,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x57,
        operation: Operation::SRE,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x58,
        operation: Operation::CLI,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x59,
        operation: Operation::EOR,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x5A,
        operation: Operation::NOP,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x5B,
        operation: Operation::SRE,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x5C,
        operation: Operation::NOP,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x5D,
        operation: Operation::EOR,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x5E,
        operation: Operation::LSR,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x5F,
        operation: Operation::SRE,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: true,
    },
    // 0x60-0x6F
    Opcode {
        opcode: 0x60,
        operation: Operation::RTS,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x61,
        operation: Operation::ADC,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x62,
        operation: Operation::KIL,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x63,
        operation: Operation::RRA,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x64,
        operation: Operation::NOP,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x65,
        operation: Operation::ADC,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x66,
        operation: Operation::ROR,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x67,
        operation: Operation::RRA,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x68,
        operation: Operation::PLA,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x69,
        operation: Operation::ADC,
        address_mode: AddressingMode::Immediate,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x6A,
        operation: Operation::ROR,
        address_mode: AddressingMode::Accumulator,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x6B,
        operation: Operation::ARR,
        address_mode: AddressingMode::Immediate,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x6C,
        operation: Operation::JMP,
        address_mode: AddressingMode::Indirect,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x6D,
        operation: Operation::ADC,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x6E,
        operation: Operation::ROR,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x6F,
        operation: Operation::RRA,
        address_mode: AddressingMode::Absolute,
        is_illegal: true,
    },
    // 0x70-0x7F
    Opcode {
        opcode: 0x70,
        operation: Operation::BVS,
        address_mode: AddressingMode::Relative,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x71,
        operation: Operation::ADC,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x72,
        operation: Operation::KIL,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x73,
        operation: Operation::RRA,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x74,
        operation: Operation::NOP,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x75,
        operation: Operation::ADC,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x76,
        operation: Operation::ROR,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x77,
        operation: Operation::RRA,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x78,
        operation: Operation::SEI,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x79,
        operation: Operation::ADC,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x7A,
        operation: Operation::NOP,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x7B,
        operation: Operation::RRA,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x7C,
        operation: Operation::NOP,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x7D,
        operation: Operation::ADC,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x7E,
        operation: Operation::ROR,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x7F,
        operation: Operation::RRA,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: true,
    },
    // 0x80-0x8F
    Opcode {
        opcode: 0x80,
        operation: Operation::NOP,
        address_mode: AddressingMode::Immediate,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x81,
        operation: Operation::STA,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x82,
        operation: Operation::NOP,
        address_mode: AddressingMode::Immediate,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x83,
        operation: Operation::SAX,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x84,
        operation: Operation::STY,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x85,
        operation: Operation::STA,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x86,
        operation: Operation::STX,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x87,
        operation: Operation::SAX,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x88,
        operation: Operation::DEY,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x89,
        operation: Operation::NOP,
        address_mode: AddressingMode::Immediate,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x8A,
        operation: Operation::TXA,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x8B,
        operation: Operation::XAA,
        address_mode: AddressingMode::Immediate,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x8C,
        operation: Operation::STY,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x8D,
        operation: Operation::STA,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x8E,
        operation: Operation::STX,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x8F,
        operation: Operation::SAX,
        address_mode: AddressingMode::Absolute,
        is_illegal: true,
    },
    // 0x90-0x9F
    Opcode {
        opcode: 0x90,
        operation: Operation::BCC,
        address_mode: AddressingMode::Relative,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x91,
        operation: Operation::STA,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x92,
        operation: Operation::KIL,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x93,
        operation: Operation::AHX,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x94,
        operation: Operation::STY,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x95,
        operation: Operation::STA,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x96,
        operation: Operation::STX,
        address_mode: AddressingMode::ZeroPageYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x97,
        operation: Operation::SAX,
        address_mode: AddressingMode::ZeroPageYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x98,
        operation: Operation::TYA,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x99,
        operation: Operation::STA,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x9A,
        operation: Operation::TXS,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x9B,
        operation: Operation::TAS,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x9C,
        operation: Operation::SHY,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x9D,
        operation: Operation::STA,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0x9E,
        operation: Operation::SHX,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0x9F,
        operation: Operation::AHX,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: true,
    },
    // 0xA0-0xAF
    Opcode {
        opcode: 0xA0,
        operation: Operation::LDY,
        address_mode: AddressingMode::Immediate,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xA1,
        operation: Operation::LDA,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xA2,
        operation: Operation::LDX,
        address_mode: AddressingMode::Immediate,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xA3,
        operation: Operation::LAX,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xA4,
        operation: Operation::LDY,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xA5,
        operation: Operation::LDA,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xA6,
        operation: Operation::LDX,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xA7,
        operation: Operation::LAX,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xA8,
        operation: Operation::TAY,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xA9,
        operation: Operation::LDA,
        address_mode: AddressingMode::Immediate,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xAA,
        operation: Operation::TAX,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xAB,
        operation: Operation::LAX,
        address_mode: AddressingMode::Immediate,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xAC,
        operation: Operation::LDY,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xAD,
        operation: Operation::LDA,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xAE,
        operation: Operation::LDX,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xAF,
        operation: Operation::LAX,
        address_mode: AddressingMode::Absolute,
        is_illegal: true,
    },
    // 0xB0-0xBF
    Opcode {
        opcode: 0xB0,
        operation: Operation::BCS,
        address_mode: AddressingMode::Relative,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xB1,
        operation: Operation::LDA,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xB2,
        operation: Operation::KIL,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xB3,
        operation: Operation::LAX,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xB4,
        operation: Operation::LDY,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xB5,
        operation: Operation::LDA,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xB6,
        operation: Operation::LDX,
        address_mode: AddressingMode::ZeroPageYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xB7,
        operation: Operation::LAX,
        address_mode: AddressingMode::ZeroPageYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xB8,
        operation: Operation::CLV,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xB9,
        operation: Operation::LDA,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xBA,
        operation: Operation::TSX,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xBB,
        operation: Operation::LAS,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xBC,
        operation: Operation::LDY,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xBD,
        operation: Operation::LDA,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xBE,
        operation: Operation::LDX,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xBF,
        operation: Operation::LAX,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: true,
    },
    // 0xC0-0xCF
    Opcode {
        opcode: 0xC0,
        operation: Operation::CPY,
        address_mode: AddressingMode::Immediate,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xC1,
        operation: Operation::CMP,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xC2,
        operation: Operation::NOP,
        address_mode: AddressingMode::Immediate,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xC3,
        operation: Operation::DCP,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xC4,
        operation: Operation::CPY,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xC5,
        operation: Operation::CMP,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xC6,
        operation: Operation::DEC,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xC7,
        operation: Operation::DCP,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xC8,
        operation: Operation::INY,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xC9,
        operation: Operation::CMP,
        address_mode: AddressingMode::Immediate,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xCA,
        operation: Operation::DEX,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xCB,
        operation: Operation::AXS,
        address_mode: AddressingMode::Immediate,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xCC,
        operation: Operation::CPY,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xCD,
        operation: Operation::CMP,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xCE,
        operation: Operation::DEC,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xCF,
        operation: Operation::DCP,
        address_mode: AddressingMode::Absolute,
        is_illegal: true,
    },
    // 0xD0-0xDF
    Opcode {
        opcode: 0xD0,
        operation: Operation::BNE,
        address_mode: AddressingMode::Relative,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xD1,
        operation: Operation::CMP,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xD2,
        operation: Operation::KIL,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xD3,
        operation: Operation::DCP,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xD4,
        operation: Operation::NOP,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xD5,
        operation: Operation::CMP,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xD6,
        operation: Operation::DEC,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xD7,
        operation: Operation::DCP,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xD8,
        operation: Operation::CLD,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xD9,
        operation: Operation::CMP,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xDA,
        operation: Operation::NOP,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xDB,
        operation: Operation::DCP,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xDC,
        operation: Operation::NOP,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xDD,
        operation: Operation::CMP,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xDE,
        operation: Operation::DEC,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xDF,
        operation: Operation::DCP,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: true,
    },
    // 0xE0-0xEF
    Opcode {
        opcode: 0xE0,
        operation: Operation::CPX,
        address_mode: AddressingMode::Immediate,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xE1,
        operation: Operation::SBC,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xE2,
        operation: Operation::NOP,
        address_mode: AddressingMode::Immediate,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xE3,
        operation: Operation::ISB,
        address_mode: AddressingMode::IndirectXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xE4,
        operation: Operation::CPX,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xE5,
        operation: Operation::SBC,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xE6,
        operation: Operation::INC,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xE7,
        operation: Operation::ISB,
        address_mode: AddressingMode::ZeroPage,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xE8,
        operation: Operation::INX,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xE9,
        operation: Operation::SBC,
        address_mode: AddressingMode::Immediate,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xEA,
        operation: Operation::NOP,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xEB,
        operation: Operation::SBC,
        address_mode: AddressingMode::Immediate,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xEC,
        operation: Operation::CPX,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xED,
        operation: Operation::SBC,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xEE,
        operation: Operation::INC,
        address_mode: AddressingMode::Absolute,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xEF,
        operation: Operation::ISB,
        address_mode: AddressingMode::Absolute,
        is_illegal: true,
    },
    // 0xF0-0xFF
    Opcode {
        opcode: 0xF0,
        operation: Operation::BEQ,
        address_mode: AddressingMode::Relative,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xF1,
        operation: Operation::SBC,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xF2,
        operation: Operation::KIL,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xF3,
        operation: Operation::ISB,
        address_mode: AddressingMode::IndirectYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xF4,
        operation: Operation::NOP,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xF5,
        operation: Operation::SBC,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xF6,
        operation: Operation::INC,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xF7,
        operation: Operation::ISB,
        address_mode: AddressingMode::ZeroPageXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xF8,
        operation: Operation::SED,
        address_mode: AddressingMode::Implied,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xF9,
        operation: Operation::SBC,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xFA,
        operation: Operation::NOP,
        address_mode: AddressingMode::Implied,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xFB,
        operation: Operation::ISB,
        address_mode: AddressingMode::AbsoluteYIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xFC,
        operation: Operation::NOP,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: true,
    },
    Opcode {
        opcode: 0xFD,
        operation: Operation::SBC,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xFE,
        operation: Operation::INC,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: false,
    },
    Opcode {
        opcode: 0xFF,
        operation: Operation::ISB,
        address_mode: AddressingMode::AbsoluteXIndexed,
        is_illegal: true,
    },
];
