pub(crate) mod interrupts;
mod opcodes;
mod registers;
mod status_flags;

use apu::Apu;
use cartridge::CpuCartridgeAddressBus;
use cpu::interrupts::Interrupt;
use cpu::opcodes::Opcode;
use cpu::opcodes::{AddressingMode, InstructionType, Operation, OPCODE_TABLE};
use cpu::registers::Registers;
use cpu::status_flags::StatusFlags;
use io::Button;
use io::Controller;
use io::Io;
use log::{debug, error, info};
use ppu::Ppu;
use ppu::SCREEN_HEIGHT;
use ppu::SCREEN_WIDTH;

#[derive(Debug, Copy, Clone)]
enum State {
    Interrupt(InterruptState),
    Cpu(CpuState),
    Dma(DmaState),
}

#[derive(Debug, Copy, Clone)]
enum DmaState {
    DummyCycle,
    OddCpuCycle,
    ReadCycle,
    WriteCycle(u8),
}

#[derive(Debug, Copy, Clone)]
enum InterruptState {
    InternalOps1(Interrupt),
    InternalOps2(Interrupt),
    PushPCH(Interrupt),
    PushPCL(Interrupt),
    PushStatusRegister(Interrupt),
    PullIRQVecLow(Interrupt),
    PullIRQVecHigh(Interrupt),
}

///
/// Cpu states are used to represent cycles of an instruction
///
#[derive(Debug, Copy, Clone)]
enum CpuState {
    // Cycle 1 is always reading the PC and incrementing it
    FetchOpcode,
    // Cycle 2 always reads the (incremented) PC, but for implied &
    // accumulator modes this value is then discarded and the PC is not
    // incremented
    ThrowawayRead {
        opcode: &'static Opcode,
        operand: Option<u8>,
    },
    // Cycles 2-5 cover reading the operand & address depending on the addressing mode
    ReadingOperand {
        opcode: &'static Opcode,
        address_low_byte: Option<u8>,
        address_high_byte: Option<u8>,
        pointer: Option<u8>,
        indirect_address_low_byte: Option<u8>,
        indirect_address_high_byte: Option<u8>,
        checked_page_boundary: bool,
    },
    BranchCrossesPageBoundary {
        opcode: &'static Opcode,
        address: Option<u16>,
        operand: Option<u8>,
    },
    PushRegisterOnStack {
        value: u8,
    },
    PreIncrementStackPointer {
        operation: Operation,
    },
    PullRegisterFromStack {
        operation: Operation,
    },
    PullPCLFromStack {
        operation: Operation,
    },
    PullPCHFromStack {
        operation: Operation,
        pcl: u8,
    },
    IncrementProgramCounter,
    WritePCHToStack {
        address: u16,
    },
    WritePCLToStack {
        address: u16,
    },
    SetProgramCounter {
        address: u16,
        was_branch_instruction: bool,
    },
    WritingResult {
        address: u16,
        value: u8,
        dummy: bool,
    },
}

pub(crate) type CpuCycle = u32;

pub(crate) struct Cpu<'a> {
    state: State,
    registers: Registers,
    pub(crate) cycles: CpuCycle,
    cpu_cycle_counter: u8,
    ram: [u8; 0x800],
    apu: &'a mut Apu,
    io: &'a mut Io,
    ppu: &'a mut Ppu,
    prg_address_bus: Box<dyn CpuCartridgeAddressBus>,
    trigger_dma: bool,
    dma_address: u16,
    polled_interrupt: Option<Interrupt>,
}

impl<'a> Cpu<'a> {
    pub(crate) fn new(
        prg_address_bus: Box<dyn CpuCartridgeAddressBus>,
        apu: &'a mut Apu,
        io: &'a mut Io,
        ppu: &'a mut Ppu,
    ) -> Self {
        // The processor starts at the RESET interrupt handler address
        let pc = prg_address_bus.read_byte(Interrupt::RESET(0).offset()) as u16
            | ((prg_address_bus.read_byte(Interrupt::RESET(0).offset().wrapping_add(1)) as u16) << 8);

        Cpu {
            state: State::Cpu(CpuState::FetchOpcode),
            registers: Registers::new(pc),
            cycles: 8,
            cpu_cycle_counter: 1,
            ram: [0; 0x800],
            apu,
            io,
            ppu,
            prg_address_bus,
            trigger_dma: false,
            dma_address: 0x0000,
            polled_interrupt: None,
        }
    }

    fn read_byte(&mut self, address: u16) -> u8 {
        debug!("CPU address space read {:04X}", address);

        match address {
            0x0000..=0x1FFF => self.ram[(address & 0x7FF) as usize],
            0x2000..=0x2007 => self.ppu.read_register(address),
            0x2008..=0x3FFF => self.ppu.read_register((address & 7) + 0x2000),
            0x4000..=0x4013 | 0x4015 => self.apu.read_byte(address), // APU registers
            0x4014 => 0x00, // TODO - Is this correct? We read 0 on the DMA register?
            0x4016..=0x4017 => self.io.read_byte(address), // Controller registers
            0x4018..=0x401F => 0x00, // TODO - Unused APU & IO registers
            0x4020..=0xFFFF => self.prg_address_bus.read_byte(address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        debug!("CPU address space write {:04X} = {:02X}", address, value);

        match address {
            0x0000..=0x1FFF => self.ram[(address & 0x7FF) as usize] = value,
            0x2000..=0x2007 => self.ppu.write_register(address, value),
            0x2008..=0x3FFF => self.ppu.write_register((address % 8) + 0x2000, value),
            0x4000..=0x4013 | 0x4015 | 0x4017 => self.apu.write_byte(address, value), // APU registers
            0x4014 => {
                self.dma_address = (value as u16) << 8;
                self.trigger_dma = true;
            } // Trigger DMA
            0x4016 => self.io.write_byte(address, value),                             // IO Register
            0x4018..=0x401F => (), // TODO - Unused APU & IO registers
            0x4020..=0xFFFF => {
                // This is a bit...terrible. In order to avoid dual mutable ownership of the PRG/CHR areas of the cartridge
                // all writes are mirrored between the two (although in practice only relevant writes are handled)
                self.prg_address_bus.write_byte(address, value, self.cycles);
                self.ppu.chr_address_bus.cpu_write_byte(address, value, self.cycles);
            }
        }
    }

    fn nes_test_log(&mut self, opcode: &Opcode) -> String {
        let pc_1 = self.read_byte(self.registers.program_counter);
        let pc_2 = self.read_byte(self.registers.program_counter + 1);
        format!(
            "{:04X}  {:} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:>3},{:>3} CYC:{:}",
            self.registers.program_counter - 1,
            opcode.nes_test_log(pc_1, pc_2),
            self.registers.a,
            self.registers.x,
            self.registers.y,
            self.registers.status_register.bits() | 0b0010_0000,
            self.registers.stack_pointer,
            self.ppu.current_scanline_cycle(),
            self.ppu.current_scanline(),
            self.cycles
        )
    }

    /// This routine simulates checking for IRQ/NMI and happens during the last
    /// cycle of an instruction based on the state of the registers at the
    /// _start_ of that instruction
    fn poll_for_interrupts(&mut self, clear_lines: bool) {
        // NMI takes precedence over an IRQ
        if let Some(interrupt) = self.ppu.check_ppu_nmi(clear_lines) {
            self.polled_interrupt = Some(interrupt);

            info!("Starting NMI interrupt");
        } else if !self
            .registers
            .status_register
            .contains(StatusFlags::INTERRUPT_DISABLE_FLAG)
            && (self.ppu.check_trigger_irq(clear_lines) || self.apu.check_trigger_irq())
        {
            self.polled_interrupt = Some(Interrupt::IRQ(self.cycles * 3));

            info!("Starting IRQ interrupt triggered by PPU");
        }
    }

    fn push_to_stack(&mut self, value: u8) {
        self.write_byte(self.registers.stack_pointer as u16 | 0x0100, value);
        self.registers.stack_pointer = self.registers.stack_pointer.wrapping_sub(1);
    }

    fn pop_from_stack(&mut self) -> u8 {
        self.registers.stack_pointer = self.registers.stack_pointer.wrapping_add(1);
        self.read_byte(self.registers.stack_pointer as u16 | 0x0100)
    }

    fn read_and_inc_program_counter(&mut self) -> u8 {
        let value = self.read_byte(self.registers.program_counter);
        self.registers.program_counter = self.registers.program_counter.wrapping_add(1);

        value
    }

    fn adc(&mut self, operand: u8) {
        let result: u16 = match self.registers.status_register.contains(StatusFlags::CARRY_FLAG) {
            true => 1u16 + self.registers.a as u16 + operand as u16,
            false => self.registers.a as u16 + operand as u16,
        };
        self.registers.status_register.set(
            StatusFlags::OVERFLOW_FLAG,
            (self.registers.a as u16 ^ result) & (operand as u16 ^ result) & 0x80 > 0,
        );
        self.registers.a = (result & 0xFF) as u8;
        self.registers
            .status_register
            .set(StatusFlags::ZERO_FLAG, self.registers.a == 0);
        self.registers
            .status_register
            .set(StatusFlags::NEGATIVE_FLAG, self.registers.a & 0b1000_0000 != 0);
        self.registers
            .status_register
            .set(StatusFlags::CARRY_FLAG, result > u8::MAX as u16);
    }

    fn compare(&mut self, operand: u8, register: u8) {
        let result = register.wrapping_sub(operand);
        self.registers
            .status_register
            .set(StatusFlags::CARRY_FLAG, register >= operand);
        self.set_negative_zero_flags(result);
    }

    fn decrement(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.set_negative_zero_flags(result);

        result
    }

    fn increment(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.set_negative_zero_flags(result);

        result
    }

    fn set_negative_zero_flags(&mut self, operand: u8) {
        self.registers.status_register.set(StatusFlags::ZERO_FLAG, operand == 0);
        self.registers
            .status_register
            .set(StatusFlags::NEGATIVE_FLAG, operand & 0b1000_0000 != 0);
    }

    fn next_absolute_mode_state(
        &mut self,
        opcode: &'static Opcode,
        address_low_byte: Option<u8>,
        address_high_byte: Option<u8>,
    ) -> State {
        match (address_low_byte, address_high_byte) {
            // Cycle 2 - Read low byte
            (None, _) => State::Cpu(CpuState::ReadingOperand {
                opcode,
                address_low_byte: Some(self.read_and_inc_program_counter()),
                address_high_byte,
                pointer: None,
                indirect_address_low_byte: None,
                indirect_address_high_byte: None,
                checked_page_boundary: false,
            }),
            // Cycle 3 - Read high byte
            (Some(low_byte), None) => {
                let high_byte = self.read_and_inc_program_counter();

                match opcode.operation.instruction_type() {
                    // Some instructions don't make use of the value at the absolute address, some do
                    InstructionType::Jump | InstructionType::Write => {
                        opcode.execute(self, None, Some(low_byte as u16 | ((high_byte as u16) << 8)))
                    }
                    _ => State::Cpu(CpuState::ReadingOperand {
                        opcode,
                        address_low_byte,
                        address_high_byte: Some(high_byte),
                        pointer: None,
                        indirect_address_low_byte: None,
                        indirect_address_high_byte: None,
                        checked_page_boundary: false,
                    }),
                }
            }
            // Cycle 4 - Read $HHLL from memory as operand
            (Some(low_byte), Some(high_byte)) => {
                let address = low_byte as u16 | ((high_byte as u16) << 8);
                let value = Some(self.read_byte(address));
                opcode.execute(self, value, Some(address))
            }
        }
    }

    fn next_absolute_indexed_mode_state(
        &mut self,
        opcode: &'static Opcode,
        address_low_byte: Option<u8>,
        address_high_byte: Option<u8>,
        checked_page_boundary: bool,
        index: u8,
    ) -> State {
        match (address_low_byte, address_high_byte) {
            // Cycle 2 - Read low byte
            (None, None) => State::Cpu(CpuState::ReadingOperand {
                opcode,
                address_low_byte: Some(self.read_and_inc_program_counter()),
                address_high_byte,
                pointer: None,
                indirect_address_low_byte: None,
                indirect_address_high_byte: None,
                checked_page_boundary: false,
            }),
            // Cycle 3 - Read high byte
            (Some(_), None) => State::Cpu(CpuState::ReadingOperand {
                opcode,
                address_low_byte,
                address_high_byte: Some(self.read_and_inc_program_counter()),
                pointer: None,
                indirect_address_low_byte: None,
                indirect_address_high_byte: None,
                checked_page_boundary: false,
            }),
            // Cycle 4 - Read $HHLL from memory as operand
            (Some(low_byte), Some(high_byte)) => {
                let unindexed_address = low_byte as u16 | ((high_byte as u16) << 8);
                let correct_address = unindexed_address.wrapping_add(index as u16);

                if checked_page_boundary {
                    let value = Some(self.read_byte(correct_address));
                    opcode.execute(self, value, Some(correct_address))
                } else {
                    let first_read_address = low_byte.wrapping_add(index) as u16 | ((high_byte as u16) << 8);

                    match opcode.operation.instruction_type() {
                        InstructionType::Read => {
                            if correct_address == first_read_address {
                                let value = Some(self.read_byte(correct_address));
                                opcode.execute(self, value, Some(correct_address))
                            } else {
                                // Dummy read, we're going to go read from the right address next
                                let _ = self.read_byte(first_read_address);
                                State::Cpu(CpuState::ReadingOperand {
                                    opcode,
                                    address_low_byte,
                                    address_high_byte,
                                    pointer: None,
                                    indirect_address_low_byte: None,
                                    indirect_address_high_byte: None,
                                    checked_page_boundary: true,
                                })
                            }
                        }
                        InstructionType::ReadModifyWrite => {
                            // Dummy read, we're going to go read from the right address next
                            let _ = self.read_byte(first_read_address);

                            // Instructions which both read & write will always read twice
                            State::Cpu(CpuState::ReadingOperand {
                                opcode,
                                address_low_byte,
                                address_high_byte,
                                pointer: None,
                                indirect_address_low_byte: None,
                                indirect_address_high_byte: None,
                                checked_page_boundary: true,
                            })
                        }
                        _ => {
                            let value = Some(self.read_byte(first_read_address));
                            opcode.execute(self, value, Some(correct_address))
                        }
                    }
                }
            }
            (_, _) => panic!(), // Coding bug, can't read high byte first
        }
    }

    fn step_interrupt_handler(&mut self, state: InterruptState) -> State {
        info!("Interrupt state: {:?} at cycle {}", state, self.cycles);

        match state {
            InterruptState::InternalOps1(i) => State::Interrupt(InterruptState::InternalOps2(i)),
            InterruptState::InternalOps2(i) => State::Interrupt(InterruptState::PushPCH(i)),
            InterruptState::PushPCH(i) => {
                self.push_to_stack((self.registers.program_counter >> 8) as u8);

                State::Interrupt(InterruptState::PushPCL(i))
            }
            InterruptState::PushPCL(i) => {
                self.push_to_stack(self.registers.program_counter as u8);
                State::Interrupt(InterruptState::PushStatusRegister(i))
            }
            InterruptState::PushStatusRegister(i) => {
                self.poll_for_interrupts(false);

                // Since we've just polled for interrupts this may affect which interrupt is now actually executed
                // NMI overrides BRK & IRQ,
                // IRQ overrides BRK
                let i = match (i, self.polled_interrupt) {
                    (_, None) => i,
                    (Interrupt::NMI(_), _) => i,
                    (Interrupt::RESET(_), _) => i,
                    (Interrupt::IRQ_BRK(_), Some(interrupt)) => {
                        info!("Interrupt {:?} overrode {:?}", interrupt, i);
                        interrupt
                    }
                    (Interrupt::IRQ(_), Some(interrupt)) => {
                        info!("Interrupt {:?} overrode {:?}", interrupt, i);
                        interrupt
                    }
                };
                self.polled_interrupt = None;

                self.push_to_stack(match i {
                    Interrupt::IRQ_BRK(_) => self.registers.status_register.bits() | 0b0011_0000,
                    _ => (self.registers.status_register.bits() | 0b0010_0000) & 0b1110_1111,
                });

                // Set interrupt disable at this point, whether this is NMI, BRK or normal IRQ
                self.registers
                    .status_register
                    .insert(StatusFlags::INTERRUPT_DISABLE_FLAG);

                State::Interrupt(InterruptState::PullIRQVecHigh(i))
            }
            InterruptState::PullIRQVecHigh(i) => {
                self.registers.program_counter = self.read_byte(i.offset()) as u16;

                State::Interrupt(InterruptState::PullIRQVecLow(i))
            }
            InterruptState::PullIRQVecLow(i) => {
                self.registers.program_counter = (self.registers.program_counter & 0b1111_1111)
                    | ((self.read_byte(i.offset().wrapping_add(1)) as u16) << 8);

                State::Cpu(CpuState::FetchOpcode)
            }
        }
    }

    fn step_cpu(&mut self, state: CpuState) -> State {
        match state {
            CpuState::FetchOpcode => {
                let opcode = &OPCODE_TABLE[self.read_and_inc_program_counter() as usize];

                info!("{}", self.nes_test_log(opcode));

                match opcode.address_mode {
                    AddressingMode::Accumulator => State::Cpu(CpuState::ThrowawayRead {
                        opcode,
                        operand: Some(self.registers.a),
                    }),
                    AddressingMode::Implied => State::Cpu(CpuState::ThrowawayRead { opcode, operand: None }),
                    _ => State::Cpu(CpuState::ReadingOperand {
                        opcode,
                        address_low_byte: None,
                        address_high_byte: None,
                        pointer: None,
                        indirect_address_low_byte: None,
                        indirect_address_high_byte: None,
                        checked_page_boundary: false,
                    }),
                }
            }
            CpuState::ReadingOperand {
                opcode,
                address_low_byte,
                address_high_byte,
                pointer,
                indirect_address_low_byte,
                indirect_address_high_byte,
                checked_page_boundary,
            } => {
                match opcode.address_mode {
                    AddressingMode::Absolute => {
                        self.next_absolute_mode_state(opcode, address_low_byte, address_high_byte)
                    }
                    AddressingMode::AbsoluteXIndexed => self.next_absolute_indexed_mode_state(
                        opcode,
                        address_low_byte,
                        address_high_byte,
                        checked_page_boundary,
                        self.registers.x,
                    ),
                    AddressingMode::AbsoluteYIndexed => self.next_absolute_indexed_mode_state(
                        opcode,
                        address_low_byte,
                        address_high_byte,
                        checked_page_boundary,
                        self.registers.y,
                    ),
                    AddressingMode::Immediate => {
                        let operand = Some(self.read_and_inc_program_counter());
                        opcode.execute(self, operand, Some(self.registers.program_counter.wrapping_sub(1)))
                    }
                    AddressingMode::Indirect => {
                        match (indirect_address_low_byte, indirect_address_high_byte, address_low_byte) {
                            (None, _, _) => {
                                // Cycle 1 - Read the indirect address low byte
                                State::Cpu(CpuState::ReadingOperand {
                                    opcode,
                                    address_low_byte: None,
                                    address_high_byte: None,
                                    pointer: None,
                                    indirect_address_low_byte: Some(self.read_and_inc_program_counter()),
                                    indirect_address_high_byte: None,
                                    checked_page_boundary: false,
                                })
                            }
                            (Some(_), None, _) => {
                                // Cycle 2 - Read the indirect address high byte
                                State::Cpu(CpuState::ReadingOperand {
                                    opcode,
                                    address_low_byte: None,
                                    address_high_byte: None,
                                    pointer: None,
                                    indirect_address_low_byte,
                                    indirect_address_high_byte: Some(self.read_and_inc_program_counter()),
                                    checked_page_boundary: false,
                                })
                            }
                            (Some(indirect_low_byte), Some(indirect_high_byte), None) => {
                                let indirect_address = (indirect_low_byte as u16) | ((indirect_high_byte as u16) << 8);

                                // Cycle 3 - Read the address low byte from the indirect address
                                State::Cpu(CpuState::ReadingOperand {
                                    opcode,
                                    address_low_byte: Some(self.read_byte(indirect_address)),
                                    address_high_byte: None,
                                    pointer: None,
                                    indirect_address_low_byte,
                                    indirect_address_high_byte,
                                    checked_page_boundary: false,
                                })
                            }
                            (Some(indirect_low_byte), Some(indirect_high_byte), Some(low_byte)) => {
                                // Cycle 4 - Read the address high byte from the indirect address and immediately set the PC as this is always a JMP instruction
                                // Note - this is deliberately "bugged", JMP (0x01FF) will jump to 0x01FF | 0x0100 << 8 NOT 0x01FF | 0x0200 << 8 as you might imagine (this is a known 6502 cpu bug)
                                let indirect_address =
                                    (indirect_low_byte.wrapping_add(1) as u16) | ((indirect_high_byte as u16) << 8);
                                let high_byte = self.read_byte(indirect_address);

                                opcode.execute(self, None, Some((low_byte as u16) | ((high_byte as u16) << 8)))
                            }
                        }
                    }
                    AddressingMode::IndirectXIndexed => {
                        match (indirect_address_low_byte, pointer, address_low_byte, address_high_byte) {
                            (None, _, _, _) => {
                                // Cycle 1 - Read the low byte of the indirect address
                                State::Cpu(CpuState::ReadingOperand {
                                    opcode,
                                    address_low_byte,
                                    address_high_byte,
                                    pointer: None,
                                    indirect_address_low_byte: Some(self.read_and_inc_program_counter()),
                                    indirect_address_high_byte,
                                    checked_page_boundary: false,
                                })
                            }
                            (Some(_), None, _, _) => {
                                // Cycle 2 - Construct the pointer to the actual address
                                State::Cpu(CpuState::ReadingOperand {
                                    opcode,
                                    address_low_byte,
                                    address_high_byte,
                                    pointer: indirect_address_low_byte,
                                    indirect_address_low_byte,
                                    indirect_address_high_byte,
                                    checked_page_boundary: false,
                                })
                            }
                            (Some(indirect_low_byte), Some(_), None, _) => {
                                // Cycle 3 - Read the low byte of the actual address
                                let address = indirect_low_byte.wrapping_add(self.registers.x) as u16;

                                State::Cpu(CpuState::ReadingOperand {
                                    opcode,
                                    address_low_byte: Some(self.read_byte(address)),
                                    address_high_byte,
                                    pointer,
                                    indirect_address_low_byte,
                                    indirect_address_high_byte,
                                    checked_page_boundary: false,
                                })
                            }
                            (Some(indirect_low_byte), Some(_), Some(address_low_byte), None) => {
                                // Cycle 4 - Read the high byte of the actual address
                                let indirect_address_high_byte =
                                    indirect_low_byte.wrapping_add(self.registers.x).wrapping_add(1) as u16;
                                let address_high_byte = self.read_byte(indirect_address_high_byte);

                                match opcode.operation.instruction_type() {
                                    InstructionType::Write => {
                                        let address = (address_low_byte as u16) | ((address_high_byte as u16) << 8);
                                        opcode.execute(self, None, Some(address))
                                    }
                                    _ => State::Cpu(CpuState::ReadingOperand {
                                        opcode,
                                        address_low_byte: Some(address_low_byte),
                                        address_high_byte: Some(address_high_byte),
                                        pointer,
                                        indirect_address_low_byte,
                                        indirect_address_high_byte: Some(indirect_address_high_byte as u8),
                                        checked_page_boundary: false,
                                    }),
                                }
                            }
                            (Some(_), Some(_), Some(low_byte), Some(high_byte)) => {
                                let address = (low_byte as u16) | ((high_byte as u16) << 8);
                                let value = Some(self.read_byte(address));

                                // Cycle 5 - Read the operand and execute operation
                                opcode.execute(self, value, Some(address))
                            }
                        }
                    }
                    AddressingMode::IndirectYIndexed => {
                        match (indirect_address_low_byte, address_low_byte, address_high_byte) {
                            (None, _, _) => {
                                // Cycle 2 - Read the low byte of the indirect address
                                State::Cpu(CpuState::ReadingOperand {
                                    opcode,
                                    address_low_byte,
                                    address_high_byte,
                                    pointer: None,
                                    indirect_address_low_byte: Some(self.read_and_inc_program_counter()),
                                    indirect_address_high_byte,
                                    checked_page_boundary: false,
                                })
                            }
                            (Some(indirect_low_byte), None, _) => {
                                // Cycle 3 - Read the low byte of the actual address
                                State::Cpu(CpuState::ReadingOperand {
                                    opcode,
                                    address_low_byte: Some(self.read_byte(indirect_low_byte as u16)),
                                    address_high_byte,
                                    pointer: None,
                                    indirect_address_low_byte,
                                    indirect_address_high_byte,
                                    checked_page_boundary: false,
                                })
                            }
                            (Some(indirect_low_byte), Some(address_low_byte), None) => {
                                // Cycle 4 - Read the high byte of the actual address
                                State::Cpu(CpuState::ReadingOperand {
                                    opcode,
                                    address_low_byte: Some(address_low_byte),
                                    address_high_byte: Some(self.read_byte(indirect_low_byte.wrapping_add(1) as u16)),
                                    pointer: Some(indirect_low_byte),
                                    indirect_address_low_byte,
                                    indirect_address_high_byte,
                                    checked_page_boundary: false,
                                })
                            }
                            (Some(_), Some(low_byte), Some(high_byte)) => {
                                // Cycle 5(/6) - Read the operand and execute the operation checking for crossing page boundary
                                let unindexed_address = (low_byte as u16) | ((high_byte as u16) << 8);
                                let dummy_read_address =
                                    low_byte.wrapping_add(self.registers.y) as u16 | ((high_byte as u16) << 8);
                                let address = unindexed_address.wrapping_add(self.registers.y as u16);

                                match opcode.operation.instruction_type() {
                                    InstructionType::Write => {
                                        // Dummy read of address without fixing the high byte (so without wrap)
                                        let _ = Some(self.read_byte(dummy_read_address));
                                        opcode.execute(self, None, Some(address))
                                    }
                                    _ => {
                                        if checked_page_boundary || (dummy_read_address == address) {
                                            let value = Some(self.read_byte(address));
                                            opcode.execute(self, value, Some(address))
                                        } else {
                                            // Dummy read of address without fixing the high byte (so without wrap)
                                            let _ = Some(self.read_byte(dummy_read_address));

                                            State::Cpu(CpuState::ReadingOperand {
                                                opcode,
                                                address_low_byte: Some(low_byte),
                                                address_high_byte: Some(high_byte),
                                                pointer: None,
                                                indirect_address_low_byte,
                                                indirect_address_high_byte,
                                                checked_page_boundary: true,
                                            })
                                        }
                                    }
                                }
                            }
                        }
                    }
                    AddressingMode::Relative => {
                        // Cycle 2 - Get the relative index and store it in the operand for use in the instruction (it'll be a signed 8 bit relative index)
                        let relative_operand = self.read_and_inc_program_counter();

                        let branch = match opcode.operation {
                            Operation::BCC => !self.registers.status_register.contains(StatusFlags::CARRY_FLAG),
                            Operation::BCS => self.registers.status_register.contains(StatusFlags::CARRY_FLAG),
                            Operation::BEQ => self.registers.status_register.contains(StatusFlags::ZERO_FLAG),
                            Operation::BMI => self.registers.status_register.contains(StatusFlags::NEGATIVE_FLAG),
                            Operation::BNE => !self.registers.status_register.contains(StatusFlags::ZERO_FLAG),
                            Operation::BPL => !self.registers.status_register.contains(StatusFlags::NEGATIVE_FLAG),
                            Operation::BVC => !self.registers.status_register.contains(StatusFlags::OVERFLOW_FLAG),
                            Operation::BVS => self.registers.status_register.contains(StatusFlags::OVERFLOW_FLAG),
                            _ => panic!(),
                        };

                        if !branch {
                            State::Cpu(CpuState::FetchOpcode)
                        } else {
                            let address = self
                                .registers
                                .program_counter
                                .wrapping_add((relative_operand as i8) as u16);

                            if (address >> 8) != (self.registers.program_counter >> 8) {
                                State::Cpu(CpuState::BranchCrossesPageBoundary {
                                    opcode,
                                    operand: Some(relative_operand),
                                    address: Some(address),
                                })
                            } else {
                                opcode.execute(self, Some(relative_operand), Some(address))
                            }
                        }
                    }
                    AddressingMode::ZeroPage => match address_low_byte {
                        None => {
                            let operand = self.read_and_inc_program_counter();

                            match opcode.operation.instruction_type() {
                                InstructionType::Write => {
                                    let address = operand as u16;
                                    let value = Some(self.read_byte(address));

                                    opcode.execute(self, value, Some(address))
                                }
                                _ => State::Cpu(CpuState::ReadingOperand {
                                    opcode,
                                    address_low_byte: Some(operand),
                                    address_high_byte: None,
                                    pointer: None,
                                    indirect_address_low_byte: None,
                                    indirect_address_high_byte: None,
                                    checked_page_boundary: false,
                                }),
                            }
                        }
                        Some(low_byte) => {
                            let address = low_byte as u16;
                            let value = Some(self.read_byte(address));

                            opcode.execute(self, value, Some(address))
                        }
                    },
                    AddressingMode::ZeroPageXIndexed => match (address_low_byte, address_high_byte) {
                        (None, _) => {
                            // Cycle 2 - Read the zero page low byte
                            State::Cpu(CpuState::ReadingOperand {
                                opcode,
                                address_low_byte: Some(self.read_and_inc_program_counter()),
                                address_high_byte: None,
                                pointer: None,
                                indirect_address_low_byte: None,
                                indirect_address_high_byte: None,
                                checked_page_boundary: false,
                            })
                        }
                        (Some(low_byte), None) => {
                            // Cycle 3 - Dummy read of the unindexed address
                            let _ = self.read_byte(low_byte as u16);

                            match opcode.operation.instruction_type() {
                                InstructionType::Write => {
                                    let address = low_byte.wrapping_add(self.registers.x) as u16;
                                    let value = Some(self.read_byte(address));

                                    opcode.execute(self, value, Some(address))
                                }
                                _ => State::Cpu(CpuState::ReadingOperand {
                                    opcode,
                                    address_low_byte,
                                    address_high_byte: Some(0x0),
                                    pointer: None,
                                    indirect_address_low_byte: None,
                                    indirect_address_high_byte: None,
                                    checked_page_boundary: false,
                                }),
                            }
                        }
                        (Some(low_byte), Some(_)) => {
                            // Cycle 4 - Read operand from the indexed zero page address
                            let address = low_byte.wrapping_add(self.registers.x) as u16;
                            let value = Some(self.read_byte(address));

                            opcode.execute(self, value, Some(address))
                        }
                    },
                    AddressingMode::ZeroPageYIndexed => match (address_low_byte, address_high_byte) {
                        (None, _) => {
                            // Cycle 2 - Read the zero page low byte
                            State::Cpu(CpuState::ReadingOperand {
                                opcode,
                                address_low_byte: Some(self.read_and_inc_program_counter()),
                                address_high_byte: None,
                                pointer: None,
                                indirect_address_low_byte: None,
                                indirect_address_high_byte: None,
                                checked_page_boundary: false,
                            })
                        }
                        (Some(low_byte), None) => {
                            // Cycle 3 - Dummy read of the unindexed address
                            let _ = self.read_byte(low_byte as u16);

                            match opcode.operation.instruction_type() {
                                InstructionType::Write => {
                                    let address = low_byte.wrapping_add(self.registers.y) as u16;
                                    let _ = Some(self.read_byte(address));

                                    opcode.execute(self, None, Some(address))
                                }
                                _ => State::Cpu(CpuState::ReadingOperand {
                                    opcode,
                                    address_low_byte,
                                    address_high_byte: Some(0x0),
                                    pointer: None,
                                    indirect_address_low_byte: None,
                                    indirect_address_high_byte: None,
                                    checked_page_boundary: false,
                                }),
                            }
                        }
                        (Some(low_byte), Some(_)) => {
                            // Cycle 4 - Read operand from the indexed zero page address
                            let address = low_byte.wrapping_add(self.registers.y) as u16;
                            let value = Some(self.read_byte(address));

                            opcode.execute(self, value, Some(address))
                        }
                    },
                    _ => panic!(
                        "Invalid, can't read operand for addressing mode {:?}",
                        opcode.address_mode
                    ),
                }
            }
            CpuState::ThrowawayRead { opcode, operand } => {
                // BRK does a throwaway read but does increment the PC
                // Normal implied operations do a throwaway the read and don't increment the PC
                if opcode.operation == Operation::BRK {
                    self.read_and_inc_program_counter();
                } else {
                    self.read_byte(self.registers.program_counter);
                }

                opcode.execute(self, operand, None)
            }
            CpuState::PushRegisterOnStack { value } => {
                self.push_to_stack(value);

                State::Cpu(CpuState::FetchOpcode)
            }
            CpuState::PreIncrementStackPointer { operation } => match operation {
                Operation::PLA | Operation::PLP | Operation::RTI => {
                    State::Cpu(CpuState::PullRegisterFromStack { operation })
                }
                Operation::RTS => State::Cpu(CpuState::PullPCLFromStack { operation }),
                _ => panic!("Attempt to access stack from invalid instruction {:?}", operation),
            },
            CpuState::PullRegisterFromStack { operation } => match operation {
                Operation::PLA => {
                    self.poll_for_interrupts(true);
                    self.registers.a = self.pop_from_stack();
                    self.set_negative_zero_flags(self.registers.a);
                    State::Cpu(CpuState::FetchOpcode)
                }
                Operation::PLP => {
                    self.poll_for_interrupts(true);
                    self.registers.status_register =
                        StatusFlags::from_bits_truncate(self.pop_from_stack() & 0b1100_1111);

                    State::Cpu(CpuState::FetchOpcode)
                }
                Operation::RTI => {
                    self.registers.status_register =
                        StatusFlags::from_bits_truncate(self.pop_from_stack() & 0b1100_1111);

                    State::Cpu(CpuState::PullPCLFromStack { operation })
                }
                _ => panic!("Attempt to access stack from invalid instruction {:?}", operation),
            },
            CpuState::PullPCLFromStack { operation } => State::Cpu(CpuState::PullPCHFromStack {
                operation,
                pcl: self.pop_from_stack(),
            }),
            CpuState::PullPCHFromStack { operation, pcl } => {
                let pch = self.pop_from_stack();
                self.registers.program_counter = ((pch as u16) << 8) | pcl as u16;

                match operation {
                    Operation::RTS => State::Cpu(CpuState::IncrementProgramCounter),
                    Operation::RTI => {
                        self.poll_for_interrupts(true);
                        State::Cpu(CpuState::FetchOpcode)
                    }
                    _ => panic!("Attempt to access stack from invalid instruction {:?}", operation),
                }
            }
            CpuState::IncrementProgramCounter => {
                self.poll_for_interrupts(true);
                self.registers.program_counter = self.registers.program_counter.wrapping_add(1);

                State::Cpu(CpuState::FetchOpcode)
            }
            CpuState::WritePCHToStack { address } => {
                self.push_to_stack((self.registers.program_counter.wrapping_sub(1) >> 8) as u8);

                State::Cpu(CpuState::WritePCLToStack { address })
            }
            CpuState::WritePCLToStack { address } => {
                self.push_to_stack((self.registers.program_counter.wrapping_sub(1) & 0xFF) as u8);

                State::Cpu(CpuState::SetProgramCounter {
                    address,
                    was_branch_instruction: false,
                })
            }
            CpuState::SetProgramCounter {
                address,
                was_branch_instruction,
            } => {
                self.poll_for_interrupts(true);
                self.registers.program_counter = address;

                State::Cpu(CpuState::FetchOpcode)
            }
            CpuState::BranchCrossesPageBoundary {
                opcode,
                operand,
                address,
            } => opcode.execute(self, operand, address),
            CpuState::WritingResult {
                value,
                address,
                dummy: true,
            } => State::Cpu(CpuState::WritingResult {
                value,
                address,
                dummy: false,
            }),
            CpuState::WritingResult {
                value,
                address,
                dummy: false,
            } => {
                // Crucially this _must_ happen before the write_byte.
                self.poll_for_interrupts(true);

                self.write_byte(address, value);

                State::Cpu(CpuState::FetchOpcode)
            }
        }
    }

    /// When the CPU is paused for DMA this steps the CPU by a single clock
    fn step_dma_handler(&mut self, state: DmaState) -> State {
        match state {
            DmaState::DummyCycle => {
                info!("Starting DMA on cycle {} from {:04X}", self.cycles, self.dma_address);
                if self.cycles & 1 == 1 {
                    State::Dma(DmaState::OddCpuCycle)
                } else {
                    State::Dma(DmaState::ReadCycle)
                }
            }
            DmaState::OddCpuCycle => State::Dma(DmaState::ReadCycle),
            DmaState::ReadCycle => {
                let value = self.read_byte(self.dma_address);
                self.dma_address += 1;

                State::Dma(DmaState::WriteCycle(value))
            }
            DmaState::WriteCycle(value) => {
                self.ppu.write_dma_byte(value, (self.dma_address - 1) as u8);

                if self.dma_address & 0x100 == 0x100 {
                    error!("Finished DMA on cycle {}", self.cycles);
                    State::Cpu(CpuState::FetchOpcode)
                } else {
                    State::Dma(DmaState::ReadCycle)
                }
            }
        }
    }

    /// Move the cpu on by a single clock cycle
    fn clock(&mut self) {
        self.state = match self.state {
            State::Cpu(state) => self.step_cpu(state),
            State::Interrupt(state) => self.step_interrupt_handler(state),
            State::Dma(state) => self.step_dma_handler(state),
        };

        if let State::Cpu(CpuState::FetchOpcode) = self.state {
            if let Some(interrupt) = self.polled_interrupt {
                self.polled_interrupt = None;

                self.state = State::Interrupt(InterruptState::InternalOps1(interrupt));
            } else if self.trigger_dma {
                // Also check whether we're starting DMA on the next cycle
                self.trigger_dma = false;
                self.state = State::Dma(DmaState::DummyCycle);

                info!("Starting DMA transfer from {:04X}", self.dma_address);
            }
        }

        self.cycles += 1;
    }

    pub(super) fn button_down(&mut self, controller: Controller, button: Button) {
        self.io.button_down(controller, button);
    }

    pub(super) fn button_up(&mut self, controller: Controller, button: Button) {
        self.io.button_up(controller, button);
    }

    pub(super) fn is_frame_complete_cycle(&self) -> bool {
        self.ppu.output_cycle()
    }

    pub(super) fn get_framebuffer(&self) -> &[u8; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize] {
        &self.ppu.frame_buffer
    }

    pub(super) fn dump_ppu_state(&mut self, vram_clone: &mut [u8; 0x4000]) -> &[u8; 0x100] {
        self.ppu.dump_state(vram_clone)
    }
}

impl<'a> Iterator for Cpu<'a> {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        // Check if we need to clock the CPU
        self.cpu_cycle_counter -= 1;
        if self.cpu_cycle_counter == 0 {
            self.cpu_cycle_counter = 3;
            self.clock();

            // Clock the APU once every CPU cycle, it decides internally which things to clock at what speed
            self.apu.next();
        }

        // Always clock the PPU
        self.ppu.next();

        // Does the cpu ever halt? If no return None, otherwise this is just an
        // infinite sequence. Maybe bad opcode? Undefined behaviour of some sort?
        None
    }
}
