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
    FetchHighByte {
        sprite_index: usize,
        y: u8,
        tile: u8,
    },
    WriteHighByte {
        sprite_index: usize,
        y: u8,
        tile: u8,
        value: u8,
    },
    FetchLowByte {
        sprite_index: usize,
        y: u8,
        tile: u8,
    },
    WriteLowByte {
        sprite_index: usize,
        value: u8,
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
        // This isn't really correct, the x_location is actually a counter which counts down to 0 at which points it's visible
        for (ix, sprite) in self
            .sprite_data
            .sprites
            .iter_mut()
            // TODO - Why on earth do I need to add 8 to x_location here to get it to appear in the right place??
            .filter(|s| ((s.x_location as u16) + 8 >= cycle) && ((s.x_location as u16) + 8 < cycle + 8) && s.visible)
            .enumerate()
        {
            let color_low_bit = sprite.low_byte_shift_register & 1;
            let color_high_bit = sprite.high_byte_shift_register & 1;
            let color_val = color_low_bit | (color_high_bit << 1);

            let palette_number = sprite.attribute_latch.palette;

            sprite.high_byte_shift_register >>= 1;
            sprite.low_byte_shift_register >>= 1;

            return (
                0b10000 | (palette_number << 2) | color_val,
                sprite.attribute_latch.priority,
                ix == 0,
            );
        }

        (0x0, false, false)
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
                SpriteState::SpriteFetch(SpriteFetch::FetchHighByte { sprite_index, y, tile })
            }
            SpriteFetch::FetchHighByte { sprite_index, y, tile } => {
                let value = if scanline >= y as u16 && scanline < y as u16 + sprite_height as u16 {
                    self.sprite_data.sprites[sprite_index].visible = true;

                    match get_sprite_address(
                        y as u16,
                        tile,
                        &self.sprite_data.sprites[sprite_index].attribute_latch,
                        sprite_height,
                        scanline,
                        pattern_table_base,
                    ) {
                        Some(address) => self.read_byte(address.wrapping_add(8)),
                        None => 0x0,
                    }
                } else {
                    self.sprite_data.sprites[sprite_index].visible = false;

                    0x0
                };

                SpriteState::SpriteFetch(SpriteFetch::WriteHighByte {
                    sprite_index,
                    y,
                    tile,
                    value,
                })
            }
            SpriteFetch::WriteHighByte {
                sprite_index,
                y,
                tile,
                value,
            } => {
                self.sprite_data.sprites[sprite_index].high_byte_shift_register = value;
                SpriteState::SpriteFetch(SpriteFetch::FetchLowByte { sprite_index, y, tile })
            }
            SpriteFetch::FetchLowByte { sprite_index, y, tile } => {
                let mut value = match get_sprite_address(
                    y as u16,
                    tile,
                    &self.sprite_data.sprites[sprite_index].attribute_latch,
                    sprite_height,
                    scanline,
                    pattern_table_base,
                ) {
                    Some(address) => self.read_byte(address),
                    None => 0x0,
                };

                // Handle horizontal flipping of bits at point of write rather than at point of read
                if self.sprite_data.sprites[sprite_index]
                    .attribute_latch
                    .flipped_horizontal
                {
                    value = value.reverse_bits();
                }

                SpriteState::SpriteFetch(SpriteFetch::WriteLowByte { sprite_index, value })
            }
            SpriteFetch::WriteLowByte { sprite_index, value } => {
                self.sprite_data.sprites[sprite_index].low_byte_shift_register = value;

                if sprite_index == MAX_SPRITES_PER_LINE - 1 {
                    debug_assert!(cycle == 320);
                    SpriteState::Waiting
                } else {
                    SpriteState::SpriteFetch(SpriteFetch::ReadY {
                        sprite_index: sprite_index + 1,
                    })
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
    attr: &SpriteAttribute,
    sprite_height: u8,
    scanline: u16,
    pattern_table_base: u16,
) -> Option<u16> {
    if scanline < y || scanline - y >= sprite_height as u16 {
        return None;
    }

    let tile_fine_y = scanline - y;
    let tile_fine_y_inc_flip = if attr.flipped_vertical {
        tile_fine_y
    } else {
        !tile_fine_y
    };

    match sprite_height {
        8 => Some(pattern_table_base + (16 * tile as u16) + (tile_fine_y_inc_flip & 7)),
        16 => {
            let base = tile as u16 & 1 * 0x1000;
            Some(
                base + (16 * (tile as u16 & 0b1111_1110))
                    + ((tile_fine_y_inc_flip & 8) << 1)
                    + (tile_fine_y_inc_flip & 7),
            )
        }
        _ => panic!("Wrong sprite height {:}", sprite_height),
    }
}

#[cfg(test)]
mod sprite_tests {
    use super::get_sprite_address;
    use ppu::sprites::SpriteAttribute;

    #[test]
    fn test_get_sprite_address_x8() {
        let s = SpriteAttribute {
            palette: 0,
            priority: false,
            flipped_vertical: false,
            flipped_horizontal: false,
        };
        assert_eq!(get_sprite_address(200, 8, &s, 8, 202, 0x0), Some(0x0000 + (8 * 16) + 2));
        assert_eq!(
            get_sprite_address(200, 8, &s, 8, 202, 0x1000),
            Some(0x1000 + (8 * 16) + 2)
        );
        assert_eq!(
            get_sprite_address(200, 8, &s, 8, 202, 0x1000),
            Some(0x1000 + (8 * 16) + 5)
        );
    }

    #[test]
    fn test_get_sprite_address_x16() {
        let s = SpriteAttribute {
            palette: 0,
            priority: false,
            flipped_vertical: false,
            flipped_horizontal: false,
        };
        assert_eq!(
            get_sprite_address(200, 8, &s, 16, 202, 0x0),
            Some(0x0000 + (8 * 16) + 2)
        );
        // assert_eq!(get_sprite_address(200, 8, 0, 16, 209, 0x0), Some(0x0000 + (8 * 16) + 9)); - TODO - I think this seems right, so maybe my calculations above are wrong and large sprites won't currently work?
        assert_eq!(
            get_sprite_address(200, 8, &s, 16, 202, 0x1000),
            Some(0x0000 + (8 * 16) + 2)
        );
    }
}
