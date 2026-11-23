//! B773: the per-frame swing update's motion changes show from the next motion step. The contact countdown's
//! update runs after the frame's motion step: a fast swing's return to speed 1 and the swing waiting behind the
//! body's turn (started 8 frames before contact) are noted in `advance_stroke` and handed to the motion command
//! after `character::tick`, so the next tick's step is the first to play them. The stroke's own start, in the
//! decision, still plays from this tick's step.

use super::*;

/// The motion changes a player's swing update made this tick, due once the tick's motion step has run.
#[derive(Clone, Copy, Default, Debug, serde::Serialize, serde::Deserialize)]
pub(in crate::play) struct AfterStep {
    /// The swing's speed goes back to 1.
    speed_one: bool,
    /// The waiting swing starts: its motion and pace.
    swing: Option<(i32, f32)>,
}

impl AfterStep {
    pub(in crate::play) fn speed_one(&mut self) {
        self.speed_one = true;
    }

    pub(in crate::play) fn swing(&mut self, anim: i32, pace: f32) {
        self.swing = Some((anim, pace));
    }
}

/// After the motion step: each player's due changes go into its motion command.
pub(in crate::play) fn apply(mut g: ResMut<Game>) {
    g.players.iter_mut().for_each(land);
}

/// Player `p`'s due changes into its motion command. Also run at the start of its next swing update, for a
/// schedule without [`apply`] (test harnesses), where nothing has set a motion in between.
pub(in crate::play) fn land(p: &mut Player) {
    let due = std::mem::take(&mut p.after_step);
    if due.speed_one {
        p.cmd.speed = 1.0;
    }
    if let Some((anim, pace)) = due.swing {
        set_motion(p, anim, pace, false, None);
    }
}
