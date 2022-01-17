extern crate clap;
extern crate rust_nes;
extern crate serde;

use clap::Parser;
use serde::Serialize;
use std::fs;
use std::io;

#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "David Tyler <davet.code@gmail.com>")]
struct Opts {
    rom_directory: String,
}

#[derive(Debug, Serialize)]
struct RomResult {
    filename: String,
    mapper: Option<u8>,
    prg_16kb_units: Option<u8>,
    chr_8kb_banks: Option<u8>,
    failure: Option<String>,
}

fn main() -> std::io::Result<()> {
    let opts: Opts = Opts::parse();
    let paths = fs::read_dir(opts.rom_directory).unwrap();

    let mut wrt = csv::Writer::from_writer(io::stdout());

    for path in paths {
        let p = path?;
        let filename = match p.file_name().into_string() {
            Ok(s) => s,
            Err(_) => "Non unicode filename".to_string(),
        };

        let result = match rust_nes::get_cartridge(p.path().to_str().unwrap()) {
            Err(why) => RomResult {
                filename,
                mapper: why.mapper,
                prg_16kb_units: None,
                chr_8kb_banks: None,
                failure: Some(why.message),
            },
            Ok((_, _, header)) => RomResult {
                filename,
                mapper: Some(header.mapper),
                prg_16kb_units: Some(header.prg_rom_16kb_units),
                chr_8kb_banks: Some(header.chr_rom_8kb_units),
                failure: None,
            },
        };

        wrt.serialize(result)?;
    }

    wrt.flush()?;

    Ok(())
}
