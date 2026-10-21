//! The original's other match camera (M7): a low view from behind the one human (17.5° half-angle), that Select
//! switches to and back with a 15-frame glide. It is one of the scripted shots of the
//! post-point cut-aways ([`crate::cutaway::Shot`]): shot 8 in singles, 9 in doubles, run for as long as it stays
//! selected, orbiting the human's ground spot (facing the other end) and turning after the ball. Behind it, the human
//! fades toward half transparency while their body hides the ball coming at them ([`Fade`]).
//!
//! Presentation only: nothing in play reads it, the stick turns with the eye's end as with the match camera.
//! Arithmetic through `ps2::F` as the other cameras.

#![deny(clippy::float_arithmetic)]

use crate::camera::{ASPECT, View};
use crate::cutaway::{CameraShot, IDENTITY, M4, Shot, frame};
use crate::ps2::F;

/// The shots: singles, doubles.
pub const SINGLES: u8 = 8;
pub const DOUBLES: u8 = 9;
/// Ticks a switch glides over.
pub const GLIDE: u32 = 15;
/// The frames the two shots name: their subject and the frame it looks at, both filled in with the human.
const SUBJECT: u8 = 113;
const LOOK_AT: u8 = 114;
/// How far out the look-at point lies.
const FAR: f32 = 100.0;
/// The share of the way to the live ball the followed ball moves each frame (B621: measured against the original,
/// whose turn after the ball lags it so).
const BALL_EASE: f32 = 0.05;

/// The camera following the human.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Follow {
    shot: Shot,
    /// The ball the shot turns after: eased toward the live one, from where it was at the first frame.
    ball: Option<[f32; 3]>,
}

impl Follow {
    pub fn new(shots: &[CameraShot], doubles: bool) -> Self {
        Follow { shot: Shot::new(shots, if doubles { DOUBLES } else { SINGLES }, false), ball: None }
    }

    /// One frame on: the human stands at `spot` (game space) at the −z end when `near`, the ball at `ball`.
    pub fn step(&mut self, spot: [f32; 3], near: bool, ball: [f32; 3]) -> View {
        let eased = match self.ball {
            Some(b) => std::array::from_fn(|i| (F(b[i]) + (F(ball[i]) - b[i]) * BALL_EASE).0),
            None => ball,
        };
        self.ball = Some(eased);
        self.shot.step(frames(spot, near, eased));
        self.shot.view()
    }
}

/// The frames the shots look up: the human's ground spot facing the other end (the subject), a point on the court's
/// long axis 100 m out beyond the other end (the look-at, as recorded), and the ball (the shots' follow target).
fn frames(spot: [f32; 3], near: bool, ball: [f32; 3]) -> impl Fn(u8) -> M4 {
    let s = if near { 1.0 } else { -1.0 };
    let at = [spot[0], 0.0, spot[2], 1.0];
    let ahead = [0.0, 0.0, (F(FAR) * s).0, 1.0];
    let facing = move |p: [f32; 4]| [[s, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, s, 0.0], p];
    move |i| match i {
        SUBJECT => facing(at),
        LOOK_AT => facing(ahead),
        frame::SPOT => [IDENTITY[0], IDENTITY[1], IDENTITY[2], [ball[0], ball[1], ball[2], 1.0]],
        // the table's first frame is a direction (w = 0): no second subject
        frame::ORIGIN => [IDENTITY[0], IDENTITY[1], IDENTITY[2], [0.0; 4]],
        _ => IDENTITY,
    }
}

/// A switch's blend weight with `left` of its [`GLIDE`] ticks to go (counted down before use): t = 1 − left/15,
/// (t^(1−t))².
pub fn glide_weight(left: u32) -> f32 {
    let t = (1.0 - F(left as f32) / GLIDE as f32).0;
    musl::powf(musl::powf(t, (1.0 - F(t)).0), 2.0)
}

/// Frames the human takes to fade out to half (and back).
pub const FADE: u8 = 5;

/// The human's fade while they hide the ball: a count of 0..=5 up each blocked frame, down otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Fade(u8);

impl Fade {
    /// One frame: `blocked` ([`blocks`] and the rally's state); `post` (after the point) always fades back.
    pub fn step(&mut self, blocked: bool, post: bool) -> f32 {
        self.0 = if blocked && !post { (self.0 + 1).min(FADE) } else { self.0.saturating_sub(1) };
        self.alpha()
    }

    /// The human's alpha: 1 down to 0.5.
    pub fn alpha(&self) -> f32 {
        (1.0 - F(0.5) * (F(self.0 as f32) / FADE as f32)).0
    }
}

/// Whether the human at `spot` hides the ball at `ball` in `view`: the ball beyond them, on the 640×448 picture
/// within 40 px across and 0..200 px above their spot.
pub fn blocks(view: &View, spot: [f32; 3], ball: [f32; 3]) -> bool {
    let d = |a: [f32; 3], b: [f32; 3]| [F(a[0]) - b[0], F(a[1]) - b[1], F(a[2]) - b[2]];
    let (beyond, toward) = (d(ball, spot), d(spot, view.eye));
    if beyond[0] * toward[0] + beyond[1] * toward[1] + beyond[2] * toward[2] <= 0.0 {
        return false;
    }
    let (Some(p), Some(b)) = (screen(view, spot), screen(view, ball)) else { return false };
    let (dx, up) = ((F(b[0]) - p[0]).abs(), F(p[1]) - b[1]);
    dx < 40.0 && 0.0 < up && up < 200.0
}

/// A point on the 640×448 picture (pixels from the top left), if in front of the eye.
fn screen(view: &View, p: [f32; 3]) -> Option<[f32; 2]> {
    let [x, y, z] = view.local(p);
    if z <= 0.0 {
        return None;
    }
    let t = F(musl::tanf(view.fov));
    Some([(F(320.0) + F(x) / (F(z) * t) * 320.0).0, (F(224.0) + F(y) / (F(z) * t * ASPECT) * 224.0).0])
}
