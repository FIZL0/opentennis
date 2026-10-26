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
use crate::quat;

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
/// The share of the way from last frame's view to this frame's the picture moves, from the shot's third frame on.
const SMOOTH: f32 = 0.05;
/// The pull-back: the ball depth's lift before the first shot, how much of the ball's lead behind the human it
/// pushes further back and at most, the baseline's depth, how far behind it the ball depth stops, and the most
/// extra gap at the net.
const QUIET_LIFT: f32 = 0.3;
const PUSH: f32 = 0.5;
const PUSH_MAX: f32 = -3.0;
const BASELINE: f32 = -11.885;
const BEHIND: f32 = 6.0;
const NET_GAP: f32 = -3.0;
/// Rally frames the pull-back stays quiet for.
const QUIET_FRAMES: u32 = 30;

/// Where the rally stands, for the pull-back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    /// Nothing struck yet this point, or the rally just begun ([`Play::new`]): the eye backs away only as far as the
    /// ball, a little ahead of it, lies behind the human.
    Quiet,
    /// The rally under way, or the point over.
    Rally,
}

impl Play {
    /// From the shots struck this point and, while the rally phase is on, how many frames it has run.
    pub fn new(shots: u32, rally: Option<u32>) -> Self {
        if shots == 0 || rally.is_some_and(|n| n < QUIET_FRAMES) { Play::Quiet } else { Play::Rally }
    }
}

/// The camera following the human.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Follow {
    shot: Shot,
    /// Frames shown so far and the last one, which the next eases from.
    shown: u32,
    last: Option<View>,
}

impl Follow {
    pub fn new(shots: &[CameraShot], doubles: bool) -> Self {
        Follow { shot: Shot::new(shots, if doubles { DOUBLES } else { SINGLES }, false), shown: 0, last: None }
    }

    /// One frame on, as shown: [`Follow::aim`] then [`Follow::show`].
    pub fn step(&mut self, spot: [f32; 3], near: bool, ball: [f32; 3], play: Play) -> View {
        let v = self.aim(spot, near, ball, play);
        self.show(v)
    }

    /// The shot's own view one frame on: the human stands at `spot` (game space) at the −z end when `near`, the
    /// ball at `ball` (the live one, held where the rally left it once the point is over, across the shot's
    /// restarts). The eye backs away along the view as the ball comes past the human, and the view tilts up to
    /// keep the ball in the picture.
    pub fn aim(&mut self, spot: [f32; 3], near: bool, ball: [f32; 3], play: Play) -> View {
        self.shot.step_with(frames(spot, near, ball), |s| {
            pull_back(s, spot, ball, play == Play::Quiet);
            let dead = s.pitch_dead_zone();
            s.head(dead, ball);
        });
        self.shot.view()
    }

    /// B658 (ours, not in the original): the doubles zoom, [`Follow::aim`] for a team's humans standing at `spots`
    /// (one: the same as `aim`). The shot orbits the team's ground spot (their middle across, the deeper one's
    /// depth) with the same pull-back and ball head-room, and the eye backs further away along the ground until
    /// every one of them is in the picture across: `width` of the shot's 4:3 width shows (less in a narrow split half).
    /// B669: then it moves back in until the nearest one stands at the bottom of the picture, widening the lens to
    /// keep them all across ([`move_in`]), so no empty ground fills the bottom.
    pub fn aim_team(&mut self, spots: &[[f32; 3]], near: bool, ball: [f32; 3], play: Play, width: f32) -> View {
        let [first, ..] = spots else { panic!("no humans") };
        if spots.len() == 1 {
            return self.aim(*first, near, ball, play);
        }
        let spot = team_spot(spots);
        self.shot.step_with(frames(spot, near, ball), |s| {
            pull_back(s, spot, ball, play == Play::Quiet);
            take_in(s, spots, width);
            let dead = s.pitch_dead_zone();
            s.head(dead, ball);
        });
        move_in(self.shot.view(), spots, width)
    }

    /// One frame of [`Follow::aim_team`], as shown, for a split half: the lens also widens to keep the ball and
    /// where it will land (`land`) in the picture ([`take_ball`]; ours, the one-picture alternate camera doesn't).
    pub fn step_team(&mut self, spots: &[[f32; 3]], near: bool, ball: [f32; 3], land: Option<[f32; 3]>, play: Play, width: f32) -> View {
        let v = take_ball(self.aim_team(spots, near, ball, play, width), std::iter::once(ball).chain(land), width);
        self.show(v)
    }

    /// The view on screen this frame, from `now` (the shot's view, or a switch's glide toward it): from the shot's
    /// third frame on, 5 % of the way from the last one shown.
    pub fn show(&mut self, now: View) -> View {
        let v = match self.last {
            Some(last) if self.shown >= 2 => ease(&last, &now),
            _ => now,
        };
        self.shown += 1;
        self.last = Some(v);
        v
    }
}

/// Backs the eye away along the view, on the ground, by how far the ball has come past the human (`quiet`: before
/// the rally is under way) or, with the human inside their baseline, at least by a share of 3 m.
fn pull_back(s: &mut Shot, spot: [f32; 3], ball: [f32; 3], quiet: bool) {
    let [fx, _, fz, _] = s.cam[2];
    let end = F(if fz < 0.0 { -1.0 } else { 1.0 });
    let (p, b) = (end * spot[2], end * ball[2]);
    let depth = if quiet {
        b + QUIET_LIFT
    } else {
        let mut d = b;
        if d < p {
            d += (F(PUSH) * (d - p)).max(F(PUSH_MAX));
        }
        d.max(F(BASELINE) - BEHIND)
    };
    let inside = (p / BASELINE).min(F::ONE);
    let limit = F(NET_GAP) * (1.0 - inside);
    let shift = (depth - p).min(limit);
    let inv = F::ONE / (F(fz) * fz).madd(fx, fx).sqrt();
    let along = [F(fx) * inv, F(fz) * inv];
    for (i, u) in [(0, along[0]), (2, along[1])] {
        s.cam[3][i] = (F::ZERO + s.cam[3][i]).madd(u, shift).0;
    }
}

/// The doubles zoom's share of the picture's half-width a human's body may reach out to, and the body's half-width
/// and middle height (m).
const TEAM_FILL: f32 = 0.95;
const BODY: f32 = 0.4;
const WAIST: f32 = 1.0;

/// A team's ground spot: the middle of its humans across, the depth of the one deepest in their end.
fn team_spot(spots: &[[f32; 3]]) -> [f32; 3] {
    let (lo, hi) = spots.iter().fold((F(f32::MAX), F(f32::MIN)), |(lo, hi), p| (lo.min(F(p[0])), hi.max(F(p[0]))));
    let deep = spots.iter().map(|p| p[2]).fold(0.0, |d: f32, z| if F(z).abs() > F(d).abs() { z } else { d });
    [((lo + hi) * 0.5).0, 0.0, deep]
}

/// Backs the eye away along the ground until every human at `spots` is within [`TEAM_FILL`] of the half-width
/// (`width` of the 4:3 one).
fn take_in(s: &mut Shot, spots: &[[f32; 3]], width: f32) {
    let (right, fwd, eye) = (s.cam[0], s.cam[2], s.cam[3]);
    let flat = (F(fwd[2]) * fwd[2]).madd(fwd[0], fwd[0]).sqrt();
    if flat <= 0.0 {
        return;
    }
    let along = [F(fwd[0]) / flat, F(fwd[2]) / flat];
    // the depth one metre back along the ground adds
    let gain = along[0] * fwd[0] + along[1] * fwd[2];
    let t = F(TEAM_FILL) * width * musl::tanf(s.fov);
    let back = spots.iter().fold(F::ZERO, |most, p| {
        let d = [F(p[0]) - eye[0], F(p[1]) + WAIST - eye[1], F(p[2]) - eye[2]];
        let x = d[0] * right[0] + d[1] * right[1] + d[2] * right[2];
        let z = d[0] * fwd[0] + d[1] * fwd[1] + d[2] * fwd[2];
        most.max(((x.abs() + BODY) / t - z) / gain)
    });
    if back > 0.0 {
        for (i, u) in [(0, along[0]), (2, along[1])] {
            s.cam[3][i] = (F(s.cam[3][i]) - u * back).0;
        }
    }
}

/// B669: where the nearest human's feet go, as a share of the half-height below the middle; the share of the
/// half-width the humans may reach out to once moved in; the widest half-angle.
const FEET: f32 = 0.85;
const MOVED_FILL: f32 = 0.9;
const WIDEST: f32 = 0.6;

/// B669: moves the eye in along the ground until the human nearest it has their feet [`FEET`] down the picture, then
/// widens the half-angle (up to [`WIDEST`]) until every one of them fits across (`width` of the 4:3 width).
fn move_in(mut v: View, spots: &[[f32; 3]], width: f32) -> View {
    let [fx, fy, fz] = v.rot[2];
    let flat = (F(fz) * fz).madd(fx, fx).sqrt();
    if flat <= 0.0 {
        return v;
    }
    let along = [F(fx) / flat, F(fz) / flat];
    // game space is y down: the feet's drop below the eye, and the nearest feet along the ground
    let (gap, drop) = spots.iter().fold((F(f32::MAX), F::ZERO), |(gap, drop), p| {
        let d = (F(p[2]) - v.eye[2]) * along[1] + (F(p[0]) - v.eye[0]) * along[0];
        if d < gap { (d, F(p[1]) - v.eye[1]) } else { (gap, drop) }
    });
    let below = musl::atanf((F(FEET) * musl::tanf(v.fov) * ASPECT).0);
    let want = drop / musl::tanf((F(musl::asinf(fy)) + below).0);
    if want <= 0.0 || gap <= want {
        return v;
    }
    let step = gap - want;
    v.eye[0] = (F(v.eye[0]) + along[0] * step).0;
    v.eye[2] = (F(v.eye[2]) + along[1] * step).0;
    let fill = F(MOVED_FILL) * width;
    let need = spots.iter().fold(F(musl::tanf(v.fov)), |most, p| {
        let [x, _, z] = v.local([p[0], (F(p[1]) - WAIST).0, p[2]]);
        if z > 0.1 { most.max((F(x).abs() + BODY) / (fill * z)) } else { most }
    });
    v.fov = musl::atanf(need.0).min(WIDEST);
    v
}

/// The share of the half-width and half-height the ball may reach out to in a split half.
const BALL_FILL: f32 = 0.9;

/// Widens the half-angle (up to [`WIDEST`]) until each of `points` in front of the eye fits across (`width` of the
/// 4:3 width) and up and down.
fn take_ball(mut v: View, points: impl IntoIterator<Item = [f32; 3]>, width: f32) -> View {
    let tan = F(musl::tanf(v.fov));
    let need = points.into_iter().fold(tan, |most, p| {
        let [x, y, z] = v.local(p);
        if z <= 0.1 {
            return most;
        }
        let across = F(x).abs() / (F(BALL_FILL) * width * z);
        let up = F(y).abs() / (F(BALL_FILL) * ASPECT * z);
        most.max(across).max(up)
    });
    if need > tan {
        v.fov = musl::atanf(need.0).min(WIDEST).max(v.fov);
    }
    v
}

/// 5 % of the way from `last` to `now`: the rotation slerped, the eye and the half-angle eased.
fn ease(last: &View, now: &View) -> View {
    let m = |v: &View| {
        let r = |i: usize| [v.rot[i][0], v.rot[i][1], v.rot[i][2], 0.0];
        [r(0), r(1), r(2), [0.0, 0.0, 0.0, 1.0]]
    };
    let r = quat::to_matrix(quat::slerp(quat::from_matrix(&m(last)), quat::from_matrix(&m(now)), SMOOTH));
    let toward = |a: f32, b: f32| (F::ZERO + a).madd(F(b) - a, SMOOTH).0;
    View {
        rot: [0, 1, 2].map(|i| [r[i][0], r[i][1], r[i][2]]),
        eye: std::array::from_fn(|i| toward(last.eye[i], now.eye[i])),
        fov: toward(last.fov, now.fov),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A split half's lens widens until a ball out to the side is back in the picture, and no wider than [`WIDEST`].
    #[test]
    fn a_wide_ball_widens_the_split_half() {
        let v = View { rot: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], eye: [0.0; 3], fov: 0.3 };
        let w = take_ball(v, [[4.0, 0.0, 10.0]], 0.75);
        assert!(w.fov > v.fov && screen(&w, [4.0, 0.0, 10.0]).is_some_and(|[x, _]| x < 320.0 + 320.0 * 0.75));
        assert_eq!(take_ball(v, [[0.0, 0.0, 10.0]], 0.75).fov, v.fov);
        assert_eq!(take_ball(v, [[40.0, 0.0, 10.0]], 0.75).fov, WIDEST);
        assert_eq!(take_ball(v, [[4.0, 0.0, -10.0]], 0.75).fov, v.fov);
        // the landing spot out wide widens it as the ball out wide does
        assert_eq!(take_ball(v, [[0.0, 0.0, 10.0], [4.0, 0.0, 10.0]], 0.75).fov, w.fov);
    }
}
