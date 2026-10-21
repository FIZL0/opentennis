//! P11n2: who of a doubles team plays the ball, the app's side of it: the partner's movement state (the give-way
//! voice), the one-frame miss mark (the own record goes "missed") and the claimant's exemption from keeping apart
//! while it strokes.

use super::*;
use crate::{Args, GameSpace};

const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Hot Shots Tennis (USA).iso");

/// A doubles match of four computer players, set up (not started).
fn doubles() -> App {
    crate::play::AUTOPLAY.store(true, std::sync::atomic::Ordering::Relaxed);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
    app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 10, stage: None, play: true, singles: false, chars: vec![6, 1, 2, 3], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
    app.init_resource::<Pads>().init_resource::<tick_input::TickPads>();
    app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
    app.add_systems(Startup, setup);
    app.add_systems(FixedUpdate, (remember, control, simulate, motions, character::tick, played_out, held_ball).chain());
    app.update();
    app
}

fn dist(a: V3, b: V3) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The stroke state's position check keeps a stroking player 1 m from its partner unless it holds the claim: the
/// claimant stays where it is, the partner (stroking too, not the claimant) is pushed out; the contact step's
/// partner is the same rule's.
#[test]
fn the_claimant_strokes_where_it_stands() {
    if !std::path::Path::new(ISO).is_file() {
        return eprintln!("no ISO, skipped");
    }
    let mut app = doubles();
    let g = &mut *app.world_mut().resource_mut::<Game>();
    let end = g.players[0].end;
    let (a, b) = ([1.0, g.players[0].pos[1], -8.0 * end], [1.5, g.players[2].pos[1], -8.0 * end]);
    (g.players[0].pos, g.players[2].pos) = (a, b);
    // both in a stroke (a press still searching)
    g.players[0].pending = Some(3);
    g.players[2].pending = Some(3);
    g.doubles_ai.claim = (0, g.doubles_ai.frame);
    assert_eq!(stroke_mate(g, 0), None);
    assert_eq!(stroke_mate(g, 2), Some(a));
    stroke_check(g, 0);
    assert_eq!(g.players[0].pos, a, "the claimant was moved");
    stroke_check(g, 2);
    let pushed = g.players[2].pos;
    assert!(dist(pushed, a) >= 0.999, "the partner stayed {} from the claimant", dist(pushed, a));
    // nobody holds it: the claimant-to-be is kept apart too
    g.players[2].pos = b;
    g.doubles_ai.claim.0 = -1;
    stroke_check(g, 0);
    assert!(dist(g.players[0].pos, b) >= 0.999);
    // not in a stroke: the run and stand moves keep apart, not this check
    g.players[0].pos = a;
    g.players[0].pending = None;
    stroke_check(g, 0);
    assert_eq!(g.players[0].pos, a);
}

/// One four-bot run (shared between the checks):
/// - a cancelled lock plays its miss: the mark is up for the player's next step only, and that step writes its own
///   record as missed (its AI is in its swing step);
/// - every miss mark lasts one frame;
/// - teammates are rarely both locked onto a ball, and the first strike cancels the other lock;
/// - players 0 and 1 (stepping before their partners) never call out while the partner is in a stroke.
#[test]
fn one_player_per_ball_and_the_miss_mark() {
    if !std::path::Path::new(ISO).is_file() {
        return eprintln!("no ISO, skipped");
    }
    let mut app = doubles();
    let (mut cancelled, mut told, mut marks, mut both, mut calls) = (None::<usize>, false, 0, 0, 0);
    let (mut pairs, mut episodes) = ([false; 2], 0);
    for _ in 0..60 * 60 * 8 {
        let busy: Vec<bool> = {
            let g = app.world().resource::<Game>();
            (0..4).map(|i| foot_fx::state(g, i) >= 2).collect()
        };
        let before: Vec<bool> = app.world().resource::<Game>().doubles_ai.me.iter().map(|m| m.mark).collect();
        let heard = app.world().resource::<Game>().whooshes.len();
        app.world_mut().run_schedule(FixedUpdate);
        let g = &mut *app.world_mut().resource_mut::<Game>();
        #[allow(clippy::needless_range_loop)] // i is the player: before, me and records together
        for i in 0..4 {
            // a mark from the last frame is gone after this one's step
            assert!(!(before[i] && g.doubles_ai.me[i].mark), "player {i}'s miss mark lasted two frames");
            if before[i] && Some(i) == cancelled && !told {
                assert_eq!(g.doubles_ai.records[i].n, record::HITTING, "player {i} didn't tell its miss");
                told = true;
            }
            marks += g.doubles_ai.me[i].mark as u32;
        }
        // (the lock cancelled above stays until its contact frame, so it doesn't count)
        let locked = |j: usize| g.players[j].contact.is_some_and(|c| !c.cancelled);
        #[allow(clippy::needless_range_loop)] // t is the team: players t and t + 2, and pairs
        for t in 0..2 {
            let pair = locked(t) && locked(t + 2);
            if pair {
                both += 1;
            } else if pairs[t] {
                // the first strike cancels the other lock (a lock that's gone is the striker's)
                let left = [t, t + 2].map(|j| g.players[j].contact.is_some_and(|c| !c.cancelled));
                assert!(!(left[0] && left[1]), "a strike left the partner's lock standing");
                episodes += 1;
            }
            pairs[t] = pair;
        }
        for &(_, j, s) in g.whooshes.iter().skip(heard) {
            if j < 2 && (s == hst_sim::sound::call_out(j, false) || s == hst_sim::sound::call_out(j, true)) {
                calls += 1;
                assert!(!busy[j + 2], "player {j} called out while its partner was in a stroke");
            }
        }
        // the first lock with a few frames to go is cancelled (as a net touch would, without the new path)
        if cancelled.is_none() && g.phase == Phase::Rally {
            for i in 0..4 {
                let swinging = g.doubles_ai.me[i].rally.sub == step::SWING;
                if let Some(c) = g.players[i].contact.as_mut().filter(|c| c.frames >= 3 && swinging) {
                    c.cancelled = true;
                    cancelled = Some(i);
                    break;
                }
            }
        }
    }
    eprintln!("cancelled {cancelled:?}, told {told}, marks {marks}, calls {calls}, both {both} in {episodes}");
    assert!(cancelled.is_some() && told, "no cancelled lock told its partner");
    // a give-way tie, or a partner finding the ball on a later tick, can lock both for a few frames (the spec's
    // quirk; match_s05: three both-presses); it must stay rare
    assert!(episodes <= 4 && both <= 12, "teammates both locked on {both} frames in {episodes} episodes");
}
