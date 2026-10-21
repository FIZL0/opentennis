//! Line calls against the user's round1 recording (fixture `context/fixtures/round1.bin` from
//! `tools/record_p2m2.py`, not in git; skipped when absent). The recorded ball object (`gm+0x98`) is the game's
//! path predictor, which steps 15 frames per game frame and makes the same line calls as the live ball. Every
//! recorded frame before its landing is called is replayed from the previous frame's object through `Flight`
//! with line calls on: the call (+0xa5) and line distance (+0x230) must match bit for bit, and so must the
//! position and velocity. Needs the extracted disc under `context/iso` for the line margin.

use hst_data::fixture;
use hst_data::exe::Game;
use hst_sim::ball::{Ball, COURTS, Flight, Params, Shot};
use hst_sim::judge::{Call, Lines};
use hst_sim::replay::frames;

const STEPS_PER_FRAME: i32 = 15;

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
    std::array::from_fn(|r| std::array::from_fn(|c| f(b, o + 16 * r + 4 * c)))
}
fn call(c: u8) -> Call {
    [Call::None, Call::In, Call::Out, Call::Net, Call::NetIn, Call::NetOut][c as usize]
}

/// The predictor's flight and shot as recorded in its ball object.
fn flight(b: &[u8]) -> (Flight, Shot) {
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
    let mut fl = Flight::new(Ball { pos: v3(b, 0xe0), vel: v3(b, 0x130), spin: f(b, 0x1a4) }, rows(b, 0x160), rows(b, 0x1c0));
    fl.frame = i(b, 0xac);
    fl.bounces = i(b, 0x224);
    fl.contacts = i(b, 0x228);
    fl.special_contacts = i(b, 0x22c);
    fl.rolling = b[0xa4] == 2;
    fl.slide = f(b, 0x200);
    fl.net = false;
    fl.call = call(b[0xa5]);
    fl.line_distance = f(b, 0x230);
    (fl, shot)
}

#[test]
fn round1_line_calls() {
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context");
    let (Ok(cnf), Ok(bin), Ok(data)) = (
        fixture::read(format!("{ctx}/iso/SYSTEM.CNF")),
        fixture::read(format!("{ctx}/iso/ZZBIN/GAME.BIN")),
        fixture::read(format!("{ctx}/fixtures/round1.bin")),
    ) else {
        return fixture::skip(format_args!("disc or round1.bin absent, skipped"));
    };
    let margin = Game::new(&cnf, &bin).unwrap().line_margin();
    let frames = frames(&data);
    // (vsync, previous ball, current ball, lines, shots this rally)
    let mut cases = Vec::new();
    for w in frames.windows(2) {
        let (a, b) = (w[0].ball(), w[1].ball());
        let open = matches!(call(a[0xa5]), Call::None | Call::Net);
        if !open || i(b, 0xac) != i(a, 0xac) + STEPS_PER_FRAME || i(a, 0xac) == 0 {
            continue;
        }
        let lines = Lines {
            shots: w[0].global(0x423060),
            doubles: w[0].global(0x422fa4) > 2,
            side: w[0].global(0x423050),
            hitter_far: f(a, 0x138) < 0.0,
            margin,
        };
        // No calls before the rally phase (match phase gm+0x55 < 3, sub-phase gm+0x56 != 4).
        let gm = w[0].gm();
        let lines = (!(gm[0x55] < 3 && gm[0x56] != 4)).then_some(lines);
        cases.push((w[1].vsync(), a, b, lines, w[0].global(0x423060)));
    }
    assert!(!cases.is_empty(), "no predictor frames in round1.bin");
    // The court surface only shapes the flight after a bounce; take the one that reproduces the most frames.
    // Returns (call mismatches, flight mismatches).
    let run = |court: usize| -> (Vec<String>, Vec<String>) {
        let (mut calls, mut flights) = (Vec::new(), Vec::new());
        for &(v, a, b, lines, shots) in &cases {
            let (mut fl, shot) = flight(a);
            fl.lines = lines;
            fl.in_play = shots > 0;
            for _ in 0..STEPS_PER_FRAME {
                fl.step(&shot, &COURTS[court]);
            }
            if (fl.call, fl.line_distance.to_bits()) != (call(b[0xa5]), f(b, 0x230).to_bits()) {
                calls.push(format!("vsync {v}: got {:?} line {}, want {:?} line {} ({lines:?})", fl.call, fl.line_distance, call(b[0xa5]), f(b, 0x230)));
            }
            let want = (v3(b, 0xe0).map(f32::to_bits), v3(b, 0x130).map(f32::to_bits));
            if (fl.ball.pos.map(f32::to_bits), fl.ball.vel.map(f32::to_bits)) != want {
                flights.push(format!("vsync {v}: pos {:?} vel {:?}, want {:?} {:?}", fl.ball.pos, fl.ball.vel, v3(b, 0xe0), v3(b, 0x130)));
            }
        }
        (calls, flights)
    };
    let (court, (calls, flights)) = (0..COURTS.len()).map(|c| (c, run(c))).min_by_key(|(_, (c, f))| c.len() + f.len()).unwrap();
    let landed = cases.iter().filter(|(_, a, b, _, _)| a[0xa5] != b[0xa5]).count();
    eprintln!("round1: {} predictor frames ({landed} calls) on court {court}: {} call and {} flight mismatches", cases.len(), calls.len(), flights.len());
    assert!(calls.is_empty(), "{}", calls[..calls.len().min(10)].join("\n"));
    assert!(flights.is_empty(), "{}", flights[..flights.len().min(10)].join("\n"));
    assert!(landed > 0);
}

/// The live ball object (`frames_live` recordings) as a flight and its shot, with its rest-check state: the
/// classification (+0xa4: 0 none, 1 bounce, 2 roll, 3 at rest) and the latest contact normal (+0x210).
fn live(b: &[u8]) -> (Flight, Shot) {
    let (mut fl, shot) = flight(b);
    fl.net = true;
    fl.touched = b[0xa4] != 0;
    fl.at_rest = b[0xa4] == 3;
    fl.normal = std::array::from_fn(|k| f(b, 0x210 + 4 * k));
    fl.last = v3(b, 0x140);
    (fl, shot)
}

/// P123: every live-ball step of the `frames_live` recordings (the ball object each frame, on the recording's
/// court) replayed through `step_world`: wherever the step comes out bit-exact (the rest of the flight is
/// `live.rs`'s), it comes to rest exactly when the game's does, with the game's call (in, out after the net and not
/// landed all turn out), step count and last move, and a ball at rest stays put frame after frame. Includes
/// `p7b_c03`'s roll to rest on a prop slope (material 34, about 31°). The ball the server carries before the toss
/// (moved by the hand, recorded velocity 0) isn't stepped. Line calls are `round1_line_calls`'s, so a landing
/// frame's call isn't compared.
#[test]
fn live_ball_comes_to_rest_as_the_game() {
    use hst_sim::{court, judge, replay::frames_live};
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context");
    let (Ok(mut iso), Ok(cnf), Ok(bin)) = (fixture::iso(), fixture::read(format!("{ctx}/iso/SYSTEM.CNF")), fixture::read(format!("{ctx}/iso/ZZBIN/GAME.BIN"))) else {
        return fixture::skip("disc absent");
    };
    let materials = court::materials(&Game::new(&cnf, &bin).unwrap());
    let mut worlds = std::collections::BTreeMap::new();
    let files = ["p7b_c03", "p7e_c03", "p7e_c07", "match_s05", "human_smash_s04", "new_recording", "p7b_c12", "p7b_c13", "1p3goodcpus2"];
    let (mut rests, mut slopes, mut frozen, mut exact, mut inexact, mut carried) = (0, 0, 0, 0, 0, 0);
    let mut turned = Vec::new();
    let mut bad = Vec::new();
    for name in files {
        let Ok(data) = fixture::read(format!("{ctx}/fixtures/{name}.bin")) else {
            fixture::skip(format_args!("{name}.bin absent"));
            continue;
        };
        let frames = frames_live(&data);
        let c = frames[0].global(0x422f90) as u32;
        let world = worlds.entry(c).or_insert_with(|| court::world(&mut iso, c).unwrap());
        for w in frames.windows(2) {
            let (a, b) = (w[0].live_ball(), w[1].live_ball());
            if w[1].vsync() != w[0].vsync() + 1 || i(b, 0xac) < i(a, 0xac) || i(b, 0xac) > i(a, 0xac) + 1 {
                continue;
            }
            if v3(a, 0x130) == [0.0; 3] {
                carried += (a[0xa4] != 3 && b[0xa4] == 3) as usize;
                continue;
            }
            let (mut fl, shot) = live(a);
            fl.rally = w[1].gm()[0x55] == 3;
            fl.step_world(&shot, &COURTS[c as usize], world, &materials);
            let want = (v3(b, 0xe0).map(f32::to_bits), v3(b, 0x130).map(f32::to_bits));
            if (fl.ball.pos.map(f32::to_bits), fl.ball.vel.map(f32::to_bits)) != want {
                inexact += 1;
                continue;
            }
            exact += 1;
            let v = w[1].vsync();
            if fl.at_rest != (b[0xa4] == 3) || i(b, 0xac) != fl.frame || v3(b, 0x140).map(f32::to_bits) != fl.last.map(f32::to_bits) {
                bad.push(format!("{name} vsync {v}: at rest {} frame {} last {:?}, game state {} frame {} last {:?}", fl.at_rest, fl.frame, fl.last, b[0xa4], i(b, 0xac), v3(b, 0x140)));
            }
            if (a[0xa5] == b[0xa5] || b[0xa4] == 3) && fl.call != call(b[0xa5]) {
                bad.push(format!("{name} vsync {v}: call {:?}, game {:?} (was {:?})", fl.call, call(b[0xa5]), call(a[0xa5])));
            }
            if a[0xa4] == 3 {
                frozen += 1;
            } else if b[0xa4] == 3 {
                rests += 1;
                slopes += (f(b, 0x214) != -1.0) as usize;
                let t = (call(a[0xa5]), judge::rested(call(a[0xa5])));
                if !turned.contains(&t) {
                    turned.push(t);
                }
            }
        }
    }
    eprintln!("{exact} live steps bit-exact ({inexact} not, skipped), {rests} rests ({slopes} off flat ground), {frozen} frozen steps, {carried} carried rests not stepped; calls {turned:?}");
    assert!(bad.is_empty(), "{} mismatches:\n{}", bad.len(), bad[..bad.len().min(20)].join("\n"));
    if fixture::exists(format!("{ctx}/fixtures/p7b_c03.bin")) {
        assert!(slopes >= 1 && rests >= 50 && frozen > 1000, "{rests} rests, {slopes} on a slope, {frozen} frozen");
        assert!([(Call::In, Call::Out), (Call::NetOut, Call::Out), (Call::Net, Call::NetOut)].iter().all(|t| turned.contains(t)), "{turned:?}");
    }
}

/// P128: the latest contact normal (+0x210) on every live-ball step of the recordings that touch the ghost
/// material (32, a bush's see-through faces), replayed as `live_ball_comes_to_rest_as_the_game` does: wherever
/// the step is bit-exact, the port's normal is the game's bit for bit, and every ghost-touch step is among them.
/// The game's per-step ghost-touch count (+0x264) never goes past 1: after the first ghost touch in a step the
/// world sweep ignores the ghost and reports the real contact behind it, so a second ghost touch (whose normal
/// the step would keep while skipping its response) never happens.
#[test]
fn live_ball_contact_normal_as_the_game() {
    use hst_sim::{court, replay::frames_live};
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context");
    let (Ok(mut iso), Ok(cnf), Ok(bin)) = (fixture::iso(), fixture::read(format!("{ctx}/iso/SYSTEM.CNF")), fixture::read(format!("{ctx}/iso/ZZBIN/GAME.BIN"))) else {
        return fixture::skip("disc absent");
    };
    let materials = court::materials(&Game::new(&cnf, &bin).unwrap());
    let mut worlds = std::collections::BTreeMap::new();
    let (mut exact, mut inexact, mut ghosts, mut ghosts_exact) = (0, 0, 0, 0);
    let mut bad = Vec::new();
    for name in ["p7e_c07", "p7b_c01", "p7b_c08", "p7b_c03", "p7b_c13"] {
        let Ok(data) = fixture::read(format!("{ctx}/fixtures/{name}.bin")) else {
            fixture::skip(format_args!("{name}.bin absent"));
            continue;
        };
        let frames = frames_live(&data);
        for (k, fr) in frames.iter().enumerate() {
            let g = i(fr.live_ball(), 0x264);
            assert!((0..=1).contains(&g), "{name} frame {k}: {g} ghost touches in one step");
            ghosts += g as usize;
        }
        let c = frames[0].global(0x422f90) as u32;
        let world = worlds.entry(c).or_insert_with(|| court::world(&mut iso, c).unwrap());
        for (k, w) in frames.windows(2).enumerate() {
            let (a, b) = (w[0].live_ball(), w[1].live_ball());
            if w[1].vsync() != w[0].vsync() + 1 || i(b, 0xac) < i(a, 0xac) || i(b, 0xac) > i(a, 0xac) + 1 || v3(a, 0x130) == [0.0; 3] {
                continue;
            }
            let (mut fl, shot) = live(a);
            fl.rally = w[1].gm()[0x55] == 3;
            fl.step_world(&shot, &COURTS[c as usize], world, &materials);
            let want = (v3(b, 0xe0).map(f32::to_bits), v3(b, 0x130).map(f32::to_bits));
            if (fl.ball.pos.map(f32::to_bits), fl.ball.vel.map(f32::to_bits)) != want {
                inexact += 1;
                continue;
            }
            exact += 1;
            ghosts_exact += (i(b, 0x264) == 1) as usize;
            let normal: [f32; 4] = std::array::from_fn(|n| f(b, 0x210 + 4 * n));
            if fl.normal.map(f32::to_bits) != normal.map(f32::to_bits) {
                bad.push(format!("{name} frame {}: normal {:?}, game {normal:?} (ghosts {})", k + 1, fl.normal, i(b, 0x264)));
            }
        }
    }
    eprintln!("{exact} live steps bit-exact ({inexact} not, skipped); {ghosts} ghost-touch steps, {ghosts_exact} of them stepped");
    assert!(bad.is_empty(), "{} mismatches:\n{}", bad.len(), bad[..bad.len().min(20)].join("\n"));
    if fixture::exists(format!("{ctx}/fixtures/p7e_c07.bin")) && fixture::exists(format!("{ctx}/fixtures/p7b_c01.bin")) {
        assert_eq!(ghosts_exact, ghosts, "ghost-touch steps not bit-exact");
        assert!(ghosts >= 15, "{ghosts} ghost touches");
    }
}

/// P123: the rally timeout, forced on `p7e_c03`'s recorded rally frames (no recording reaches it): a ball still
/// in the rally phase 1801 steps after its launch comes to rest on its next step (1800 isn't enough, nor 1801 out
/// of the rally phase), whatever it is doing, and stays put after; its call turns out (out after the net when it
/// touched the net first), and the point-over check ends the point on it: out (the hitter loses) in a rally, a
/// fault on the serve, out after the net. A ball that touched the net first and leaves the arena is out after the net.
#[test]
fn rally_timeout_rests_the_ball_and_ends_the_point() {
    use hst_sim::judge::{BallState, Rally, verdict};
    use hst_sim::{court, replay::frames_live};
    let ctx = concat!(env!("CARGO_MANIFEST_DIR"), "/../../context");
    let (Ok(mut iso), Ok(cnf), Ok(bin), Ok(data)) =
        (fixture::iso(), fixture::read(format!("{ctx}/iso/SYSTEM.CNF")), fixture::read(format!("{ctx}/iso/ZZBIN/GAME.BIN")), fixture::read(format!("{ctx}/fixtures/p7e_c03.bin")))
    else {
        return fixture::skip("disc or p7e_c03.bin absent");
    };
    let materials = court::materials(&Game::new(&cnf, &bin).unwrap());
    let frames = frames_live(&data);
    let c = frames[0].global(0x422f90) as u32;
    let world = court::world(&mut iso, c).unwrap();
    let step = |fl: &mut Flight, shot: &Shot| fl.step_world(shot, &COURTS[c as usize], &world, &materials);
    // the first rally-phase frame in flight (not slow) with each call
    let mut seen = Vec::new();
    for fr in &frames {
        let b = fr.live_ball();
        let (fl, shot) = live(b);
        if fr.gm()[0x55] != 3 || b[0xa4] == 3 || v3(b, 0x130)[2].abs() < 0.01 || seen.iter().any(|(_, f, _): &(u32, Flight, Shot)| f.call == fl.call) {
            continue;
        }
        seen.push((fr.vsync(), fl, shot));
    }
    let calls: Vec<Call> = seen.iter().map(|s| s.1.call).collect();
    assert!([Call::None, Call::In].iter().all(|c| calls.contains(c)), "{calls:?}");
    for (v, start, shot) in seen {
        let mut fl = start;
        fl.rally = true;
        fl.frame = 1800;
        step(&mut fl, &shot);
        assert!(!fl.at_rest, "vsync {v}: at rest after 1800 steps");
        let mut fl = Flight { rally: false, frame: 1801, ..start };
        step(&mut fl, &shot);
        assert!(!fl.at_rest, "vsync {v}: at rest out of the rally phase");
        let mut fl = Flight { rally: true, frame: 1801, ..start };
        step(&mut fl, &shot);
        let want = if start.call == Call::Net { Call::NetOut } else { Call::Out };
        assert!(fl.at_rest && fl.call == want && fl.frame == 1802, "vsync {v}: {:?} → {:?} at {}", start.call, fl.call, fl.frame);
        let rested = fl;
        step(&mut fl, &shot);
        assert_eq!(format!("{fl:?}"), format!("{rested:?}"), "vsync {v}: moved at rest");
        // the point-over check: hitter 0 (serving) loses a rally ball, faults a serve
        let view = BallState { call: fl.call, contacts: fl.contacts, stopped: fl.at_rest, pos: fl.ball.pos };
        for shots in [1, 3] {
            let mut r = Rally::default();
            assert!(r.check(&view, shots, 0, 0, None, true), "vsync {v}: point not over");
            let got = r.judge(None);
            let want = match (fl.call, shots) {
                (_, 1) => (verdict::FAULT, None),
                (Call::NetOut, _) => (verdict::NET_OUT, Some(1)),
                _ => (verdict::OUT, Some(1)),
            };
            assert_eq!((got.call, got.winner), want, "vsync {v}: {:?} on shot {shots}", start.call);
        }
        eprintln!("vsync {v}: {:?} → {:?}", start.call, fl.call);
    }
    // touched the net first: resting turns it out after the net, and so does leaving the arena
    assert_eq!(hst_sim::judge::rested(Call::Net), Call::NetOut);
    assert_eq!(hst_sim::judge::left_arena(Call::Net, [0.0, 0.0, 19.9]), Call::NetOut);
    assert_eq!(hst_sim::judge::left_arena(Call::Net, [10.6, 0.0, 19.8]), Call::Net);
    assert_eq!(hst_sim::judge::left_arena(Call::In, [11.0, 0.0, 0.0]), Call::In);
}

/// P123: a serve that hits a receiver is a let only if it touched the net first; otherwise the receiver simply
/// loses the point. On the server's team it is a fault either way.
#[test]
fn serve_body_hit_is_a_let_only_off_the_net() {
    use hst_sim::judge::{BallState, Rally, verdict};
    for (call, body, want) in [
        (Call::Net, 1, (verdict::LET, None)),
        (Call::None, 1, (verdict::POINT, Some(0))),
        (Call::NetOut, 1, (verdict::POINT, Some(0))),
        (Call::Net, 2, (verdict::FAULT, None)),
    ] {
        let mut r = Rally::default();
        let b = BallState { call, contacts: 0, stopped: true, pos: [0.0; 3] };
        assert!(r.check(&b, 1, 0, 0, Some(body), true));
        let v = r.judge(Some(body));
        assert_eq!((v.call, v.winner), want, "{call:?} on player {body}");
    }
}
