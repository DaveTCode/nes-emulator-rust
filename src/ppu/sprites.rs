use log::info;

pub(super) const MAX_SPRITES: usize = 64;
pub(super) const MAX_SPRITES_PER_LINE: usize = 8;

#[derive(Debug, Copy, Clone)]
enum SpriteEvaluation {
    ReadY,
    WriteY { y: u8 },
    ReadByte { count: u8 },
    WriteByte { count: u8, value: u8 },
    Completed,
}

#[derive(Debug, Copy, Clone)]
enum SpriteFetch {
    ReadY {
        sprite_index: usize,
    },
    ReadTile {
        sprite_index: usize,
        y: u8,
    },
    ReadAttr {
        sprite_index: usize,
        y: u8,
        tile: u8,
    },
    ReadX {
        sprite_index: usize,
        y: u8,
        tile: u8,
    },
    FetchByte {
        sprite_index: usize,
        y: u8,
        tile: u8,
        is_high_byte: bool,
    },
    WriteByte {
        sprite_index: usize,
        y: u8,
        tile: u8,
        value: u8,
        is_high_byte: bool,
    },
    Completed,
}

#[derive(Debug, Clone)]
struct SpriteAttribute {
    palette: u8,
    priority: bool,
    flipped_horizontal: bool,
    flipped_vertical: bool,
}

impl SpriteAttribute {
    fn set(&mut self, byte: u8) {
        self.palette = byte & 0b11;
        self.priority = byte & 0b0010_0000 == 0;
        self.flipped_horizontal = byte & 0b0100_0000 == 0b0100_0000;
        self.flipped_vertical = byte & 0b1000_0000 == 0b1000_0000;
    }
}

#[derive(Debug, Clone)]
struct Sprite {
    high_byte_shift_register: u8,
    low_byte_shift_register: u8,
    /// Holds the attribute byte for this sprite tile
    attribute_latch: SpriteAttribute,
    /// Counts down to when the sprite is made visible
    x_location: u8,
    /// Not sure about this implementation, set on each sprite during fetch to
    /// determine whether to ignore during sprite rendering.
    visible: bool,
}

pub(super) struct SpriteData {
    /// PPU register 0x2003
    oam_addr: u8,
    pub(super) oam_ram: [u8; MAX_SPRITES * 4],
    secondary_oam_ram: [u8; MAX_SPRITES_PER_LINE * 4],
    sprites: Vec<Sprite>,
    /// Internal representation of the pointer into secondary OAM RAM, reflects how many sprites have been copied
    secondary_oam_ram_pointer: usize,
    eval_state: SpriteEvaluation,
    fetch_state: SpriteFetch,
}

impl SpriteData {
    pub(super) fn new() -> Self {
        let default_sprite = Sprite {
            high_byte_shift_register: 0,
            low_byte_shift_register: 0,
            attribute_latch: SpriteAttribute {
                palette: 0,
                priority: false,
                flipped_horizontal: false,
                flipped_vertical: false,
            },
            x_location: 0,
            visible: false,
        };
        SpriteData {
            oam_addr: 0,
            oam_ram: [0; MAX_SPRITES * 4],
            secondary_oam_ram: [0xFF; MAX_SPRITES_PER_LINE * 4],
            sprites: vec![default_sprite; 8],
            secondary_oam_ram_pointer: 0,
            eval_state: SpriteEvaluation::ReadY,
            fetch_state: SpriteFetch::ReadY { sprite_index: 0 },
        }
    }

    pub(super) fn clear_sprites(&mut self) {
        for sprite in &mut self.sprites {
            sprite.visible = false;
        }
    }

    pub(super) fn write_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    pub(super) fn write_oam_data(&mut self, value: u8) {
        // Attribute byte bits always read 0, fix at set time to remove cost of masking on read
        let masked_value = if self.oam_addr & 0b11 == 0b10 {
            value & 0xE3
        } else {
            value
        };

        self.oam_ram[self.oam_addr as usize] = masked_value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub(super) fn read_oam_data(&self, cycle: u16, rendering_enabled: bool) -> u8 {
        match (cycle, rendering_enabled) {
            (0..=64, true) => 0xFF, // Return FF whilst clearing secondary OAM RAM
            _ => self.oam_ram[self.oam_addr as usize],
        }
    }

    pub(super) fn dma_write(&mut self, value: u8, dma_byte: u8) {
        // Attribute byte bits always read 0, fix at set time to remove cost of masking on read
        let masked_value = if dma_byte & 0b11 == 0b10 { value & 0xE3 } else { value };

        // Note that OAM DMA doesn't affect oam_addr
        self.oam_ram[self.oam_addr.wrapping_add(dma_byte) as usize] = masked_value;
    }
}

impl super::Ppu {
    /// Returns the index into palette RAM based upon the current state of the sprite
    /// shift registers and latches
    /// Note: Also shift the high/low byte shift registers
    pub(super) fn get_sprite_pixel(&mut self, x: u32) -> (u8, bool, bool) {
        let mut found_pixel = false;
        let mut result = (0x0u8, false, false);

        for sprite_index in 0..MAX_SPRITES_PER_LINE {
            // Skip sprites which aren't yet visible on this line
            if !self.sprite_data.sprites[sprite_index].visible
                || (self.sprite_data.sprites[sprite_index].x_location as u32 + 8) <= x
                || (self.sprite_data.sprites[sprite_index].x_location as u32) > x
            {
                continue;
            }

            if !found_pixel {
                let color_low_bit = (self.sprite_data.sprites[sprite_index].low_byte_shift_register & 0b1000_0000) >> 7;
                let color_high_bit =
                    (self.sprite_data.sprites[sprite_index].high_byte_shift_register & 0b1000_0000) >> 7;
                let color_val = color_low_bit | (color_high_bit << 1);

                // Keep looking until we find a non-transparent pixel
                if color_val != 0 {
                    let palette_number = self.sprite_data.sprites[sprite_index].attribute_latch.palette;

                    result = (
                        0b10000 | (palette_number << 2) | color_val,
                        self.sprite_data.sprites[sprite_index].attribute_latch.priority,
                        sprite_index == 0,
                    );

                    found_pixel = true;
                }
            }

            // Shift the registers
            self.sprite_data.sprites[sprite_index].high_byte_shift_register <<= 1;
            self.sprite_data.sprites[sprite_index].low_byte_shift_register <<= 1;
        }

        result
    }

    pub(super) fn process_sprite_cycle(
        &mut self,
        scanline: u16,
        cycle: u16,
        sprite_height: u8,
        pattern_table_base: u16,
    ) {
        match cycle {
            // Clear secondary OAM RAM
            0 => (),
            1..=64 => self.sprite_data.secondary_oam_ram[cycle as usize >> 2] = 0xFF,
            // Sprite evaluation
            65..=256 => {
                // Skip sprite evaluation on pre-render
                if scanline != 261 {
                    if cycle == 65 {
                        self.sprite_data.secondary_oam_ram_pointer = 0;
                        self.sprite_data.eval_state = SpriteEvaluation::ReadY;
                    }
                    self.step_sprite_eval_machine(scanline, sprite_height)
                }
            }
            // Sprite fetch
            257..=320 => {
                if cycle == 257 {
                    self.sprite_data.fetch_state = SpriteFetch::ReadY { sprite_index: 0 };
                }
                self.sprite_data.oam_addr = 0;
                self.step_sprite_fetch_machine(scanline, sprite_height, pattern_table_base)
            }
            // Read from secondary OAM RAM (but not tracking that read anywhere atm)
            321..=340 => (),
            _ => panic!("Shouldn't be calling sprite handler at dot {}", cycle),
        };
    }

    fn step_sprite_eval_machine(&mut self, scanline: u16, sprite_height: u8) {
        self.sprite_data.eval_state = match self.sprite_data.eval_state {
            SpriteEvaluation::ReadY => {
                if (self.sprite_data.oam_addr as usize) < self.sprite_data.oam_ram.len() {
                    SpriteEvaluation::WriteY {
                        y: self.sprite_data.oam_ram[self.sprite_data.oam_addr as usize],
                    }
                } else {
                    SpriteEvaluation::Completed
                }
            }
            SpriteEvaluation::WriteY { y } => {
                if self.sprite_data.secondary_oam_ram_pointer < self.sprite_data.secondary_oam_ram.len() {
                    self.sprite_data.secondary_oam_ram[self.sprite_data.secondary_oam_ram_pointer] = y;
                }

                if scanline >= y as u16 && scanline < y as u16 + sprite_height as u16 {
                    // Start moving this sprite into OAMRAM
                    self.sprite_data.secondary_oam_ram_pointer += 1;

                    if (self.sprite_data.oam_addr as usize + 1) < self.sprite_data.oam_ram.len() {
                        self.sprite_data.oam_addr += 1;

                        // Check for sprite overflow
                        if self.sprite_data.secondary_oam_ram_pointer >= self.sprite_data.secondary_oam_ram.len() {
                            self.ppu_status.sprite_overflow = true;
                            info!(
                                "Setting sprite overflow flag to true at oam_addr {}, scanline {}, dot {}, cycle {}",
                                self.sprite_data.oam_addr - 1,
                                self.scanline_state.scanline,
                                self.scanline_state.scanline_cycle,
                                self.total_cycles
                            );
                        }

                        SpriteEvaluation::ReadByte { count: 1 }
                    } else {
                        SpriteEvaluation::Completed
                    }
                } else {
                    let mut next_oam_addr = self.sprite_data.oam_addr as usize + 4;
                    // Sprite overflow bug, increment oam_addr once too many when sprite doesn't overlap
                    if self.sprite_data.secondary_oam_ram_pointer >= self.sprite_data.secondary_oam_ram.len() {
                        if next_oam_addr & 3 == 3 {
                            next_oam_addr -= 4;
                        }
                        next_oam_addr += 1;
                    }

                    // Skip to the next sprite, this one doesn't overlap
                    if next_oam_addr < self.sprite_data.oam_ram.len() {
                        self.sprite_data.oam_addr = next_oam_addr as u8;
                        SpriteEvaluation::ReadY
                    } else {
                        SpriteEvaluation::Completed
                    }
                }
            }
            SpriteEvaluation::ReadByte { count } => {
                if (self.sprite_data.oam_addr as usize) < self.sprite_data.oam_ram.len() {
                    let value = self.sprite_data.oam_ram[self.sprite_data.oam_addr as usize];

                    SpriteEvaluation::WriteByte { count, value }
                } else {
                    SpriteEvaluation::Completed
                }
            }
            SpriteEvaluation::WriteByte { count, value } => {
                if self.sprite_data.secondary_oam_ram_pointer < self.sprite_data.secondary_oam_ram.len() {
                    self.sprite_data.secondary_oam_ram[self.sprite_data.secondary_oam_ram_pointer] = value;
                    self.sprite_data.secondary_oam_ram_pointer += 1;
                }

                if (self.sprite_data.oam_addr as usize) >= self.sprite_data.oam_ram.len() - 1 {
                    SpriteEvaluation::Completed
                } else if count == 3 {
                    self.sprite_data.oam_addr += 1;
                    SpriteEvaluation::ReadY
                } else {
                    self.sprite_data.oam_addr += 1;
                    SpriteEvaluation::ReadByte { count: count + 1 }
                }
            }
            SpriteEvaluation::Completed => SpriteEvaluation::Completed,
        };
    }

    /// This part of the sprite fetch pipeline does the following
    /// Fetch Y, Tile, Attr, X from Secondary OAM (4 cycles)
    /// Fetch tile from PPU whilst refetching X from secondary OAM (4 cycles)
    fn step_sprite_fetch_machine(&mut self, scanline: u16, sprite_height: u8, pattern_table_base: u16) {
        self.sprite_data.fetch_state = match self.sprite_data.fetch_state {
            SpriteFetch::ReadY { sprite_index } => SpriteFetch::ReadTile {
                sprite_index,
                y: self.sprite_data.secondary_oam_ram[sprite_index * 4],
            },
            SpriteFetch::ReadTile { sprite_index, y } => SpriteFetch::ReadAttr {
                sprite_index,
                y,
                tile: self.sprite_data.secondary_oam_ram[sprite_index * 4 + 1],
            },
            SpriteFetch::ReadAttr { sprite_index, y, tile } => {
                self.sprite_data.sprites[sprite_index]
                    .attribute_latch
                    .set(self.sprite_data.secondary_oam_ram[sprite_index * 4 + 2]);
                SpriteFetch::ReadX { sprite_index, y, tile }
            }
            SpriteFetch::ReadX { sprite_index, y, tile } => {
                self.sprite_data.sprites[sprite_index].x_location =
                    self.sprite_data.secondary_oam_ram[sprite_index * 4 + 3];
                SpriteFetch::FetchByte {
                    sprite_index,
                    y,
                    tile,
                    is_high_byte: false,
                }
            }
            SpriteFetch::FetchByte {
                sprite_index,
                y,
                tile,
                is_high_byte,
            } => {
                let mut value = self.read_byte(get_sprite_address(
                    y as u16,
                    tile,
                    self.sprite_data.sprites[sprite_index].attribute_latch.flipped_vertical,
                    sprite_height,
                    scanline,
                    pattern_table_base,
                    is_high_byte,
                ));

                if scanline >= y as u16 && scanline < y as u16 + sprite_height as u16 {
                    self.sprite_data.sprites[sprite_index].visible = true;
                } else {
                    self.sprite_data.sprites[sprite_index].visible = false;
                }

                // Handle horizontal flipping of bits at point of write rather than at point of read
                if self.sprite_data.sprites[sprite_index]
                    .attribute_latch
                    .flipped_horizontal
                {
                    value = value.reverse_bits();
                }

                SpriteFetch::WriteByte {
                    sprite_index,
                    y,
                    tile,
                    value,
                    is_high_byte,
                }
            }
            SpriteFetch::WriteByte {
                sprite_index,
                y,
                tile,
                value,
                is_high_byte,
            } => {
                match is_high_byte {
                    true => self.sprite_data.sprites[sprite_index].high_byte_shift_register = value,
                    false => self.sprite_data.sprites[sprite_index].low_byte_shift_register = value,
                };

                match (sprite_index, is_high_byte) {
                    (7, _) => SpriteFetch::Completed,
                    (_, false) => SpriteFetch::FetchByte {
                        sprite_index,
                        y,
                        tile,
                        is_high_byte: true,
                    },
                    (_, true) => SpriteFetch::ReadY {
                        sprite_index: sprite_index + 1,
                    },
                }
            }
            SpriteFetch::Completed => SpriteFetch::Completed,
        }
    }
}

fn get_sprite_address(
    y: u16,
    tile: u8,
    flipped_vertical: bool,
    sprite_height: u8,
    scanline: u16,
    pattern_table_base: u16,
    is_high_byte: bool,
) -> u16 {
    let mut fine_y = match flipped_vertical {
        true => (sprite_height as u16 - 1).saturating_sub(scanline.saturating_sub(y)),
        false => scanline.saturating_sub(y),
    };

    if (scanline.saturating_sub(y)) > 7 {
        fine_y += 8;
    }

    let top_tile_byte = match sprite_height {
        8 => tile as u16 * 16 + pattern_table_base,
        16 => ((tile as u16) & 0b1111_1110) * 16 + ((tile as u16 & 1) * 0x1000),
        _ => panic!("Wrong sprite height {:}", sprite_height),
    };

    top_tile_byte + fine_y + if is_high_byte { 8 } else { 0 }
}

#[cfg(test)]
mod sprite_tests {
    use super::get_sprite_address;

    #[test]
    fn test_get_sprite_address_x8() {
        for tile in 0..0xFF {
            let tile_address = tile as u16 * 16;

            // No pattern base for 8*8 pixel tiles
            assert_eq!(
                get_sprite_address(200, tile, false, 8, 200, 0x0000, false),
                tile_address,
                "Tile {:02X} has the wrong address in 8 pixel mode",
                tile
            );

            // Correct pattern base for 8*8 pixel tiles
            assert_eq!(
                get_sprite_address(200, tile, false, 8, 200, 0x1000, false),
                tile_address + 0x1000,
                "Tile {:02X} has the wrong address in 8 pixel mode with pattern base 0x1000",
                tile
            );

            for fine_y in 0..8 {
                assert_eq!(
                    get_sprite_address(200, tile, false, 8, 200 + fine_y, 0x0000, false),
                    tile_address + fine_y,
                    "Tile {:02X} on line {:} has the wrong address in 8 pixel mode",
                    tile,
                    200 + fine_y
                );

                assert_eq!(
                    get_sprite_address(200, tile, true, 8, 200 + fine_y, 0x0000, false),
                    tile_address + (7 - fine_y),
                    "Tile {:02X} on line {:} has the wrong address in 8 pixel mode flipped vertically",
                    tile,
                    200 + fine_y
                );
            }
        }
    }

    #[test]
    fn test_get_sprite_address_x16() {
        for tile in 0..0xFF {
            let tile_address = ((tile as u16) & 0b1111_1110) * 16 + ((tile as u16 & 1) * 0x1000);

            // Ignore pattern base for 16 pixel tiles
            assert_eq!(
                get_sprite_address(200, tile, false, 16, 200, 0x1000, false),
                tile_address,
                "Tile {:02X} has the wrong address in 16 pixel mode when mucking with the pattern base",
                tile
            );

            for fine_y in 0..16 {
                let not_flipped_fine_y = fine_y + if fine_y > 7 { 8 } else { 0 };
                let flipped_fine_y = (15 - fine_y) + if fine_y > 7 { 8 } else { 0 };

                // Check that the low byte of the tile has the right byte
                assert_eq!(
                    get_sprite_address(200, tile, false, 16, 200 + fine_y, 0x0, false),
                    tile_address + not_flipped_fine_y,
                    "Tile {:02X} at scanline {:} low byte has the wrong address in 16 pixel mode",
                    tile,
                    200 + fine_y
                );

                // Check that the high byte of the tile has the right byte
                assert_eq!(
                    get_sprite_address(200, tile, false, 16, 200 + fine_y, 0x0, true),
                    tile_address + not_flipped_fine_y + 8,
                    "Tile {:02X} at scanline {:} high byte has the wrong address in 16 pixel mode",
                    tile,
                    200 + fine_y
                );

                // Check that the low byte of the tile has the right byte for non-zero fine y if flipped vertically
                assert_eq!(
                    get_sprite_address(200, tile, true, 16, 200 + fine_y, 0x0, false),
                    tile_address + flipped_fine_y,
                    "Tile {:02X} at scanline {:} low byte has the wrong address in 16 pixel mode when flipped",
                    tile,
                    200 + fine_y
                );

                // Check that the low byte of the tile has the right byte for non-zero fine y if flipped vertically
                assert_eq!(
                    get_sprite_address(200, tile, true, 16, 200 + fine_y, 0x0, true),
                    tile_address + flipped_fine_y + 8,
                    "Tile {:02X} at scanline {:} high byte has the wrong address in 16 pixel mode when flipped",
                    tile,
                    200 + fine_y
                );
            }
        }
    }
}
