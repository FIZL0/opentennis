//! B722: a smash's contact IK. The game reaches for a smash's contact point with the overhead solve the serve uses,
//! fed the smash motion's arm (its racket point at frame 8), so the body steps sideways (and forward or back) into
//! the smash over the frames to contact, as a stroke's does; the jump it also gives isn't applied (players stay on
//! the ground, as the serve's).

use hst_sim::motion::id;
use hst_sim::pose::ArmIk;
use hst_sim::serve;

use super::Game;

/// Player `i` starts a smash at `ball`, `frames` ticks before its contact: its contact IK, ticked with the strokes'.
pub(super) fn start(g: &mut Game, i: usize, ball: [f32; 3], frames: i32) {
    let Some(d) = g.data.get(i) else { return };
    let Some(clip) = d.motions.get(&(id::SMASH as usize)) else { return };
    let arm = serve::serve_arm(&d.skeleton, clip, d.paths.get(&(id::SMASH as usize)), false);
    let p = &mut g.players[i];
    let pos = [p.pos[0], p.pos[1], p.pos[2], 1.0];
    // the side scale is +1 on the −z end (`end`); the second size factor is 1 in normal play
    let (solve, _jump) = serve::overhead_solve(&arm, [ball[0], ball[1], ball[2], 1.0], pos, [p.end, 1.0]);
    // the ramp runs over every frame to contact, as the serve's (a stroke's takes the last 8)
    p.ik = Some(ArmIk { solve, frames, n: 0, on: true, free: true, pos0: pos, seen: pos });
}
