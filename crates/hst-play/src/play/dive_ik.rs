//! B716: a contact dive's arm solve on the posed nodes. When a dive finds a ball it can reach, the right upper arm
//! turns toward the ball (from where the slide ends) and the racket turns about its own y to face the ball's
//! incoming direction (`hst_sim::pose::dive_solve`); each tick the turns go on by the dive ramp's weight
//! (`hst_sim::pose::DiveIk`) as the contact IK's arm turn does (`Player::arm`), the racket's as its turned local
//! (`Player::racket`), for the drawing and the next doubles player's stand test (`doubles_ai::scratch`).

use super::{Game, Player};
use hst_sim::motion;
use hst_sim::pose::{DiveIk, dive_rig, dive_solve};

const ID: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

/// The dive `d` player `i` just started: a contact dive solves its arm and takes over the arm solve.
pub(super) fn start(g: &mut Game, i: usize, d: &hst_sim::swing::Dive) {
    if !d.contact {
        return;
    }
    let Some(data) = g.data.get(i) else { return };
    let Some(clip) = data.motions.get(&(motion::id::RECEIVE as usize)) else { return };
    let Some(ball) = g.doubles_ai.record.window().get(d.frame).copied() else { return };
    let rig = dive_rig(&data.skeleton, clip);
    let p = &g.players[i];
    let (b, v) = (ball.pos, ball.vel);
    let solve = dive_solve(&rig, p.pos, d.dir, g.reaches[i].hand, d.slide, [b[0], b[1], b[2]], [v[0], v[1], v[2]]);
    let p = &mut g.players[i];
    p.ik = None;
    p.dive_ik = Some(DiveIk::new(solve, rig.racket, d.frame as i32));
}

/// The dive solve's tick, after the contact IK's (which a dive's solve replaces): the arm turn and its weight, the
/// racket's turned local while the weight is above 0.
pub(super) fn tick(p: &mut Player) {
    p.racket = None;
    let Some(ik) = p.dive_ik.as_mut() else { return };
    let Some(w) = ik.tick() else {
        p.dive_ik = None;
        return;
    };
    p.arm = Some((ik.quats(), w));
    if w > 0.0 {
        // only its rotation rows are read: each reader keeps its own translation
        p.racket = Some(ik.racket_local(w, &ID));
    }
}
