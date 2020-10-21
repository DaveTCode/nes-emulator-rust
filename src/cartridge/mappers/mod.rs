pub(super) mod mmc1; // Mapper 1
pub(super) mod nrom; // Mapper 0
pub(super) mod uxrom; // Mapper 2, 94, 180

pub(crate) enum ChrData {
    Rom(Vec<u8>),
    Ram(Box<[u8; 0x2000]>),
}
