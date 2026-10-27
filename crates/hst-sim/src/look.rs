//! A player's head turning toward the ball. After the motion clip has posed the skeleton, one bone (the head) is
//! turned by two angles: a nod of 0–30° and a turn of ±50°, both measured from the rest pose and shifted so the clip's
//! own pose is always allowed, eased at most 7° a tick. The target is the ball as seen from 0.15 m above the head
//! bone, or "release" (back toward the clip pose). Body hits read the turned head, so this is gameplay: FPU angles
//! (`ps2`), VU0 matrices (`world`).
//!
//! `Look::turn` is one call; `Gate` decides each tick whether a player calls it and with which target.
#![deny(clippy::float_arithmetic)]

use crate::libm::atan2f;
use crate::ps2::F;
use crate::world::{M4, PI, TWO_PI, inverse, mat_mul, rot_x, rot_y, IDENTITY};

/// Degrees to radians, one chopped multiply.
const DEG: f32 = 0.017453292;
/// How far up the nod may go from rest, and how far the turn either way (degrees).
const NOD_UP: f32 = 30.0;
const TURN: f32 = 50.0;
/// A new motion whose rest forward moved more than this (degrees) starts the head from the clip's pose.
const RESEED: f32 = 15.0;
/// Most an angle moves in a tick (degrees).
const STEP: f32 = 7.0;
/// The eye above the head bone's origin (m, along world up: −y).
const EYE: f32 = 0.15;
/// Ticks every player's look waits after anyone's contact (doubles).
pub const HOLD: i32 = 15;
/// Frames after a player's own contact before its head may start tracking again.
pub const JUST_HIT: i32 = 6;

fn rad(deg: f32) -> F {
    F(deg) * DEG
}

/// Nod and turn of a direction: atan2(−y, √(x² + z²)) and atan2(x, z).
fn angles(v: [f32; 4]) -> [f32; 2] {
    let flat = (F(v[0]) * v[0] + F(v[2]) * v[2]).sqrt();
    [atan2f((-F(v[1])).0, flat.0), atan2f(v[0], v[2])]
}

/// a − b folded into (−π, π]: the nearer of d, d ± 2π.
fn wrapped(a: f32, b: f32) -> F {
    let d = (F(a) + PI) - (F(b) + PI);
    let (p, m) = (d + TWO_PI, d - TWO_PI);
    if p.abs() <= d.abs() || m.abs() <= d.abs() {
        if m.abs() <= p.abs() { m } else { p }
    } else {
        d
    }
}

/// `v` clamped into [lo, hi] (the low end checked first).
fn clamp(v: F, lo: F, hi: F) -> F {
    if v < lo {
        lo
    } else if hi < v {
        hi
    } else {
        v
    }
}

/// What the head looks at in a call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    /// The ball's position (x, y, z, w as the ball keeps it).
    Ball([f32; 4]),
    /// Back toward the clip's pose.
    Release,
}

/// One call's view of the skeleton.
pub struct Pose {
    /// The head's local matrix as the clip posed it this tick.
    pub clip: M4,
    /// The head's rest local matrix (the model's).
    pub rest: M4,
    /// The head's parent's world matrix.
    pub parent: M4,
    /// The model is drawn mirrored (a left-hander).
    pub mirrored: bool,
    /// The clip is on or past its last frame (a motion with no clip counts as done).
    pub clip_done: bool,
    /// 1, or 1/(n + 1) on a slow-motion tick.
    pub factor: f32,
}

/// One player's head-look state.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Look {
    pub ready: bool,
    pub reseed: bool,
    /// Skip the per-tick limit once.
    pub snap: bool,
    /// The nod and turn applied last call.
    pub applied: [f32; 2],
    /// The rest forward's nod and turn, recorded while the clip runs.
    pub rest: [f32; 2],
    /// The head's forward nod and turn as last written.
    pub last: [f32; 2],
    /// Ticks the clip has sat at its end.
    pub at_end: i32,
}

impl Look {
    /// Back to the start: not ready, re-seeding on the next call, skipping the limit once if the scene is switching.
    pub fn reset(&mut self, switching: bool) {
        self.ready = false;
        self.last = [0.0; 2];
        self.at_end = 0;
        self.reseed = true;
        self.snap = switching;
    }

    /// One call toward `target`: the head's new local matrix, or `None` when it keeps the clip's.
    pub fn turn(&mut self, pose: &Pose, target: Target) -> Option<M4> {
        // the working copy of the parent: translation w 1, and for a mirrored model each axis's x negated
        let mut parent = pose.parent;
        parent[3][3] = 1.0;
        if pose.mirrored {
            for row in &mut parent[..3] {
                row[0] = (-F(row[0])).0;
            }
        }
        let posed = mat_mul(&pose.clip, &parent);
        let fwd = angles(posed[2]);
        let rest = angles(mat_mul(&pose.rest, &parent)[2]);
        if pose.clip_done {
            if self.at_end > 0 {
                self.applied = [0.0; 2];
            }
            self.at_end += 1;
        } else {
            self.at_end = 0;
        }
        let mut head = None;
        let mut final_fwd = fwd;
        if !self.ready {
            self.ready = true;
        } else {
            let want = match target {
                Target::Ball(ball) => {
                    let mut q = inverse(&mat_mul(&pose.clip, &pose.parent));
                    q[3] = std::array::from_fn(|k| (F(q[3][k]) + (-F(q[1][k])) * (-F(EYE)).0).0);
                    let l = crate::vu0::transform(&q, ball);
                    angles([l[1], l[0], (-F(l[2])).0, 0.0])
                }
                Target::Release => angles([0.0, 0.0, 1.0, 0.0]),
            };
            if self.reseed {
                self.reseed = false;
                self.applied = [(-wrapped(self.last[0], fwd[0])).0, wrapped(self.last[1], fwd[1]).0];
                let limit = rad(RESEED);
                if limit < wrapped(rest[0], self.rest[0]).abs() || limit < wrapped(rest[1], self.rest[1]).abs() {
                    self.applied = [0.0; 2];
                }
            }
            // the windows, from rest, shifted to hold the clip's pose
            let e = wrapped(rest[0], fwd[0]);
            let up = rad(NOD_UP);
            let excess = if up < e { e - up } else if e < 0.0 { e - rad(0.0) } else { F(0.0) };
            let s = -e + excess;
            let nod = clamp(F(want[0]), rad(0.0) + s, up + s);
            let d = wrapped(rest[1], fwd[1]);
            let e = -d;
            let (lo, hi) = (-rad(TURN), -(-rad(TURN)));
            let excess = if e < lo { e + hi } else if hi < e { e - hi } else { F(0.0) };
            let s = d + excess;
            let turn = clamp(F(want[1]), lo + s, hi + s);
            let (nod, turn) = if self.snap {
                self.snap = false;
                (nod, turn)
            } else {
                let step = rad(STEP) * pose.factor;
                let [pn, pt] = self.applied.map(F);
                (clamp(nod, pn - step, pn + step), clamp(turn, pt - step, pt + step))
            };
            let (nod, turn) = ((nod * 1.0).0, (turn * 1.0).0);
            self.applied = [nod, turn];
            let world = mat_mul(&rot(rot_y(nod)), &mat_mul(&rot(rot_x(turn)), &posed));
            final_fwd = angles(world[2]);
            head = Some(mat_mul(&world, &inverse(&parent)));
        }
        if self.at_end == 0 {
            self.rest = rest;
        }
        self.last = final_fwd;
        head
    }
}

/// A rotation as the game builds it: the identity passed through it.
fn rot(m: M4) -> M4 {
    mat_mul(&IDENTITY, &m)
}
