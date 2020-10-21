use log::error;

pub(super) const MAX_SPRITES: usize = 64;
pub(super) const MAX_SPRITES_PER_LINE: usize = 8;

#[derive(Debug, Copy, Clone)]
enum SpriteState {
    ClearingSecondaryOam { pointer: usize, even_cycle: bool },
    SpriteEvaluation(SpriteEvaluation),
    SpriteFetch(SpriteFetch),
    Waiting,
}

#[derive(Debug, Copy, Clone)]
enum SpriteEvaluation {
    ReadY,
    WriteY { y: u8 },
    ReadByte { count: u8 },
    WriteByte { count: u8, value: u8 },
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
    state: SpriteState,
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
            state: SpriteState::Waiting,
        }
    }

    pub(super) fn write_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    pub(super) fn write_oam_data(&mut self, value: u8) {
        self.oam_ram[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    pub(super) fn read_oam_data(&self) -> u8 {
        match self.state {
            SpriteState::ClearingSecondaryOam {
                pointer: _,
                even_cycle: _,
            } => 0xFF,
            _ => self.oam_ram[self.oam_addr as usize],
        }
    }

    pub(super) fn dma_write(&mut self, value: u8, dma_byte: u8) {
        self.oam_ram[self.oam_addr.wrapping_add(dma_byte) as usize] = value;
    }
}

impl super::Ppu {
    /// Returns the index into palette RAM based upon the current state of the sprite
    /// shift registers and latches
    /// Note: Also shift the high/low byte shift registers
    pub(super) fn get_sprite_pixel(&mut self, cycle: u16) -> (u8, bool, bool) {
        let mut found_pixel = false;
        let mut result = (0x0u8, false, false);

        for sprite_index in 0..MAX_SPRITES_PER_LINE {
            // Skip sprites which aren't yet visible on this line
            if !self.sprite_data.sprites[sprite_index].visible
                || (self.sprite_data.sprites[sprite_index].x_location as u16 + 8) < cycle
                || (self.sprite_data.sprites[sprite_index].x_location as u16) >= cycle
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
        // First cycle is always NOOP, so use it to initialize the sprite data state machine
        if cycle == 0 {
            self.sprite_data.state = initialise_state_machine_for_scanline(scanline);
            return;
        } else if cycle == 257 {
            self.sprite_data.oam_addr = 0;
            self.sprite_data.state = SpriteState::SpriteFetch(SpriteFetch::ReadY { sprite_index: 0 });
        } else if cycle > 257 && cycle <= 320 {
            self.sprite_data.oam_addr = 0;
        }

        match scanline {
            0..=239 | 261 => self.process_frame_cycle(scanline, cycle, sprite_height, pattern_table_base),
            _ => (),
        }
    }

    fn process_frame_cycle(&mut self, scanline: u16, cycle: u16, sprite_height: u8, pattern_table_base: u16) {
        self.sprite_data.state = match self.sprite_data.state {
            SpriteState::ClearingSecondaryOam { pointer, even_cycle } => {
                debug_assert!(cycle >= 1 && cycle <= 64, "{:}", cycle);
                let new_index = if even_cycle {
                    self.sprite_data.secondary_oam_ram[pointer] = 0xFF;
                    pointer + 1
                } else {
                    pointer
                };

                if cycle == 64 {
                    self.sprite_data.secondary_oam_ram_pointer = 0;
                    SpriteState::SpriteEvaluation(SpriteEvaluation::ReadY)
                } else {
                    SpriteState::ClearingSecondaryOam {
                        pointer: new_index,
                        even_cycle: !even_cycle,
                    }
                }
            }
            SpriteState::SpriteEvaluation(eval_state) => {
                self.step_sprite_eval_machine(eval_state, scanline, cycle, sprite_height)
            }
            SpriteState::SpriteFetch(fetch_state) => {
                self.step_sprite_fetch_machine(fetch_state, scanline, cycle, sprite_height, pattern_table_base)
            }
            SpriteState::Waiting => SpriteState::Waiting,
        };
    }

    fn step_sprite_eval_machine(
        &mut self,
        state: SpriteEvaluation,
        scanline: u16,
        cycle: u16,
        sprite_height: u8,
    ) -> SpriteState {
        match state {
            SpriteEvaluation::ReadY => {
                debug_assert!(cycle >= 65 && cycle <= 256);
                if (self.sprite_data.oam_addr as usize) < self.sprite_data.oam_ram.len() {
                    SpriteState::SpriteEvaluation(SpriteEvaluation::WriteY {
                        y: self.sprite_data.oam_ram[self.sprite_data.oam_addr as usize],
                    })
                } else {
                    SpriteState::Waiting
                }
            }
            SpriteEvaluation::WriteY { y } => {
                debug_assert!(cycle >= 65 && cycle <= 256);
                if self.sprite_data.secondary_oam_ram_pointer < self.sprite_data.secondary_oam_ram.len() {
                    self.sprite_data.secondary_oam_ram[self.sprite_data.secondary_oam_ram_pointer] = y;
                }

                if scanline >= y as u16 && scanline < y as u16 + sprite_height as u16 {
                    // Start moving this sprite into OAMRAM
                    self.sprite_data.secondary_oam_ram_pointer += 1;

                    if (self.sprite_data.oam_addr as usize + 1) < self.sprite_data.oam_ram.len() {
                        self.sprite_data.oam_addr += 1;
                        SpriteState::SpriteEvaluation(SpriteEvaluation::ReadByte { count: 1 })
                    } else {
                        SpriteState::Waiting
                    }
                } else {
                    // Skip to the next sprite, this one doesn't overlap
                    if (self.sprite_data.oam_addr as usize + 4) < self.sprite_data.oam_ram.len() {
                        self.sprite_data.oam_addr += 4;
                        SpriteState::SpriteEvaluation(SpriteEvaluation::ReadY)
                    } else {
                        SpriteState::Waiting
                    }
                }
            }
            SpriteEvaluation::ReadByte { count } => {
                debug_assert!(cycle >= 65 && cycle <= 256);
                if (self.sprite_data.oam_addr as usize) < self.sprite_data.oam_ram.len() {
                    let value = self.sprite_data.oam_ram[self.sprite_data.oam_addr as usize];
                    self.sprite_data.oam_addr += 1;

                    if self.sprite_data.oam_addr as usize == self.sprite_data.oam_ram.len() - 1 {
                        SpriteState::Waiting
                    } else {
                        SpriteState::SpriteEvaluation(SpriteEvaluation::WriteByte { count, value })
                    }
                } else {
                    SpriteState::Waiting
                }
            }
            SpriteEvaluation::WriteByte { count, value } => {
                debug_assert!(cycle >= 65 && cycle <= 256);
                if self.sprite_data.secondary_oam_ram_pointer < self.sprite_data.secondary_oam_ram.len() {
                    self.sprite_data.secondary_oam_ram[self.sprite_data.secondary_oam_ram_pointer] = value;
                    self.sprite_data.secondary_oam_ram_pointer += 1;
                }

                // TODO - Somewhere here we need to consider whether to set the sprite overflow flag
                if count == 3 {
                    SpriteState::SpriteEvaluation(SpriteEvaluation::ReadY)
                } else {
                    SpriteState::SpriteEvaluation(SpriteEvaluation::ReadByte { count: count + 1 })
                }
            }
        }
    }

    /// This part of the sprite fetch pipeline does the following
    /// Fetch Y, Tile, Attr, X from Secondary OAM (4 cycles)
    /// Fetch tile from PPU whilst refetching X from secondary OAM (4 cycles)
    fn step_sprite_fetch_machine(
        &mut self,
        state: SpriteFetch,
        scanline: u16,
        cycle: u16,
        sprite_height: u8,
        pattern_table_base: u16,
    ) -> SpriteState {
        debug_assert!(cycle >= 257 && cycle <= 320);

        match state {
            SpriteFetch::ReadY { sprite_index } => SpriteState::SpriteFetch(SpriteFetch::ReadTile {
                sprite_index,
                y: self.sprite_data.secondary_oam_ram[sprite_index * 4],
            }),
            SpriteFetch::ReadTile { sprite_index, y } => SpriteState::SpriteFetch(SpriteFetch::ReadAttr {
                sprite_index,
                y,
                tile: self.sprite_data.secondary_oam_ram[sprite_index * 4 + 1],
            }),
            SpriteFetch::ReadAttr { sprite_index, y, tile } => {
                self.sprite_data.sprites[sprite_index]
                    .attribute_latch
                    .set(self.sprite_data.secondary_oam_ram[sprite_index * 4 + 2]);
                SpriteState::SpriteFetch(SpriteFetch::ReadX { sprite_index, y, tile })
            }
            SpriteFetch::ReadX { sprite_index, y, tile } => {
                self.sprite_data.sprites[sprite_index].x_location =
                    self.sprite_data.secondary_oam_ram[sprite_index * 4 + 3];
                SpriteState::SpriteFetch(SpriteFetch::FetchByte {
                    sprite_index,
                    y,
                    tile,
                    is_high_byte: false,
                })
            }
            SpriteFetch::FetchByte {
                sprite_index,
                y,
                tile,
                is_high_byte,
            } => {
                let mut value = if scanline >= y as u16 && scanline < y as u16 + sprite_height as u16 {
                    self.sprite_data.sprites[sprite_index].visible = true;

                    match get_sprite_address(
                        y as u16,
                        tile,
                        self.sprite_data.sprites[sprite_index].attribute_latch.flipped_vertical,
                        sprite_height,
                        scanline,
                        pattern_table_base,
                        is_high_byte,
                    ) {
                        Some(address) => self.read_byte(address),
                        None => 0x0,
                    }
                } else {
                    self.sprite_data.sprites[sprite_index].visible = false;

                    0x0
                };

                // Handle horizontal flipping of bits at point of write rather than at point of read
                if self.sprite_data.sprites[sprite_index]
                    .attribute_latch
                    .flipped_horizontal
                {
                    value = value.reverse_bits();
                }

                SpriteState::SpriteFetch(SpriteFetch::WriteByte {
                    sprite_index,
                    y,
                    tile,
                    value,
                    is_high_byte,
                })
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
                    (7, _) => SpriteState::Waiting,
                    (_, false) => SpriteState::SpriteFetch(SpriteFetch::FetchByte {
                        sprite_index,
                        y,
                        tile,
                        is_high_byte: true,
                    }),
                    (_, true) => SpriteState::SpriteFetch(SpriteFetch::ReadY {
                        sprite_index: sprite_index + 1,
                    }),
                }
            }
        }
    }
}

fn initialise_state_machine_for_scanline(scanline: u16) -> SpriteState {
    if scanline == 261 {
        SpriteState::Waiting
    } else {
        SpriteState::ClearingSecondaryOam {
            pointer: 0,
            even_cycle: false,
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
) -> Option<u16> {
    if scanline < y || scanline - y >= sprite_height as u16 {
        return None;
    }

    let mut fine_y = match flipped_vertical {
        true => (sprite_height as u16 - 1) - (scanline - y),
        false => scanline - y,
    };

    if (scanline - y) > 7 {
        fine_y += 8;
    }

    let top_tile_byte = match sprite_height {
        8 => tile as u16 * 16 + pattern_table_base,
        16 => ((tile as u16) & 0b1111_1110) * 16 + ((tile as u16 & 1) * 0x1000),
        _ => panic!("Wrong sprite height {:}", sprite_height),
    };

    Some(top_tile_byte + fine_y + if is_high_byte { 8 } else { 0 })
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
                Some(tile_address),
                "Tile {:02X} has the wrong address in 8 pixel mode",
                tile
            );

            // Correct pattern base for 8*8 pixel tiles
            assert_eq!(
                get_sprite_address(200, tile, false, 8, 200, 0x1000, false),
                Some(tile_address + 0x1000),
                "Tile {:02X} has the wrong address in 8 pixel mode with pattern base 0x1000",
                tile
            );

            for fine_y in 0..8 {
                assert_eq!(
                    get_sprite_address(200, tile, false, 8, 200 + fine_y, 0x0000, false),
                    Some(tile_address + fine_y),
                    "Tile {:02X} on line {:} has the wrong address in 8 pixel mode",
                    tile,
                    200 + fine_y
                );

                assert_eq!(
                    get_sprite_address(200, tile, true, 8, 200 + fine_y, 0x0000, false),
                    Some(tile_address + (7 - fine_y)),
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
                Some(tile_address),
                "Tile {:02X} has the wrong address in 16 pixel mode when mucking with the pattern base",
                tile
            );

            for fine_y in 0..16 {
                let not_flipped_fine_y = fine_y + if fine_y > 7 { 8 } else { 0 };
                let flipped_fine_y = (15 - fine_y) + if fine_y > 7 { 8 } else { 0 };

                // Check that the low byte of the tile has the right byte
                assert_eq!(
                    get_sprite_address(200, tile, false, 16, 200 + fine_y, 0x0, false),
                    Some(tile_address + not_flipped_fine_y),
                    "Tile {:02X} at scanline {:} low byte has the wrong address in 16 pixel mode",
                    tile,
                    200 + fine_y
                );

                // Check that the high byte of the tile has the right byte
                assert_eq!(
                    get_sprite_address(200, tile, false, 16, 200 + fine_y, 0x0, true),
                    Some(tile_address + not_flipped_fine_y + 8),
                    "Tile {:02X} at scanline {:} high byte has the wrong address in 16 pixel mode",
                    tile,
                    200 + fine_y
                );

                // Check that the low byte of the tile has the right byte for non-zero fine y if flipped vertically
                assert_eq!(
                    get_sprite_address(200, tile, true, 16, 200 + fine_y, 0x0, false),
                    Some(tile_address + flipped_fine_y),
                    "Tile {:02X} at scanline {:} low byte has the wrong address in 16 pixel mode when flipped",
                    tile,
                    200 + fine_y
                );

                // Check that the low byte of the tile has the right byte for non-zero fine y if flipped vertically
                assert_eq!(
                    get_sprite_address(200, tile, true, 16, 200 + fine_y, 0x0, true),
                    Some(tile_address + flipped_fine_y + 8),
                    "Tile {:02X} at scanline {:} high byte has the wrong address in 16 pixel mode when flipped",
                    tile,
                    200 + fine_y
                );
            }
        }
    }
}
