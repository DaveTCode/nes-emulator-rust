#[macro_use]
extern crate bitflags;
extern crate clap;
extern crate log;
extern crate log4rs;

mod cartridge;
mod cpu;
mod mmu;

use clap::Clap;
use cpu::Cpu;
use log::info;
use mmu::Mmu;

#[derive(Clap)]
#[clap(version = "1.0", author = "David Tyler <davet.code@gmail.com>")]
struct Opts {
    #[clap(short, long, default_value = "config/log4rs.yaml")]
    log_config: String,
    rom_file: String,
}

fn main() {
    let opts: Opts = Opts::parse();
    log4rs::init_file(opts.log_config, Default::default()).unwrap();

    info!("Application started");

    let cartridge = match cartridge::from_file(&opts.rom_file) {
        Err(why) => panic!("Failed to load cartridge: {}", why.message),
        Ok(cartridge) => cartridge,
    };

    let mut mmu = Mmu::new(&cartridge);
    let mut cpu = Cpu::new(&mut mmu);

    loop {
        // Step the CPU by 1 cycle
        cpu.next();
    }
}
