//! B40g2 (concept): offbeat rule 1, irregular bounce, against the original. `context/b40g2/run2/*.bin` (not in git)
//! are bot matches from save-state slot 5 (court 10) with the rule set on after the load, recorded every frame in
//! `record_live.py`'s format (`research/journal/2026-10-21-b40g2-offbeat-rules/scripts/b40g2_irregular.py`). Each
//! frame's ball object is loaded into a `Flight`, its irregular turn made from the two launch uniforms the game keeps
//! on the ball, stepped once and compared bit for bit with the next frame. Skips without the recordings or the disc.

use hst_data::exe;
use hst_data::fixture;
use hst_sim::ball::{self, Ball, COURTS, Flight, Material, Params, Shot};
use hst_sim::court;

const SAMPLE: usize = 4 + 0x290 + 0x290 + 0x40;
const COURT: usize = 10;
/// Where the ball object keeps the launch's two uniforms (found by search: the only pair that reproduces the
/// recordings' deflected bounces).
const PITCH_UNIFORM: usize = 0x258;
const YAW_UNIFORM: usize = 0x25c;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn v3(b: &[u8], o: usize) -> [f32; 3] {
    [f(b, o), f(b, o + 4), f(b, o + 8)]
}
fn rows(b: &[u8], o: usize) -> [[f32; 4]; 4] {
    std::array::from_fn(|r| std::array::from_fn(|k| f(b, o + 16 * r + 4 * k)))
}

fn load(b: &[u8]) -> (Flight, Shot) {
    let ball = Ball { pos: v3(b, 0xe0), vel: v3(b, 0x130), spin: f(b, 0x1a4) };
    let mut fl = Flight::new(ball, rows(b, 0x160), rows(b, 0x1c0));
    fl.frame = i(b, 0xac);
    fl.bounces = i(b, 0x224);
    fl.contacts = i(b, 0x228);
    fl.special_contacts = i(b, 0x22c);
    fl.rolling = b[0xa4] == 2;
    fl.slide = f(b, 0x200);
    let shot = Shot {
        params: Params::default(),
        curve: f(b, 0x254),
        bend: f(b, 0x250),
        side: v3(b, 0x90),
        curve_frames: i(b, 0x260),
        wind: v3(b, 0x240),
        first_bounce_spin: f(b, 0x1a8),
        first_bounce_restitution: f(b, 0x1ac),
        class: b[0x58],
        kind: i(b, 0x5c),
        bounce_turn: f(b, 0x1b0),
    };
    fl.irregular = Some(ball::irregular(shot.kind, f(b, PITCH_UNIFORM), f(b, YAW_UNIFORM)));
    (fl, shot)
}

fn disc(court: u32) -> Option<(hst_sim::mesh::World, Vec<Material>)> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(mut iso), Ok(cnf), Ok(bin)) = (
        fixture::iso(),
        fixture::read(format!("{root}/context/iso/SYSTEM.CNF")),
        fixture::read(format!("{root}/context/iso/ZZBIN/GAME.BIN")),
    ) else {
        return None;
    };
    Some((court::world(&mut iso, court).unwrap(), court::materials(&exe::Game::new(&cnf, &bin).unwrap())))
}

/// Every recorded flying frame: (frames bit-exact, frames the rule got wrong, deflected bounces by shot kind, frames
/// both the rule's step and the plain one miss alike).
#[allow(clippy::type_complexity)]
fn replay(data: &[u8], world: &hst_sim::mesh::World, materials: &[Material]) -> (usize, Vec<String>, std::collections::BTreeMap<i32, usize>, Vec<i32>) {
    let s: Vec<(i32, &[u8])> = data.as_chunks::<SAMPLE>().0.iter().map(|w| (i(w, 0), &w[4..4 + 0x290])).collect();
    let (mut exact, mut failures, mut turned, mut unruled) = (0, vec![], std::collections::BTreeMap::new(), vec![]);
    let step = |fl: &mut Flight, shot: &Shot| {
        fl.step_world(shot, &COURTS[COURT], world, materials);
        [fl.ball.pos, fl.ball.vel].concat()
    };
    for w in s.windows(2) {
        let ((va, a), (vsync, b)) = (w[0], w[1]);
        // only frames the ball physically flies (as `live.rs`)
        let carried = |o: &[u8]| v3(o, 0x130) == [0.0; 3];
        if vsync != va + 1 || i(b, 0xac) != i(a, 0xac) + 1 || a[0xa4] > 1 || b[0xa4] > 1 || carried(a) || carried(b) {
            continue;
        }
        let want = [v3(b, 0xe0), v3(b, 0x130)].concat();
        let same = |got: &[f32]| (0..6).all(|k| got[k].to_bits() == want[k].to_bits());
        let (mut fl, shot) = load(a);
        let mut plain = fl;
        plain.irregular = None;
        let got = step(&mut fl, &shot);
        let off = step(&mut plain.clone(), &shot);
        if same(&got) {
            exact += 1;
        } else if got == off {
            // the rule didn't touch this frame and the plain step misses it too: not the rule's
            unruled.push(vsync);
        } else {
            failures.push(format!("vsync {vsync} material {} kind {} bounces {}: got {got:?} want {want:?}", b[0x220], shot.kind, i(b, 0x224)));
        }
        if !same(&step(&mut plain, &shot)) {
            *turned.entry(shot.kind).or_insert(0) += 1;
        }
    }
    (exact, failures, turned, unruled)
}

/// Every bounce of a rally turns by the launch's uniforms, bit for bit as the original, and only bounces do.
#[test]
fn irregular_bounces_match_the_game() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let Some((world, materials)) = disc(COURT as u32) else {
        fixture::skip(format_args!("disc missing, skipped"));
        return;
    };
    let mut total = (0, 0);
    for name in ["r1_b2", "r1_long"] {
        let Ok(data) = fixture::read(format!("{root}/context/b40g2/run2/{name}.bin")) else {
            fixture::skip(format_args!("{name} missing, skipped"));
            return;
        };
        let (exact, failures, turned, unruled) = replay(&data, &world, &materials);
        eprintln!("{name}: {exact} frames bit-exact, deflected bounces by shot kind {turned:?}, missed with the rule off too {unruled:?}");
        // r1_long has two 2-frame stretches (a serve, a rolling ball after its point) where the game moves the ball
        // without accelerating it, rule or not; not this rule's
        assert!(unruled.len() <= 4, "{name}: {unruled:?}");
        assert!(failures.is_empty(), "{name}: {} frames diverged:\n{}", failures.len(), failures[..failures.len().min(20)].join("\n"));
        total = (total.0 + exact, total.1 + turned.values().sum::<usize>());
    }
    assert!(total.0 > 7000 && total.1 > 90, "{total:?}");
}

/// The turn's range: 0°..15° down, ±15° sideways (±20° for a lob); a uniform of ½ goes straight on.
#[test]
fn irregular_turn_range() {
    use hst_sim::shot::kind;
    let deg = |r: f32| r.to_degrees();
    let [p, y] = ball::irregular(kind::TOPSPIN, 0.0, 0.0);
    assert_eq!((p, (deg(y) * 1e4).round() / 1e4), (0.0, -15.0));
    let [p, y] = ball::irregular(kind::LOB, 0.999_999_9, 0.5);
    assert!((deg(p) - 15.0).abs() < 1e-4 && y == 0.0, "{p} {y}");
    let [_, y] = ball::irregular(kind::LOB, 0.0, 0.0);
    assert!((deg(y) + 20.0).abs() < 1e-4);
}
