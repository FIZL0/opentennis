//! Research (M163a2): can the ported flight regenerate the US disc's trajectory tables bit for bit? It measures,
//! it doesn't build. Findings in `research/journal/2026-11-16-m163a2-eu-disc/`.
#![allow(clippy::float_arithmetic)]

use super::*;
use hst_data::xb::Archive;

fn table(c: usize, ab: &str, file: &str) -> Option<Table> {
    let mut iso = hst_data::fixture::iso().ok()?;
    let data = iso.read(&format!("TRAJ/TRAJ{c:02}{ab}.XB")).ok()?;
    let arc = Archive::parse(&data).ok()?;
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(file))?;
    Table::parse(&arc.read(e).ok()?)
}

fn serve_spin(c: usize, kind: usize) -> Option<f32> {
    let mut iso = hst_data::fixture::iso().ok()?;
    let (cnf, bin) = (iso.read("SYSTEM.CNF").ok()?, iso.read("ZZBIN/GAME.BIN").ok()?);
    let src = hst_data::exe::Game::new(&cnf, &bin).ok()?.shot_params();
    let params = crate::params::ShotParams::build(&src.base, &src.kinds, &src.weights, src.middle_mix);
    Some(crate::params::spin(params.record(0, kind, crate::params::record_of(c))))
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

/// Big Chief's flat serve table against the ported flight. Every cell, flown as stored, lands within about
/// −12…+8 cm of its target, and its landing time's fraction of a frame is spread evenly. With the stored elevation
/// given, the speed that lands exactly on the target matches the stored speed (truncated) in only ~13% of cells,
/// whichever ground height or elevation rounding is assumed: the leftover spans several speed units, so the
/// authors' offline solver stopped at a landing tolerance, and its stopping points can't be reproduced.
/// Measurement only (prints); run with `tools/check.sh -p hst-sim regen -- --ignored --nocapture`.
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
