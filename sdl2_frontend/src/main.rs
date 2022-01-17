mod sdl2_app;

extern crate clap;
extern crate crc32fast;
extern crate log;
extern crate log4rs;
extern crate rust_nes;
extern crate sdl2;

use clap::Parser;
use log::info;

#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "David Tyler <davet.code@gmail.com>")]
struct Opts {
    rom_file: String,
    #[clap(short = 'l', long = "log_config", default_value = "config/log4rs.yaml")]
    log_config: String,
    #[clap(short = 'w', long = "width", default_value = "256")]
    screen_width: u32,
    #[clap(short = 'h', long = "height", default_value = "240")]
    screen_height: u32,
}

fn main() -> std::io::Result<()> {
    let opts: Opts = Opts::parse();
    log4rs::init_file(opts.log_config, Default::default()).unwrap();

    info!("Logging Configured");

    let (prg_address_bus, chr_address_bus, cartridge_header) = match rust_nes::get_cartridge(&opts.rom_file) {
        Err(why) => panic!("Failed to load cartridge: {}", why.message),
        Ok(cartridge) => cartridge,
    };

    info!("Running cartridge {:?}", cartridge_header);
    sdl2_app::run(
        opts.screen_width,
        opts.screen_height,
        prg_address_bus,
        chr_address_bus,
        cartridge_header,
    )?;

    Ok(())
}
