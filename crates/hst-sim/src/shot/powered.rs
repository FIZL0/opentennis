//! Generated trajectory tables (a mod's §6 `shots` `power`): a stock table with every cell's speed × power and its
//! elevation re-solved so the ball still lands where the stock cell's did. Built once per table when a match loads;
//! the match then looks them up like any other table, so a hit costs nothing extra. Stock characters never use them.
//!
//! Each cell is flown in the vertical plane through its hit and target on the ball's own step (`Ball::fly`: drag,
//! Magnus, gravity on PS2 floats), so every machine builds the same bits. The stock cell flown the same way gives the
//! landing to keep and the height it passes the net at; the faster cell keeps that landing on its flatter solution.
//! A cell that can't clear the net as well (up to the centre cord plus a ball and a margin) steps its power down toward the
//! stock speed; none clears, it stays the stock cell. Its flight frames move by the difference the model flies.

use super::{ABOVE_GROUND, ANGLE_UNIT, Bounds, N, RADIUS, SPEED_UNIT, Table, launch, low_bound};
use crate::ball::{Ball, Params, V3};
use crate::ps2::F;

/// The speed field's top: 12 bits signed, 2047/1024 m a frame (~268 mph).
pub const MAX_SPEED: i32 = 2047;
/// Elevations searched: −45° (any hit lands short) to +45°, in table units (π/4096).
const DOWN: i32 = -1024;
const UP: i32 = 1024;
/// A flight that hasn't landed by then flies long.
const MAX_FRAMES: i32 = 400;
/// The height (m) a cell must pass the net at, when its stock cell does: the cord at the centre (0.914 m) plus the
/// ball and ~6 cm.
const CLEAR: f32 = 1.04;
/// Power steps from the asked power down to the stock speed.
const STEPS: i32 = 10;

/// One modelled flight: where it lands past the net (m), the frames to it, and the height it crosses the net at (m
/// up; −∞ when it lands short of the net).
#[derive(Clone, Copy)]
struct Flown {
    landing: F,
    frames: i32,
    over: F,
}

/// `cell`'s fields as integers: elevation and speed in table units, frames.
fn fields(w: u32) -> (i32, i32, i32) {
    let sext = |v: u32| (v << 20) as i32 >> 20;
    (sext(w & 0xfff), sext((w >> 12) & 0xfff), (w >> 24) as i32)
}

fn word(elevation: i32, speed: i32, frames: i32) -> u32 {
    (elevation as u32 & 0xfff) | ((speed as u32 & 0xfff) << 12) | ((frames.clamp(0, 255) as u32) << 24)
}

/// The ball struck at `hit` (Y-down, z < 0) toward +z at `elevation`/`speed` (table units) with `spin`.
fn fly(hit: V3, elevation: i32, speed: i32, spin: f32) -> Option<Flown> {
    fly_at(hit, (F(elevation as f32) * ANGLE_UNIT).0, (F(speed as f32) * SPEED_UNIT).0, spin)
}

fn fly_at(hit: V3, e: f32, s: f32, spin: f32) -> Option<Flown> {
    let mut ball = Ball { pos: hit, vel: launch(hit, [0.0, 0.0, 10.0], e, s), spin };
    let p = Params::default();
    let mut over = F(f32::NEG_INFINITY);
    let ground = -F(RADIUS);
    for n in 1..=MAX_FRAMES {
        let [_, y0, z0] = ball.pos.map(F);
        ball.fly(&p);
        let [_, y1, z1] = ball.pos.map(F);
        if z0 < 0.0 && z1 >= 0.0 {
            over = -(y0 + (y1 - y0) * (-z0 / (z1 - z0)));
        }
        if y1 >= ground {
            let t = if y1 == y0 { F::ONE } else { (ground - y0) / (y1 - y0) };
            return Some(Flown { landing: z0 + (z1 - z0) * t, frames: n, over });
        }
    }
    None
}

/// The elevation at `speed` whose landing is nearest `want`, searched up from −45° on the rising side (a flight that
/// never lands counts as long). None when even `top` or +45° lands short. The bracket closes by false position
/// (Illinois: a side kept twice counts half), halving when a flight never lands.
fn solve(hit: V3, speed: i32, spin: f32, want: F, top: i32) -> Option<(i32, Flown)> {
    let long = |f: &Option<Flown>| f.is_none_or(|f| f.landing >= want);
    let low = fly(hit, DOWN, speed, spin);
    if long(&low) {
        return None;
    }
    let (mut lo, mut lo_f) = (DOWN, low?);
    let (mut hi, mut hi_f) = [top, UP].into_iter().map(|e| (e, fly(hit, e, speed, spin))).find(|(_, f)| long(f))?;
    // the misses false position weighs the ends by (halved when an end stays put)
    let (mut short, mut over) = (lo_f.landing - want, hi_f.map(|f| f.landing - want));
    let mut kept = 0;
    while hi - lo > 1 {
        let mid = match over {
            Some(o) if o > short => (lo + ((F((hi - lo) as f32) * (-short / (o - short))).0 as i32)).clamp(lo + 1, hi - 1),
            _ => lo + (hi - lo) / 2,
        };
        let f = fly(hit, mid, speed, spin);
        if long(&f) {
            (hi, hi_f, over) = (mid, f, f.map(|f| f.landing - want));
            kept = if kept < 0 { kept - 1 } else { -1 };
            if kept < -1 {
                short *= 0.5;
            }
        } else {
            let f = f.expect("lands short");
            (lo, lo_f, short) = (mid, f, f.landing - want);
            kept = if kept > 0 { kept + 1 } else { 1 };
            if kept > 1 {
                over = over.map(|o| o * 0.5);
            }
        }
    }
    match hi_f {
        Some(h) if h.landing - want < want - lo_f.landing => Some((hi, h)),
        _ => Some((lo, lo_f)),
    }
}

/// One cell at `power`: the stock word when the stock flight never lands, or no step clears the net.
fn cell(w: u32, hit: V3, spin: f32, power: F) -> u32 {
    let (e0, s0, f0) = fields(w);
    if s0 <= 0 {
        return w;
    }
    let Some(stock) = fly(hit, e0, s0, spin) else { return w };
    let clear = if stock.over < CLEAR { stock.over } else { F(CLEAR) };
    let at = |step: i32| {
        let k = power - (power - 1.0) * (F(step as f32) / STEPS as f32);
        let speed = ((F(s0 as f32) * k).0.round() as i32).min(MAX_SPEED);
        solve(hit, speed, spin, stock.landing, e0).filter(|(_, f)| f.over >= clear).map(|(e, f)| word(e, speed, f0 + f.frames - stock.frames))
    };
    if let Some(w) = at(0) {
        return w;
    }
    // the fastest clearing step, found by halving (slower clears more easily)
    let (mut fast, mut slow, mut best) = (0, STEPS, w);
    while slow - fast > 1 {
        let mid = (fast + slow) / 2;
        match at(mid) {
            Some(w) => (slow, best) = (mid, w),
            None => fast = mid,
        }
    }
    best
}

/// Cell `i`'s hit (Y-down, before the net at z 0) and target on axes `b`: the hitter's distance to the net crossing,
/// the hit height and the crossing's to the target.
fn place(b: &Bounds, i: usize) -> (V3, V3) {
    let at = |i: usize| F(i as f32) / (N - 1) as f32;
    let (x, y, z) = (i % N, i / N % N, i / (N * N));
    let to_net = -(F(b.near) + (F(b.far) - b.near) * at(x));
    let low = if b.sloped { low_bound((-to_net).0) } else { b.low };
    let height = (F(low) + (F(b.high) - low) * at(y)).min(ABOVE_GROUND);
    let past = F(b.short) + (F(b.long) - b.short) * at(z);
    ([0.0, height.0, (-to_net).0], [0.0, 0.0, past.0])
}

impl Table {
    /// This table at `power` (1 = stock) over its axes `b` (see the module notes). `spin(hit, target)` is the
    /// launch spin a shot from `hit` to `target` gets (the shot record's).
    pub fn powered(&self, b: &Bounds, power: f32, spin: impl Fn(V3, V3) -> f32) -> Table {
        let words = (0..N * N * N)
            .map(|i| {
                let (hit, target) = place(b, i);
                cell(self.0[i], hit, spin(hit, target), F(power))
            })
            .collect();
        Table(words)
    }

    /// The mean speed (m a frame) of [`Table::powered`]'s cells over every `step`th cell along each axis, those
    /// whose stock speed is above zero: the generator's own cells, at a fraction of the whole table's cost (the
    /// select card grades a powered mod by it). Power 1 is the stock table's.
    pub fn mean_speed(&self, b: &Bounds, power: f32, spin: impl Fn(V3, V3) -> f32, step: usize) -> f32 {
        let picked = |i: usize| [i % N, i / N % N, i / (N * N)].iter().all(|&a| a % step.max(1) == 0);
        let speeds: Vec<i32> = (0..N * N * N)
            .filter(|&i| picked(i) && fields(self.0[i]).1 > 0)
            .map(|i| {
                let (hit, target) = place(b, i);
                let w = if power == 1.0 { self.0[i] } else { cell(self.0[i], hit, spin(hit, target), F(power)) };
                fields(w).1
            })
            .collect();
        let sum: i64 = speeds.iter().map(|&s| s as i64).sum();
        (F(sum as f32) / speeds.len().max(1) as f32 * SPEED_UNIT).0
    }

    /// The highest speed in the table (m a frame).
    pub fn top_speed(&self) -> f32 {
        let top = self.0.iter().map(|&w| fields(w).1).max().unwrap_or(0);
        (F(top as f32) * SPEED_UNIT).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hst_data::xb::Archive;

    /// Character `c`'s table `file` from `TRAJ<c><ab>.XB`, and its serve record's spin for `kind`.
    fn serve(c: usize, ab: &str, file: &str, kind: usize) -> Option<(Table, f32)> {
        let mut iso = hst_data::fixture::iso().ok()?;
        let data = iso.read(&format!("TRAJ/TRAJ{c:02}{ab}.XB")).ok()?;
        let arc = Archive::parse(&data).ok()?;
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(file))?;
        let t = Table::parse(&arc.read(e).ok()?)?;
        let src = hst_data::exe::Game::new(&iso.read("SYSTEM.CNF").ok()?, &iso.read("ZZBIN/GAME.BIN").ok()?).ok()?.shot_params();
        let params = crate::params::ShotParams::build(&src.base, &src.kinds, &src.weights, src.middle_mix);
        Some((t, crate::params::spin(params.record(0, kind, crate::params::record_of(c)))))
    }

    /// The model is the tables' own: Big Chief's flat serve cells, flown with their record's spin, land in the very
    /// frames the table gives (to a frame: the authors cut the landing finer). Powered 1.4 the table builds in a second or two (once per match load, off the hit),
    /// the fastest serve goes 1.4× as fast, the same bits every build (the hash pins them for every platform; a
    /// change to the generator moves it, and online peers must then all run it), and at power 2 no speed passes the
    /// 12-bit field's top.
    #[test]
    fn powered_serve_table() {
        let Some((t, spin)) = serve(7, "A", "tr_pc07_serv2.dat", 2) else { return hst_data::fixture::skip("no ISO") };
        let b = Bounds::serve(false, RADIUS);
        let at = |i: usize| F(i as f32) / (N - 1) as f32;
        let hit = |i: usize| {
            let (x, y) = (i % N, i / N % N);
            [0.0, (F(b.low) + (F(b.high) - b.low) * at(y)).min(ABOVE_GROUND).0, (F(b.near) + (F(b.far) - b.near) * at(x)).0]
        };
        let same = (0..N * N * N).filter(|&i| fly(hit(i), fields(t.0[i]).0, fields(t.0[i]).1, spin).is_some_and(|f| (f.frames - fields(t.0[i]).2).abs() <= 1)).count();
        assert!(same * 100 >= 95 * N * N * N, "{same} of {} cells fly their table's frames", N * N * N);

        let start = std::time::Instant::now();
        let p = t.powered(&b, 1.4, |_, _| spin);
        let took = start.elapsed();
        eprintln!("built in {took:?}; top speed {} -> {}", t.top_speed(), p.top_speed());
        assert!(took.as_secs_f32() < 5.0, "{took:?}");
        assert!(p.top_speed() > 1.35 * t.top_speed(), "{} vs {}", p.top_speed(), t.top_speed());
        assert!(p == t.powered(&b, 1.4, |_, _| spin));
        let hash = p.0.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &w| (h ^ w as u64).wrapping_mul(0x100_0000_01b3));
        assert_eq!(hash, 10488497513274300435, "generated bits moved");
        let top = t.powered(&b, 2.0, |_, _| spin).top_speed();
        assert!(top <= (F(MAX_SPEED as f32) * SPEED_UNIT).0 && top > 1.9, "{top}");
    }
}
