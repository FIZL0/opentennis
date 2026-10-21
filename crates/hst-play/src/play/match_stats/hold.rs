//! B600: the match's last picture until the ceremony takes over. The tick the match ends on goes straight on to the
//! next match's first serve (players on their marks, the serve camera), and a few frames pass before the ceremony
//! begins (its dissolve waits for the screenshot of the frame on screen): those frames, and the still the ceremony
//! dissolves from, showed the players back on their serve marks. The original holds the players and its camera
//! still from her match call into its match-over phase, then dissolves from that picture.
//!
//! Presentation only: through her match call each tick keeps a copy of the match as it was drawn (`Game`, the
//! figures' motions, this machine's own camera); on the tick the match ends that copy, stilled between ticks, is
//! what the frames draw, swapped in after each fixed tick and out before the next, so every tick still runs on the
//! real match. It ends once the ceremony (or the result page) is up. Only with a window: headless runs never draw.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::super::own_camera::OwnCam;
use super::super::{Figure, Game, Phase};
use super::{Screen, ceremony::Ceremony};
use crate::character::Motion;

/// The most frames it holds if neither the ceremony nor the page comes (a frame's still never arrives).
const MOST: u32 = 120;

/// The match as drawn: its state, the figures' motions and this machine's own camera.
#[derive(Clone)]
struct Picture {
    game: Box<Game>,
    motions: Vec<(Entity, Motion)>,
    own: Option<OwnCam>,
}

#[derive(Resource, Default)]
struct Hold {
    /// The last tick's picture, through her match call.
    last: Option<Picture>,
    /// The held picture, and the real match while it is shown.
    shown: Option<Picture>,
    real: Option<Picture>,
    frames: u32,
}

pub(super) fn plugin(app: &mut App) {
    let windowed = any_with_component::<PrimaryWindow>;
    app.init_resource::<Hold>()
        .add_systems(FixedPreUpdate, give_back.run_if(windowed))
        .add_systems(FixedUpdate, (keep.before(super::super::remember), begin.after(super::super::simulate)).run_if(windowed))
        .add_systems(FixedPostUpdate, show.run_if(windowed))
        .add_systems(Update, end.after(super::ceremony::begin).run_if(windowed));
}

/// Swaps `p` in for what the world has, returning that.
fn swap(p: Picture, g: &mut Game, figures: &mut Query<(Entity, &mut Motion), With<Figure>>, own: Option<&mut OwnCam>) -> Picture {
    let game = Box::new(std::mem::replace(g, *p.game));
    let mut motions = Vec::new();
    for (e, m) in p.motions {
        if let Ok((_, mut now)) = figures.get_mut(e) {
            motions.push((e, std::mem::replace(&mut *now, m)));
        }
    }
    let own = match (own, p.own) {
        (Some(now), Some(o)) => Some(std::mem::replace(now, o)),
        _ => None,
    };
    Picture { game, motions, own }
}

/// Before each tick: the real match back.
fn give_back(mut hold: ResMut<Hold>, g: Option<ResMut<Game>>, mut figures: Query<(Entity, &mut Motion), With<Figure>>, own: Option<ResMut<OwnCam>>) {
    if let (Some(real), Some(mut g)) = (hold.real.take(), g) {
        swap(real, &mut g, &mut figures, own.map(|o| o.into_inner()));
    }
}

/// Through her match call: the match as the last tick left it.
fn keep(mut hold: ResMut<Hold>, g: Res<Game>, figures: Query<(Entity, &Motion), With<Figure>>, own: Option<Res<OwnCam>>) {
    hold.last = g.post.as_ref().is_some_and(|p| p.calling_match).then(|| Picture {
        game: Box::new(g.clone()),
        motions: figures.iter().map(|(e, m)| (e, *m)).collect(),
        own: own.map(|o| o.clone()),
    });
}

/// The tick the match ended on (it left the point for the next match's serve): the last picture, stilled, is held.
fn begin(mut hold: ResMut<Hold>, g: Res<Game>) {
    if g.phase == Phase::Post {
        return;
    }
    let Some(mut p) = hold.last.take() else { return };
    // the ceremony and the page read the finished match's result from it
    p.game.match_stats = g.match_stats.clone();
    let v = &mut *p.game;
    for pl in &mut v.players {
        pl.prev = pl.pos;
        pl.prev_facing = pl.facing;
    }
    v.prev_ball = v.flight.ball.pos;
    v.prev_view = v.cam.view;
    for (_, m) in &mut p.motions {
        m.prev = m.clock.sampled;
        m.drawn_prev = m.drawn.sampled;
    }
    if let Some(o) = &mut p.own {
        o.freeze();
    }
    (hold.shown, hold.frames) = (Some(p), 0);
}

/// After each tick: the held picture in for the frames.
fn show(mut hold: ResMut<Hold>, g: Option<ResMut<Game>>, mut figures: Query<(Entity, &mut Motion), With<Figure>>, own: Option<ResMut<OwnCam>>) {
    let (Some(p), Some(mut g)) = (hold.shown.clone(), g) else { return };
    if hold.real.is_none() {
        hold.real = Some(swap(p, &mut g, &mut figures, own.map(|o| o.into_inner())));
    }
}

/// The ceremony (or the page) is up: the real match back, as before.
fn end(
    mut hold: ResMut<Hold>,
    g: Option<ResMut<Game>>,
    cer: Option<Res<Ceremony>>,
    screen: Res<Screen>,
    mut figures: Query<(Entity, &mut Motion), With<Figure>>,
    own: Option<ResMut<OwnCam>>,
) {
    if hold.shown.is_none() {
        return;
    }
    hold.frames += 1;
    if cer.is_none() && !screen.open && hold.frames < MOST {
        return;
    }
    hold.shown = None;
    if let (Some(real), Some(mut g)) = (hold.real.take(), g) {
        swap(real, &mut g, &mut figures, own.map(|o| o.into_inner()));
    }
}

/// A bot match one point from its end, with and without a window: every tick runs the same (the same state hash),
/// and once the match is over the frames show the players where the last point left them, not the next match's
/// serve marks (needs the ISO at the repository root, skipped without).
#[cfg(test)]
mod tests {
    use super::super::super::*;
    use crate::{Args, GameSpace};

    const ISO: &str = hst_data::fixture::ISO_PATH;

    fn app(windowed: bool) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.init_asset::<crate::gs::GsMaterial>();
        app.insert_resource(Args { iso: ISO.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 1, pads: None });
        app.insert_resource(Pads { seats: Some([false; 4]), autoplay: true, ..default() });
        app.init_resource::<tick_input::TickPads>();
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        if windowed {
            app.world_mut().spawn(bevy::window::PrimaryWindow);
        }
        app.add_plugins((cutaway::plugin, bodyhit::plugin, surprise::plugin, reaction_voices::plugin, match_stats::plugin, panel::plugin, popups::plugin, npcs::plugin, foot_fx::plugin, weather::plugin, heard::plugin));
        app.add_systems(PostStartup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, tick_input::systems(), control, simulate, start_effects, age_balloons, motions, character::tick, played_out, held_ball).chain());
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(Startup);
        app.world_mut().run_schedule(PostStartup);
        app
    }

    /// One tick (the real match's state hash after it); first, match point at every serve.
    fn tick(app: &mut App) -> u64 {
        let w = app.world_mut();
        w.run_schedule(FixedPreUpdate);
        {
            let mut g = w.resource_mut::<Game>();
            if g.phase == Phase::Serve && !g.score.match_over {
                // match point: the first point player 0 wins ends it, however long the bots' rallies run
                (g.score.points, g.score.games) = ([3, 0], [1, 0]);
            }
        }
        w.run_schedule(FixedUpdate);
        let hash = snapshot::state_hash(w);
        w.run_schedule(FixedPostUpdate);
        hash
    }

    fn spots(app: &App) -> Vec<V3> {
        app.world().resource::<Game>().players.iter().map(|p| p.pos).collect()
    }

    #[test]
    fn the_match_end_holds_its_last_picture() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        let (mut plain, mut held) = (app(false), app(true));
        let (mut last, mut after) = (Vec::new(), 0);
        for t in 0..60 * 60 * 2 {
            assert_eq!(tick(&mut plain), tick(&mut held), "tick {t} ran differently with the hold");
            if plain.world().resource::<Game>().match_stats.result.is_none() {
                last = spots(&held);
                continue;
            }
            // the real match went on to the next match's serve; the frames still show the point's end
            assert_eq!(plain.world().resource::<Game>().phase, Phase::Serve);
            assert_ne!(spots(&plain), last, "the next match's marks differ from the point's end");
            assert_eq!(spots(&held), last, "tick {t}: the frames show the point's end");
            assert!(held.world().resource::<Game>().match_stats.result.is_some(), "the held picture carries the result");
            after += 1;
            if after == 30 {
                return;
            }
        }
        panic!("the match didn't end");
    }
}
