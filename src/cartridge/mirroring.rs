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
            MirroringMode::Vertical => {
                if adjusted_address > 0x800 {
                    adjusted_address - 0x800
                } else {
                    adjusted_address
                }
            }
            MirroringMode::Horizontal => {
                if adjusted_address > 0x800 {
                    ((adjusted_address - 0x800) % 0x400) + 0x400
                } else {
                    adjusted_address % 0x400
                }
            }
            MirroringMode::OneScreenLowerBank => adjusted_address % 0x400,
            MirroringMode::OneScreenUpperBank => (adjusted_address % 0x400) + 0x400,
        }
    }
}
