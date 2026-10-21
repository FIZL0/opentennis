//! P117: the court pull of a rally shot's aim (`hst_sim::shot::Margins::inside`) with the smash's low margins and
//! a special shot's unscaled ones, through the oracle. Needs `context/iso/` and `context/ram/s05.bin` (skips
//! without them).

use hst_oracle::{Ee, HstEe, addr};
use hst_sim::shot::Margins;

fn rng(seed: &mut u64) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 32) as u32
}
fn uni(seed: &mut u64, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * (rng(seed) as f32 / u32::MAX as f32)
}
fn int(seed: &mut u64, lo: u32, hi: u32) -> i32 {
    (lo + rng(seed) % (hi - lo + 1)) as i32
}

/// The disc's SYSTEM.CNF and GAME.BIN (for `hst_data::exe::Game`).
fn disc() -> Option<(Vec<u8>, Vec<u8>)> {
    let read = |f: &str| hst_data::fixture::read(format!("context/iso/{f}")).ok();
    Some((read("SYSTEM.CNF")?, read("ZZBIN/GAME.BIN")?))
}

/// The game's debug switch that forces every special effect on (off on the retail disc).
const FORCE_SPECIALS: u32 = 0x41_0870;

/// 20 000 pulls over random classes, kinds, blends (low, −0, 0, high), player counts, characters, hits and aims in
/// the court, with every player's latched lock (the match's per-player copy of the last lock grade and offset,
/// which the special gate reads) graded at random and a random last hitter: the game's pull delta added to the aim
/// against the port, bit for bit. A pull is special when the hitter's latched grade is clean (1 or 2) and the
/// launching character's effect row for (class, kind) bends or curves.
#[test]
fn court_pull() {
    pulls(false, 0x6a09_e667_f3bc_c908);
}

/// The same with the debug switch forcing the special gate: a pull is special exactly when the launching
/// character's effect row for (class, kind) has a bend or curve (a rally slice by characters 3 and 8, every
/// stroke lob), and then keeps its margins unscaled.
#[test]
fn court_pull_forced() {
    pulls(true, 0xbb67_ae85_84ca_a73b);
}

fn pulls(forced: bool, seed: u64) {
    let (Some(mut ee), Some((cnf, bin))) = (Ee::from_ram("s05.bin"), disc()) else { return };
    let g = hst_data::exe::Game::new(&cnf, &bin).unwrap();
    let (lines, angle) = g.court_margins();
    let m = Margins { lines, angle };
    let effects: Vec<_> = (0..14).map(|c| g.shot_effects(c)).collect();
    let (at, out, hit_at, aim_at) = (addr("court_pull"), 0x01e1_0000u32, 0x01e1_0010u32, 0x01e1_0020u32);
    let gm = ee.read(0x42_2f80, 4) as u32;
    let players: Vec<u32> = (0..4).map(|k| ee.read(gm + 0xa8 + 4 * k, 4) as u32).collect();
    let latch = ee.read(gm + 0xb8, 4) as u32 + 0x1_0f24;
    ee.write(FORCE_SPECIALS, 1, forced as u128);
    let f = |v: f32| v.to_bits() as u128;
    let (mut seed, mut bad, n) = (seed, vec![], 20_000);
    let (mut low_moved, mut special_moved) = (0, 0);
    for _ in 0..n {
        let s = &mut seed;
        let class = int(s, 1, 3) as u8;
        let kind = int(s, 0, if class == 3 { 1 } else { 4 });
        let blend = [-0.5, -0.0, 0.0, 0.5, uni(s, -1.0, 1.0)][int(s, 0, 4) as usize];
        let count = [2, 4][int(s, 0, 1) as usize];
        let ch = int(s, 0, 13) as usize;
        let end = if rng(s) & 1 == 0 { 1.0 } else { -1.0 };
        let hit = [uni(s, -7.0, 7.0), uni(s, 0.2, 2.5), -end * uni(s, 0.5, 13.0)];
        // an aim outside the court takes the game off the pure path (a report through the kernel); play never
        // aims there
        let side = if count > 2 { 5.48 } else { 4.11 };
        let aim = [uni(s, -side, side), 0.0, end * uni(s, 1.0, 11.88)];
        for &q in &players {
            ee.write(q + 0x3ee8, 1, int(s, 0, 3) as u128);
            ee.write(q + 0x3fa0, 4, (int(s, 0, 8) - 4) as u32 as u128);
        }
        let lat: Vec<(u8, i32)> = (0..4).map(|_| (int(s, 0, 3) as u8, int(s, 0, 8) - 4)).collect();
        for (k, &(grade, offset)) in lat.iter().enumerate() {
            ee.write(latch + 4 * k as u32, 4, grade as u128);
            ee.write(latch + 0x10 + 4 * k as u32, 4, offset as u32 as u128);
        }
        let hitter = int(s, 0, count as u32 - 1) as usize;
        ee.write(0x42_3058, 4, hitter as u128);
        ee.write(0x42_2fa4, 4, count as u128);
        for (a, v) in [(hit_at, hit), (aim_at, aim)] {
            for (k, x) in v.iter().chain(&[0.0]).enumerate() {
                ee.write(a + 4 * k as u32, 4, f(*x));
            }
        }
        ee.write(out, 16, 0);
        ee.call(at, &[players[0] as u64, out as u64, class as u64, kind as u64, ch as u64 + 3, hit_at as u64, aim_at as u64], &[blend], 1_000_000);
        let d = [0, 8].map(|o| f32::from_bits(ee.read(out + o, 4) as u32));
        let want = [hst_sim::ps2::add(aim[0], d[0]), hst_sim::ps2::add(aim[2], d[1])];
        let e = &effects[ch];
        let row = match (class, kind) {
            (1, 1) => e.bend[2],
            (1, 3) => e.curve,
            _ => 0.0,
        };
        let (grade, offset) = lat[hitter];
        let gate = forced || hst_sim::shot::special(class, kind, false, grade, offset);
        let (special, low, doubles) = (gate && row != 0.0, blend < 0.0, count > 2);
        let pulled = m.inside(class, kind, low, doubles, ch, special, hit, aim);
        low_moved += (low && pulled != m.inside(class, kind, false, doubles, ch, special, hit, aim)) as usize;
        special_moved += (special && pulled != m.inside(class, kind, low, doubles, ch, false, hit, aim)) as usize;
        if [pulled[0], pulled[2]].map(f32::to_bits) != want.map(f32::to_bits) {
            bad.push(format!("class {class} kind {kind} blend {blend} players {count} ch {ch} hit {hit:?} aim {aim:?}: {pulled:?} want {want:?}"));
        }
    }
    for b in bad.iter().take(10) {
        eprintln!("{b}");
    }
    eprintln!("{n} pulls, {low_moved} moved by the low margins, {special_moved} by a special's unscaled ones, {} wrong", bad.len());
    assert!(bad.is_empty() && low_moved > 100 && special_moved > 10, "{} of {n} wrong", bad.len());
}
