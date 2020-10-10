mod palette;
mod registers;

use cartridge::PpuCartridgeAddressBus;
use log::{debug, info};
use ppu::palette::PaletteRam;
use ppu::registers::ppuctrl::{IncrementMode, PpuCtrl};
use ppu::registers::ppumask::PpuMask;
use ppu::registers::ppustatus::PpuStatus;

pub(crate) const SCREEN_WIDTH: u32 = 256;
pub(crate) const SCREEN_HEIGHT: u32 = 240;

struct ScanlineState {
    nametable_byte: u8,
    attribute_table_byte: u8,
    bg_low_byte: u8,
    bg_high_byte: u8,
    scanline: u16,
    scanline_cycle: u16,
    bg_shift_register_high: u16,
    bg_shift_register_low: u16,
    at_shift_register_high: u8,
    at_shift_register_low: u8,
}

impl ScanlineState {
    fn next_cycle(&mut self) {
        self.scanline_cycle += 1;
        if self.scanline_cycle == 341 {
            self.scanline_cycle = 0;
            self.scanline += 1;
            if self.scanline == 262 {
                self.scanline = 0;
            }
        }
    }

    fn reload_shift_registers(&mut self) {
        self.bg_shift_register_high |= self.bg_high_byte as u16;
        self.bg_shift_register_low |= self.bg_low_byte as u16;

        // TODO - Load attribute shift registers
    }

    /// Returns the index into the palette memory (0x00-0x3F) based on the
    /// current values of the shift registers
    fn bg_pixel_palette(&self, fine_x_scroll: u8) -> u8 {
        debug_assert!(fine_x_scroll <= 7);

        let color_low_bit = (self.bg_shift_register_low >> (15 - fine_x_scroll)) & 1;
        let color_high_bit = (self.bg_shift_register_high >> (15 - fine_x_scroll)) & 1;
        let color_index = color_low_bit | (color_high_bit << 1);

        let pl_low_bit = (self.at_shift_register_low >> (7 - fine_x_scroll)) & 1;
        let pl_high_bit = (self.at_shift_register_high >> (7 - fine_x_scroll)) & 1;
        let palette_index = pl_low_bit | (pl_high_bit << 1);

        (palette_index << 2) | color_index as u8
    }
}

struct InternalRegisters {
    vram_addr: u16,
    temp_vram_addr: u16,
    fine_x_scroll: u8,
    write_toggle: bool,
    // Since each load takes two cycles, this handles the address to read from the PPU bus during the first of the two cycles
    next_address: u16,
}

impl InternalRegisters {
    fn coarse_x(&self) -> u8 {
        (self.vram_addr & 0b0001_1111) as u8
    }

    fn coarse_y(&self) -> u8 {
        ((self.vram_addr >> 5) & 0b0001_1111) as u8
    }

    fn nametable(&self) -> u8 {
        ((self.vram_addr >> 10) & 0b111) as u8
    }

    fn fine_y(&self) -> u8 {
        ((self.vram_addr >> 12) & 0b111) as u8
    }

    /// Shamelessly taken from https://wiki.nesdev.com/w/index.php?title=PPU_scrolling&redirect=no#Wrapping_around
    fn increment_effective_scroll_x(&mut self) {
        if self.vram_addr & 0x001F == 31 {
            self.vram_addr &= !0x001F;
            self.vram_addr ^= 0x0400;
        } else {
            self.vram_addr += 1;
        }
    }

    /// Shamelessly taken from https://wiki.nesdev.com/w/index.php?title=PPU_scrolling&redirect=no#Wrapping_around
    fn incremement_effective_scroll_y(&mut self) {
        if self.fine_y() < 7 {
            self.vram_addr += 0x1000;
        } else {
            self.vram_addr &= !0x7000;
            let mut y = (self.vram_addr & 0x03E0) >> 5;
            if y == 29 {
                y = 0;
                self.vram_addr ^= 0x0800;
            } else if y == 31 {
                y = 0;
            } else {
                y += 1;
            }

            self.vram_addr = (self.vram_addr & !0x03E0) | (y << 5);
        }
    }
}

pub(crate) struct Ppu {
    scanline_state: ScanlineState,
    oam_ram: [u8; 0x100],
    palette_ram: PaletteRam,
    ppu_ctrl: PpuCtrl,
    ppu_mask: PpuMask,
    ppu_status: PpuStatus,
    internal_registers: InternalRegisters,
    oam_addr: u8,
    last_written_byte: u8, // Stores the value last written onto the latch - TODO implement decay over time
    is_short_frame: bool,  // Every other frame the pre-render scanline takes one fewer cycle
    pub(crate) trigger_nmi: bool,
    pub(crate) frame_buffer: [u8; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize],
    priorities: [u8; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize],
    pub(crate) chr_address_bus: Box<dyn PpuCartridgeAddressBus>,
}

impl Ppu {
    pub(super) fn new(chr_address_bus: Box<dyn PpuCartridgeAddressBus>) -> Self {
        Ppu {
            scanline_state: ScanlineState {
                scanline: 0,
                nametable_byte: 0,
                attribute_table_byte: 0,
                bg_high_byte: 0,
                bg_low_byte: 0,
                scanline_cycle: 0,
                bg_shift_register_high: 0,
                bg_shift_register_low: 0,
                at_shift_register_high: 0,
                at_shift_register_low: 0,
            },
            oam_ram: [0; 0x100],
            palette_ram: PaletteRam { data: [0; 0x20] },
            ppu_ctrl: PpuCtrl::new(),
            ppu_mask: PpuMask::new(),
            ppu_status: PpuStatus::new(),
            internal_registers: InternalRegisters {
                vram_addr: 0,
                temp_vram_addr: 0,
                fine_x_scroll: 0,
                write_toggle: false,
                next_address: 0,
            },
            oam_addr: 0x0,
            last_written_byte: 0x0,
            is_short_frame: false,
            trigger_nmi: false,
            frame_buffer: [0; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize],
            priorities: [0; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize],
            chr_address_bus,
        }
    }

    pub(crate) fn dump_state(&self, vram_copy: &mut [u8; 0x4000]) -> (&[u8; 0x100], &[u8; 0x20]) {
        for i in 0..=0x3FFF {
            vram_copy[i] = self.read_byte(i as u16);
        }

        (&self.oam_ram, &self.palette_ram.data)
    }

    pub(crate) fn current_scanline(&self) -> u16 {
        self.scanline_state.scanline
    }

    pub(crate) fn current_scanline_cycle(&self) -> u16 {
        self.scanline_state.scanline_cycle
    }

    /// Return whether or not we're in the cycle immediately after rendering
    /// visible lines is complete
    pub(crate) fn output_cycle(&self) -> bool {
        self.scanline_state.scanline == 240 && self.scanline_state.scanline_cycle == 0
    }

    /// Writes to the various PPU registers mapped into the CPU address space.
    pub(crate) fn write_register(&mut self, address: u16, value: u8) {
        // TODO - Handle writes during rendering being off
        debug_assert!(address >= 0x2000 && address <= 0x2007);
        debug!("PPU register write {:04X}={:02X}", address, value);

        self.last_written_byte = value;

        match address {
            0x2000 => {
                // PPUCTRL
                self.ppu_ctrl.write_byte(value);
                self.internal_registers.temp_vram_addr = (self.internal_registers.temp_vram_addr
                    & 0xF3FF)
                    | ((value & 0b11) as u16) << 10;
            }
            0x2001 => self.ppu_mask.write_byte(value), // PPUMASK
            0x2002 => (),                              // PPUSTATUS
            0x2003 => self.oam_addr = value,           // OAMADDR
            0x2004 => {
                // OAMDATA
                self.oam_ram[self.oam_addr as usize] = value;
                self.oam_addr = self.oam_addr.wrapping_add(1);
            }
            0x2005 => {
                // PPUSCROLL
                match self.internal_registers.write_toggle {
                    false => {
                        self.internal_registers.temp_vram_addr =
                            (self.internal_registers.temp_vram_addr & 0xFFE0) | (value as u16) >> 3;
                        self.internal_registers.fine_x_scroll = value & 0x7;
                    }
                    true => {
                        self.internal_registers.temp_vram_addr =
                            (self.internal_registers.temp_vram_addr & 0x8FFF)
                                | (((value & 0x7) as u16) << 12);
                        self.internal_registers.temp_vram_addr =
                            (self.internal_registers.temp_vram_addr & 0xFC1F)
                                | (((value & 0xF8) as u16) << 2);
                    }
                };
                self.internal_registers.write_toggle = !self.internal_registers.write_toggle;
            }
            0x2006 => {
                // PPUADDR
                match self.internal_registers.write_toggle {
                    false => {
                        self.internal_registers.temp_vram_addr =
                            (self.internal_registers.temp_vram_addr & 0xFF)
                                | (((value as u16) & 0b0011_1111) << 8);
                    }
                    true => {
                        self.internal_registers.temp_vram_addr =
                            (self.internal_registers.temp_vram_addr & 0xFF00) | value as u16;
                        self.internal_registers.vram_addr = self.internal_registers.temp_vram_addr;
                    }
                };
                self.internal_registers.write_toggle = !self.internal_registers.write_toggle;
            }
            0x2007 => {
                // PPUDATA
                self.write_byte(self.internal_registers.vram_addr, value);
                match self.ppu_ctrl.increment_mode {
                    IncrementMode::Add1GoingAcross => {
                        self.internal_registers.vram_addr =
                            (self.internal_registers.vram_addr + 1) & 0x3FFF; // TODO - Does it wrap at 15 bits?
                    }
                    IncrementMode::Add32GoingDown => {
                        self.internal_registers.vram_addr =
                            (self.internal_registers.vram_addr + 32) & 0x3FFF; // TODO - Does it wrap at 15 bits?
                    }
                };
            }
            _ => panic!("Write to {:04X} not valid for PPU ({:02X})", address, value),
        }
    }

    /// Reads from the various PPU registers mapped into the CPU address space.
    pub(crate) fn read_register(&mut self, address: u16) -> u8 {
        // TODO - Handle behaviour where rendering is off
        debug_assert!(address >= 0x2000 && address <= 0x2007);
        //debug!("PPU register read {:04X}", address);

        match address {
            0x2000 => self.last_written_byte,
            0x2001 => self.last_written_byte,
            0x2002 => {
                self.internal_registers.write_toggle = false;
                self.ppu_status.read(self.last_written_byte)
            }
            0x2003 => self.last_written_byte,
            0x2004 => self.oam_ram[self.oam_addr as usize],
            0x2005 => self.last_written_byte,
            0x2006 => self.last_written_byte,
            0x2007 => {
                let value = self.read_byte(self.internal_registers.vram_addr);
                match self.ppu_ctrl.increment_mode {
                    IncrementMode::Add1GoingAcross => {
                        self.internal_registers.vram_addr += 1; // TODO - Does it wrap at 15 bits?
                    }
                    IncrementMode::Add32GoingDown => {
                        self.internal_registers.vram_addr += 32; // TODO - Does it wrap at 15 bits?
                    }
                };
                value
            }
            _ => panic!("Read from {:04X} not valid for PPU", address),
        }
    }

    /// Reads from the PPU address space
    fn read_byte(&self, address: u16) -> u8 {
        debug_assert!(address <= 0x3FFF);
        //debug!("PPU address space read {:04X}", address);

        match address {
            0x0000..=0x3EFF => self.chr_address_bus.read_byte(address),
            0x3F00..=0x3FFF => self.palette_ram.read_byte(address),
            _ => panic!("Invalid address for PPU {:04X}", address),
        }
    }

    pub(crate) fn write_dma_byte(&mut self, value: u8) {
        self.oam_ram[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    /// Writes to the PPU address space
    fn write_byte(&mut self, address: u16, value: u8) {
        debug_assert!(address <= 0x3FFF);
        debug!("PPU address space write: {:04X}={:02X}", address, value);

        match address {
            0x0000..=0x3EFF => self.chr_address_bus.write_byte(address, value, 0),
            0x3F00..=0x3FFF => {
                self.palette_ram.write_byte(address, value);
            }
            _ => panic!(
                "Invalid address for PPU write {:04X}={:02X}",
                address, value
            ),
        }
    }

    fn fetch_data(&mut self, cycle: u16) {
        if cycle == 0 {
            // On a short frame we skip the last dot of the pre-render line, so we need to load the nametable byte here instead
            // Note that this is "not short frame" because that's already been reset by this point
            // Otherwise cycle 0 is always a blank cycle with no fetches
            if !self.is_short_frame && self.ppu_mask.is_rendering_enabled() {
                self.scanline_state.nametable_byte =
                    self.read_byte(0x2000 | (self.internal_registers.vram_addr & 0x0FFF));
            }
        } else if cycle == 256 {
            // Move to the next row of tiles at dot 256
            self.internal_registers.incremement_effective_scroll_y();
        } else if cycle == 257 {
            // Copy horizontal data from temporary vram address to vram address at dot 257
            self.internal_registers.vram_addr = (self.internal_registers.vram_addr
                & 0b1111_1011_1110_0000)
                | (self.internal_registers.temp_vram_addr & 0b0000_0100_0001_1111);
        } else if cycle <= 256 || cycle >= 328 {
            match cycle % 8 {
                0 => {
                    self.scanline_state.bg_high_byte =
                        self.read_byte(self.internal_registers.next_address);

                    // Go to the next tile every 8 dots
                    self.internal_registers.increment_effective_scroll_x();
                }
                1 => {
                    self.internal_registers.next_address =
                        0x2000 | (self.internal_registers.vram_addr & 0x0FFF);

                    if cycle != 1 {
                        self.scanline_state.reload_shift_registers(); // TODO - Is this right? On cycle 9, 17, 25 ..., 257
                    }
                }
                2 => {
                    self.scanline_state.nametable_byte =
                        self.read_byte(self.internal_registers.next_address);
                }
                3 => {
                    self.internal_registers.next_address = 0x23C0
                        | (self.internal_registers.vram_addr & 0x0C00)
                        | ((self.internal_registers.vram_addr >> 4) & 0x38)
                        | ((self.internal_registers.vram_addr >> 2) & 0x07);
                }
                4 => {
                    self.scanline_state.attribute_table_byte =
                        self.read_byte(self.internal_registers.next_address);
                }
                5 => {
                    let tile_index = self.scanline_state.nametable_byte as u16 * 16;
                    self.internal_registers.next_address =
                        self.ppu_ctrl.background_tile_table_select
                            + tile_index
                            + self.internal_registers.fine_y() as u16;
                }
                6 => {
                    self.scanline_state.bg_low_byte =
                        self.read_byte(self.internal_registers.next_address);
                }
                7 => {
                    let tile_index = self.scanline_state.nametable_byte as u16 * 16;
                    self.internal_registers.next_address =
                        self.ppu_ctrl.background_tile_table_select
                            + tile_index
                            + self.internal_registers.fine_y() as u16
                            + 8;
                }
                _ => panic!("Coding error, cycle {:}", cycle),
            }
        }
    }

    /// Perform the dot based rendering for each cycle in a visible scanline
    fn draw_pixel(&mut self, scanline: u16, cycle: u16) {
        let x = cycle as u32 - 1;
        let y = scanline as u32;

        // Get background pixel
        // TODO - Handle masking left hand side
        let bg_pixel = match (
            self.ppu_mask.show_background,
            self.ppu_mask.show_background_left_side,
            cycle,
        ) {
            (false, _, _) => 0x0,
            (true, false, 0..=8) => 0x0,
            _ => self
                .scanline_state
                .bg_pixel_palette(self.internal_registers.fine_x_scroll),
        };

        // Get sprite pixel
        // TODO - Handle masking left hand side for sprites
        let _sprite_pixel = match (
            self.ppu_mask.show_sprites,
            self.ppu_mask.show_sprites_left_side,
            cycle,
        ) {
            (false, _, _) => 0x0,
            (true, false, 0..=8) => 0x0,
            _ => 0x0, // TODO - Get the right sprite pixel
        };

        // TODO - Handle priorities & transparency

        // Read the palette value for the current pixel
        let palette_index = self.read_byte(0x3F00 | bg_pixel as u16) & 0x3F;

        let color = palette::PALETTE_2C02[palette_index as usize];
        let offset = ((SCREEN_WIDTH * y + x) * 4) as usize;
        self.frame_buffer[offset] = (color & 0xFF) as u8; // Blue channel
        self.frame_buffer[offset + 1] = ((color >> 8) & 0xFF) as u8; // Green channel
        self.frame_buffer[offset + 2] = (color >> 16) as u8; // Red channel
        self.frame_buffer[offset + 3] = 0x00; // Alpha channel

        // Finally shift the registers one bit to get ready for the next dot
        self.scanline_state.bg_shift_register_high <<= 1;
        self.scanline_state.bg_shift_register_low <<= 1;
        self.scanline_state.at_shift_register_low <<= 1;
        self.scanline_state.at_shift_register_high <<= 1;
    }

    fn handle_prerender_scanline_cycle(&mut self, cycle: u16) {
        if cycle == 1 {
            self.ppu_status.vblank_started = false;
            self.ppu_status.sprite_zero_hit = false;
            self.frame_buffer.iter_mut().for_each(|m| *m = 0);
            self.priorities.iter_mut().for_each(|m| *m = 0);
        } else if (cycle >= 280) && (cycle <= 304) {
            if self.ppu_mask.is_rendering_enabled() {
                // Repeatedly copy vertical bits from temp addr to real addr to reinitialise pre-render
                self.internal_registers.vram_addr = (self.internal_registers.temp_vram_addr
                    & 0b1111_1011_1110_0000)
                    | (self.internal_registers.vram_addr & 0b0000_0100_0001_1111);

                if cycle == 304 {
                    debug!(
                        "Starting frame t={:04X} v={:04X}",
                        self.internal_registers.temp_vram_addr, self.internal_registers.vram_addr
                    );
                }
            }
        }
    }
}

impl Iterator for Ppu {
    type Item = ();

    fn next(&mut self) -> Option<()> {
        let mut trigger_cycle_skip = false;

        if self.scanline_state.scanline == 0 && self.scanline_state.scanline_cycle == 0 {
            self.is_short_frame = !self.is_short_frame;
        }

        match self.scanline_state.scanline {
            0..=239 => {
                if self.ppu_mask.is_rendering_enabled() {
                    self.fetch_data(self.scanline_state.scanline_cycle);

                    if self.scanline_state.scanline_cycle >= 1
                        && self.scanline_state.scanline_cycle <= 256
                    {
                        self.draw_pixel(
                            self.scanline_state.scanline,
                            self.scanline_state.scanline_cycle,
                        );
                    }
                }
            }
            240..=260 => {
                // PPU in idle state during scanline 240 and during VBlank except for trigering NMI
                if self.scanline_state.scanline_cycle == 1 && self.scanline_state.scanline == 241 {
                    self.ppu_status.vblank_started = true;

                    // Trigger a NMI as both vblank flag and nmi enabled are pulled up
                    if self.ppu_ctrl.nmi_enable {
                        self.trigger_nmi = true;
                        info!("Triggering NMI");
                    }
                }
            }
            261 => {
                if self.ppu_mask.is_rendering_enabled() {
                    self.fetch_data(self.scanline_state.scanline_cycle);
                }
                self.handle_prerender_scanline_cycle(self.scanline_state.scanline_cycle);

                // TODO - Technically we should also defer the nametable byte read
                if self.scanline_state.scanline_cycle == 339 && self.is_short_frame {
                    trigger_cycle_skip = true;
                }
            }
            _ => panic!("Invalid scanline {:}", self.scanline_state.scanline),
        };

        self.scanline_state.next_cycle();
        if trigger_cycle_skip && self.ppu_mask.is_rendering_enabled() {
            self.scanline_state.next_cycle()
        }

        None // PPU never exits by itself
    }
}

#[cfg(test)]
mod ppu_tests {
    use super::Ppu;
    use ppu::PpuCartridgeAddressBus;

    struct FakeCartridge {}

    impl PpuCartridgeAddressBus for FakeCartridge {
        fn read_byte(&self, _: u16) -> u8 {
            0x0
        }

        fn write_byte(&mut self, _: u16, _: u8, _: u32) {}

        fn cpu_write_byte(&mut self, _: u16, _: u8, _: u32) {}
    }

    #[test]
    fn test_setting_vram_addr() {
        let mut ppu = Ppu::new(Box::new(FakeCartridge {}));
        ppu.write_register(0x2000, 0);
        ppu.read_register(0x2002);
        ppu.write_register(0x2005, 0x7D);
        assert_eq!(ppu.internal_registers.fine_x_scroll, 0b101);
        ppu.write_register(0x2005, 0x5E);
        assert_eq!(ppu.internal_registers.temp_vram_addr, 0b1100001_01101111);
        assert_eq!(ppu.internal_registers.vram_addr, 0);
        ppu.write_register(0x2006, 0x3D);
        assert_eq!(ppu.internal_registers.temp_vram_addr, 0b0111101_01101111);
        assert_eq!(ppu.internal_registers.vram_addr, 0);
        ppu.write_register(0x2006, 0xF0);
        assert_eq!(ppu.internal_registers.temp_vram_addr, 0b0111101_11110000);
        assert_eq!(ppu.internal_registers.vram_addr, 0b0111101_11110000);
        assert_eq!(ppu.internal_registers.fine_x_scroll, 0b101);
    }

    #[test]
    fn test_setting_vram_addr_v2() {
        let mut ppu = Ppu::new(Box::new(FakeCartridge {}));
        ppu.write_register(0x2006, 0x04);
        assert_eq!(ppu.internal_registers.temp_vram_addr, 0b0000100_00000000);
        ppu.write_register(0x2005, 0x3E);
        assert_eq!(ppu.internal_registers.temp_vram_addr, 0b1100100_11100000);
        ppu.write_register(0x2005, 0x7D);
        assert_eq!(ppu.internal_registers.temp_vram_addr, 0b1100100_11101111);
        assert_eq!(ppu.internal_registers.vram_addr, 0);
        assert_eq!(ppu.internal_registers.fine_x_scroll, 0b101);
        ppu.write_register(0x2006, 0xEF);
        assert_eq!(ppu.internal_registers.temp_vram_addr, 0b1100100_11101111);
        assert_eq!(ppu.internal_registers.vram_addr, 0b1100100_11101111);
        assert_eq!(ppu.internal_registers.fine_x_scroll, 0b101);
    }
}
