//! The AI generator's draws before the first point, and the AI objects nobody plays.
//!
//! A player has an AI object when it is a CPU, or a human whose doubles partner is a CPU. Each one is reset with the
//! rest at every point (the net pick, its timing errors, its picks, its team's hit count); a human's is never
//! updated past that. Making the players draws two resets for each AI object, and the intro resets every one of
//! them again before the CPUs take their opening roles. Every AI object made restarts the AI generator, and the
//! first point's reset overwrites what the intro drew, so only how many draws the last one and the intro take
//! counts, not their values.

use super::{ai_roll, Game};

/// Whether player `i` has an AI object: a CPU, or a human whose doubles partner is a CPU.
pub(super) fn has_ai(g: &Game, i: usize) -> bool {
    let cpu = |k: usize| g.humans.get(k) != Some(&true);
    cpu(i) || g.players.len() == 4 && cpu(i ^ 2)
}

/// Whether a match-start reset of player `i` draws the net pick: always in singles, in doubles once its partner has
/// an AI object (on any team with one, both players have one).
pub(super) fn net_pick(g: &Game, i: usize) -> bool {
    g.players.len() == 2 || has_ai(g, i ^ 2)
}

/// The draws of one reset with no shot to react to: the net pick when `full`, eight timing errors, eight picks, the
/// hit count.
fn reset_draws(g: &mut Game, full: bool) {
    let n = if full { 18 } else { 17 };
    let mut roll = ai_roll(&mut g.rng);
    (0..n).for_each(|_| {
        roll();
    });
}

/// After every player is made and the server is known: the AI generator as the first point finds it. The last AI
/// object made drew two full resets after its restart (its partner, if any, was made before it); the intro then
/// resets every AI object (full: every partner exists by now) and each CPU enters its opening role, as at a point's
/// start: the server's serve redraws its timing errors, picks and count (17), the receiver draws nothing, a doubles
/// player in the rally role rolls its return to the centre (1).
pub(super) fn match_start(g: &mut Game) {
    let n = g.players.len();
    let ai: Vec<usize> = (0..n).filter(|&i| has_ai(g, i)).collect();
    if ai.is_empty() {
        return;
    }
    (0..2 + ai.len()).for_each(|_| reset_draws(g, true));
    for i in ai {
        if g.humans.get(i) == Some(&true) {
            continue;
        }
        if g.score.server == i as i32 {
            reset_draws(g, false);
        } else if g.score.receiver != i as i32 {
            ai_roll(&mut g.rng)();
        }
    }
}

/// A human's AI object (beside a CPU partner) is reset with the others when a point starts, the CPUs' in `bot`:
/// full at the first point, plain after. It takes no role and hears nothing.
pub(super) fn reset_human(g: &mut Game, i: usize) {
    if !has_ai(g, i) || g.players[i].ai_heard != 0 {
        return;
    }
    let first = g.players[i].ai_mind.is_none();
    let full = first && net_pick(g, i);
    reset_draws(g, full);
    g.players[i].ai_heard = 1;
    g.players[i].ai_mind = Some(hst_sim::ai::Mind::default());
}

/// The AI generator's index from setup into the first point against the original's (`context/p11m2`, probed with
/// `research/journal/2026-10-20-p11m2-ai-setup-draws/scripts/p11m2_intro_probe.py` from a Confirm Settings state): singles with a human serving to a CPU, a CPU
/// serving to a CPU, and doubles with a human beside a CPU against two CPUs.
#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::{Args, GameSpace};
    use std::path::Path;

    const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

    fn app(singles: bool, seats: [bool; 4]) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.init_asset::<crate::gs::GsMaterial>();
        let chars = if singles { vec![6, 1] } else { vec![6, 1, 2, 3] };
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles, chars, outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 1, pads: None });
        app.insert_resource(Pads { seats: Some(seats), autoplay: false, ..default() });
        app.init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, bodyhit::plugin, surprise::plugin, reaction_voices::plugin, match_stats::plugin, panel::plugin, popups::plugin, npcs::plugin, foot_fx::plugin, weather::plugin, heard::plugin));
        app.add_systems(PostStartup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, tick_input::systems(), control, simulate, start_effects, age_balloons, motions, character::tick, played_out, held_ball).chain());
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(Startup);
        app.world_mut().run_schedule(PostStartup);
        app
    }

    /// The AI generator's index after setup, then each new one over the first ticks.
    fn indices(singles: bool, seats: [bool; 4], ticks: usize) -> Vec<usize> {
        let mut app = app(singles, seats);
        let at = |app: &App| app.world().resource::<Game>().rng.ai.0 .1;
        let mut seen = vec![at(&app)];
        for _ in 0..ticks {
            app.insert_resource(tick_input::Replay([[tick_input::IDLE; 4]].into()));
            app.world_mut().run_schedule(FixedUpdate);
            let now = at(&app);
            if seen.last() != Some(&now) {
                seen.push(now);
            }
        }
        seen
    }

    #[test]
    fn first_point_index_matches_the_original() {
        if !Path::new(ISO).is_file() {
            return;
        }
        // human P0 serves to a CPU: the intro left 54, the CPU's first-point reset adds 18
        assert_eq!(indices(true, [true, false, false, false], 4)[..2], [54, 72]);
        // CPU against CPU: 89 into the point, two resets, the serve's entry (the serve routine's own draws after it
        // hang on the serve level drawn, and this isn't the captured match's seed)
        assert_eq!(indices(true, [false; 4], 4)[..3], [89, 125, 142]);
        // doubles, human P0 beside a CPU: four full resets (the human's AI object too), then two rally entries
        assert_eq!(indices(false, [true, false, false, false], 4)[..3], [110, 182, 184]);
    }

    /// A later point in doubles: four plain resets (17 each, the human's AI object's too).
    #[test]
    fn later_points_reset_plain() {
        if !Path::new(ISO).is_file() {
            return;
        }
        let mut app = app(false, [true, false, false, false]);
        let tick = |app: &mut App| {
            app.insert_resource(tick_input::Replay([[tick_input::IDLE; 4]].into()));
            app.world_mut().run_schedule(FixedUpdate);
            app.world().resource::<Game>().rng.ai.0 .1
        };
        (0..4).for_each(|_| _ = tick(&mut app));
        // the next point's placement, as `next_point` makes it
        reset_positions(&mut app.world_mut().resource_mut::<Game>());
        let before = app.world().resource::<Game>().rng.ai.0 .1;
        assert_eq!(tick(&mut app), before + 4 * 17);
    }
}
