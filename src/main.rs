#[macro_use]
extern crate bitflags;
extern crate clap;
extern crate log;
extern crate log4rs;
extern crate sdl2;

mod apu;
mod cartridge;
mod cpu;
mod io;
mod ppu;
mod sdl2_app;

use clap::Clap;
use log::info;

#[derive(Clap)]
#[clap(version = "1.0", author = "David Tyler <davet.code@gmail.com>")]
struct Opts {
    rom_file: String,
    #[clap(short = 'l', long = "log_config", default_value = "config/log4rs.yaml")]
    log_config: String,
    #[clap(long, default_value = "sdl2", possible_values = &["sdl2", "dummy"])]
    renderer: String,
}

fn main() {
    let opts: Opts = Opts::parse();
    log4rs::init_file(opts.log_config, Default::default()).unwrap();

    info!("Logging Configured");

    let (prg_address_bus, chr_address_bus, cartridge_header) =
        match cartridge::from_file(&opts.rom_file) {
            Err(why) => panic!("Failed to load cartridge: {}", why.message),
            Ok(cartridge) => cartridge,
        };

    info!("Catridge Loaded {:}", cartridge_header);

    match opts.renderer.as_str() {
        "sdl2" => sdl2_app::run(256, 240, prg_address_bus, chr_address_bus, cartridge_header),
        "dummy" => todo!("Dummy renderer not yet implemented"),
        _ => panic!("{:} renderer not implemented", opts.renderer.as_str()),
    }
}
