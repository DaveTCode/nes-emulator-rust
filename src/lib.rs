#[macro_use]
extern crate bitflags;
extern crate clap;
extern crate crc32fast;
extern crate log;
extern crate log4rs;
extern crate sdl2;
extern crate zip;

mod apu;
mod cartridge;
mod cpu;
mod io;
mod ppu;
mod sdl2_app;

use apu::Apu;
use cpu::Cpu;
use io::Io;
use log::info;
use ppu::Ppu;
use ppu::SCREEN_HEIGHT;
use ppu::SCREEN_WIDTH;

pub fn run(rom_file: String) {
    let (prg_address_bus, chr_address_bus, cartridge_header) = match cartridge::from_file(&rom_file)
    {
        Err(why) => panic!("Failed to load cartridge: {}", why.message),
        Ok(cartridge) => cartridge,
    };

    info!("Catridge Loaded {:}", cartridge_header);

    sdl2_app::run(256, 240, prg_address_bus, chr_address_bus, cartridge_header);
}

/// Run a rom for N cycles and return the CRC32 checksum of the framebuffer
pub fn run_headless_cycles(
    rom_file: &str,
    cycles: usize,
) -> [u8; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize] {
    let (prg_address_bus, chr_address_bus, _) = match cartridge::from_file(&rom_file) {
        Err(why) => panic!("Failed to load cartridge: {}", why.message),
        Ok(cartridge) => cartridge,
    };

    let mut apu = Apu::new();
    let mut io = Io::new();
    let mut ppu = Ppu::new(chr_address_bus);
    let mut cpu = Cpu::new(prg_address_bus, &mut apu, &mut io, &mut ppu);

    for _ in 0..cycles {
        cpu.next();
    }

    *cpu.get_framebuffer()
}
