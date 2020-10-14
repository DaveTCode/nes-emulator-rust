use log::error;

pub(super) const PALETTE_2C02: [u32; 0x40] = [
    0x7C7C7C, 0x0000FC, 0x0000BC, 0x4428BC, 0x940084, 0xA80020, 0xA81000, 0x881400, 0x503000, 0x007800, 0x006800,
    0x005800, 0x004058, 0x000000, 0x000000, 0x000000, 0xBCBCBC, 0x0078F8, 0x0058F8, 0x6844FC, 0xD800CC, 0xE40058,
    0xF83800, 0xE45C10, 0xAC7C00, 0x00B800, 0x00A800, 0x00A844, 0x008888, 0x000000, 0x000000, 0x000000, 0xF8F8F8,
    0x3CBCFC, 0x6888FC, 0x9878F8, 0xF878F8, 0xF85898, 0xF87858, 0xFCA044, 0xF8B800, 0xB8F818, 0x58D854, 0x58F898,
    0x00E8D8, 0x787878, 0x000000, 0x000000, 0xFCFCFC, 0xA4E4FC, 0xB8B8F8, 0xD8B8F8, 0xF8B8F8, 0xF8A4C0, 0xF0D0B0,
    0xFCE0A8, 0xF8D878, 0xD8F878, 0xB8F8B8, 0xB8F8D8, 0x00FCFC, 0xF8D8F8, 0x000000, 0x000000,
];

const PALETTE_MIRRORS: [Option<usize>; 0x20] = [
    Some(0x10), None, None, None, None, None, None, None,
    Some(0x18), None, None, None, None, None, None, None,
    Some(0x00), None, None, None, None, None, None, None,
    Some(0x08), None, None, None, None, None, None, None,
];

pub(super) struct PaletteRam {
    pub(super) data: [u8; 0x20],
}

impl PaletteRam {
    pub(super) fn read_byte(&self, address: u16) -> u8 {
        debug_assert!(address >= 0x3F00 && address <= 0x3FFF);

        self.data[address as usize & 0x1F]
    }

    pub(super) fn write_byte(&mut self, address: u16, value: u8) {
        debug_assert!(address >= 0x3F00 && address <= 0x3FFF);

        let index = address as usize & 0x1F;
        let mirror = PALETTE_MIRRORS[index];
        self.data[index] = value;
        
        if let Some(mirrored_address) = mirror {
            self.data[mirrored_address] = value;
        }
    }
}

#[cfg(test)]
mod palette_ram_tests {
    use super::PaletteRam;

    #[test]
    fn test_mirrors() {
        let p = PaletteRam {
            data: [
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
                0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
            ],
        };

        for i in 0x0..=0x20 {
            for bank in 0..7 {
                let base_address = i + 0x3F00;
                let mirrored_address = base_address + bank * 0x20;
                assert_eq!(
                    p.read_byte(base_address),
                    p.read_byte(mirrored_address),
                    "{:04X}!={:04X}",
                    base_address,
                    mirrored_address
                );
            }
        }
    }
}
