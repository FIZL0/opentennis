//! The alternate match camera (M7) against the original: slot 3's doubles serve with the camera switched on
//! (recorded in PCSX2 copy 2 over PINE, 2026-10-21: the human at (3, 0, −12.25), the ball in their hand at
//! (2.939, −0.086, −12.353); the camera settled at eye (3.314, −5.576, −23.989), forward (−0.027, 0.276, 0.961),
//! half-angle 0.3054 (17.5°); the game stores its shot views rounded to 0.001).

use hst_data::exe::Game;
use hst_data::fixture;
use hst_sim::alt_cam::{Fade, Follow, GLIDE, Play, blocks, glide_weight};
use hst_sim::camera::View;

#[test]
fn behind_the_human_as_the_original() {
    let Ok(mut iso) = fixture::iso() else {
        return fixture::skip(format_args!("ISO absent, skipped"));
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let shots = Game::new(&cnf, &bin).unwrap().camera_shots();
    let mut f = Follow::new(&shots, true);
    let mut v = f.step([3.0, 0.0, -12.25], true, [2.939, -0.086, -12.353], Play::Quiet);
    for _ in 0..60 {
        v = f.step([3.0, 0.0, -12.25], true, [2.939, -0.086, -12.353], Play::Quiet);
    }
    let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.0015);
    assert!(near(v.eye, [3.314, -5.576, -23.989]), "eye {:?}", v.eye);
    assert!(near(v.rot[2], [-0.027, 0.276, 0.961]), "forward {:?}", v.rot[2]);
    assert!(near(v.rot[0], [1.0, 0.0, 0.028]), "right {:?}", v.rot[0]);
    // the script's 35° is the full angle: the half-angle is 17.5°, as recorded (0.3054)
    assert!((v.fov - 0.3054).abs() < 1e-4, "fov {}", v.fov);
    // the other end: the same view turned round the court centre
    let mut g = Follow::new(&shots, true);
    let mut w = v;
    for _ in 0..61 {
        w = g.step([-3.0, 0.0, 12.25], false, [-2.939, -0.086, 12.353], Play::Quiet);
    }
    let turned = [-v.eye[0], v.eye[1], -v.eye[2]];
    assert!(turned.iter().zip(w.eye).all(|(x, y)| (x - y).abs() < 1e-3), "far end eye {:?}", w.eye);
    // singles: shot 8, nearer and higher pitched framing
    let mut s = Follow::new(&shots, false);
    let e = s.step([3.0, 0.0, -12.25], true, [2.939, -0.086, -12.353], Play::Quiet).eye;
    assert!(e[2] < -12.25 && e[2] > v.eye[2], "singles eye {e:?} not between the human and the doubles eye");
}

#[test]
fn the_switch_glides_over_15_ticks() {
    assert_eq!(glide_weight(GLIDE), 0.0);
    assert_eq!(glide_weight(0), 1.0);
    let w: Vec<f32> = (0..GLIDE).rev().map(glide_weight).collect();
    assert!(w.windows(2).all(|p| p[0] < p[1]), "{w:?}");
}

#[test]
fn the_human_fades_to_half_over_5_frames() {
    let mut f = Fade::default();
    let a: Vec<f32> = (0..6).map(|_| f.step(true, false)).collect();
    let want = [0.9, 0.8, 0.7, 0.6, 0.5, 0.5];
    assert!(a.iter().zip(want).all(|(x, y)| (x - y).abs() < 1e-6), "{a:?}");
    assert!((f.step(true, true) - 0.6).abs() < 1e-6, "after the point it fades back");
    assert!((f.step(false, false) - 0.7).abs() < 1e-6);
    // seen from behind and above: a ball beyond the human, a little higher on screen, is hidden; one behind the eye
    // side of the human or far off to the side isn't
    let v = View { rot: [[1.0, 0.0, 0.028], [0.008, 0.961, -0.276], [-0.027, 0.276, 0.961]], eye: [3.314, -5.576, -23.989], fov: 0.3054326 };
    let spot = [3.0, 0.0, -12.25];
    assert!(blocks(&v, spot, [3.0, -0.5, -9.0]), "beyond, just above on screen");
    assert!(!blocks(&v, spot, [3.0, -1.0, -14.0]), "between the eye and the human");
    assert!(!blocks(&v, spot, [2.0, -1.0, -8.0]), "off to the side");
    assert!(!blocks(&v, spot, [3.0, -9.0, -10.0]), "far above their head");
}


/// B621/B628: shot 9 lock-step against the original through ten points of slot 4's doubles (recorded over PINE in
/// PCSX2 copy 4, 2026-10-21, Select pressed after the load, Cross tapped every second: `context/b621/alt_follow_s04.bin`,
/// per frame vsync, the match's state (its phase at +5: 2 the serve's set-up, 3 the rally, 4 after the point, 1 the
/// change of ends; frames in it at +8), the camera director, the shot camera's view and number, four player matrices
/// and the ball). Each run of the shot starts fresh at its first frame (the serve's set-up after a cut-away or the
/// change of ends) and follows the human (player 0) and the ball, the eye backing away as the ball comes past them,
/// the picture easing after the shot's view. Every frame matches to the stored views' rounding (0.001). The recording
/// reads some game frames twice: a repeat doesn't step the camera.
#[test]
fn follows_through_the_rally_as_the_original() {
    const GM: usize = 4;
    const CH: usize = GM + 0x10;
    const CAM: usize = CH + 8;
    const NUM: usize = CAM + 0x50;
    const PL: usize = NUM + 8;
    const BALL: usize = PL + 4 * 0x40;
    const SAMPLE: usize = BALL + 0x10;
    let Ok(data) = fixture::read("context/b621/alt_follow_s04.bin") else {
        return fixture::skip(format_args!("alt_follow_s04.bin absent, skipped"));
    };
    let Ok(mut iso) = fixture::iso() else {
        return fixture::skip(format_args!("ISO absent, skipped"));
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let shots = Game::new(&cnf, &bin).unwrap().camera_shots();
    let f = |s: &[u8], o: usize| f32::from_le_bytes(s[o..o + 4].try_into().unwrap());
    let v3 = |s: &[u8], o: usize| [f(s, o), f(s, o + 4), f(s, o + 8)];
    let frames: Vec<&[u8]> = data.as_chunks::<SAMPLE>().0.iter().map(|s| s.as_slice()).collect();
    let (mut follow, mut worst, mut n, mut runs) = (None::<Follow>, 0.0f32, 0, 0);
    // the serve struck this point: the rally phase's ball leaving the server's hand
    let mut struck = false;
    // the ball the camera looks at: held where the rally left it once the point is over
    let mut held = [0.0; 3];
    for (k, s) in frames.iter().enumerate() {
        if k > 0 && s[GM..] == frames[k - 1][GM..] {
            continue;
        }
        let phase = s[GM + 5];
        let ran = u16::from_le_bytes([s[GM + 8], s[GM + 9]]) as u32;
        if phase == 2 {
            struck = false;
        } else if phase == 3 && k > 0 {
            let (b, a) = (v3(s, BALL), v3(frames[k - 1], BALL));
            struck |= (b[2] - a[2]).abs() > 0.05;
        }
        if phase != 4 {
            held = v3(s, BALL);
        }
        // the change of ends' first frame: the camera still sees the ball reset to the court centre (the recorder
        // reads it later that frame, already in the server's hand)
        if phase == 1 && ran == 1 {
            held = [0.0; 3];
        }
        // the director's mode and the shot camera's shot both 9 (the first frame of a switch still has the old mode)
        // and each point's set-up starts it again
        if s[CH + 1] != 9 || s[NUM + 4] != 9 || phase == 2 && frames[k - 1][GM + 5] != 2 {
            follow = None;
            continue;
        }
        let spot = v3(s, PL + 0x30);
        let play = Play::new(struck as u32, (phase == 3).then_some(ran));
        let v = follow
            .get_or_insert_with(|| {
                runs += 1;
                Follow::new(&shots, true)
            })
            .step(spot, spot[2] < 0.0, held, play);
        let want = View { rot: [v3(s, CAM), v3(s, CAM + 0x10), v3(s, CAM + 0x20)], eye: v3(s, CAM + 0x30), fov: f(s, CAM + 0x44) };
        let err = (0..3)
            .map(|j| (v.eye[j] - want.eye[j]).abs())
            .chain((0..3).map(|j| (v.rot[2][j] - want.rot[2][j]).abs()))
            .fold((v.fov - want.fov).abs(), f32::max);
        if err > worst {
            worst = err;
            eprintln!("frame {k} (phase {phase}, {ran}): err {err:.4} port eye {:?} fwd {:?} | game eye {:?} fwd {:?}", v.eye, v.rot[2], want.eye, want.rot[2]);
        }
        n += 1;
    }
    eprintln!("{runs} runs, {n} frames compared");
    assert!(runs >= 5 && n > 2000, "{runs} runs, {n} frames");
    assert!(worst < 0.0015, "worst error {worst}");
}

