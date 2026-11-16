//! The European disc alone at 60 Hz (M170): its trajectory tables were re-solved by the authors for its 50 Hz tick
//! (speeds ~×1.2, frames ~×0.83), so a 60 Hz match regenerates each one on the ball's own 60 Hz step. Close to the US
//! game, not the original: the US tables can't be rebuilt bit for bit (M163a2's journal), so a match on these is
//! never played online (the gameplay digest reads the disc's own bytes and differs from every other release's).
//!
//! A cell keeps its 50 Hz elevation (three cells in four are the US cell's) and takes the 60 Hz speed that lands
//! nearest the cell's target, searched from the 50 Hz speed ÷1.2; its frames are the flight's to that landing. A
//! cell the authors' solver didn't land on its target either (the scaled cell lands over [`REACHED`] from it: a short
//! target from deep in the court), or that no speed reaches, is the 50 Hz cell scaled (speed ÷1.2, frames ×1.2):
//! there the US cell follows the scaled one, not the flight.
//! Each cell is flown with the launch spin its shot record gives (the stroke and volley spin shaped by the hit,
//! `params::rally_spin`), from the program's records turned back to their 60 Hz values (`hst_data::exe`).
//!
//! How close it comes to the US disc is measured in `research/journal/2026-11-16-m170-eu-alone-60hz/`.
#![allow(clippy::float_arithmetic)]

use super::{MAX_SPEED, Table, fields, fly, place, word};
use crate::ball::V3;
use crate::params::{self, ShotParams};
use crate::ps2::F;
use crate::shot::{Bounds, N, RADIUS, class, kind};
use hst_data::exe::{Game, ShotVariant};
use hst_data::iso::Iso;
use hst_data::xb::{self, Archive};

/// 50 Hz ÷ 60 Hz: what the authors scaled the per-frame amounts by.
const RATE: f64 = 1.2;

/// Routes the disc reads of a 50 Hz disc's trajectory archives through [`rewrite`] (once per run; harmless again).
pub fn install() {
    hst_data::iso::set_retime(rewrite);
}

/// How a table's cells get their launch spin.
#[derive(Clone)]
pub enum Spin {
    /// One record spin (serves, smashes).
    Fixed(f32),
    /// A stroke's or volley's record (class, kind), shaped by the hit.
    Rally(Vec<f32>, usize, usize),
}

impl Spin {
    pub fn at(&self, hit: V3, target: V3) -> f32 {
        match self {
            Spin::Fixed(s) => *s,
            Spin::Rally(rec, c, k) => params::rally_spin(rec, *c, *k, hit, target).0,
        }
    }
}

/// What a table file is, from its name (`…\tr_pc07_serv2.dat`, `tr_pc00_strk1_dw2.dat`): shot class, kind and
/// variant (0 `up1`, 1 `dw1`, 2 `dw2`, 3 `dw3`).
pub fn what(name: &str) -> Option<(u8, usize, Option<usize>)> {
    let name = name.to_ascii_lowercase().replace('\\', "/");
    let file = name.rsplit('/').next()?.strip_suffix(".dat")?;
    let rest = file.strip_prefix("tr_pc")?.get(3..)?;
    let (base, variant) = match rest.split_once('_') {
        Some((b, v)) => (b, Some(["up1", "dw1", "dw2", "dw3"].iter().position(|&x| x == v)?)),
        None => (rest, None),
    };
    let (stem, k) = base.split_at(base.len().checked_sub(1)?);
    let k: usize = k.parse().ok()?;
    let c = match stem {
        "serv" => class::SERVE,
        "strk" => class::STROKE,
        "voly" => class::VOLLEY,
        "smsh" => class::SMASH,
        _ => return None,
    };
    Some((c, k, variant))
}

/// A table's axes and launch spin for character `c`, by what it is.
pub fn shape(params: &ShotParams, variants: &[ShotVariant], c: usize, (cl, k, v): (u8, usize, Option<usize>)) -> (Bounds, Spin) {
    let r = params::record_of(c);
    let rec: Vec<f32> = match v.and_then(|v| variants.iter().find(|x| x.class == cl as usize && x.kind == k && x.uses[v] != 0)) {
        Some(x) => params.variant(cl as usize, k, r, x.weight).to_vec(),
        None => params.record(cl as usize, k, r).to_vec(),
    };
    match cl {
        class::SERVE => (Bounds::serve(k as i32 == kind::UNDERHAND, RADIUS), Spin::Fixed(params::spin(&rec))),
        class::SMASH => (Bounds::smash(k as i32), Spin::Fixed(params::spin(&rec))),
        class::STROKE => (Bounds::stroke(k as i32, 0.0), Spin::Rally(rec, cl as usize, k)),
        _ => (Bounds::volley(k as i32, 0.0), Spin::Rally(rec, cl as usize, k)),
    }
}

/// The 50 Hz cell scaled to 60 Hz: speed ÷1.2, frames ×1.2.
fn scaled(w: u32) -> u32 {
    let (e, s, f) = fields(w);
    word(e, (s as f64 / RATE).round() as i32, (f as f64 * RATE).round() as i32)
}

/// How far (m) the scaled cell may land from its target for the cell to be solved onto it: further, the authors'
/// solver couldn't reach the target either (a short target from deep in the court, the speed at its cap) and the
/// scaled cell is kept as it lands.
const REACHED: f32 = 0.5;

/// One 50 Hz cell at 60 Hz (see the module notes).
fn cell(w: u32, hit: V3, want: F, spin: f32) -> u32 {
    let (e, s, _) = fields(w);
    if s <= 0 {
        return scaled(w);
    }
    // the landing at speed `s` (None: never lands, counts as long)
    let land = |s: i32| fly(hit, e, s, spin);
    let long = |s: i32| land(s).is_none_or(|f| f.landing >= want);
    let guess = ((s as f64 / RATE).round() as i32).clamp(1, MAX_SPEED);
    let Some(kept) = land(guess) else { return scaled(w) };
    if (kept.landing - want).0.abs() > REACHED {
        return scaled(w);
    }
    // bracket the target between a short speed and a long one, stepping out from the guess
    let (mut lo, mut hi);
    let mut step = 1;
    if long(guess) {
        hi = guess;
        loop {
            lo = guess - step;
            if lo < 1 {
                return scaled(w);
            }
            if !long(lo) {
                break;
            }
            hi = lo;
            step *= 2;
        }
    } else {
        lo = guess;
        loop {
            hi = (guess + step).min(MAX_SPEED);
            if long(hi) {
                break;
            }
            if hi == MAX_SPEED {
                return scaled(w);
            }
            lo = hi;
            step *= 2;
        }
    }
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if long(mid) { hi = mid } else { lo = mid }
    }
    let short = land(lo).expect("lands short");
    let pick = match land(hi) {
        Some(h) if h.landing - want < want - short.landing => (hi, h),
        _ => (lo, short),
    };
    word(e, pick.0, pick.1.frames)
}

impl Table {
    /// This 50 Hz table at 60 Hz over its axes `b` (see the module notes); `spin(hit, target)` is a shot's launch
    /// spin.
    pub fn sixty_hz(&self, b: &Bounds, spin: impl Fn(V3, V3) -> f32) -> Table {
        Table(
            (0..N * N * N)
                .map(|i| {
                    let (hit, target) = place(b, i);
                    cell(self.0[i], hit, F(target[2]), spin(hit, target))
                })
                .collect(),
        )
    }
}

/// Rebuilt archives by path and the disc bytes' hash.
type Built = std::collections::HashMap<(String, u64), Vec<u8>>;

/// Archives rebuilt so far this run.
static BUILT: std::sync::LazyLock<std::sync::Mutex<Built>> = std::sync::LazyLock::new(Default::default);

fn fnv(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

/// `TRAJ/TRAJnn{A,B}.XB` of a 50 Hz disc with every table at 60 Hz ([`Table::sixty_hz`], side by side on threads),
/// as an archive of raw entries; anything else (or an archive that doesn't parse) as it is. Built once per run.
pub fn rewrite(iso: &mut Iso, path: &str, data: Vec<u8>) -> Vec<u8> {
    let Some(c) = path.strip_prefix("TRAJ/TRAJ").and_then(|r| r.get(..2)).and_then(|n| n.parse::<usize>().ok()) else { return data };
    let key = (path.to_string(), fnv(&data));
    if let Some(d) = BUILT.lock().unwrap().get(&key) {
        return d.clone();
    }
    let (Ok(cnf), Ok(bin)) = (iso.read_unlogged("SYSTEM.CNF"), iso.read_unlogged("ZZBIN/GAME.BIN")) else { return data };
    let Ok(game) = Game::new(&cnf, &bin) else { return data };
    let src = game.shot_params();
    let params = ShotParams::build(&src.base, &src.kinds, &src.weights, src.middle_mix);
    let variants = game.shot_variants(c);
    let Some(out) = rebuild(&data, |name, t| what(name).map(|w| {
        let (b, spin) = shape(&params, &variants, c, w);
        t.sixty_hz(&b, |h, t| spin.at(h, t))
    })) else {
        return data;
    };
    BUILT.lock().unwrap().insert(key, out.clone());
    out
}

/// `data`'s archive with each table `f` regenerates replaced (built on threads; the build is pure, so any order
/// gives the same bits).
fn rebuild(data: &[u8], f: impl Fn(&str, &Table) -> Option<Table> + Sync) -> Option<Vec<u8>> {
    let arc = Archive::parse(data).ok()?;
    let files: Vec<(String, Vec<u8>)> = arc.entries.iter().map(|e| Some((e.name.clone(), arc.read(e).ok()?))).collect::<Option<_>>()?;
    let one = |(name, bytes): &(String, Vec<u8>)| {
        let t = Table::parse(bytes).and_then(|t| f(name, &t));
        (name.clone(), t.map_or_else(|| bytes.clone(), |t| t.0.iter().flat_map(|w| w.to_le_bytes()).collect()))
    };
    #[cfg(not(target_arch = "wasm32"))]
    let out: Vec<(String, Vec<u8>)> = std::thread::scope(|s| files.iter().map(|x| s.spawn(|| one(x))).collect::<Vec<_>>().into_iter().map(|h| h.join().unwrap()).collect());
    #[cfg(target_arch = "wasm32")]
    let out: Vec<(String, Vec<u8>)> = files.iter().map(one).collect();
    Some(xb::write(&out))
}

#[cfg(test)]
mod tests;
