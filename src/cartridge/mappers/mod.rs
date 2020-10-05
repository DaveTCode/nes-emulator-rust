pub(super) mod mmc1;
pub(super) mod nrom;

pub(crate) enum ChrData {
    Rom(Vec<u8>),
    Ram([u8; 0x2000]),
}
