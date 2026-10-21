//! Reading Hot Shots Tennis game data: the disc image and its `.XB` archives.
pub mod exe;
#[cfg(any(test, feature = "testing"))]
pub mod fixture;
pub mod iso;
pub mod layout;
pub mod ani;
pub mod mdl;
pub mod mor;
pub mod mtl;
pub mod noi;
pub mod snd;
pub mod texhash;
pub mod tim2;
pub mod uselog;
pub mod xb;

/// The little-endian `i32` at byte `o` of `b` (models, recordings).
pub fn i32_at(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

/// The little-endian `f32` at byte `o` of `b`.
pub fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
