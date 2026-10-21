//! B633: a tall window (a phone held upright) shows the match camera's whole court as a thin strip with the players
//! small. While the match plays (serve and rally, the plain match view on screen) such a window splits instead: the
//! team at the end the match camera looks from on the top half, the other team on the bottom half, each half that
//! team's own match camera (B637): the one a singles match with one human at that end shows, behind that end, in
//! doubles framing both of the team. Everything else (the intro, cut-aways, calls, replays, the
//! change of ends, the alternate camera) shows the one picture as before.
//!
//! Presentation only: the match camera (`Game::cam`) and this machine's own (`own_camera`) step as before and nothing
//! in play reads these views; the main camera takes the top half's view after `camera` has placed it, so the
//! billboards and the eye-hiding passes that follow the main camera follow the top half. B634: the bottom half's
//! camera is the second view (`twin_view`): the camera-facing quads get copies turned to it, drawn there only, and
//! the crowd's and the eye-hiding culls keep what either half needs.
//!
//! M146: on a phone or tablet, two humans playing singles against each other on the one device share the match
//! camera, which keeps its end: one of them always plays from the far end. 1P's Select (only 1P's) switches to a
//! split instead, at any window shape: each player's half shows their own match camera, the original's one-human
//! camera behind them (as `own_camera` online), so both play from behind themselves. Side by side on a wide window
//! (1P left), stacked on a tall one (1P on top). Each human's stick and d-pad turn with their own half's picture
//! as they're read, before the tick's pads are recorded (`turn_sticks`), so instant replays and the tick hash
//! replay what was played. Shown only when the plain match view would be (as above), switched only in live play,
//! the choice kept in `split.txt`.

use bevy::camera::Viewport;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::sync::atomic::{AtomicBool, Ordering};

use hst_sim::camera::{self as sim_cam, Scene, View};

use super::own_camera::{self, OwnCam};
use super::{Game, Pads, Phase, controls};
use crate::twin_view::{self, SecondEye};
use crate::{Args, Orbit};

/// Windows narrower than this (width over height) split.
const TALL: f32 = 0.8;

pub fn plugin(app: &mut App) {
    app.add_plugins(crate::twin_view::plugin);
    app.init_resource::<Split>()
        .init_resource::<StickTurn>()
        .init_resource::<TeamCams>()
        .add_systems(Startup, load)
        .add_systems(Update, (press.run_if(|w: &World| !super::front_end::paused(w) && !super::intro::running(w)), split.after(super::camera).before(super::draw)))
        // as `own_camera::step`: on the scene the tick starts from
        .add_systems(FixedUpdate, (step_teams, step).chain().after(super::control).before(super::simulate));
}

/// The picture is split now (the score panel puts the near team's half on top, `panel`).
static ON: AtomicBool = AtomicBool::new(false);

pub(super) fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// B635: whether player `i`'s stick is turned round on top of `own_camera::flipped` while the picture is split: its
/// half looks from behind its own end, and the stick steers by the shared camera's (a local human on the far team
/// when humans play on both teams; online each machine's camera is already behind its player). The input turns the
/// raw stick before it's recorded (`tick_input`), so replays and the tick hash stay as played.
pub(super) fn turned(g: &Game, i: usize) -> bool {
    on() && g.players.get(i).is_some_and(|p| (p.end < 0.0) != own_camera::flipped(g, i))
}

/// The bottom half's camera.
#[derive(Component)]
struct SplitCam;

#[derive(Resource, Default)]
struct Split {
    cam: Option<Entity>,
}

/// Whether the picture splits now: a tall window, live play on the plain match view, the remaster's own look.
fn live(w: &World, aspect: f32) -> bool {
    aspect < TALL && plain_play(w)
}

/// Live play on the plain match view, the remaster's own look.
fn plain_play(w: &World) -> bool {
    let Some(g) = w.get_resource::<Game>() else { return false };
    g.players.len() >= 2
        && matches!(g.phase, Phase::Serve | Phase::Rally)
        && !w.get_resource::<crate::graphics::Graphics>().is_some_and(|gr| gr.original)
        && !super::intro::running(w)
        && !super::instant_replay::playing(w)
        && !w.get_resource::<super::cutaway::Director>().is_some_and(|d| d.showing())
        && w.get_resource::<OwnCam>().is_none_or(|o| o.plain_view())
}

pub(super) fn split(w: &mut World) {
    let aspect = w.query_filtered::<&Window, With<PrimaryWindow>>().single(w).ok().map_or(1.0, crate::play::widescreen::scene_aspect);
    let own = w.get_resource::<Personal>().is_some_and(|p| p.showing) && plain_play(w);
    let on = (own || live(w, aspect)) && w.get_resource::<TeamCams>().is_some_and(|t| t.0.len() == 2);
    // M146: side by side, the score panels stay as they are
    ON.store(on && !(own && aspect >= 1.0), Ordering::Relaxed);
    let mut state = w.remove_resource::<Split>().unwrap_or_default();
    if own {
        // 1P left (or on top), 2P right (or below)
        place(w, &mut state, aspect, aspect >= 1.0, [0, 1]);
    } else if on {
        // B637: the team at the end the match camera looks from on top, the other below
        let g = w.resource::<Game>();
        let eye = own_camera::shown(g, w.get_resource::<OwnCam>()).1.eye[2];
        let top = if g.players[0].pos[2] * eye > 0.0 { 0 } else { 1 };
        place(w, &mut state, aspect, false, [top, 1 - top]);
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

/// Each team's own match camera (team 0's first), the original's one-human camera behind that team's end as a
/// singles match with one human there shows it, and the view it showed the tick before. Stepped every tick (cheap)
/// so a split shows it settled however it comes on.
#[derive(Resource, Default)]
struct TeamCams(Vec<(sim_cam::Camera, View)>);

/// M146: per seat (1P..4P), whether the stick and d-pad turn round as they're read (`turn_sticks`).
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
pub(super) struct StickTurn(pub(super) [bool; 4]);

/// `split.txt` in the per-user data folder.
fn path(iso: &str) -> std::path::PathBuf {
    crate::store::path(iso, "split.txt")
}

fn load(mut commands: Commands, args: Res<Args>) {
    // `HST_SPLIT2=on`: on from the start (shots)
    let on = crate::store::read_to_string(path(&args.iso)).is_ok_and(|t| t.trim() == "split")
        || std::env::var("HST_SPLIT2").is_ok_and(|v| v == "on");
    commands.insert_resource(Personal { on, ..default() });
}

fn save(iso: &str, on: bool) {
    // tests never write beside the disc
    if cfg!(test) {
        return;
    }
    if let Err(e) = crate::store::write(path(iso), if on { "split\n" } else { "shared\n" }) {
        warn!("split.txt: {e}");
    }
}

/// The personal split can play: singles, both players human on this device, not online, a phone or tablet
/// (`HST_SPLIT2` set: any machine; `on` also switches it on), the remaster's own look.
fn eligible(g: &Game, pads: &Pads, online: bool, original: bool) -> bool {
    let device = matches!(crate::device::PLATFORM, crate::device::Platform::Android | crate::device::Platform::Ios)
        || cfg!(test)
        || std::env::var_os("HST_SPLIT2").is_some();
    device && !online && !original && g.players.len() == 2 && (0..2).all(|i| pads.slot_of(i, 2).is_some())
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
    args: Res<Args>,
) {
    let (Some(mut p), Some(g), Some(pads)) = (p, g, pads) else { return };
    let press = std::mem::take(&mut p.press);
    if !eligible(&g, &pads, np.is_some(), gr.is_some_and(|gr| gr.original)) {
        p.showing = false;
        *turn = StickTurn::default();
        return;
    }
    let live = matches!(g.phase, Phase::Serve | Phase::Rally) && !d.is_some_and(|d| d.showing()) && !g.instant.replaying;
    if press && live {
        p.on = !p.on;
        save(&args.iso, p.on);
        info!("split view {}", if p.on { "on" } else { "off" });
    }
    p.showing = p.on;
    if !p.on {
        *turn = StickTurn::default();
        return;
    }
    let mut t = StickTurn::default();
    for i in 0..2 {
        if let Some(s) = pads.slot_of(i, 2) {
            t.0[s] = (g.players[i].end < 0.0) != own_camera::flipped(&g, i);
        }
    }
    if *turn != t {
        *turn = t;
    }
}

/// Steps each team's own camera on the scene the tick starts from (as `own_camera::step`).
fn step_teams(g: Option<Res<Game>>, mut t: ResMut<TeamCams>) {
    let Some(g) = g.filter(|g| g.players.len() >= 2) else {
        t.0.clear();
        return;
    };
    let fresh = t.0.is_empty();
    if fresh {
        t.0 = (0..2).map(|_| (sim_cam::Camera::from_view(g.cam.view), g.cam.view)).collect();
    }
    let players: Vec<_> = g.players.iter().map(|p| p.pos).collect();
    let scene = Scene { players: &players, ball: g.flight.ball.pos };
    for (team, (cam, prev)) in t.0.iter_mut().enumerate() {
        let turned = g.players[team].end < 0.0;
        // a new point's placement (the shared camera's cut), its end changed, or just started: cut straight to it
        let cut = fresh || g.cam_cut || cam.turned != turned;
        if cut {
            cam.cut();
            cam.turned = turned;
        }
        *prev = cam.view;
        cam.step(&scene);
        if cut {
            *prev = cam.view;
        }
    }
}

/// Shows team `order[0]`'s own camera on the first half (left, or top) and `order[1]`'s on the second.
fn place(w: &mut World, state: &mut Split, aspect: f32, wide: bool, order: [usize; 2]) {
    let a = w.resource::<Time<Fixed>>().overstep_fraction();
    let Some(target) = target(w) else { return };
    let half = if wide { aspect / 2.0 } else { aspect * 2.0 };
    let views = if wide {
        let x = target.x / 2;
        [(UVec2::ZERO, UVec2::new(x, target.y)), (UVec2::new(x, 0), UVec2::new(target.x - x, target.y))]
    } else {
        let y = target.y / 2;
        [(UVec2::ZERO, UVec2::new(target.x, y)), (UVec2::new(0, y), UVec2::new(target.x, target.y - y))]
    };
    let t = w.resource::<TeamCams>();
    let eye = |i: usize| {
        let (cam, v0) = &t.0[i];
        let v = cam.view;
        let mix = |x: [f32; 3], y: [f32; 3]| Vec3::from(x).lerp(Vec3::from(y), a).to_array();
        let (eye, pitch, yaw) = super::orbit_from(mix(v0.eye, v.eye), mix(v0.rot[2], v.rot[2]));
        let fov = v0.fov + (v.fov - v0.fov) * a;
        // as the match camera's: never less than the game's 4:3 picture across
        let fov = 2.0 * (fov.tan() * super::SHOWN_ASPECT.max(1.0 / half)).atan();
        (Transform::from_translation(eye).with_rotation(Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)), fov)
    };
    let eyes = order.map(eye);
    show(w, state, eyes, views, half);
}

/// M146: turns this tick's local pads round for the seats `turn` says (stick and d-pad), before they're recorded.
pub(super) fn turn_sticks(pads: &mut [hst_sim::replay::Pad; 4], turn: Option<&StickTurn>) {
    use hst_sim::player::pad::{DOWN, LEFT, RIGHT, UP};
    let Some(turn) = turn else { return };
    let mirror = |b: u8| {
        let v = b as f32 - 128.0;
        super::stick_byte(-v / if v < 0.0 { 128.0 } else { 127.0 })
    };
    for (p, _) in pads.iter_mut().zip(turn.0).filter(|(_, t)| *t) {
        (p.lx, p.ly) = (mirror(p.lx), mirror(p.ly));
        let b = p.buttons;
        let swap = |from: u16, to: u16| if b & from != 0 { to } else { 0 };
        p.buttons = b & !(UP | DOWN | LEFT | RIGHT) | swap(UP, DOWN) | swap(DOWN, UP) | swap(LEFT, RIGHT) | swap(RIGHT, LEFT);
    }
}

/// The main camera's layers while split: the scene's and its own billboards' (`twin_view::TOP`).
fn top_layers() -> RenderLayers {
    RenderLayers::from_layers(&[0, twin_view::TOP])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M146: a turned seat's stick runs the other way (within the pad bytes' rounding), its d-pad too; the others are
    /// left alone.
    #[test]
    fn a_turned_stick_runs_the_other_way() {
        use hst_sim::player::{pad, pad_dir};
        use hst_sim::replay::Pad;
        let turn = StickTurn([false, true, false, false]);
        for lx in (0..=255u8).step_by(5) {
            for ly in (0..=255u8).step_by(5) {
                let p = Pad { buttons: 0, lx, ly, rx: 0x80, ry: 0x80 };
                let mut pads = [p; 4];
                turn_sticks(&mut pads, Some(&turn));
                assert_eq!(pads[0], p);
                let ([x, z], [tx, tz]) = (pad_dir(0, lx, ly, 3), pad_dir(0, pads[1].lx, pads[1].ly, 3));
                assert!((x + tx).abs() < 0.02 && (z + tz).abs() < 0.02, "{lx:02x},{ly:02x}: {x},{z} turned {tx},{tz}");
            }
        }
        let mut pads = [Pad { buttons: pad::UP | pad::LEFT | 0x2000, lx: 0x80, ly: 0x80, rx: 0x80, ry: 0x80 }; 4];
        turn_sticks(&mut pads, Some(&turn));
        assert_eq!(pads[1].buttons, pad::DOWN | pad::RIGHT | 0x2000);
        assert_eq!(pads[0].buttons, pad::UP | pad::LEFT | 0x2000);
    }

    /// M146: 2P singles with the personal split on: each player's camera looks from behind them, the stick of the one
    /// the shared camera has at the far end turns, and the match plays exactly as without it (skipped without the
    /// ISO).
    #[test]
    fn each_human_has_their_own_camera() {
        use super::super::*;
        if !hst_data::fixture::has_iso() {
            return;
        }
        let app = |split: bool| match_app(split, true);
        let (mut a, mut b) = (app(true), app(false));
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
                let eye = p.0[i].0.view.eye;
                assert!(eye[2] * z > 0.0 && eye[2].abs() > z.abs(), "tick {tick}: player {i}'s eye {eye:?}, at z {z}");
                // turned when its own picture's end isn't the shared camera's
                let shared = g.cam.view.eye[2] * z > 0.0;
                assert_eq!(w.resource::<super::StickTurn>().0[i], !shared, "tick {tick}: player {i}'s stick");
            }
            assert_eq!(w.resource::<super::StickTurn>().0.iter().filter(|t| **t).count(), 1);
        }
    }

    /// A match app (singles: both human, doubles: all CPUs), this module's plugin added or not.
    fn match_app(split: bool, singles: bool) -> App {
        use super::super::*;
        use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
        app.insert_resource(Args { iso: hst_data::fixture::ISO_PATH.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles, chars: if singles { vec![6, 1] } else { vec![6, 1, 2, 3] }, outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
        app.insert_resource(Pads { slots: default(), connected: 0, seats: Some([singles, singles, false, false]), autoplay: false });
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

    /// B637: in doubles each team's own camera looks from behind that team's end, both its players in front of the
    /// eye, and stepping them changes nothing in the match (skipped without the ISO).
    #[test]
    fn each_team_has_its_own_camera() {
        use super::super::*;
        if !hst_data::fixture::has_iso() {
            return;
        }
        let (mut a, mut b) = (match_app(true, false), match_app(false, false));
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
            assert!(!w.resource::<super::Personal>().showing);
            for team in 0..2 {
                let (cam, _) = t.0[team];
                assert_eq!(cam.turned, g.players[team].end < 0.0);
                let eye = cam.view.eye;
                for p in g.players.iter().skip(team).step_by(2) {
                    assert!(eye[2] * p.pos[2] > 0.0 && eye[2].abs() > p.pos[2].abs(), "tick {tick}: team {team}'s eye {eye:?}, a player at {:?}", p.pos);
                }
            }
        }
    }
}
