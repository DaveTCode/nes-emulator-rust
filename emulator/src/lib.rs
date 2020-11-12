#[macro_use]
extern crate bitflags;
extern crate log;
extern crate log4rs;
extern crate zip;

pub mod apu;
pub mod cartridge;
pub mod cpu;
pub mod io;
pub mod ppu;

use apu::Apu;
use cartridge::{CartridgeError, CartridgeHeader, CpuCartridgeAddressBus, PpuCartridgeAddressBus};
use cpu::Cpu;
use io::Io;
use ppu::Ppu;
use ppu::SCREEN_HEIGHT;
use ppu::SCREEN_WIDTH;

pub type Cartridge = (
    Box<dyn CpuCartridgeAddressBus>,
    Box<dyn PpuCartridgeAddressBus>,
    CartridgeHeader,
);

/// Load a cartridge
pub fn get_cartridge(rom_file: &str) -> Result<Cartridge, CartridgeError> {
    cartridge::from_file(rom_file)
}

/// Run a rom for N cycles and return the CRC32 checksum of the framebuffer
pub fn run_headless_cycles(cartridge: Cartridge, cycles: usize) -> [u8; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize] {
    let mut apu = Apu::new();
    let mut io = Io::new();
    let mut ppu = Ppu::new(cartridge.1);
    let mut cpu = Cpu::new(cartridge.0, &mut apu, &mut io, &mut ppu);

    for _ in 0..cycles {
        cpu.next();
    }

    *cpu.get_framebuffer()
}
