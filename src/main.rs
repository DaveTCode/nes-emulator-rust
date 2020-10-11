extern crate clap;
extern crate log;
extern crate log4rs;

use clap::Clap;
use log::info;

#[derive(Clap)]
#[clap(version = "1.0", author = "David Tyler <davet.code@gmail.com>")]
struct Opts {
    rom_file: String,
    #[clap(short = 'l', long = "log_config", default_value = "config/log4rs.yaml")]
    log_config: String,
}

fn main() {
    let opts: Opts = Opts::parse();
    log4rs::init_file(opts.log_config, Default::default()).unwrap();

    info!("Logging Configured");

    rust_nes::run(opts.rom_file);
}
