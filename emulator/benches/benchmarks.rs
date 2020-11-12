extern crate criterion;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use std::path::Path;

fn criterion_benchmark(c: &mut Criterion) {
    let rom_path = Path::new("..")
        .join("roms")
        .join("test")
        .join("spritecans-2011")
        .join("spritecans.nes");

    c.bench_function("spritecans 100 frames", |b| {
        b.iter_batched(
            || match rust_nes::get_cartridge(rom_path.to_str().unwrap()) {
                Err(why) => panic!("Failed to load cartridge: {}", why.message),
                Ok(cartridge) => cartridge,
            },
            |cartridge| rust_nes::run_headless_cycles(cartridge, 29_780_50),
            BatchSize::LargeInput,
        )
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
