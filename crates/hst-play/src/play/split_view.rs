//! B633/M146: split screen for exactly one human on each team on one machine (singles, or doubles with one human a
//! side: each half is a one-player camera, so a team with two humans plays on the shared picture; never online, not
//! in the original look). Every match starts on the normal shared camera; 1P's Select (only 1P's) switches the split
//! on and off in live play, and the choice isn't kept (user 2026-10-21). Split, each half
//! shows the original's alternate camera (`alt_camera`) low behind that team's human, so everyone plays from behind
//! themselves. Side by side on a wide window (1P's team left),
//! stacked on a tall one (1P's team on top). Shown on the plain match view (serve, rally and B650 the umpire's
//! call and pop-ups after the point, as the one-human alternate camera stays through them): the intro, cut-aways,
//! replays and the change of ends show the one picture as before.
//!
//! Each human's stick and d-pad turn with their own half's picture as they're read, before the tick's pads are
//! recorded (`tick_input`), so instant replays and the tick hash replay what was played.
//!
//! Presentation only: the match camera (`Game::cam`) and this machine's own (`own_camera`) step as before and nothing
//! in play reads these views; the main camera takes the first half's view after `camera` has placed it, so the
//! billboards and the eye-hiding passes that follow the main camera follow that half. B634: the second half's
//! camera is the second view (`twin_view`): the camera-facing quads get copies turned to it, drawn there only, and
//! the crowd's and the eye-hiding culls keep what either half needs.

use bevy::camera::Viewport;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::sync::atomic::{AtomicBool, Ordering};

use hst_sim::alt_cam::{self, Follow};
use hst_sim::camera::View;

use super::own_camera::{self, OwnCam};
use super::{Game, Pads, Phase, controls};
use crate::twin_view::{self, SecondEye};
use crate::Orbit;

pub fn plugin(app: &mut App) {
    app.add_plugins(crate::twin_view::plugin);
    app.init_resource::<Split>()
        .init_resource::<StickTurn>()
        .init_resource::<TeamCams>()
        .init_resource::<Personal>()
        .add_systems(Update, (press.run_if(|w: &World| !super::front_end::paused(w) && !super::intro::running(w)), split.after(super::camera).before(super::draw)))
        // as `own_camera::step`: on the scene the tick starts from
        .add_systems(FixedUpdate, (step, step_teams).chain().after(super::control).before(super::simulate));
}

/// The picture is split top/bottom now (the score panel puts the near team's half on top, `panel`).
static ON: AtomicBool = AtomicBool::new(false);

pub(super) fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// The bottom half's camera.
#[derive(Component)]
struct SplitCam;

#[derive(Resource, Default)]
struct Split {
    cam: Option<Entity>,
    /// The main camera's field of view `both` last wrote and the 4:3 picture's half-height tangent it came from: a
    /// camera that doesn't rewrite its fov every frame leaves the written one, which mustn't be refitted again.
    wrote: Option<(f32, f32)>,
}

/// Live play or the post-point call (B650) on the plain match view, the remaster's own look.
fn plain_play(w: &World) -> bool {
    let Some(g) = w.get_resource::<Game>() else { return false };
    g.players.len() >= 2
        && matches!(g.phase, Phase::Serve | Phase::Rally | Phase::Post)
        && !w.get_resource::<crate::graphics::Graphics>().is_some_and(|gr| gr.original)
        && !super::intro::running(w)
        && !super::instant_replay::playing(w)
        && !w.get_resource::<super::cutaway::Director>().is_some_and(|d| d.showing())
        && w.get_resource::<OwnCam>().is_none_or(|o| o.plain_view())
}

pub(super) fn split(w: &mut World) {
    let aspect = w.query_filtered::<&Window, With<PrimaryWindow>>().single(w).ok().map_or(1.0, crate::play::widescreen::scene_aspect);
    // chosen, the split stays through everything (user 2026-10-21: cutting back to one picture looks bad); the
    // halves follow their humans in live play and both show the one picture otherwise (intro, calls, replays)
    let on = w.get_resource::<Personal>().is_some_and(|p| p.showing) && w.contains_resource::<Game>();
    let own = on && plain_play(w) && w.get_resource::<TeamCams>().is_some_and(|t| t.0.len() == 2);
    // side by side, the score panels stay as they are
    ON.store(on && aspect < 1.0, Ordering::Relaxed);
    let mut state = w.remove_resource::<Split>().unwrap_or_default();
    if own {
        // 1P's team left (or on top), the other right (or below)
        place(w, &mut state, aspect, aspect >= 1.0, [0, 1]);
    } else if on {
        both(w, &mut state, aspect, aspect >= 1.0);
    } else {
        // (also after a mode's teardown dropped `Split` mid-split: the main camera still has its split layers)
        let split = w.query_filtered::<&RenderLayers, With<Orbit>>().single(w).is_ok_and(|l| *l == top_layers());
        if state.cam.is_none() && !split {
            w.insert_resource(state);
            return;
        }
        if let Some(e) = state.cam.take() {
            w.despawn(e);
        }
        let mut q = w.query_filtered::<(Entity, &mut Camera, Option<&RenderLayers>), With<Orbit>>();
        if let Ok((e, mut cam, layers)) = q.single_mut(w) {
            cam.viewport = None;
            if layers == Some(&top_layers()) {
                w.entity_mut(e).remove::<RenderLayers>();
            }
        }
        w.insert_resource(SecondEye(None));
    }
    w.insert_resource(state);
}

/// The main camera's picture size (physical px).
fn target(w: &mut World) -> Option<UVec2> {
    let mut q = w.query_filtered::<&Camera, With<Orbit>>();
    q.single(w).ok()?.physical_target_size()
}

/// Puts the main camera (`Orbit`) and the second one at the halves' eyes, each with its vertical field of view and
/// its part of the picture (position, size; physical px); `aspect` is a half's width over height.
fn show(w: &mut World, state: &mut Split, eyes: [(Transform, f32); 2], views: [(UVec2, UVec2); 2], aspect: f32) {
    let cam = *state.cam.get_or_insert_with(|| {
        w.spawn((
            SplitCam,
            Camera3d::default(),
            // the main camera clears the whole picture first (`hud_gamma` leaves a camera that clears nothing as it is)
            // B636: and it writes its half as drawn: Bevy blends a later camera's picture onto the target by its alpha,
            // and the players' draws leave alpha under 1 (their texels' A), which showed what lay beneath through them
            Camera {
                order: 2,
                clear_color: ClearColorConfig::None,
                output_mode: bevy::camera::CameraOutputMode::Write { blend_state: Some(bevy::render::render_resource::BlendState::REPLACE), clear_color: ClearColorConfig::None },
                ..default()
            },
            bevy::core_pipeline::tonemapping::Tonemapping::None,
            Projection::Perspective(PerspectiveProjection { far: 20_000.0, ..default() }),
            Transform::default(),
            RenderLayers::from_layers(&[0, twin_view::BOTTOM]),
        ))
        .id()
    });
    let set = |(eye, fov): (Transform, f32), (at, size): (UVec2, UVec2), tf: &mut Transform, proj: &mut Projection, cam: &mut Camera| {
        *tf = eye;
        if let Projection::Perspective(p) = proj {
            p.fov = fov;
            p.near = 0.1;
        }
        let v = Some(Viewport { physical_position: at, physical_size: size.max(UVec2::ONE), ..default() });
        if cam.viewport.as_ref().map(|v| (v.physical_position, v.physical_size)) != v.as_ref().map(|v| (v.physical_position, v.physical_size)) {
            cam.viewport = v;
        }
    };
    let mut q = w.query_filtered::<(Entity, &mut Transform, &mut Projection, &mut Camera, Option<&RenderLayers>), With<Orbit>>();
    let Ok((main_e, mut tf, mut proj, mut main, layers)) = q.single_mut(w) else { return };
    let relayer = layers != Some(&top_layers());
    set(eyes[0], views[0], &mut tf, &mut proj, &mut main);
    if relayer {
        w.entity_mut(main_e).insert(top_layers());
    }
    let mut q = w.query_filtered::<(&mut Transform, &mut Projection, &mut Camera), With<SplitCam>>();
    let mut second = None;
    if let Ok((mut tf, mut proj, mut c)) = q.get_mut(w, cam) {
        set(eyes[1], views[1], &mut tf, &mut proj, &mut c);
        if let Projection::Perspective(p) = &mut *proj {
            // the billboards size themselves by this half's picture
            p.aspect_ratio = aspect;
            second = Some((cam, *tf, p.clone()));
        }
    }
    w.insert_resource(SecondEye(second));
}

/// M146: the personal split's state.
#[derive(Resource, Default)]
pub(super) struct Personal {
    /// Chosen (1P's Select).
    on: bool,
    /// A Select press waiting for the next tick.
    press: bool,
    /// It plays (chosen, and the match can have it).
    showing: bool,
}

/// Each team's half (team 0's first) while split: the alternate camera behind that team's human, as a match with
/// one human shows it (`alt_camera`), the rally ticks so far and the ball the shots look at (the live one, held
/// where the rally left it once the point is over, as `alt_camera`'s).
// ponytail: no glide on switching and no see-through fade per half (the fade's materials are shared by both views)
#[derive(Resource, Default)]
struct TeamCams(Vec<Half>, u32, [f32; 3]);

struct Half {
    /// The human it follows and their end (−z when near).
    human: usize,
    near: bool,
    follow: Follow,
    /// The view shown last tick and this tick.
    prev: View,
    now: View,
}

/// M146: per seat (1P..4P), whether the stick and d-pad turn round as they're read (`tick_input`).
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
pub(super) struct StickTurn(pub(super) [bool; 4]);

/// Every match starts on the shared camera and the choice isn't kept (user 2026-10-21); `HST_SPLIT2=on`: split from
/// the start (shots).
fn start_on() -> bool {
    std::env::var("HST_SPLIT2").is_ok_and(|v| v == "on")
}

/// The split can play: exactly one human on each team on this device (singles or doubles; each half is a one-player
/// camera), not online, the remaster's own look.
fn eligible(g: &Game, pads: &Pads, online: bool, original: bool) -> bool {
    let n = g.players.len();
    let humans = |team: usize| (0..n).filter(|&i| i & 1 == team && pads.slot_of(i, n).is_some()).count();
    !online && !original && n >= 2 && humans(0) == 1 && humans(1) == 1
}

/// Latches 1P's Select (their keyboard or pad) for the next tick.
fn press(
    p: Option<ResMut<Personal>>,
    pads: Option<Res<Pads>>,
    keys: Res<ButtonInput<KeyCode>>,
    bind: Option<Res<controls::Bindings>>,
    held: Option<Res<controls::PadSlots>>,
    gamepads: Query<&Gamepad>,
) {
    let (Some(mut p), Some(pads), Some(bind), Some(held)) = (p, pads, bind, held) else { return };
    let Some(slot) = pads.slot_of(0, 2) else { return };
    let key = held.keys() == Some(slot) && bind.key_just_pressed(&keys, controls::Action::Camera);
    let pad = held.0[slot].and_then(|e| gamepads.get(e).ok()).is_some_and(|g| bind.pad_just_pressed(g, controls::Action::Camera));
    p.press |= key || pad;
}

/// Switches on Select in live play (the serve or the rally, nothing else on screen) and sets which sticks turn: those whose own picture looks from the other end than the one their stick follows
/// in play (`own_camera::flipped`, the shared camera's).
fn step(
    p: Option<ResMut<Personal>>,
    g: Option<Res<Game>>,
    pads: Option<Res<Pads>>,
    np: Option<Res<super::netplay::Netplay>>,
    d: Option<Res<super::cutaway::Director>>,
    gr: Option<Res<crate::graphics::Graphics>>,
    mut turn: ResMut<StickTurn>,
) {
    let (Some(mut p), Some(g), Some(pads)) = (p, g, pads) else { return };
    let press = std::mem::take(&mut p.press);
    if g.is_added() {
        p.on = start_on();
    }
    if !eligible(&g, &pads, np.is_some(), gr.is_some_and(|gr| gr.original)) {
        p.showing = false;
        *turn = StickTurn::default();
        return;
    }
    let live = matches!(g.phase, Phase::Serve | Phase::Rally) && !d.is_some_and(|d| d.showing()) && !g.instant.replaying;
    if press && live {
        p.on = !p.on;
        info!("split view {}", if p.on { "on" } else { "off" });
    }
    p.showing = p.on;
    if !p.on {
        *turn = StickTurn::default();
        return;
    }
    let mut t = StickTurn::default();
    let n = g.players.len();
    for i in 0..n {
        if let Some(s) = pads.slot_of(i, n) {
            t.0[s] = (g.players[i].end < 0.0) != own_camera::flipped(&g, i);
        }
    }
    if *turn != t {
        *turn = t;
    }
}

/// Steps each team's alternate camera on the scene the tick starts from, while split.
fn step_teams(g: Option<Res<Game>>, p: Option<Res<Personal>>, pads: Option<Res<Pads>>, d: Option<Res<super::cutaway::Director>>, mut t: ResMut<TeamCams>) {
    let (Some(g), Some(pads), Some(d)) = (g, pads, d) else { return };
    if !p.is_some_and(|p| p.showing) {
        t.0.clear();
        return;
    }
    let n = g.players.len();
    t.1 = if g.phase == Phase::Rally { t.1 + 1 } else { 0 };
    let play = alt_cam::Play::new(g.shots.max(0) as u32, (g.phase == Phase::Rally).then_some(t.1));
    if matches!(g.phase, Phase::Serve | Phase::Rally) || t.0.is_empty() {
        t.2 = g.flight.ball.pos;
    }
    let ball = t.2;
    for team in 0..2 {
        // the team's human (eligible: every team has exactly one)
        let Some(human) = (0..n).find(|&i| i & 1 == team && pads.slot_of(i, n).is_some()) else { return };
        let pl = &g.players[human];
        let (spot, near) = (pl.pos, pl.end > 0.0);
        // a new shot for a new human or end (the change of ends) or a new point: cut straight to it
        let fresh = t.0.get(team).is_none_or(|h| (h.human, h.near) != (human, near)) || g.cam_cut;
        if fresh {
            let mut follow = Follow::new(&d.shots, n == 4);
            let now = follow.step(spot, near, ball, play);
            let h = Half { human, near, follow, prev: now, now };
            if team < t.0.len() { t.0[team] = h } else { t.0.push(h) }
        } else {
            let h = &mut t.0[team];
            h.prev = h.now;
            h.now = h.follow.step(spot, near, ball, play);
        }
    }
}

/// Each half's aspect (width over height) and its part of the picture (position, size; physical px).
fn halves(target: UVec2, aspect: f32, wide: bool) -> (f32, [(UVec2, UVec2); 2]) {
    if wide {
        let x = target.x / 2;
        (aspect / 2.0, [(UVec2::ZERO, UVec2::new(x, target.y)), (UVec2::new(x, 0), UVec2::new(target.x - x, target.y))])
    } else {
        let y = target.y / 2;
        (aspect * 2.0, [(UVec2::ZERO, UVec2::new(target.x, y)), (UVec2::new(0, y), UVec2::new(target.x, target.y - y))])
    }
}

/// The one picture the main camera was given this frame, whole in both halves (the 4:3 picture fitted to each, as
/// the full window fits it).
fn both(w: &mut World, state: &mut Split, aspect: f32, wide: bool) {
    let Some(target) = target(w) else { return };
    let (half, views) = halves(target, aspect, wide);
    let mut q = w.query_filtered::<(&Transform, &Projection), With<Orbit>>();
    let Ok((tf, Projection::Perspective(p))) = q.single(w) else { return };
    let tan = match state.wrote {
        Some((fov, tan)) if fov == p.fov => tan,
        _ => (p.fov / 2.0).tan() / super::SHOWN_ASPECT.max(1.0 / aspect),
    };
    let fov = 2.0 * (tan * super::SHOWN_ASPECT.max(1.0 / half)).atan();
    let tf = *tf;
    state.wrote = Some((fov, tan));
    show(w, state, [(tf, fov); 2], views, half);
}

/// Shows team `order[0]`'s own camera on the first half (left, or top) and `order[1]`'s on the second.
fn place(w: &mut World, state: &mut Split, aspect: f32, wide: bool, order: [usize; 2]) {
    let a = w.resource::<Time<Fixed>>().overstep_fraction();
    let Some(target) = target(w) else { return };
    let (half, views) = halves(target, aspect, wide);
    state.wrote = None;
    let t = w.resource::<TeamCams>();
    let eye = |i: usize| {
        let (v0, v) = (t.0[i].prev, t.0[i].now);
        let mix = |x: [f32; 3], y: [f32; 3]| Vec3::from(x).lerp(Vec3::from(y), a).to_array();
        let (eye, pitch, yaw) = super::orbit_from(mix(v0.eye, v.eye), mix(v0.rot[2], v.rot[2]));
        let fov = v0.fov + (v.fov - v0.fov) * a;
        // the game's 4:3 picture's height: a half is near square, so fitting its width would show everyone small
        let fov = 2.0 * (fov.tan() * super::SHOWN_ASPECT).atan();
        (Transform::from_translation(eye).with_rotation(Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)), fov)
    };
    let eyes = order.map(eye);
    show(w, state, eyes, views, half);
}

/// The main camera's layers while split: the scene's and its own billboards' (`twin_view::TOP`).
fn top_layers() -> RenderLayers {
    RenderLayers::from_layers(&[0, twin_view::TOP])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M146: 2P singles with the split on: each player's camera looks from behind them, the stick of the one
    /// the shared camera has at the far end turns, and the match plays exactly as without it (skipped without the
    /// ISO).
    #[test]
    fn each_human_has_their_own_camera() {
        use super::super::*;
        if !hst_data::fixture::has_iso() {
            return;
        }
        let app = |split: bool| match_app(split, true, [true, true, false, false]);
        let (mut a, mut b) = (app(true), app(false));
        // after the match's first tick, which starts it on the shared camera
        a.world_mut().run_schedule(FixedUpdate);
        b.world_mut().run_schedule(FixedUpdate);
        a.world_mut().resource_mut::<super::Personal>().on = true;
        let state = |app: &App| {
            let g = app.world().resource::<Game>();
            (g.cam.view, g.flight.ball.pos, g.players.iter().map(|p| p.pos).collect::<Vec<_>>(), app.world().resource::<tick_input::TickPads>().now())
        };
        for tick in 0..300 {
            a.world_mut().run_schedule(FixedUpdate);
            b.world_mut().run_schedule(FixedUpdate);
            assert!(state(&a) == state(&b), "tick {tick}: the match changed");
            let w = a.world();
            let (g, p) = (w.resource::<Game>(), w.resource::<super::TeamCams>());
            assert!(w.resource::<super::Personal>().showing);
            assert_eq!(p.0.len(), 2);
            for i in 0..2 {
                let z = g.players[i].pos[2];
                let eye = p.0[i].now.eye;
                assert!(eye[2] * z > 0.0 && eye[2].abs() > z.abs(), "tick {tick}: player {i}'s eye {eye:?}, at z {z}");
                // turned when its own picture's end isn't the shared camera's
                let shared = g.cam.view.eye[2] * z > 0.0;
                assert_eq!(w.resource::<super::StickTurn>().0[i], !shared, "tick {tick}: player {i}'s stick");
            }
            assert_eq!(w.resource::<super::StickTurn>().0.iter().filter(|t| **t).count(), 1);
        }
    }

    /// A match app (singles: both human, doubles: all CPUs), this module's plugin added or not.
    fn match_app(split: bool, singles: bool, seats: [bool; 4]) -> App {
        use super::super::*;
        use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: hst_data::fixture::ISO_PATH.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles, chars: if singles { vec![6, 1] } else { vec![6, 1, 2, 3] }, outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(Pads { slots: default(), connected: 0, seats: Some(seats), autoplay: false });
        app.init_resource::<tick_input::TickPads>().init_resource::<ButtonInput<KeyCode>>();
        app.world_mut().spawn((crate::GameSpace, Transform::default(), Visibility::default()));
        app.add_plugins((cutaway::plugin, change_ends::plugin, own_camera::plugin, tick_input::plugin));
        if split {
            app.add_plugins(super::plugin);
        }
        app.add_systems(Startup, setup);
        let time = |mut g: ResMut<Game>| g.match_stats.tick();
        app.add_systems(FixedUpdate, (remember, control, simulate, time, motions, character::tick, played_out, held_ball).chain());
        app.update();
        app
    }

    /// B650: split, the halves keep their own cameras through the umpire's call after a point or fault (the post-point
    /// phase), and only the change of ends goes back to the one picture (skipped without the ISO).
    #[test]
    fn halves_stay_through_the_call() {
        use super::super::*;
        if !hst_data::fixture::has_iso() {
            return;
        }
        let mut a = match_app(true, true, [true, true, false, false]);
        a.world_mut().run_schedule(FixedUpdate);
        a.world_mut().resource_mut::<super::Personal>().on = true;
        for _ in 0..30 {
            a.world_mut().run_schedule(FixedUpdate);
        }
        assert!(super::plain_play(a.world()), "serve");
        let held = a.world().resource::<super::TeamCams>().2;
        a.world_mut().resource_mut::<Game>().phase = Phase::Post;
        assert!(super::plain_play(a.world()), "the call");
        // the shots keep looking where the point ended
        a.world_mut().resource_mut::<Game>().flight.ball.pos[1] += 5.0;
        a.world_mut().run_system_cached(super::step_teams).unwrap();
        assert_eq!(a.world().resource::<super::TeamCams>().2, held);
        assert_eq!(a.world().resource::<super::TeamCams>().0.len(), 2);
        a.world_mut().resource_mut::<Game>().phase = Phase::ChangeEnds(0);
        assert!(!super::plain_play(a.world()), "the change of ends");
    }

    /// A team with two humans (1P, 2P and 3P) can't split: each half is a one-player camera.
    #[test]
    fn two_humans_on_a_team_never_split() {
        use super::super::*;
        if !hst_data::fixture::has_iso() {
            return;
        }
        let mut a = match_app(true, false, [true, true, true, false]);
        a.world_mut().run_schedule(FixedUpdate);
        a.world_mut().resource_mut::<super::Personal>().on = true;
        for _ in 0..30 {
            a.world_mut().run_schedule(FixedUpdate);
        }
        assert!(!a.world().resource::<super::Personal>().showing);
    }

    /// B637: in doubles with a human on each team (1P and 3P), split on, each half's camera looks from behind that
    /// team's human, and the match plays as without it (skipped without the ISO).
    #[test]
    fn each_team_has_its_own_camera() {
        use super::super::*;
        if !hst_data::fixture::has_iso() {
            return;
        }
        // 1P plays player 0 (the first team), 3P player 1 (the other)
        let humans = [true, false, true, false];
        let (mut a, mut b) = (match_app(true, false, humans), match_app(false, false, humans));
        a.world_mut().run_schedule(FixedUpdate);
        b.world_mut().run_schedule(FixedUpdate);
        a.world_mut().resource_mut::<super::Personal>().on = true;
        let state = |app: &App| {
            let g = app.world().resource::<Game>();
            (g.cam.view, g.flight.ball.pos, g.players.iter().map(|p| p.pos).collect::<Vec<_>>())
        };
        for tick in 0..300 {
            a.world_mut().run_schedule(FixedUpdate);
            b.world_mut().run_schedule(FixedUpdate);
            assert!(state(&a) == state(&b), "tick {tick}: the match changed");
            let w = a.world();
            let (g, t) = (w.resource::<Game>(), w.resource::<super::TeamCams>());
            assert_eq!(g.players.len(), 4);
            assert!(w.resource::<super::Personal>().showing);
            for team in 0..2 {
                // the team's human: 1P is player 0, 3P player 1
                let (h, z) = (&t.0[team], g.players[team].pos[2]);
                assert_eq!(h.human, team);
                let eye = h.now.eye;
                assert!(eye[2] * z > 0.0 && eye[2].abs() > z.abs(), "tick {tick}: team {team}'s eye {eye:?}, its human at z {z}");
            }
        }
    }
}
