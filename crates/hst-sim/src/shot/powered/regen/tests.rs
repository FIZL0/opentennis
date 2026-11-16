//! How close the European disc's tables at 60 Hz come to the US disc's (M170), and M163a2's finding that the US
//! tables can't be rebuilt bit for bit. Needs the US disc and `fixtures/discs/SCES-54535.iso` (skipped without it,
//! even under `HST_REQUIRE_DISC`: it isn't the user's disc).
use super::super::{ANGLE_UNIT, MAX_FRAMES, SPEED_UNIT, launch};
use super::*;
use crate::ball::{Ball, Params};

const EUROPE: &str = "fixtures/discs/SCES-54535.iso";

fn europe() -> Option<Iso> {
    let p = hst_data::fixture::path(EUROPE);
    if !p.exists() {
        eprintln!("skipped: no {EUROPE}");
        return None;
    }
    Iso::open(p).ok()
}

/// Every table in `TRAJ<c><ab>.XB`, by its file name (lower case, no directory).
fn tables(iso: &mut Iso, c: usize, ab: &str) -> Vec<(String, Table)> {
    let Ok(data) = iso.read_unlogged(&format!("TRAJ/TRAJ{c:02}{ab}.XB")) else { return vec![] };
    let arc = Archive::parse(&data).unwrap();
    arc.entries
        .iter()
        .filter_map(|e| {
            let name = e.name.to_ascii_lowercase().replace('\\', "/").rsplit('/').next()?.to_string();
            Some((name, Table::parse(&arc.read(e).ok()?)?))
        })
        .collect()
}

fn shot_params(iso: &mut Iso, c: usize) -> (ShotParams, Vec<ShotVariant>) {
    let (cnf, bin) = (iso.read_unlogged("SYSTEM.CNF").unwrap(), iso.read_unlogged("ZZBIN/GAME.BIN").unwrap());
    let g = Game::new(&cnf, &bin).unwrap();
    let src = g.shot_params();
    (ShotParams::build(&src.base, &src.kinds, &src.weights, src.middle_mix), g.shot_variants(c))
}

/// One table against the US one: per cell (US speed above zero) the speed difference (km/h), the frames
/// difference, and the landing difference (m) with both flown on the 60 Hz step.
#[derive(Default)]
struct Gap {
    kmh: Vec<f32>,
    frames: Vec<i32>,
    landing: Vec<f32>,
    exact: usize,
}

impl Gap {
    fn add(&mut self, ours: &Table, us: &Table, b: &Bounds, spin: &Spin) {
        for i in 0..N * N * N {
            let (hit, target) = place(b, i);
            let (u, o) = (fields(us.0[i]), fields(ours.0[i]));
            if u.1 <= 0 {
                continue;
            }
            let s = spin.at(hit, target);
            self.exact += (ours.0[i] == us.0[i]) as usize;
            self.kmh.push((o.1 - u.1) as f32 * SPEED_UNIT * 216.0);
            self.frames.push(o.2 - u.2);
            if let (Some(a), Some(b)) = (fly(hit, o.0, o.1, s), fly(hit, u.0, u.1, s)) {
                self.landing.push((a.landing - b.landing).0);
            }
        }
    }

    fn report(&self, what: &str) -> String {
        let abs = |v: &[f32]| {
            let mut a: Vec<f32> = v.iter().map(|x| x.abs()).collect();
            a.sort_by(f32::total_cmp);
            let mean = a.iter().sum::<f32>() / a.len().max(1) as f32;
            (mean, a.get(a.len() * 95 / 100).copied().unwrap_or(0.0), a.last().copied().unwrap_or(0.0))
        };
        let (k, l) = (abs(&self.kmh), abs(&self.landing));
        let n = self.frames.len().max(1) as f32;
        let same = self.frames.iter().filter(|&&d| d == 0).count() as f32 / n;
        let one = self.frames.iter().filter(|&&d| d.abs() <= 1).count() as f32 / n;
        let worst = self.frames.iter().map(|d| d.abs()).max().unwrap_or(0);
        format!(
            "{what}: {} cells, {:.1}% exact | km/h off mean {:.2} p95 {:.2} max {:.2} | frames same {:.1}% ±1 {:.1}% max {worst} | landing off mean {:.1} p95 {:.1} max {:.1} cm",
            self.frames.len(),
            100.0 * self.exact as f32 / n,
            k.0,
            k.1,
            k.2,
            100.0 * same,
            100.0 * one,
            l.0 * 100.0,
            l.1 * 100.0,
            l.2 * 100.0
        )
    }
}

/// Character `c`'s tables (every one, or those `pick` keeps) regenerated from the European disc and scaled, each
/// against the US one: (regenerated gap, scaled gap).
fn measure(eu: &mut Iso, us: &mut Iso, c: usize, pick: impl Fn(&str) -> bool) -> (Gap, Gap) {
    let (params, variants) = shot_params(eu, c);
    let (mut ours, mut naive) = (Gap::default(), Gap::default());
    for ab in ["A", "B"] {
        let theirs: std::collections::HashMap<String, Table> = tables(us, c, ab).into_iter().collect();
        let mine: Vec<_> = tables(eu, c, ab).into_iter().filter(|(n, _)| pick(n)).filter_map(|(n, t)| Some((what(&n)?, theirs.get(&n)?.clone(), t))).collect();
        let built: Vec<Table> = std::thread::scope(|s| {
            let hs: Vec<_> = mine.iter().map(|(w, _, t)| s.spawn(|| {
                let (b, spin) = shape(&params, &variants, c, *w);
                t.sixty_hz(&b, |h, t| spin.at(h, t))
            })).collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        for ((w, u, t), r) in mine.iter().zip(&built) {
            let (b, spin) = shape(&params, &variants, c, *w);
            ours.add(r, u, &b, &spin);
            naive.add(&Table(t.0.iter().map(|&w| scaled(w)).collect()), u, &b, &spin);
        }
    }
    (ours, naive)
}

/// Big Chief's (pc07) serve, stroke, volley and smash tables from the European disc at 60 Hz come close to the US
/// disc's: speeds about 3 km/h apart on average (half a km/h where the 50 Hz solver kept the US elevation, tens where
/// it chose another arc), frames within one in four cells of five, landings 6 cm apart on average, closer than the
/// 50 Hz tables merely scaled on speed and landing. Four tables build in a few seconds.
#[test]
fn european_tables_at_60_hz_come_close_to_the_us_ones() {
    let Some(mut eu) = europe() else { return };
    let Ok(mut us) = hst_data::fixture::iso() else { return hst_data::fixture::skip("no ISO") };
    let start = std::time::Instant::now();
    let pick = |n: &str| ["serv2.dat", "strk0.dat", "voly2.dat", "smsh0.dat"].iter().any(|s| n.ends_with(s));
    let (ours, naive) = measure(&mut eu, &mut us, 7, pick);
    eprintln!("{} in {:?}\n{}", ours.report("regenerated"), start.elapsed(), naive.report("scaled"));
    let mean = |v: &[f32]| v.iter().map(|x| x.abs()).sum::<f32>() / v.len() as f32;
    let within = |g: &Gap, d: i32| g.frames.iter().filter(|f| f.abs() <= d).count() as f32 / g.frames.len() as f32;
    assert!(ours.frames.len() > 4 * 3000);
    assert!(mean(&ours.kmh) < 3.0 && mean(&ours.kmh) < mean(&naive.kmh), "{}", ours.report("regenerated"));
    assert!(mean(&ours.landing) < 0.07 && mean(&ours.landing) < mean(&naive.landing), "{}", ours.report("regenerated"));
    assert!(within(&ours, 1) > 0.8, "{}", ours.report("regenerated"));
}

/// Every character's every table, regenerated and scaled, against the US disc (the journal's numbers). Minutes:
/// `tools/check.sh -p hst-sim every_european_table -- --ignored --nocapture`.
#[test]
#[ignore]
fn every_european_table_against_the_us_disc() {
    let Some(mut eu) = europe() else { return };
    let Ok(mut us) = hst_data::fixture::iso() else { return hst_data::fixture::skip("no ISO") };
    let (mut all, mut all_naive) = (Gap::default(), Gap::default());
    let mut by_class: Vec<(String, Gap)> = ["serv", "strk", "voly", "smsh"].iter().map(|s| (s.to_string(), Gap::default())).collect();
    for c in 0..14 {
        for (stem, gap) in &mut by_class {
            let s = stem.clone();
            let (o, n) = measure(&mut eu, &mut us, c, |name| name.contains(&s));
            eprintln!("pc{c:02} {}", o.report(stem));
            for (to, from) in [(&mut *gap, &o), (&mut all, &o), (&mut all_naive, &n)] {
                to.kmh.extend(&from.kmh);
                to.frames.extend(&from.frames);
                to.landing.extend(&from.landing);
                to.exact += from.exact;
            }
        }
    }
    for (stem, gap) in &by_class {
        eprintln!("{}", gap.report(&format!("all {stem}")));
    }
    eprintln!("{}\n{}", all.report("all regenerated"), all_naive.report("all scaled"));
}

/// A rebuilt archive reads back with every table at 60 Hz and every other entry as it was; the disc read on a
/// European disc goes through it once installed, and the digest's read doesn't.
#[test]
fn a_european_archive_reads_back_at_60_hz() {
    let Some(mut eu) = europe() else { return };
    install();
    let raw = eu.read_unlogged("TRAJ/TRAJ07B.XB").unwrap();
    let start = std::time::Instant::now();
    let built = eu.read("TRAJ/TRAJ07B.XB").unwrap();
    eprintln!("TRAJ07B rebuilt in {:?}", start.elapsed());
    assert_ne!(raw, built);
    assert_eq!(eu.read("TRAJ/TRAJ07B.XB").unwrap(), built, "built once, then the same bytes");
    let (a, b) = (Archive::parse(&raw).unwrap(), Archive::parse(&built).unwrap());
    assert_eq!(a.entries.len(), b.entries.len());
    let (params, variants) = shot_params(&mut eu, 7);
    for (x, y) in a.entries.iter().zip(&b.entries) {
        assert_eq!(x.name, y.name);
        let (old, new) = (a.read(x).unwrap(), b.read(y).unwrap());
        match what(&x.name) {
            Some(w) => {
                let (bounds, spin) = shape(&params, &variants, 7, w);
                let want = Table::parse(&old).unwrap().sixty_hz(&bounds, |h, t| spin.at(h, t));
                assert!(Table::parse(&new).unwrap() == want, "{}", x.name);
            }
            None => assert_eq!(old, new, "{}", x.name),
        }
    }
}

/// File names say what a table is.
#[test]
fn table_names() {
    assert_eq!(what("data\\hatsuyama\\traj\\tr_pc07_serv2.dat"), Some((class::SERVE, 2, None)));
    assert_eq!(what("tr_pc00_strk1_dw2.dat"), Some((class::STROKE, 1, Some(2))));
    assert_eq!(what("TR_PC13_VOLY4_UP1.DAT"), Some((class::VOLLEY, 4, Some(0))));
    assert_eq!(what("tr_pc03_smsh1.dat"), Some((class::SMASH, 1, None)));
    assert_eq!(what("tr_pc03_other1.dat"), None);
    assert_eq!(what("readme.txt"), None);
}

fn table(c: usize, ab: &str, file: &str) -> Option<Table> {
    let mut iso = hst_data::fixture::iso().ok()?;
    tables(&mut iso, c, ab).into_iter().find(|(n, _)| n == file).map(|(_, t)| t)
}

fn serve_spin(c: usize, kind: usize) -> Option<f32> {
    let mut iso = hst_data::fixture::iso().ok()?;
    let (params, _) = shot_params(&mut iso, c);
    Some(params::spin(params.record(0, kind, params::record_of(c))))
}

/// Where the ball first reaches `ground` (z, interpolated), and when (frames, fractional).
fn land(hit: V3, e: f32, s: f32, spin: f32, ground: f32) -> Option<(f32, f32)> {
    let mut ball = Ball { pos: hit, vel: launch(hit, [0.0, 0.0, 10.0], e, s), spin };
    let p = Params::default();
    for n in 1..MAX_FRAMES {
        let [_, y0, z0] = ball.pos;
        ball.fly(&p);
        let [_, y1, z1] = ball.pos;
        if y1 >= ground {
            let u = (ground - y0) / (y1 - y0);
            return Some((z0 + (z1 - z0) * u, (n - 1) as f32 + u));
        }
    }
    None
}

/// The speed (m a frame) at elevation `e` that lands on `d`, by bisection, and its landing time.
fn speed_for(hit: V3, e: f32, d: f32, spin: f32, ground: f32) -> Option<(f64, f32)> {
    let (mut lo, mut hi) = (0.01f64, 3.0f64);
    for _ in 0..60 {
        let m = 0.5 * (lo + hi);
        match land(hit, e, m as f32, spin, ground) {
            Some((z, _)) if z < d => lo = m,
            _ => hi = m,
        }
    }
    let s = 0.5 * (lo + hi);
    land(hit, e, s as f32, spin, ground).map(|(_, t)| (s, t))
}

/// M163a2: Big Chief's flat serve table against the ported flight. Every cell, flown as stored, lands within about
/// −12…+8 cm of its target, and its landing time's fraction of a frame is spread evenly. With the stored elevation
/// given, the speed that lands exactly on the target matches the stored speed (truncated) in only ~13% of cells,
/// whichever ground height or elevation rounding is assumed: the leftover spans several speed units, so the
/// authors' offline solver stopped at a landing tolerance, and its stopping points can't be reproduced.
/// Measurement only (prints); run with `tools/check.sh -p hst-sim stock_serve_table -- --ignored --nocapture`.
#[test]
#[ignore]
fn stock_serve_table_against_the_flight() {
    let (Some(t), Some(spin)) = (table(7, "A", "tr_pc07_serv2.dat"), serve_spin(7, 2)) else {
        return hst_data::fixture::skip("no ISO");
    };
    let b = Bounds::serve(false, RADIUS);
    let (mut worst, mut frames, mut all) = ((f32::MAX, f32::MIN), 0, 0);
    let mut fraction = [0usize; 10];
    for i in 0..N * N * N {
        let (hit, target) = place(&b, i);
        let (e, s, f) = fields(t.0[i]);
        let Some((z, when)) = land(hit, e as f32 * ANGLE_UNIT, s as f32 * SPEED_UNIT, spin, -RADIUS) else { continue };
        let miss = z - target[2];
        worst = (worst.0.min(miss), worst.1.max(miss));
        frames += (when.ceil() as i32 == f) as usize;
        fraction[((when.fract() * 10.0) as usize).min(9)] += 1;
        all += 1;
    }
    eprintln!("{all} cells land {:+.3}..{:+.3} m off target; {frames} fly their stored frames; landing-time fraction deciles {fraction:?}", worst.0, worst.1);
    for ground in [-RADIUS, -0.05, 0.0] {
        for half in [0.0, 0.5] {
            let (mut exact, mut n) = (0, 0);
            for i in (0..N * N * N).step_by(7) {
                let (hit, target) = place(&b, i);
                let (e, s, _) = fields(t.0[i]);
                let Some((v, _)) = speed_for(hit, (e as f32 + half) * ANGLE_UNIT, target[2], spin, ground) else { continue };
                exact += ((v * 1024.0).floor() as i32 == s) as usize;
                n += 1;
            }
            eprintln!("ground {ground}, elevation +{half} unit: exact stored speed in {exact}/{n}");
        }
    }
}

