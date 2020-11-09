[![Build Status](https://dev.azure.com/DavidATyler/Nes%20Emulator/_apis/build/status/DaveTCode.nes-emulator-rust?branchName=master)](https://dev.azure.com/DavidATyler/Nes%20Emulator/_build/latest?definitionId=5&branchName=master)

# Rust Nes Emulator

This project is a learning project to attempt writing a cycle accurate NES emulator in rust. It is not intended to be
used to play games or to debug other emulators and has no features beyond "run this rom".

<table>
  <tr>
    <td><img src="./.github/images/ninja_gaiden.png" width="200" height="200"></td>
    <td><img src="./.github/images/super-mario-bros.png" width="200" height="200"></td>
    <td><img src="./.github/images/zelda.png" width="200" height="200"></td>
  </tr>
 </table>

## Key Missing Features

- APU is only partially complete (no DMC) and does not yet output audio (no mixer and no provision for sending 
the samples anywhere)
- Only mappers 1-4 are complete although that covers a large percentage of the rom space
- No support for peripherals beyond a standard NES controller
- Support only provided for NTSC device

## Architecture

There are many rust emulators, this one differs slightly in that it is entirely compile time checked code, it contains
no unsafe blocks (except those in dependencies) and no Rc<RefCell<>> for runtime checking. This is achieved through the 
following architecture:

![Architecture](./.github/images/nes-emulator.png)

The two key architectural decisions here are that the CPU owns all other components and is responsible for the top 
level "step" function to move a single cycle (note here that a single cycle is one PPU cycle, not one CPU cycle) and 
that the cartridge is broken into two parts, the PRG ROM/RAM that is attached to the CPU address bus and the CHR ROM/RAM
which is attached to the PPU address bus. In order for register writes that update mappers to be reflected the CPU
must therefore write each value mapped to 0x4020..=0xFFFF through to _both_ cartridge components.

## Development

### Pre-requisites

I developed the emulator on Windows using the stable rust toolchain at version 1.47.0, tests run on Mac/Windows/Linux 
against stable and nightly on each push.

### Running Tests

The tests are full integration tests of the entire emulator using the test roms collated here 
[roms/test](https://github.com/DaveTCode/nes-emulator-rust/tree/master/roms/test). The original source of these tests is
[the nesdev wiki](https://wiki.nesdev.com/w/index.php/Emulator_tests).

```shell script
cargo test  
```

The tests can take a few minutes to complete but should all pass on all machines. If a test fails it will print, in 
ascii art, the screenshot at the time of the failure.

### Benchmarks

At present there's only a single benchmark, it runs the "spritecans" test rom for 100 frames and the reports are not
presently checked into the code base.

```shell script
Benchmarking spritecans 100 frames: Warming up for 3.0000 s
Warning: Unable to complete 100 samples in 5.0s. You may wish to increase target time to 15.1s, or reduce sample count to 30.
spritecans 100 frames   time:   [136.67 ms 139.07 ms 141.79 ms]
                        change: [-16.859% -13.771% -10.553%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 5 outliers among 100 measurements (5.00%)
  2 (2.00%) high mild
  3 (3.00%) high severe
```

is the most recent execution, note that the high percentage change appears to fluctuate. This area needs work as the 
benchmark includes loading the file from disk as well as actually rendering frames. 
