#[derive(Clone, Copy, Debug)]
pub(crate) enum MirroringMode {
    OneScreenLowerBank,
    OneScreenUpperBank,
    Vertical,
    Horizontal,
}

impl MirroringMode {
    pub(crate) fn get_mirrored_address(&self, address: u16) -> u16 {
        let adjusted_address = address - 0x2000;

        // TODO - This is full of divisions, can we make it faster with some clever bit ops instead?
        match self {
            MirroringMode::Vertical => adjusted_address & 0x7FF,
            MirroringMode::Horizontal => {
                if adjusted_address >= 0x800 {
                    ((adjusted_address - 0x800) & 0x3FF) + 0x400
                } else {
                    adjusted_address & 0x3FF
                }
            },
            MirroringMode::OneScreenLowerBank => adjusted_address % 0x400,
            MirroringMode::OneScreenUpperBank => (adjusted_address % 0x400) + 0x400,
        }
    }
}

#[cfg(test)]
mod mirroring_tests {
    use super::MirroringMode;

    #[test]
    fn test_one_screen_lower_bank() {
        for i in 0x2000..=0x2CFF {
            let result = MirroringMode::OneScreenLowerBank.get_mirrored_address(i);
            assert_eq!(result, (i & 0x23FF) - 0x2000);
        }
    }

    #[test]
    fn test_one_screen_upper_bank() {
        for i in 0x2000..=0x2CFF {
            let result = MirroringMode::OneScreenUpperBank.get_mirrored_address(i);
            assert_eq!(result, (i & 0x23FF) - 0x2000 + 0x400);
        }
    }

    // #[test]
    // fn test_horizontal_mirroring() {
    //     for i in 0x2000..=0x2CFF {
    //         let result = MirroringMode::Horizontal.get_mirrored_address(i);
    //         let expected_result = if i >= 0x2400 && i <= 0x27FF { i - 0x400 } else if i >= 0x2C00 { i - 0x400 } else { i } - 0x2000;

    //         assert_eq!(result, expected_result, "index={:02X}", i);
    //     }
    // }

    #[test]
    fn test_vertical_mirroring() {
        for i in 0x2000..=0x2CFF {
            let result = MirroringMode::Vertical.get_mirrored_address(i);
            let expected_result = i % 0x800;

            assert_eq!(result, expected_result, "index={:02X}", i);
        }
    }
}