//! The Japanese release (SCPS-15113, task M163a1a): its own km/h routines against `hst_sim::build::Build::Japan`,
//! run from a RAM image of a bot match on that disc (`fixtures/ram/jp-match.bin`: `7z e -so <state.p2s>
//! eeMemory.bin`, the state saved mid-match on `fixtures/discs/SCPS-15113.iso`). Skipped without either, even under
//! `HST_REQUIRE_DISC`: it isn't the user's disc.

use hst_oracle::{Ee, addr};
use hst_sim::build::Build;

fn rng(seed: &mut u64) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 32) as u32
}
fn uni(seed: &mut u64, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * (rng(seed) as f32 / u32::MAX as f32)
}

/// The Japanese match's RAM, its GAME overlay checked against the Japanese disc's.
fn japan() -> Option<Ee> {
    let root = hst_oracle::root();
    let (iso, ram) = (root.join("fixtures/discs/SCPS-15113.iso"), root.join("fixtures/ram/jp-match.bin"));
    if !iso.exists() || !ram.exists() {
        eprintln!("skipped: no fixtures/discs/SCPS-15113.iso or fixtures/ram/jp-match.bin");
        return None;
    }
    let game = hst_data::iso::Iso::open(&iso).ok()?.read("ZZBIN/GAME.BIN").ok()?;
    let ram = std::fs::read(ram).ok()?;
    let load = u32::from_le_bytes(game[8..12].try_into().unwrap()) as usize;
    let (at, n) = (load + 0x100, 0x1000);
    assert_eq!(ram[at..at + n], game[0x100..0x100 + n], "jp-match.bin holds another GAME overlay");
    Some(Ee::with_ram(ram))
}

/// Both of the Japanese program's km/h routines (the ball's last move, and the vector the other callers pass)
/// against the Japanese formula, over the speeds a ball reaches and well past them.
#[test]
fn japanese_kmh() {
    let Some(mut ee) = japan() else { return };
    let obj = 0x01e0_0000u32;
    for (name, at) in [("jp_ball_kmh", 0x150u32), ("jp_vec_kmh", 0x130)] {
        let f = addr(name);
        let (mut seed, mut bad, mut us_differs, n) = (0x2545_f491_4f6c_dd1du64, vec![], 0, 20_000);
        for _ in 0..n {
            let s = &mut seed;
            let v: [f32; 3] = std::array::from_fn(|_| uni(s, -1.2, 1.2) * [1.0, 0.01, 0.3][rng(s) as usize % 3]);
            for (i, c) in v.iter().enumerate() {
                ee.write(obj + at + 4 * i as u32, 4, c.to_bits() as u128);
            }
            ee.call(f, &[obj as u64], &[], 100_000);
            let (want, got) = (ee.f0(), Build::Japan.ball_kmh(v));
            if want.to_bits() != got.to_bits() {
                bad.push(format!("{v:?}: game {want}, port {got}"));
            }
            us_differs += (Build::Us.ball_kmh(v).to_bits() != want.to_bits()) as u32;
        }
        assert!(bad.is_empty(), "{name}: {} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(5)].join("\n"));
        // the check can tell the builds apart: the US formula misses the last bit on some speeds
        assert!(us_differs > 0, "{name}: the US formula matched every speed");
    }
}
