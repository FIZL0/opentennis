//! The alternate match camera (M7) against the original: slot 3's doubles serve with the camera switched on
//! (recorded in PCSX2 copy 2 over PINE, 2026-10-21: the human at (3, 0, −12.25), the ball in their hand at
//! (2.939, −0.086, −12.353); the camera settled at eye (3.314, −5.576, −23.989), forward (−0.027, 0.276, 0.961),
//! half-angle 0.3054 (17.5°); the game stores its shot views rounded to 0.001).

use hst_data::exe::Game;
use hst_data::fixture;
use hst_sim::alt_cam::{Fade, Follow, GLIDE, blocks, glide_weight};
use hst_sim::camera::View;

#[test]
fn behind_the_human_as_the_original() {
    let Ok(mut iso) = fixture::iso() else {
        return fixture::skip(format_args!("ISO absent, skipped"));
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/GAME.BIN").unwrap());
    let shots = Game::new(&cnf, &bin).unwrap().camera_shots();
    let mut f = Follow::new(&shots, true);
    let mut v = f.step([3.0, 0.0, -12.25], true, [2.939, -0.086, -12.353]);
    for _ in 0..60 {
        v = f.step([3.0, 0.0, -12.25], true, [2.939, -0.086, -12.353]);
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
        w = g.step([-3.0, 0.0, 12.25], false, [-2.939, -0.086, 12.353]);
    }
    let turned = [-v.eye[0], v.eye[1], -v.eye[2]];
    assert!(turned.iter().zip(w.eye).all(|(x, y)| (x - y).abs() < 1e-3), "far end eye {:?}", w.eye);
    // singles: shot 8, nearer and higher pitched framing
    let mut s = Follow::new(&shots, false);
    let e = s.step([3.0, 0.0, -12.25], true, [2.939, -0.086, -12.353]).eye;
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

