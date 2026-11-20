//! The match's weather moves on a game at a time: the original steps its weather and wind counters after each
//! game and applies them when the next point is set up. Rain makes running slower to pick up (`Stats::new`'s
//! agility ×1.5). In rain each player also casts a round blob (`shadow.tm2`, 1.2 m, vertex colour
//! (128, 128, 128, 77)) just above the ground between the feet, on top of the projected shadow.

mod rain;

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};

use super::{panel, Figure, Game, Phase};
use crate::character::Rig;
use crate::weather::Weather;
use crate::Args;
use hst_sim::flow::CHANGE_ENDS;

pub fn plugin(app: &mut App) {
    app.add_plugins(rain::plugin).add_systems(PostStartup, setup.after(super::setup)).add_systems(FixedUpdate, (tick_start.before(super::remember), step.after(tick_start).after(super::remember))).add_systems(Update, (blobs, fade.before(crate::weather::apply)));
}

/// A player's rain blob.
#[derive(Component)]
struct Blob(usize);

fn setup(mut commands: Commands, args: Res<Args>, g: Res<Game>, mut images: ResMut<Assets<Image>>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso.read("PCDATA/PCCG0.XB").expect("PCCG0 archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let Some(e) = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with("other/shadow.tm2")) else { return };
    let tex = panel::image(&mut images, &arc.read(e).expect("shadow.tm2 bytes"));
    let quad = meshes.add(Plane3d::default().mesh().size(1.2, 1.2));
    // GS alpha: texture 252 × vertex 77 / 128 ≈ 1.18 at the centre, so the blob's core is fully dark.
    let m = materials.add(StandardMaterial { base_color: Color::WHITE, base_color_texture: Some(tex), unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
    for i in 0..g.players.len() {
        commands.spawn((Blob(i), Mesh3d(quad.clone()), MeshMaterial3d(m.clone()), Transform::from_xyz(0.0, -100.0, 0.0), Visibility::Inherited));
    }
}

/// Between the feet (`Bip01RFoot`, `Bip01LFoot`), 0.0075 above the ground (Bevy space: game y is down).
fn blobs(w: Option<Res<Weather>>, figures: Query<(&Figure, &Rig)>, joints: Query<&GlobalTransform>, mut blobs: Query<(&Blob, &mut Transform, &mut Visibility)>) {
    let rain = w.is_some_and(|w| hst_sim::weather::rain(w.today().weather));
    for (b, mut t, mut v) in &mut blobs {
        let feet = figures.iter().find(|(f, _)| f.0 == b.0).and_then(|(_, r)| {
            let at = |n: &str| r.data.joints.iter().position(|j| j.name == n).and_then(|i| joints.get(r.joints[i]).ok()).map(|g| g.translation());
            Some((at("Bip01RFoot")? + at("Bip01LFoot")?) * 0.5)
        });
        match feet.filter(|_| rain) {
            Some(p) => {
                t.translation = Vec3::new(p.x, 0.0075, p.z);
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
}

/// B743: the weather the tick plays (`Game::weather`), taken from the snapshot-kept `Weather` before anything in
/// the tick runs, re-run ticks of a rollback included.
pub(super) fn tick_start(mut g: ResMut<Game>, w: Option<Res<Weather>>) {
    if let Some(w) = w {
        let now = w.today().weather;
        if g.weather != now {
            g.weather = now;
        }
    }
}

/// Each player's clear-weather agility, taken at the first tick.
fn step(mut g: ResMut<Game>, w: Option<ResMut<Weather>>, mut clear: Local<Vec<i32>>) {
    let Some(mut w) = w else { return };
    if g.phase == Phase::Serve && w.game != g.score.games_played as usize {
        w.game = g.score.games_played as usize;
    }
    if clear.len() != g.players.len() {
        *clear = g.players.iter().map(|p| p.stats.agility).collect();
    }
    let rain = hst_sim::weather::rain(w.today().weather);
    for (p, a) in g.players.iter_mut().zip(clear.iter()) {
        p.stats.agility = if rain { a * 150 / 100 } else { *a };
    }
}

/// B360 (user 2026-10-15): the original switches the court's look in one frame when the next game's weather lands
/// on the first point set-up after a change of ends (B377), which shows as the whole picture brightening or
/// darkening. The picture fades into the new game's look over the change of ends instead and holds it until the
/// weather lands. Only the look: `Weather` and the sim keep the original's timing.
fn fade(g: Res<Game>, w: Option<Res<Weather>>, f: Option<ResMut<crate::weather::Fade>>) {
    let (Some(w), Some(mut f)) = (w, f) else { return };
    let game = g.score.games_played as usize;
    let mix = match g.phase {
        Phase::ChangeEnds(n) => Some((Weather { game, ..w.clone() }.today().weather, weight(n * CHANGE_ENDS / hst_sim::flow::change_ends(g.build)))),
        // the change of ends is over but its point hasn't been set up yet
        _ if w.game != game => f.0.map(|(to, _)| (to, 1.0)),
        _ => None,
    };
    f.set_if_neq(crate::weather::Fade(mix));
}

/// How far the look has faded at change-ends tick `n` (counting down to 0): eased in and out, 0 on the phase's
/// first tick, 1 on its last.
fn weight(n: u32) -> f32 {
    let t = 1.0 - n.min(CHANGE_ENDS) as f32 / CHANGE_ENDS as f32;
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fade_runs_over_the_change_of_ends() {
        assert_eq!(weight(CHANGE_ENDS), 0.0);
        assert_eq!(weight(0), 1.0);
        assert_eq!(weight(CHANGE_ENDS / 2), 0.5);
        assert!((0..CHANGE_ENDS).all(|n| weight(n) > weight(n + 1)));
    }
}


/// B743: a rollback over the tick the weather changes re-runs it as the first run went. The ticks read
/// `Game::weather`, taken from the snapshot-kept `Weather` at each tick's start, not the last drawn frame's weather
/// (`crate::weather::now`, set by the render-side `apply`), which after the first run is already the new game's rain:
/// re-run with that, the ticks before the change would skip the flare's `rand()` draws.
#[cfg(test)]
mod b743 {
    use super::super::*;
    use crate::weather::{CourtLook, Weather};
    use crate::{Args, GameSpace};
    use bevy::ecs::system::RunSystemOnce;
    use std::collections::{HashSet, VecDeque};

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>();
        app.init_asset::<crate::gs::GsMaterial>();
        app.insert_resource(Args { iso: hst_data::fixture::ISO_PATH.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 1, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(Pads { seats: Some([false; 4]), autoplay: true, ..default() });
        app.init_resource::<tick_input::TickPads>();
        // the first game clear, the second in the rain
        let mut schedule = [hst_sim::weather::Game { weather: 0, speed: 0.0, degrees: 0.0 }; hst_sim::weather::GAMES];
        schedule[1].weather = 2;
        app.insert_resource(Weather { schedule, game: 0, fixed: None });
        // what the drawn frame's `apply` needs to set `now()` (no court: nothing to redo)
        let none = HashSet::new;
        app.insert_resource(CourtLook { envir: vec![], sky: None, looks: [[0.0; 6]; 6], players: [0.0; 4], bg: none(), skies: none(), holes: none(), decals: none(), acc: vec![], clo: vec![], last: None, mix: None });
        app.world_mut().spawn((GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, bodyhit::plugin, surprise::plugin, reaction_voices::plugin, match_stats::plugin, panel::plugin, popups::plugin, npcs::plugin, foot_fx::plugin, super::plugin, heard::plugin));
        app.add_systems(PostStartup, setup);
        app.add_systems(FixedUpdate, (remember, effects::tick, tick_input::systems(), control, simulate, start_effects, age_balloons, motions, character::tick, played_out, held_ball).chain());
        app.add_systems(FixedUpdate, flare_tick.after(remember).before(control));
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(Startup);
        app.world_mut().run_schedule(PostStartup);
        app
    }

    /// One tick and the frame drawn after it (`apply`); the state's hash after the tick.
    fn tick(app: &mut App) -> u64 {
        app.world_mut().run_schedule(FixedUpdate);
        let h = super::super::snapshot::state_hash(app.world_mut());
        app.world_mut().run_system_once(crate::weather::apply).expect("apply runs");
        h
    }

    #[test]
    fn a_rollback_across_a_weather_change_replays_the_same_ticks() {
        if !hst_data::fixture::has_iso() {
            return;
        }
        const BACK: usize = 8;
        let mut app = app();
        let mut kept: VecDeque<(super::super::snapshot::Snapshot, u64)> = VecDeque::new();
        let mut changed = None;
        for t in 0..60 * 60 * 10 {
            let before = super::super::snapshot::save(app.world_mut());
            let h = tick(&mut app);
            kept.push_back((before, h));
            if kept.len() > 2 * BACK {
                kept.pop_front();
            }
            if changed.is_none() && app.world().resource::<Weather>().game == 1 {
                changed = Some(t);
            }
            if changed.is_some_and(|c| t == c + BACK - 1) {
                break;
            }
        }
        let c = changed.expect("the first game ended");
        let g = app.world().resource::<Game>();
        assert_eq!((g.weather, crate::weather::now()), (2, 2), "the second game plays in the rain");
        // back to BACK ticks before the change: the drawn frames already show the rain
        let first: Vec<u64> = kept.iter().map(|k| k.1).collect();
        super::super::snapshot::restore(app.world_mut(), &kept[0].0);
        assert_eq!(app.world().resource::<Weather>().game, 0);
        let again: Vec<u64> = (0..first.len())
            .map(|_| {
                app.world_mut().run_schedule(FixedUpdate);
                super::super::snapshot::state_hash(app.world_mut())
            })
            .collect();
        assert_eq!(app.world().resource::<Game>().weather, 2);
        let split = first.iter().zip(&again).position(|(a, b)| a != b);
        assert_eq!(split, None, "the re-run split from the first run {} ticks before the weather changed (tick {c})", BACK as i64 - split.unwrap_or(0) as i64);
    }
}
