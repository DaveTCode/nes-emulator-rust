#[derive(Debug)]
#[repr(u8)]
pub(crate) enum IncrementMode {
    Add1GoingAcross,
    Add32GoingDown,
}

#[derive(Debug)]
#[repr(u8)]
pub(crate) enum SpriteSize {
    X8,
    X16,
}

impl SpriteSize {
    pub(crate) fn pixels(&self) -> u8 {
        match self {
            SpriteSize::X8 => 8,
            SpriteSize::X16 => 16,
        }
    }
}

#[derive(Debug)]
pub(crate) struct PpuCtrl {
    pub(crate) base_name_table_select: u16,
    pub(crate) increment_mode: IncrementMode,
    pub(crate) sprite_tile_table_select: u16,
    pub(crate) background_tile_table_select: u16,
    pub(crate) sprite_size: SpriteSize,
    pub(crate) ppu_master_slave: bool,
    pub(crate) nmi_enable: bool,
}

impl PpuCtrl {
    pub(crate) fn new() -> Self {
        PpuCtrl {
            base_name_table_select: 0x0000,
            increment_mode: IncrementMode::Add1GoingAcross,
            sprite_tile_table_select: 0x0000,
            background_tile_table_select: 0x0000,
            sprite_size: SpriteSize::X8,
            ppu_master_slave: false,
            nmi_enable: false,
        }
    }

    pub(crate) fn write_byte(&mut self, value: u8) {
        self.base_name_table_select = match value & 0b11 {
            0b00 => 0x2000,
            0b01 => 0x2400,
            0b10 => 0x2800,
            0b11 => 0x2C00,
            _ => panic!("Invalid value {:} for base name table", value),
        };
        self.increment_mode = if value & 0b100 == 0 {
            IncrementMode::Add1GoingAcross
        } else {
            IncrementMode::Add32GoingDown
        };
        self.sprite_tile_table_select = if value & 0b1000 == 0 { 0x0000 } else { 0x1000 };
        self.background_tile_table_select = if value & 0b1_0000 == 0 { 0x0000 } else { 0x1000 };
        self.sprite_size = if value & 0b10_0000 == 0 {
            SpriteSize::X8
        } else {
            SpriteSize::X16
        };
        self.ppu_master_slave = value & 0b100_0000 != 0;
        self.nmi_enable = value & 0b1000_0000 != 0; // TODO - This should trigger immediate interrupt if in vblank area
    }
}
