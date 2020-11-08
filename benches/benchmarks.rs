extern crate criterion;

use criterion::{criterion_group, criterion_main, Criterion};
use std::path::Path;

fn spritecans() {
    let rom_path = Path::new(".")
        .join("roms")
        .join("test")
        .join("spritecans-2011")
        .join("spritecans.nes");
    rust_nes::run_headless_cycles(rom_path.to_str().unwrap(), 29_780_50);
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("spritecans 100 frames", |b| b.iter(|| spritecans()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
