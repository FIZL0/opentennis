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
