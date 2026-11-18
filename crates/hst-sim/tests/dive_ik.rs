//! B716: the contact dive's arm solve as the play state runs it (`pose::DiveIk`), against the recorded dives.

use hst_data::fixture;
use hst_sim::pose::{DiveIk, DiveSolve};
use hst_sim::replay::{Frame, frames_live};

fn p_u8(fr: Frame, p: usize, off: usize) -> u8 {
    fr.player_f32(p, off & !3).to_bits().to_le_bytes()[off & 3]
}
fn p_i32(fr: Frame, p: usize, off: usize) -> i32 {
    fr.player_f32(p, off).to_bits() as i32
}

/// Every recorded dive: a contact dive starts the arm solve on its first frame with a ramp of 5 (+0x3fc0), its
/// counter (+0x3fc4) and flag (+0x3fc8) then go as `DiveIk::tick` runs them, the flag dropping on the tick the
/// counter reaches the contact frame + 5; a miss dive never starts it.
#[test]
fn dive_ik_counts_as_recorded() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures").into());
    let mut files: Vec<String> = ["new_recording.bin", "match_s05.bin", "lob_smash_s05.bin", "human_smash_s04.bin", "1p3goodcpus.bin", "p7e_c03.bin", "p7e_c07.bin"].map(String::from).to_vec();
    files.extend((0..14).map(|c| format!("p7b_c{c:02}.bin")));
    let (mut contacts, mut misses) = (0, 0);
    for file in &files {
        let Ok(data) = fixture::read(format!("{dir}/{file}")) else {
            fixture::skip(format_args!("{file} absent, skipped"));
            continue;
        };
        let frames = frames_live(&data);
        for k in 1..frames.len() {
            for p in 0..4 {
                let (a, fr) = (frames[k - 1], frames[k]);
                if p_u8(fr, p, 0x3f58) != 1 || p_u8(a, p, 0x3f58) != 0 {
                    continue;
                }
                let contact = p_i32(fr, p, 0x3f84) - 50;
                let at = format!("{file} vsync {} p{p}", fr.vsync());
                if p_i32(fr, p, 0x3ec4) < 0 {
                    for fj in frames[k..].iter().take(contact as usize + 6) {
                        assert_eq!(p_u8(*fj, p, 0x3fc8), 0, "{at}: a miss dive's arm solve");
                    }
                    misses += 1;
                    continue;
                }
                assert!(frames.len() > k + contact as usize + 6, "{at}: recording ends mid-dive");
                assert_eq!(p_i32(fr, p, 0x3fc0), DiveIk::RAMP, "{at}: ramp");
                let mut ik = DiveIk::new(DiveSolve { upper: [0.0, 0.0, 0.0, 1.0], angle: 0.0 }, [[0.0; 4]; 4], contact);
                let mut on = true;
                for (j, fj) in frames[k..].iter().take(contact as usize + 6).enumerate() {
                    if on {
                        on = ik.tick().is_some();
                    }
                    let want = (p_u8(*fj, p, 0x3fc8) == 1, p_i32(*fj, p, 0x3fc4));
                    assert_eq!((on, ik.n), want, "{at} frame {j}");
                }
                assert!(!on, "{at}: the solve outlived its ramp");
                contacts += 1;
            }
        }
    }
    eprintln!("{contacts} contact dives, {misses} miss dives");
    if contacts + misses > 0 {
        assert!(contacts >= 2, "the p7e/p7b recordings' contact dives");
    }
}

/// The solve's upper-arm turn on Kaito's receive pose: with the turn on, the racket point lies on the line from the
/// shoulder to the ball (from where the slide ends), and the racket angle is within a quarter turn.
#[test]
fn dive_solve_points_the_arm_at_the_ball() {
    use hst_data::{ani, xb::Archive};
    use hst_sim::pose::{Clip, Skeleton, dive_rig, dive_solve, vmul};
    let Ok(mut iso) = fixture::iso() else {
        return fixture::skip(format_args!("disc absent, skipped"));
    };
    let xb = iso.read("PCANI/PC03ANI.XB").unwrap();
    let arc = Archive::parse(&xb).unwrap();
    let stem = ani::motion_name(0x1e, 3).unwrap().to_ascii_lowercase();
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))).unwrap();
    let receive = ani::parse(&arc.read(e).unwrap()).unwrap();
    let md = iso.read("PC/PC03C00.XB").unwrap();
    let marc = Archive::parse(&md).unwrap();
    let e = marc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(".mdl")).unwrap();
    let m = hst_data::mdl::parse(&marc.read(e).unwrap()).unwrap();
    let sk = Skeleton { names: m.node_names.clone(), parent: m.node_parent.clone(), rest: m.node_local.clone() };
    let rig = dive_rig(&sk, &Clip::new(&sk, &receive));
    let tf = |m: &[[f32; 4]; 4], v: [f32; 4]| -> [f32; 3] { std::array::from_fn(|c| (0..4).map(|r| v[r] * m[r][c]).sum()) };
    let mut n = 0;
    for (a, hand) in [(0.3f32, 1.0f32), (-1.2, -1.0), (2.5, 1.0), (-2.9, -1.0)] {
        let dir = [a.sin(), a.cos()];
        let (pos, slide) = ([1.0, 0.0, -11.0], 1.4);
        let ball = [pos[0] + dir[0] * 2.6 + dir[1] * 0.4, 0.8, pos[2] + dir[1] * 2.6 - dir[0] * 0.4];
        let s = dive_solve(&rig, pos, dir, hand, slide, ball, [0.1, -0.05, 0.3]);
        assert!(s.angle.abs() <= std::f32::consts::FRAC_PI_2, "angle {}", s.angle);
        // the arm with the turn on, in the dive's facing (rotation only, from the body)
        let side = [dir[1] * hand, 0.0, -dir[0] * hand, 0.0];
        let turn = [side, [0.0, 1.0, 0.0, 0.0], [dir[0], 0.0, dir[1], 0.0], [0.0, 0.0, 0.0, 1.0]];
        let upper = vmul(&rig.upper, &hst_sim::quat::to_matrix(s.upper));
        let upper = [upper[0], upper[1], upper[2], rig.upper[3]];
        let shoulder = vmul(&vmul(&upper, &rig.above), &turn)[3];
        let chain = [rig.hand, rig.forearm, upper, rig.above, turn].iter().fold(rig.racket, |m, x| vmul(&m, x));
        let tip = tf(&chain, [0.0, 0.7, 0.0, 1.0]);
        let to_tip: [f32; 3] = std::array::from_fn(|k| tip[k] - shoulder[k]);
        let end = [pos[0] + dir[0] * slide, pos[1], pos[2] + dir[1] * slide];
        let to_ball: [f32; 3] = std::array::from_fn(|k| ball[k] - end[k] - shoulder[k]);
        let len = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let cos = (0..3).map(|k| to_tip[k] * to_ball[k]).sum::<f32>() / (len(to_tip) * len(to_ball));
        assert!(cos > 0.9999, "dir {dir:?} hand {hand}: racket point off the ball line, cos {cos}");
        n += 1;
    }
    assert_eq!(n, 4);
}

/// B721: the contact dive's arm solve against the game's posed nodes, from research/journal/2026-11-17-b721-dive-arm-check/scripts/b721_dive_nodes.py (P1 dives on
/// purpose; `<file>.nodes` holds P1's node array on each dive frame: vsync, base, 64 nodes of 0x120 bytes, each its
/// world rows, its local rows at +0x80, its held rotation +0xe0, its node index +0x10c). On every tick the solve's
/// weight w isn't 0 (ramping up and down) the Racket local's rotation is Y(angle · w) · Lr whatever the dive motion
/// does, and the right upper arm's local is its animated rows (the held rotation's) times the solved turn slerped by
/// w: both bit for bit from `dive_solve` on the recorded start (position, facing, slide, hand) and the path's ball at
/// the contact frame in game space (y down, so the height goes in negative) with its pre-hit velocity there (the
/// recorded ball one frame earlier, flown one frame: the live ball's velocity at the contact frame is already the
/// return's). `<file>.dump` (P1's object at the dive's start), if there, has the game's own turn and angle: equal too.
#[test]
fn dive_solve_as_recorded() {
    use hst_data::{ani, xb::Archive};
    use hst_sim::ball::{Ball, Params};
    use hst_sim::pose::{Clip, M4, Skeleton, dive_rig, dive_solve, q_matrix, vmul};
    use hst_sim::quat;
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures").into());
    // Ashley (character 0) as P1, a singles match from the copy's Challenge slot
    let file = "b721_dive_c00";
    let (Ok(data), Ok(nodes)) = (fixture::read(format!("{dir}/{file}.bin")), fixture::read(format!("{dir}/{file}.bin.nodes"))) else {
        return fixture::skip(format_args!("{file} absent, skipped"));
    };
    let Ok(mut iso) = fixture::iso() else {
        return fixture::skip(format_args!("disc absent, skipped"));
    };
    const NODE: usize = 0x120;
    let f = |b: &[u8], o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let u = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let rows = |b: &[u8], o: usize| -> M4 { std::array::from_fn(|r| std::array::from_fn(|c| f(b, o + 16 * r + 4 * c))) };
    let v3 = |b: &[u8], o: usize| [f(b, o), f(b, o + 4), f(b, o + 8)];
    let node_frames: Vec<&[u8]> = nodes.as_chunks::<{ 8 + 64 * NODE }>().0.iter().map(|c| &c[..]).collect();
    // model node `i` of P1 on the frame of vsync `v`
    let node = |v: u32, i: usize| -> &[u8] {
        let n = node_frames.iter().find(|n| u(n, 0) == v).unwrap_or_else(|| panic!("no nodes at vsync {v}"));
        (0..64).map(|q| &n[8 + q * NODE..8 + (q + 1) * NODE]).find(|nd| u(nd, 0x10c) as usize == i).unwrap()
    };
    let bits = |m: &M4| m.map(|r| r.map(f32::to_bits));
    // P1's object at each contact dive's start (vsync, object), if caught
    let dumps = fixture::read(format!("{dir}/{file}.bin.dump")).unwrap_or_default();
    let dumps: Vec<&[u8]> = dumps.as_chunks::<{ 8 + 0x8000 }>().0.iter().map(|c| &c[..]).collect();
    let frames = frames_live(&data);
    let mut dives = 0;
    for k in 1..frames.len() {
        let (a, fr) = (frames[k - 1], frames[k]);
        if p_u8(fr, 0, 0x3f58) != 1 || p_u8(a, 0, 0x3f58) != 0 || p_i32(fr, 0, 0x3ec4) < 0 {
            continue;
        }
        let contact = (p_i32(fr, 0, 0x3f84) - 50) as usize;
        let at = format!("{file} vsync {}", fr.vsync());
        assert!(frames.len() > k + contact, "{at}: recording ends mid-dive");
        for j in 0..=5 {
            assert_eq!(frames[k + j].vsync(), fr.vsync() + j as u32, "{at}: a frame missed in the ramp");
        }
        let pos = [0x3f60, 0x3f64, 0x3f68].map(|o| fr.player_f32(0, o));
        let face = [fr.player_f32(0, 0x3e60), fr.player_f32(0, 0x3e68)];
        let slide = fr.player_f32(0, 0x3f80);
        let side = [fr.player_f32(0, 0x3d40), fr.player_f32(0, 0x3d48)];
        let hand = if side[0] * face[1] - side[1] * face[0] < 0.0 { -1.0 } else { 1.0 };
        let before = frames[k + contact - 1].live_ball();
        let mut ball = Ball { pos: v3(before, 0xe0), vel: v3(before, 0x130), spin: f(before, 0x1a4) };
        ball.fly(&Params::default());
        assert_eq!(ball.pos.map(f32::to_bits), v3(frames[k + contact].live_ball(), 0xe0).map(f32::to_bits), "{at}: the ball flown to the contact frame");
        // the character's skeletons (a costume may differ) and receive motion
        let c = 0;
        let xb = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
        let arc = Archive::parse(&xb).unwrap();
        let stem = ani::motion_name(0x1e, c).unwrap().to_ascii_lowercase();
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))).unwrap();
        let receive = ani::parse(&arc.read(e).unwrap()).unwrap();
        // the game's own solve, caught at the dive's start: the upper arm's turn and the racket angle
        let caught = dumps.iter().find(|d| u(d, 0) == fr.vsync()).map(|d| ([0, 4, 8, 12].map(|o| u(d, 8 + 0x4030 + o)), u(d, 8 + 0x400c)));
        let mut matched = None;
        for costume in 0..10 {
            let Ok(md) = iso.read(&format!("PC/PC{c:02}C{costume:02}.XB")) else { continue };
            let marc = Archive::parse(&md).unwrap();
            let Some(e) = marc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(".mdl")) else { continue };
            let m = hst_data::mdl::parse(&marc.read(e).unwrap()).unwrap();
            let sk = Skeleton { names: m.node_names.clone(), parent: m.node_parent.clone(), rest: m.node_local.clone() };
            let at_node = |n: &str| sk.names.iter().position(|x| x == n).unwrap();
            let (racket, upper) = (at_node("Racket"), at_node("Bip01RUpperArm"));
            let rig = dive_rig(&sk, &Clip::new(&sk, &receive));
            let s = dive_solve(&rig, pos, face, hand, slide, ball.pos, ball.vel);
            let mut ik = DiveIk::new(s, rig.racket, contact as i32);
            let mut ok = true;
            // every tick the solve turns the arm (weight > 0) but the first (weight 0); B724: on the hold between the
            // ramps (weight 0 after the ramp-up) the pose is frozen at the full turn
            for j in 0.. {
                let Some(w) = ik.tick() else { break };
                let w = if w == 0.0 && j > DiveIk::RAMP { 1.0 } else { w };
                if w == 0.0 {
                    continue;
                }
                let v = fr.vsync() + j as u32;
                let got = rows(node(v, racket), 0x80);
                ok &= bits(&ik.racket_local(w, &got))[..3] == bits(&got)[..3];
                // the upper arm: its animated rows (the held rotation's) times the turn at weight w
                let nd = node(v, upper);
                let got = rows(nd, 0x80);
                let anim = q_matrix([0xe0, 0xe4, 0xe8, 0xec].map(|o| f(nd, o)));
                let want = vmul(&anim, &quat::to_matrix(quat::slerp([0.0, 0.0, 0.0, 1.0], s.upper, w)));
                ok &= bits(&want)[..3] == bits(&got)[..3];
            }
            if ok {
                if let Some((q, angle)) = caught {
                    assert_eq!((s.upper.map(f32::to_bits), s.angle.to_bits()), (q, angle), "{at}: the game's caught solve");
                }
                matched = Some(costume);
                break;
            }
        }
        assert!(matched.is_some(), "{at}: no costume's racket turn matches the game's");
        dives += 1;
    }
    assert!(dives >= 1, "{file}: no contact dive");
}

/// B724: the dive's hold. On every recorded dive of P1 in research/journal/2026-11-17-b721-dive-arm-check/scripts/b721_dive_nodes.py's
/// `<file>.nodes` (contact or miss), every node's local rows and held rotation are left exactly as the tick before
/// on dive ticks 6 to the contact frame, the ticks `DiveIk` weighs 0 between its ramps, and on no other tick of the
/// dive: the dive motion pauses and nothing re-poses the nodes (only the placement moves with the slide), so a
/// contact dive's arm and racket stay at the full turn the ramp-up ended on (`dive_solve_as_recorded` checks those
/// locals).
#[test]
fn dive_hold_freezes_the_pose() {
    let dir = std::env::var("HST_FIXTURES").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures").into());
    let file = "b721_dive_c00";
    let (Ok(data), Ok(nodes)) = (fixture::read(format!("{dir}/{file}.bin")), fixture::read(format!("{dir}/{file}.bin.nodes"))) else {
        return fixture::skip(format_args!("{file} absent, skipped"));
    };
    const NODE: usize = 0x120;
    let node_frames: Vec<&[u8]> = nodes.as_chunks::<{ 8 + 64 * NODE }>().0.iter().map(|c| &c[..]).collect();
    // P1's node array on the frame of vsync `v`: each node's local rows (+0x80) and held rotation (+0xe0)
    let pose = |v: u32| -> Vec<(&[u8], &[u8])> {
        let n = node_frames.iter().find(|n| u32::from_le_bytes(n[..4].try_into().unwrap()) == v).unwrap_or_else(|| panic!("no nodes at vsync {v}"));
        (0..64).map(|q| &n[8 + q * NODE..]).map(|nd| (&nd[0x80..0xc0], &nd[0xe0..0xf0])).collect()
    };
    let frames = frames_live(&data);
    let (mut contacts, mut misses) = (0, 0);
    for k in 1..frames.len() {
        let (a, fr) = (frames[k - 1], frames[k]);
        if p_u8(fr, 0, 0x3f58) != 1 || p_u8(a, 0, 0x3f58) != 0 {
            continue;
        }
        let contact = p_i32(fr, 0, 0x3f84) - 50;
        let at = format!("{file} vsync {}", fr.vsync());
        // tick j of the dive is frame k + j; the tick after the ramp-down's first is past the contact frame
        let mut ik = DiveIk::new(DiveSolve { upper: [0.0, 0.0, 0.0, 1.0], angle: 0.0 }, [[0.0; 4]; 4], contact);
        for j in 0..=contact as usize + 2 {
            let held = ik.tick() == Some(0.0) && j > DiveIk::RAMP as usize;
            assert_eq!(held, (6..=contact as usize).contains(&j), "{at} tick {j}: the hold's ticks");
            if j == 0 {
                continue;
            }
            let v = fr.vsync() + j as u32;
            assert_eq!(frames[k + j].vsync(), v, "{at}: a frame missed");
            assert_eq!(pose(v) == pose(v - 1), held, "{at} tick {j}: the pose frozen");
        }
        if p_i32(fr, 0, 0x3ec4) < 0 { misses += 1 } else { contacts += 1 }
    }
    eprintln!("{contacts} contact dives, {misses} miss dives");
    assert!(contacts >= 1 && misses >= 1, "{file}: the recording's contact and miss dives");
}
