//! P131: when a computer server's serve state ends: the hand-over to the rally on the frame its stroke's recovery
//! runs out (+ 1, its AI update comes before its own step), in the serve or rally phase only.

use super::*;
use crate::{Args, GameSpace};

const ISO: &str = hst_data::fixture::ISO_PATH;

/// A match of computer players only, set up (not started).
fn cpus(singles: bool) -> App {
    crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
    let chars = if singles { vec![6, 1] } else { vec![6, 1, 2, 3] };
    app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 10, stage: None, play: true, singles, chars, outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
    app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
    app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
    app.add_systems(Startup, setup);
    app.add_systems(FixedUpdate, (remember, control, simulate, motions, character::tick, played_out, held_ball).chain());
    app.update();
    app
}

fn tick(app: &mut App) -> &mut Game {
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().resource_mut::<Game>().into_inner()
}

/// Every point's server leaves its serve state on the tick contact + recovery + 1, in the rally phase: that tick it
/// makes the rally's entry and stays put (no stick); up to it the server stays in its serve state, still too.
#[test]
fn a_server_hands_over_at_recovery_plus_one() {
    if !hst_data::fixture::has_iso() {
        return;
    }
    for singles in [true, false] {
        let mut app = cpus(singles);
        let (mut contact, mut handovers) = (None, 0);
        for t in 0..5000 {
            let g = app.world().resource::<Game>();
            let s = g.score.server as usize;
            let (shots, phase, pos) = (g.shots, g.phase.clone(), g.players[s].pos);
            let serving = g.players[s].ai_mind.is_some_and(|m| m.phase == Ai::Serve);
            let g = tick(&mut app);
            if shots == 0 && g.shots == 1 {
                contact = Some((t, g.players[s].recover));
            }
            let Some((at, recover)) = contact.filter(|&(at, _)| serving && t > at) else { continue };
            let left = g.players[s].ai_mind.is_some_and(|m| m.phase == Ai::Rally);
            let due = t == at + recover + 1;
            if phase != Phase::Rally {
                assert!(!left, "singles {singles} t{t}: handed over in {phase:?}");
                continue;
            }
            assert_eq!(left, due, "singles {singles} t{t}: contact t{at}, recovery {recover}");
            assert_eq!(g.players[s].pos, pos, "singles {singles} t{t}: the server moved before its rally routine");
            if left {
                assert_eq!(g.doubles_ai.me[s].phase, Some(Ai::Rally), "singles {singles}: no rally entry");
                (contact, handovers) = (None, handovers + 1);
            }
        }
        assert!(handovers >= 4, "singles {singles}: {handovers} hand-overs");
    }
}

/// A point over before the hand-over's tick (forced: a fault called 5 ticks after the contact) leaves its server in
/// its serve state to the next point: no hand-over, no rally entry.
#[test]
fn a_point_over_first_gives_no_rally_entry() {
    if !hst_data::fixture::has_iso() {
        return;
    }
    for singles in [true, false] {
        let mut app = cpus(singles);
        let (mut contact, mut forced) = (None, 0);
        for t in 0..5000 {
            let g = tick(&mut app);
            let s = g.score.server as usize;
            if g.shots == 0 {
                contact = None;
                continue;
            }
            let at = *contact.get_or_insert(t);
            if t == at + 5 && g.phase == Phase::Rally {
                g.post = Some(hst_sim::flow::PostPoint::called(None, 2, &g.board));
                g.phase = Phase::Post;
                forced += 1;
            }
            if t >= at + 5 {
                let mind = g.players[s].ai_mind.map(|m| m.phase);
                assert_eq!(mind, Some(Ai::Serve), "singles {singles} t{t}: contact t{at}, {:?}", g.phase);
                assert_eq!(g.doubles_ai.me[s].phase, Some(Ai::Serve), "singles {singles} t{t}: a rally entry");
            }
        }
        assert!(forced >= 4, "singles {singles}: {forced} points");
    }
}
