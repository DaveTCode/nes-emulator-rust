#[derive(Debug)]
pub(crate) struct PpuMask {
    pub(crate) is_grayscale: bool,
    pub(crate) show_background_left_side: bool,
    pub(crate) show_sprites_left_side: bool,
    pub(crate) show_background: bool,
    pub(crate) show_sprites: bool,
    pub(crate) emphasize_red: bool,
    pub(crate) emphasize_green: bool,
    pub(crate) emphasize_blue: bool,
}

impl PpuMask {
    pub(crate) fn new() -> Self {
        PpuMask {
            is_grayscale: false,
            show_background_left_side: false,
            show_sprites_left_side: false,
            show_background: false,
            show_sprites: false,
            emphasize_red: false,
            emphasize_green: false,
            emphasize_blue: false,
        }
    }

    pub(crate) fn write_byte(&mut self, value: u8) {
        self.is_grayscale = value & 0b1 == 1;
        self.show_background_left_side = value & 0b10 == 0b10;
        self.show_sprites_left_side = value & 0b100 == 0b100;
        self.show_background = value & 0b1000 == 0b1000;
        self.show_sprites = value & 0b1_0000 == 0b1_0000;
        self.emphasize_red = value & 0b10_0000 == 0b10_0000;
        self.emphasize_green = value & 0b100_0000 == 0b100_0000;
        self.emphasize_blue = value & 0b1000_0000 == 0b1000_0000;
    }

    pub(crate) fn is_rendering_enabled(&self) -> bool {
        self.show_background || self.show_sprites
    }
}
