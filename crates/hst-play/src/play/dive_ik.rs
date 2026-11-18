//! B716: a contact dive's arm solve on the posed nodes. When a dive finds a ball it can reach, the right upper arm
//! turns toward the ball (from where the slide ends) and the racket turns about its own y to face the ball's
//! incoming direction (`hst_sim::pose::dive_solve`); each tick the turns go on by the dive ramp's weight
//! (`hst_sim::pose::DiveIk`) as the contact IK's arm turn does (`Player::arm`), the racket's as its turned local
//! (`Player::racket`), for the drawing and the next doubles player's stand test (`doubles_ai::scratch`).
//! Between the ramps (B724) the turn holds at full, as the game leaves the paused dive pose turned.

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
    // the window holds the ball y up; the solve takes it in game space, y down (B721: the game's recorded dive)
    let (b, v) = (ball.pos, ball.vel);
    let solve = dive_solve(&rig, p.pos, d.dir, g.reaches[i].hand, d.slide, [b[0], -b[1], b[2]], [v[0], -v[1], v[2]]);
    let p = &mut g.players[i];
    p.ik = None;
    p.dive_ik = Some(DiveIk::new(solve, rig.racket, d.frame as i32));
}

/// The dive solve's tick, after the contact IK's (which a dive's solve replaces): the arm turn and its weight, the
/// racket's turned local while the weight is above 0. B724: between the ramps (weight 0 after the ramp-up) the
/// dive motion is paused and the game leaves the pose as it was, so the arm and racket hold the full turn.
pub(super) fn tick(p: &mut Player) {
    p.racket = None;
    let Some(ik) = p.dive_ik.as_mut() else { return };
    let Some(w) = ik.tick() else {
        p.dive_ik = None;
        return;
    };
    let w = held(ik.n - 1, w);
    p.arm = Some((ik.quats(), w));
    if w > 0.0 {
        // only its rotation rows are read: each reader keeps its own translation
        p.racket = Some(ik.racket_local(w, &ID));
    }
}

/// The weight the pose shows on dive tick `n` with the ramp's weight `w`: on the hold (weight 0 after the ramp-up)
/// the full turn the ramp-up ended on.
fn held(n: i32, w: f32) -> f32 {
    if w == 0.0 && n > DiveIk::RAMP { 1.0 } else { w }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hst_sim::pose::DiveSolve;

    /// B724: the arm turn's weights over a dive with contact frame 9: unturned on the first tick, the full turn from
    /// the end of the ramp-up through the hold, then the ramp-down.
    #[test]
    fn dive_hold_keeps_the_full_turn() {
        let mut ik = DiveIk::new(DiveSolve { upper: [0.0, 0.0, 0.0, 1.0], angle: 0.5 }, ID, 9);
        let w: Vec<f32> = std::iter::from_fn(|| ik.tick().map(|w| held(ik.n - 1, w))).collect();
        assert_eq!(w, [0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.0, 1.0, 1.0, 1.0, 0.8, 0.6, 0.39999998, 0.19999999]);
    }
}
