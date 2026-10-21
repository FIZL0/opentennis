//! P130: a computer server's hand-over from its serve to the rally, and its serve aim's frame.

use super::*;
use crate::{Args, GameSpace};
use hst_sim::rng::Mt;

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

fn tick(app: &mut App) -> &Game {
    app.world_mut().run_schedule(FixedUpdate);
    app.world().resource::<Game>()
}

/// Draws from `before` to `after` (fewer than a few thousand).
fn drawn(before: &Mt, after: &Mt) -> usize {
    let mut m = before.clone();
    let probe = |m: &Mt| (m.0 .1, m.clone().next());
    let want = probe(after);
    (0..5000).find(|_| probe(&m) == want || {
        m.next();
        false
    })
    .expect("generator reached")
}

/// The original enters the rally off its serve as from any other state: the Return roll, then (its team's hits used
/// up) the net pick and the count (P11k6 spec, the recorded 1 → 3 update). Forced to repick, each server's entry
/// draws them in that order, on every point.
#[test]
fn a_server_enters_its_rally_return_roll_first() {
    if !hst_data::fixture::has_iso() {
        return;
    }
    for singles in [true, false] {
        let (mut entries, mut telling) = (0, 0);
        let mut app = cpus(singles);
        for _ in 0..5000 {
            let g = app.world().resource::<Game>();
            let s = g.score.server as usize;
            let serving = g.players[s].ai_mind.is_some_and(|m| m.phase == Ai::Serve);
            let g = tick(&mut app);
            let Some(mut mind) = g.players[s].ai_mind.filter(|m| serving && m.phase == Ai::Rally) else { continue };
            if g.phase != Phase::Rally {
                continue;
            }
            assert_eq!(g.doubles_ai.me[s].phase, Some(Ai::Serve), "singles {singles}: the step enters from the serve");
            mind.net_left = 0;
            let g = &mut *app.world_mut().resource_mut::<Game>();
            g.players[s].ai_mind = Some(mind);
            let row = g.players[s].ai;
            let rate = if singles { row.singles_center_rate } else { row.doubles_center_rate };
            let before = g.rng.ai.clone();
            let (mut r, mut want) = (before.clone(), mind);
            let back = hst_sim::position::Return::new(rate, &mut || r.next());
            want.rally(&mut || r.next());
            // the old order: the pick and the count, then the Return roll
            let (mut r, mut old) = (before.clone(), mind);
            old.rally(&mut || r.next());
            let old_back = hst_sim::position::Return::new(rate, &mut || r.next());
            telling += (old != want || old_back != back) as i32;
            assert!(step(g, s, &mind));
            let got = g.players[s].ai_mind.unwrap();
            assert!(drawn(&before, &g.rng.ai) >= 3);
            assert_eq!((got.net, got.net_left), (want.net, want.net_left), "singles {singles}");
            assert_eq!(g.doubles_ai.me[s].rally.back.going, back.going, "singles {singles}");
            entries += 1;
        }
        assert!(entries >= 6 && telling >= 2, "singles {singles}: {entries} entries, {telling} telling");
    }
}

/// A later point's first state entries in doubles draw as recorded: the server's serve entry 17, the receiver
/// nothing, the two others the Return roll each (they enter the rally again on every point, not only the first).
#[test]
fn every_point_enters_again() {
    if !hst_data::fixture::has_iso() {
        return;
    }
    let mut app = cpus(false);
    let (mut points, mut reset) = (0, false);
    for _ in 0..5000 {
        let before = app.world().resource::<Game>().rng.ai.clone();
        let g = tick(&mut app);
        if std::mem::replace(&mut reset, g.players.iter().all(|p| p.ai_mind.is_some_and(|m| m.phase == Ai::Start))) {
            assert_eq!(drawn(&before, &g.rng.ai), 17 + 1 + 1, "point {points}");
            points += 1;
        }
    }
    assert!(points >= 4, "{points} points");
}

/// The serve aim is drawn on the frame before the contact of the swing the serve starts, also on a press as late
/// as the AI's pick allows (a serve error of 8 presses at the frame its own pick puts the contact on): the swing still
/// takes at least 2 frames, so the aim never comes at the press.
#[test]
fn the_serve_aims_on_the_frame_before_its_contact() {
    if !hst_data::fixture::has_iso() {
        return;
    }
    for singles in [true, false] {
        let mut app = cpus(singles);
        let mut serves = 0;
        let (mut pressed, mut aimed) = (None, None);
        for t in 0..5000 {
            let g = &mut *app.world_mut().resource_mut::<Game>();
            g.players.iter_mut().for_each(|p| p.ai_timing.serve = 8);
            let s = g.score.server as usize;
            let (swing, shots) = (g.players[s].ai_serve_swing.is_some(), g.shots);
            let g = tick(&mut app);
            match (swing, g.players[s].ai_serve_swing.is_some()) {
                (false, true) => pressed = Some(t),
                (true, false) => aimed = Some(t),
                _ => {}
            }
            if shots == 0 && g.shots == 1 {
                let (at, press) = (aimed.take().expect("aimed"), pressed.take().expect("pressed"));
                assert!(at + 1 == t && press + 2 <= t, "singles {singles}: press t{press} aim t{at} contact t{t}");
                serves += 1;
            }
        }
        assert!(serves >= 6, "singles {singles}: {serves} serves");
    }
}
