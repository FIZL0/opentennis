//! Hot Shots Tennis on the interpreter: the user's disc (checked by `hst_data::exe::Game`), RAM dumps from
//! `context/ram/` and the function names in `research/oracle.txt`. Everything game-specific in this crate is here.

use crate::{Ee, RAM};
use std::path::PathBuf;

/// The repo root (for `context/` and `research/`).
pub fn root() -> PathBuf {
    hst_data::fixture::root().to_path_buf()
}

/// Function addresses by name from `research/oracle.txt` (`name hexaddr` per line, `#` comments).
pub fn addr(name: &str) -> u32 {
    let t = std::fs::read_to_string(root().join("research/oracle.txt")).expect("research/oracle.txt");
    t.lines()
        .filter_map(|l| l.split('#').next()?.split_once(char::is_whitespace))
        .find(|(n, _)| *n == name)
        .map(|(_, a)| u32::from_str_radix(a.trim().trim_start_matches("0x"), 16).unwrap())
        .unwrap_or_else(|| panic!("{name} not in research/oracle.txt"))
}

/// The boot ELF and the GAME overlay (bytes, load address) from `context/iso/`, the disc checked to be the supported
/// one; `None` without a disc (failing instead under `HST_REQUIRE_DISC`: `hst_data::fixture`).
pub fn program() -> Option<(Vec<u8>, Vec<u8>, u32)> {
    let read = |f: &str| hst_data::fixture::read(format!("context/iso/{f}")).ok();
    let (cnf, boot, game) = (read("SYSTEM.CNF")?, read("SCUS_976.10")?, read("ZZBIN/GAME.BIN")?);
    hst_data::exe::Game::new(&cnf, &game).expect("supported disc");
    // the overlay header (checked just above) names its own load address
    let load = u32::from_le_bytes(game[8..12].try_into().unwrap());
    Some((boot, game, load))
}

/// The game's machines: `use hst_oracle::HstEe` to get `Ee::game()` and `Ee::from_ram(name)`.
pub trait HstEe: Sized {
    /// The boot program and the GAME overlay as loaded in a match, from `context/iso/`; `None` without a disc.
    fn game() -> Option<Self>;
    /// A 32 MB EE RAM dump (`context/ram/<name>`) as the machine's memory, its GAME overlay checked against the
    /// disc's; `None` when either is missing.
    fn from_ram(name: &str) -> Option<Self>;
}

impl HstEe for Ee {
    fn game() -> Option<Self> {
        let (boot, game, load) = program()?;
        let mut ee = Self::from_program(RAM, &boot, &[(load, &game)]).expect("boot ELF");
        boot_vu0(&mut ee);
        Some(ee)
    }

    fn from_ram(name: &str) -> Option<Self> {
        let ram = hst_data::fixture::read(format!("context/ram/{name}")).ok()?;
        let (_, game, load) = program()?;
        let (at, n) = (load as usize + 0x100, 0x1000); // code, past the overlay header
        assert_eq!(ram[at..at + n], game[0x100..0x100 + n], "{name} holds a different GAME overlay");
        let mut ee = Self::with_ram(ram);
        boot_vu0(&mut ee);
        Some(ee)
    }
}

/// VU0's micro programs (slerp among them), uploaded as the game does it at boot: the VIF0 DMA chain in the boot
/// program's data, run through `ps2-oracle`'s VIF0. A RAM dump doesn't hold VU0 memory, but the chain is in it too.
fn boot_vu0(ee: &mut Ee) {
    ee.vif0_dma(addr("vu0_boot_chain"), false);
}
