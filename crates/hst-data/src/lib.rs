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
