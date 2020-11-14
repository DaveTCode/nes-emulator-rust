mod palette;
mod registers;
mod sprites;

use cartridge::PpuCartridgeAddressBus;
use cpu::interrupts::Interrupt;
use log::{debug, info};
use ppu::palette::PaletteRam;
use ppu::registers::ppuctrl::{IncrementMode, PpuCtrl};
use ppu::registers::ppumask::PpuMask;
use ppu::registers::ppustatus::PpuStatus;
use ppu::sprites::SpriteData;

pub(crate) const SCREEN_WIDTH: u32 = 256;
pub(crate) const SCREEN_HEIGHT: u32 = 240;

/// This type is used to represent a PPU cycle to make it clearer when
/// we're talking about cycles which type (PPU, CPU, APU) we mean
pub(crate) type PpuCycle = u32;

#[derive(Debug)]
struct ScanlineState {
    nametable_byte: u8,
    attribute_table_byte: u8,
    bg_low_byte: u8,
    bg_high_byte: u8,
    scanline: u16,
    dot: u16,
    bg_shift_register_high: u16,
    bg_shift_register_low: u16,
    at_shift_register_high: u8,
    at_shift_register_low: u8,
    at_shift_latch_high: u8,
    at_shift_latch_low: u8,
}

impl ScanlineState {
    fn next_cycle(&mut self) {
        self.dot += 1;
        if self.dot == 341 {
            self.dot = 0;
            self.scanline += 1;
            if self.scanline == 262 {
                self.scanline = 0;
            }
        }
    }

    fn shift_bg_registers(&mut self) {
        self.bg_shift_register_high <<= 1;
        self.bg_shift_register_low <<= 1;
        self.at_shift_register_low = (self.at_shift_register_low << 1) | self.at_shift_latch_low;
        self.at_shift_register_high = (self.at_shift_register_high << 1) | self.at_shift_latch_high;
    }

    fn reload_shift_registers(&mut self, coarse_x: u8, coarse_y: u8) {
        self.bg_shift_register_high |= self.bg_high_byte as u16;
        self.bg_shift_register_low |= self.bg_low_byte as u16;

        let at_bits = match (coarse_x & 0b10, coarse_y & 0b10) {
            (0, 0) => self.attribute_table_byte & 0b11,
            (2, 0) => (self.attribute_table_byte >> 2) & 0b11,
            (0, 2) => (self.attribute_table_byte >> 4) & 0b11,
            (2, 2) => (self.attribute_table_byte >> 6) & 0b11,
            _ => panic!(),
        };

        self.at_shift_latch_low = at_bits & 1;
        self.at_shift_latch_high = (at_bits >> 1) & 1;
    }

    /// Returns the index into the palette memory (0x00-0x3F) based on the
    /// current values of the shift registers
    fn bg_pixel_palette(&self, fine_x_scroll: u8) -> u8 {
        debug_assert!(fine_x_scroll <= 7);

        let color_low_bit = (self.bg_shift_register_low >> (15 - fine_x_scroll)) & 1;
        let color_high_bit = (self.bg_shift_register_high >> (15 - fine_x_scroll)) & 1;
        let color_index = color_low_bit | (color_high_bit << 1);

        let palette_low_bit = (self.at_shift_register_low >> (7 - fine_x_scroll)) & 1;
        let palette_high_bit = (self.at_shift_register_high >> (7 - fine_x_scroll)) & 1;
        let palette_index = palette_low_bit | (palette_high_bit << 1);

        (palette_index << 2) | color_index as u8
    }
}

#[derive(Debug)]
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
    fn increment_effective_scroll_y(&mut self) {
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

    fn increment_vram_addr(&mut self, mode: &IncrementMode) {
        match mode {
            IncrementMode::Add1GoingAcross => {
                self.vram_addr = (self.vram_addr + 1) & 0x3FFF;
            }
            IncrementMode::Add32GoingDown => {
                self.vram_addr = (self.vram_addr + 32) & 0x3FFF;
            }
        };
    }
}

pub struct Ppu {
    pub(crate) total_cycles: PpuCycle,
    frame_number: u32,
    scanline_state: ScanlineState,
    sprite_data: SpriteData,
    palette_ram: PaletteRam,
    ppu_ctrl: PpuCtrl,
    ppu_mask: PpuMask,
    ppu_status: PpuStatus,
    last_ppu_status_read_cycle: PpuCycle,
    internal_registers: InternalRegisters,
    ppu_data_buffer: u8,   // Internal buffer returned on PPUDATA reads
    last_written_byte: u8, // Stores the value last written onto the latch - TODO implement decay over time
    nmi_interrupt: Option<Interrupt>,
    pub(crate) frame_buffer: [u8; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize],
    priorities: [u8; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize],
    pub(crate) chr_address_bus: Box<dyn PpuCartridgeAddressBus>,
}

impl Ppu {
    pub fn new(chr_address_bus: Box<dyn PpuCartridgeAddressBus>) -> Self {
        Ppu {
            total_cycles: 27,
            frame_number: 1,
            scanline_state: ScanlineState {
                scanline: 0,
                nametable_byte: 0,
                attribute_table_byte: 0,
                bg_high_byte: 0,
                bg_low_byte: 0,
                dot: 27, // Skip the startup sequence but correctly set the PPU cycles consumed during it
                bg_shift_register_high: 0,
                bg_shift_register_low: 0,
                at_shift_register_high: 0,
                at_shift_register_low: 0,
                at_shift_latch_high: 0,
                at_shift_latch_low: 0,
            },
            sprite_data: SpriteData::new(),
            palette_ram: PaletteRam { data: [0; 0x20] },
            ppu_ctrl: PpuCtrl::new(),
            ppu_mask: PpuMask::new(),
            ppu_status: PpuStatus::new(),
            last_ppu_status_read_cycle: 0,
            internal_registers: InternalRegisters {
                vram_addr: 0,
                temp_vram_addr: 0,
                fine_x_scroll: 0,
                write_toggle: false,
                next_address: 0,
            },
            last_written_byte: 0x0,
            ppu_data_buffer: 0x0,
            nmi_interrupt: None,
            frame_buffer: [0; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize],
            priorities: [0; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize],
            chr_address_bus,
        }
    }

    pub(crate) fn check_trigger_irq(&mut self, clear: bool) -> bool {
        self.chr_address_bus.check_trigger_irq(clear)
    }

    pub(crate) fn dump_state(&mut self, vram_copy: &mut [u8; 0x4000]) -> &[u8; 0x100] {
        for i in 0..=0x3FFF {
            vram_copy[i] = self.read_byte(i as u16);
        }

        &self.sprite_data.oam_ram
    }

    pub(crate) fn check_ppu_nmi(&mut self, clear: bool) -> Option<Interrupt> {
        if let Some(Interrupt::NMI(cycles)) = self.nmi_interrupt {
            // Due to us checking for interrupts _after_ the last operation we might catch an interrupt
            // a CPU instruction early (STA 2002 can cause an NMI, last cycle of STA is the write, should have
            // checked for interrupts first but instead we check whether the interrupt occurred in the last 3 PPU
            // cycles.
            if cycles <= self.total_cycles - 3 {
                if clear {
                    self.nmi_interrupt = None;
                }
                return Some(Interrupt::NMI(cycles));
            }
        }

        None
    }

    pub(crate) fn current_scanline(&self) -> u16 {
        self.scanline_state.scanline
    }

    pub(crate) fn current_scanline_cycle(&self) -> u16 {
        self.scanline_state.dot
    }

    /// Return whether or not we're in the cycle immediately after rendering
    /// visible lines is complete
    pub(crate) fn output_cycle(&self) -> bool {
        self.scanline_state.scanline == 240 && self.scanline_state.dot == 0
    }

    /// Writes to the various PPU registers mapped into the CPU address space.
    pub(crate) fn write_register(&mut self, address: u16, value: u8) {
        // TODO - Handle writes during rendering being off
        debug_assert!(address >= 0x2000 && address <= 0x2007);
        debug!("PPU register write {:04X}={:02X}", address, value);

        self.last_written_byte = value;

        match address {
            0x2000 => {
                // PPUCTRL - Setting NMI enable during vblank from low to high will immediately cause an NMI
                if !self.ppu_ctrl.nmi_enable && value & 0b1000_0000 != 0 && self.ppu_status.vblank_started {
                    // Doesn't affect if vblank about to be turned off
                    if self.scanline_state.scanline != 261 || self.scanline_state.dot != 1 {
                        self.nmi_interrupt = Some(Interrupt::NMI(self.total_cycles));
                        info!("Triggering NMI");
                    }
                }

                self.ppu_ctrl.write_byte(value);

                // Setting NMI disabled within 1 cycle of triggering it will suppress as well
                if let Some(Interrupt::NMI(cycles)) = self.nmi_interrupt {
                    if !self.ppu_ctrl.nmi_enable && cycles >= self.total_cycles - 2 {
                        self.nmi_interrupt = None;
                    }
                }

                self.internal_registers.temp_vram_addr =
                    (self.internal_registers.temp_vram_addr & 0xF3FF) | ((value & 0b11) as u16) << 10;
            }
            0x2001 => self.ppu_mask.write_byte(value),        // PPUMASK
            0x2002 => (),                                     // PPUSTATUS
            0x2003 => self.sprite_data.write_oam_addr(value), // OAMADDR
            0x2004 => self.sprite_data.write_oam_data(value), // OAMDATA
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
                            (self.internal_registers.temp_vram_addr & 0x8FFF) | (((value & 0x7) as u16) << 12);
                        self.internal_registers.temp_vram_addr =
                            (self.internal_registers.temp_vram_addr & 0xFC1F) | (((value & 0xF8) as u16) << 2);
                    }
                };
                self.internal_registers.write_toggle = !self.internal_registers.write_toggle;
            }
            0x2006 => {
                // PPUADDR
                match self.internal_registers.write_toggle {
                    false => {
                        self.internal_registers.temp_vram_addr =
                            (self.internal_registers.temp_vram_addr & 0xFF) | (((value as u16) & 0b0011_1111) << 8);
                    }
                    true => {
                        self.internal_registers.temp_vram_addr =
                            (self.internal_registers.temp_vram_addr & 0xFF00) | value as u16;
                        self.internal_registers.vram_addr = self.internal_registers.temp_vram_addr;
                        self.chr_address_bus
                            .update_vram_address(self.internal_registers.vram_addr, self.total_cycles);
                    }
                };
                self.internal_registers.write_toggle = !self.internal_registers.write_toggle;
            }
            0x2007 => {
                // PPUDATA
                self.write_byte(self.internal_registers.vram_addr, value);
                self.internal_registers
                    .increment_vram_addr(&self.ppu_ctrl.increment_mode);
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
            // PPUSTATUS
            0x2002 => {
                debug!(
                    "PPUSTATUS read on scanline {}, dot {}",
                    self.scanline_state.scanline, self.scanline_state.dot
                );
                // Suppress NMI if it was triggered within the last 2 PPU cycles
                match self.nmi_interrupt {
                    None => (),
                    Some(Interrupt::NMI(cycles)) => {
                        if cycles >= self.total_cycles - 2 {
                            info!("Suppressing NMI due to proximity to PPUSTATUS read");
                            self.nmi_interrupt = None;
                        }
                    }
                    Some(_) => panic!(),
                }
                self.internal_registers.write_toggle = false;
                self.last_ppu_status_read_cycle = self.total_cycles;
                self.ppu_status.read(self.last_written_byte)
            }
            0x2003 => self.last_written_byte,
            0x2004 => self
                .sprite_data
                .read_oam_data(self.scanline_state.dot, self.ppu_mask.is_rendering_enabled()),
            0x2005 => self.last_written_byte,
            0x2006 => self.last_written_byte,
            0x2007 => {
                let mut value = self.ppu_data_buffer;
                self.ppu_data_buffer = match self.internal_registers.vram_addr {
                    0x0000..=0x3EFF => self.read_byte(self.internal_registers.vram_addr),
                    0x3F00..=0x3FFF => {
                        value = self.palette_ram.read_byte(self.internal_registers.vram_addr);
                        self.read_byte(self.internal_registers.vram_addr - 0x1000)
                    }
                    _ => panic!("Invalid address for PPU {:04X}", self.internal_registers.vram_addr),
                };
                self.internal_registers
                    .increment_vram_addr(&self.ppu_ctrl.increment_mode);
                value
            }
            _ => panic!("Read from {:04X} not valid for PPU", address),
        }
    }

    /// Reads from the PPU address space
    fn read_byte(&mut self, address: u16) -> u8 {
        debug_assert!(
            address <= 0x3FFF,
            "PPU address space is 14 bit wide, access attempted at {:04X}",
            address
        );
        //debug!("PPU address space read {:04X}", address);

        match address {
            0x0000..=0x3EFF => {
                self.chr_address_bus.update_vram_address(address, self.total_cycles);
                self.chr_address_bus.read_byte(address, self.total_cycles)
            }
            0x3F00..=0x3FFF => self.palette_ram.read_byte(address),
            _ => 0x0,
        }
    }

    pub(crate) fn write_dma_byte(&mut self, value: u8, dma_byte: u8) {
        self.sprite_data.dma_write(value, dma_byte);
    }

    /// Writes to the PPU address space
    fn write_byte(&mut self, address: u16, value: u8) {
        debug_assert!(address <= 0x3FFF);
        debug!("PPU address space write: {:04X}={:02X}", address, value);

        match address {
            0x0000..=0x3EFF => {
                self.chr_address_bus.update_vram_address(address, self.total_cycles);
                self.chr_address_bus.write_byte(address, value, self.total_cycles);
            }
            0x3F00..=0x3FFF => {
                self.palette_ram.write_byte(address, value);
            }
            _ => (),
        }
    }

    /// Handles the PPU fetch pipeline (ignoring sprites as those are handled by the SpriteData
    /// state machine)
    fn fetch_data(&mut self, cycle: u16) {
        if cycle == 0 {
            // On a short frame we skip the last dot of the pre-render line, so we need to load the
            // nametable byte here instead.
            // Note that this is even frames because that's already been incremented by this point
            if self.frame_number & 1 == 0 && self.ppu_mask.is_rendering_enabled() {
                self.scanline_state.nametable_byte =
                    self.read_byte(0x2000 | (self.internal_registers.vram_addr & 0x0FFF));
            }

            return;
        }

        match cycle & 7 {
            0 => {
                if cycle <= 256 || (cycle >= 321 && cycle <= 336) {
                    self.scanline_state.bg_high_byte = self.read_byte(self.internal_registers.next_address);

                    // Go to the next tile every 8 dots
                    self.internal_registers.increment_effective_scroll_x();

                    // Once per frame increment y to the next row
                    if cycle == 256 {
                        // Move to the next row of tiles at dot 256
                        self.internal_registers.increment_effective_scroll_y();
                    }
                }
            }
            1 => {
                if (cycle >= 9 && cycle <= 257) || (cycle >= 322 && cycle <= 337) {
                    // Note we've _just_ incremented X at the previous dot so we decrement here to
                    // get the right value for the attribute calculation
                    self.scanline_state.reload_shift_registers(
                        self.internal_registers.coarse_x().wrapping_sub(1), // TODO - Why can X ever be zero here???
                        self.internal_registers.coarse_y(),
                    );

                    if cycle == 257 {
                        // Copy horizontal data from temporary vram address to vram address at dot 257
                        self.internal_registers.vram_addr = (self.internal_registers.vram_addr & 0b1111_1011_1110_0000)
                            | (self.internal_registers.temp_vram_addr & 0b0000_0100_0001_1111);
                    }
                }

                self.internal_registers.next_address = 0x2000 | (self.internal_registers.vram_addr & 0x0FFF);
            }
            2 => {
                if cycle <= 256 || (cycle >= 321 && cycle <= 336) {
                    self.scanline_state.nametable_byte = self.read_byte(self.internal_registers.next_address);
                } else {
                    self.read_byte(self.internal_registers.next_address); // Garbage nametable byte during sprite read & end of line fetches
                }
            }
            3 => {
                self.internal_registers.next_address = 0x23C0
                    | (self.internal_registers.vram_addr & 0x0C00)
                    | ((self.internal_registers.vram_addr >> 4) & 0x38)
                    | ((self.internal_registers.vram_addr >> 2) & 0x07);
            }
            4 => {
                if cycle <= 256 || (cycle >= 321 && cycle <= 336) {
                    self.scanline_state.attribute_table_byte = self.read_byte(self.internal_registers.next_address);
                } else {
                    self.read_byte(self.internal_registers.next_address); // Garbage attribute byte during sprite read & end of line fetches
                }
            }
            5 => {
                if cycle <= 256 || cycle >= 321 {
                    let tile_index = self.scanline_state.nametable_byte as u16 * 16;
                    self.internal_registers.next_address = self.ppu_ctrl.background_tile_table_select
                        + tile_index
                        + self.internal_registers.fine_y() as u16;
                }
            }
            6 => {
                if cycle <= 256 || (cycle >= 321 && cycle <= 336) {
                    self.scanline_state.bg_low_byte = self.read_byte(self.internal_registers.next_address);
                }
            }
            7 => {
                if cycle <= 256 || cycle >= 321 {
                    let tile_index = self.scanline_state.nametable_byte as u16 * 16;
                    self.internal_registers.next_address = self.ppu_ctrl.background_tile_table_select
                        + tile_index
                        + self.internal_registers.fine_y() as u16
                        + 8;
                }
            }
            _ => panic!("Coding error, cycle {:}", cycle),
        }
    }

    /// Perform the dot based rendering for each cycle in a visible scanline
    fn draw_pixel(&mut self, scanline: u16, cycle: u16) {
        let x = cycle as PpuCycle - 1;
        let y = scanline as u32;
        let offset = ((SCREEN_WIDTH * y + x) * 4) as usize;

        let color = if self.ppu_mask.is_rendering_enabled() {
            // Get background pixel
            let bg_pixel = match (
                self.ppu_mask.show_background,
                self.ppu_mask.show_background_left_side,
                x,
            ) {
                (false, _, _) => 0x0,
                (true, false, 0..=7) => 0x0,
                _ => self
                    .scanline_state
                    .bg_pixel_palette(self.internal_registers.fine_x_scroll),
            };

            // Get sprite pixel
            let (sprite_pixel, sprite_priority_over_bg, is_sprite_zero) =
                match (self.ppu_mask.show_sprites, self.ppu_mask.show_sprites_left_side, x) {
                    (false, _, _) => (0x0, false, false),
                    (true, false, 0..=7) => {
                        self.get_sprite_pixel(x); // Throwaway read to force a register shift for relevant sprites even if the left side is masked
                        (0x0, false, false)
                    }
                    _ => self.get_sprite_pixel(x),
                };

            if is_sprite_zero
                && (sprite_pixel & 0b11) != 0
                && (bg_pixel & 0b11) != 0
                && x != 0xFF
                && !self.ppu_status.sprite_zero_hit
            {
                info!(
                    "Sprite zero hit on cycle {} scanline {} dot {} bg_pixel {:02X} sprite_pixel {:02X}",
                    self.total_cycles, self.scanline_state.scanline, self.scanline_state.dot, bg_pixel, sprite_pixel
                );
                self.ppu_status.sprite_zero_hit = true;
            }

            // Pass the resulting values through a priority multiplexer to get the final pixel value
            let multiplexed_pixel = match (bg_pixel & 0b11, sprite_pixel & 0b11, sprite_priority_over_bg) {
                (0, 0, _) => 0x0,
                (0, _, _) => sprite_pixel,
                (_, 0, _) => bg_pixel,
                (_, _, true) => sprite_pixel,
                (_, _, false) => bg_pixel,
            };

            // Read the palette value for the current pixel
            let palette_index = self.read_byte(0x3F00 | multiplexed_pixel as u16) & 0x3F;

            palette::PALETTE_2C02[palette_index as usize]
        } else if self.internal_registers.vram_addr & 0x3F00 == 0x3F00 {
            palette::PALETTE_2C02[self.internal_registers.vram_addr as usize & 0x1F]
        } else {
            0x0
        };

        self.frame_buffer[offset] = (color & 0xFF) as u8; // Blue channel
        self.frame_buffer[offset + 1] = ((color >> 8) & 0xFF) as u8; // Green channel
        self.frame_buffer[offset + 2] = (color >> 16) as u8; // Red channel
        self.frame_buffer[offset + 3] = 0x00; // Alpha channel
    }

    fn handle_prerender_scanline_cycle(&mut self, cycle: u16) {
        if cycle == 0 {
            self.ppu_status.sprite_overflow = false;
            self.ppu_status.sprite_zero_hit = false;
            self.frame_buffer.iter_mut().for_each(|m| *m = 0);
            self.priorities.iter_mut().for_each(|m| *m = 0);
            self.sprite_data.clear_sprites();
        } else if cycle == 1 {
            self.ppu_status.vblank_started = false;
        } else if (cycle >= 280) && (cycle <= 304) && self.ppu_mask.is_rendering_enabled() {
            // Repeatedly copy vertical bits from temp addr to real addr to reinitialise pre-render
            self.internal_registers.vram_addr = (self.internal_registers.temp_vram_addr & 0b1111_1011_1110_0000)
                | (self.internal_registers.vram_addr & 0b0000_0100_0001_1111);
        }
    }
}

impl Iterator for Ppu {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        let mut trigger_cycle_skip = false;

        match self.scanline_state.scanline {
            0..=239 | 261 => {
                if self.ppu_mask.is_rendering_enabled() {
                    // Background registers shift on dots 2-256 322-337 inclusive EXCEPT on pre-render where they only shift during 322-337
                    if (self.scanline_state.dot >= 2
                        && self.scanline_state.dot <= 256
                        && self.scanline_state.scanline != 261)
                        || (self.scanline_state.dot >= 322 && self.scanline_state.dot <= 337)
                    {
                        self.scanline_state.shift_bg_registers();
                    }

                    self.fetch_data(self.scanline_state.dot);

                    self.process_sprite_cycle(
                        self.scanline_state.scanline,
                        self.scanline_state.dot,
                        self.ppu_ctrl.sprite_size.pixels(),
                        self.ppu_ctrl.sprite_tile_table_select,
                    );

                    if self.scanline_state.scanline == 261
                        && self.scanline_state.dot == 339
                        && self.frame_number & 1 == 1
                    {
                        trigger_cycle_skip = true;
                    }
                }

                if self.scanline_state.scanline != 261 && self.scanline_state.dot >= 1 && self.scanline_state.dot <= 256
                {
                    self.draw_pixel(self.scanline_state.scanline, self.scanline_state.dot);
                }

                if self.scanline_state.scanline == 261 {
                    self.handle_prerender_scanline_cycle(self.scanline_state.dot);
                }
            }
            240..=260 => {
                // PPU in idle state during scanline 240 and during VBlank except for triggering NMI
                if self.scanline_state.dot == 1 && self.scanline_state.scanline == 241 {
                    info!("Vblank set cycle {}", self.total_cycles);
                    if self.last_ppu_status_read_cycle != self.total_cycles {
                        self.ppu_status.vblank_started = true;

                        // Trigger a NMI as both vblank flag and nmi enabled are pulled up
                        if self.ppu_ctrl.nmi_enable {
                            self.nmi_interrupt = Some(Interrupt::NMI(self.total_cycles));
                            info!("Triggering NMI");
                        }
                    } else {
                        info!("Skipping NMI because PPUSTATUS read was 1 cycle ago");
                    }
                }
            }
            _ => panic!("Invalid scanline {:}", self.scanline_state.scanline),
        };

        self.scanline_state.next_cycle();
        if trigger_cycle_skip && self.ppu_mask.is_rendering_enabled() {
            self.scanline_state.next_cycle()
        }

        // Check for rendering enabled update (delayed by one cycle from write)
        self.ppu_mask.update_rendering_enabled();

        if self.scanline_state.dot == 0 && self.scanline_state.scanline == 0 {
            self.frame_number += 1;
        }

        // Track total PPU cycles for components which need to know. Bit sketchy here that it wraps
        self.total_cycles = self.total_cycles.wrapping_add(1);

        None // PPU never exits by itself
    }
}

#[cfg(test)]
mod ppu_tests {
    use cartridge::PpuCartridgeAddressBus;
    use cpu::CpuCycle;
    use ppu::Ppu;
    use ppu::PpuCycle;

    struct FakeCartridge {}

    impl PpuCartridgeAddressBus for FakeCartridge {
        fn check_trigger_irq(&mut self, _: bool) -> bool {
            false
        }

        fn update_vram_address(&mut self, _: u16, _: PpuCycle) {}

        fn read_byte(&mut self, _: u16, _: PpuCycle) -> u8 {
            0x0
        }

        fn write_byte(&mut self, _: u16, _: u8, _: PpuCycle) {}

        fn cpu_write_byte(&mut self, _: u16, _: u8, _: CpuCycle) {}
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
