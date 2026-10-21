//! B633: a tall window (a phone held upright) shows the match camera's whole court as a thin strip with the players
//! small. While the match plays (serve and rally, the plain match view on screen) such a window splits instead: the
//! team at the end the match camera looks from on the top half, the other team on the bottom half, each half its own
//! camera low behind that team's end, framing its players (two side by side in doubles, a 2×2 grid of players in all)
//! and following the ball across when it's on their side. Everything else (the intro, cut-aways, calls, replays, the
//! change of ends, the alternate camera) shows the one picture as before.
//!
//! Presentation only: the match camera (`Game::cam`) and this machine's own (`own_camera`) step as before and nothing
//! in play reads these views; the main camera takes the top half's view after `camera` has placed it, so the
//! billboards and the eye-hiding passes that follow the main camera follow the top half.
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
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::sync::atomic::{AtomicBool, Ordering};

use hst_sim::camera::{self as sim_cam, Scene, View};

use super::own_camera::{self, OwnCam};
use super::{Game, Pads, Phase, controls};
use crate::{Args, Orbit};

/// Windows narrower than this (width over height) split.
const TALL: f32 = 0.8;
/// The halves' cameras: how far they look down, the narrowest horizontal half-angle, the widest vertical one.
const PITCH: f32 = 28.0 * std::f32::consts::PI / 180.0;
const NARROWEST: f32 = 22.0 * std::f32::consts::PI / 180.0;
const WIDEST: f32 = 50.0 * std::f32::consts::PI / 180.0;
/// The eye never goes further from the net than this (m): back fences stand a little past it, and only the main
/// camera's eye has the props around it hidden (`net_hide`).
const BACK: f32 = 17.5;
/// The eye's nearest to the view's centre (m): a lone player isn't shown closer.
const NEAREST: f32 = 7.0;
/// The view's centre: this far ahead of the team toward the net (m), at this height.
const AHEAD: f32 = 2.0;
const HEIGHT: f32 = 1.0;
/// Room kept beside the outermost player (or the ball) across the picture (m), and their heads' height.
const SIDE: f32 = 1.6;
const HEAD: f32 = 1.8;
/// Where the deepest player's feet stand: this far below the half's centre (of its half-height).
const FEET: f32 = 0.7;
/// How quickly a half follows its team (per second), and a jump that cuts instead (a new point's set-up).
const FOLLOW: f32 = 5.0;
const CUT: f32 = 3.0;

pub fn plugin(app: &mut App) {
    app.init_resource::<Split>()
        .init_resource::<StickTurn>()
        .add_systems(Startup, load)
        .add_systems(Update, (press.run_if(|w: &World| !super::front_end::paused(w) && !super::intro::running(w)), split.after(super::camera).before(super::draw)))
        // as `own_camera::step`: on the scene the tick starts from
        .add_systems(FixedUpdate, step.after(super::control).before(super::simulate));
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
    /// Shown last frame: the team on top, each half's rig.
    shown: Option<(usize, [Rig; 2])>,
}

/// One half's camera: where it looks, from how far back, how far down, and its horizontal and vertical half-angles.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rig {
    look: Vec3,
    dist: f32,
    pitch: f32,
    half: f32,
    vertical: f32,
    /// The team's side, Bevy +z or −z.
    side: f32,
}

impl Rig {
    fn forward(&self) -> Vec3 {
        Vec3::new(0.0, -self.pitch.sin(), -self.side * self.pitch.cos())
    }

    fn eye(&self) -> Vec3 {
        self.look - self.forward() * self.dist
    }

    fn transform(&self) -> Transform {
        Transform::from_translation(self.eye()).looking_to(self.forward(), Vec3::Y)
    }

    fn eased(self, to: Rig, k: f32) -> Rig {
        let l = |a: f32, b: f32| a + (b - a) * k;
        Rig {
            look: self.look.lerp(to.look, k),
            dist: l(self.dist, to.dist),
            pitch: l(self.pitch, to.pitch),
            half: l(self.half, to.half),
            vertical: l(self.vertical, to.vertical),
            side: to.side,
        }
    }
}

/// The rig framing `team` (players `team`, `team + 2`, …; Bevy space) in a half `aspect` wide per unit of height, the
/// ball's x kept in when it's on their side; `covered`: the share of the half's height at its bottom under the touch
/// pad's buttons, the players kept above it.
fn frame(players: &[Vec3], team: usize, ball: Vec3, aspect: f32, covered: f32) -> Option<Rig> {
    let mine: Vec<Vec3> = players.iter().copied().skip(team).step_by(2).collect();
    let first = *mine.first()?;
    let z = mine.iter().map(|p| p.z).sum::<f32>();
    let side = if z != 0.0 { z.signum() } else { first.z.signum() };
    let depth = |p: &Vec3| (p.z * side).max(0.0);
    let (near, deep) = mine.iter().map(depth).fold((f32::MAX, 0.0f32), |(a, b), d| (a.min(d), b.max(d)));
    let mut xs: Vec<f32> = mine.iter().map(|p| p.x).collect();
    if ball.z * side > 0.0 && ball.z * side < deep + 3.0 {
        xs.push(ball.x);
    }
    let (lo, hi) = xs.iter().fold((f32::MAX, f32::MIN), |(a, b), &x| (a.min(x), b.max(x)));
    let centre = ((lo + hi) / 2.0).clamp(-4.0, 4.0);
    let width = (hi - centre).max(centre - lo) + SIDE;
    let look = Vec3::new(centre, HEIGHT, side * ((near + deep) / 2.0 - AHEAD));
    let pitch = PITCH;
    let far = ((BACK - look.z * side) / pitch.cos()).max(NEAREST);
    let dist = (width / NARROWEST.tan()).clamp(NEAREST, far);
    let mut rig = Rig { look, dist, pitch, half: (width / dist).atan().max(NARROWEST), vertical: 0.0, side };
    // tall enough for the across fit, and for the lowest feet to stand at `FEET` of the part of the half left
    // uncovered, and every head inside it
    let (eye, fwd) = (rig.eye(), rig.forward());
    let up = (Vec3::X * side).cross(fwd).normalize();
    let tan = |p: Vec3| {
        let d = p - eye;
        d.dot(up) / d.dot(fwd).max(0.5)
    };
    let feet = mine.iter().map(|&p| -tan(p)).fold(0.0f32, f32::max);
    let head = mine.iter().map(|&p| tan(p + Vec3::Y * HEAD)).fold(0.0f32, f32::max);
    let at = (1.0 - (1.0 + FEET) * (1.0 - covered)).min(-0.3).abs();
    let need = (feet / at).max(head / 0.9);
    rig.vertical = (rig.half.tan() / aspect.max(0.1)).atan().max(need.atan()).min(WIDEST);
    rig.half = rig.half.max((rig.vertical.tan() * aspect).atan());
    Some(rig)
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

fn split(w: &mut World) {
    // the window's size (logical px) and the picture's width over height
    let win = w.query_filtered::<&Window, With<PrimaryWindow>>().single(w).ok().map(|win| (Vec2::new(win.width(), win.height()), crate::play::widescreen::scene_aspect(win)));
    let (size, aspect) = win.unwrap_or((Vec2::ONE, 1.0));
    let own = w.get_resource::<Personal>().is_some_and(|p| p.cams.len() == 2) && plain_play(w);
    let on = own || live(w, aspect);
    // M146: side by side, the score panels stay as they are
    ON.store(on && !(own && aspect >= 1.0), Ordering::Relaxed);
    let mut state = w.remove_resource::<Split>().unwrap_or_default();
    if own {
        state.shown = None;
        place_own(w, &mut state, aspect);
    } else if on {
        place(w, &mut state, size, aspect);
    } else if state.shown.take().is_some() || state.cam.is_some() {
        if let Some(e) = state.cam.take() {
            w.despawn(e);
        }
        let mut q = w.query_filtered::<&mut Camera, With<Orbit>>();
        if let Ok(mut cam) = q.single_mut(w) {
            cam.viewport = None;
        }
    }
    w.insert_resource(state);
}

fn place(w: &mut World, state: &mut Split, size: Vec2, aspect: f32) {
    // each half's
    let aspect = aspect * 2.0;
    // the bottom half keeps its players above the touch pad's face buttons
    let pad = super::touch_pad::face_top(size).map_or(0.0, |top| ((size.y - top) / (size.y / 2.0).max(1.0)).clamp(0.0, 0.6));
    let (a, dt) = (w.resource::<Time<Fixed>>().overstep_fraction(), w.resource::<Time>().delta_secs());
    let g = w.resource::<Game>();
    let players: Vec<Vec3> = g.players.iter().map(|p| bevy_space(Vec3::from(p.prev).lerp(Vec3::from(p.pos), a))).collect();
    let ball = bevy_space(Vec3::from(g.prev_ball).lerp(Vec3::from(g.flight.ball.pos), a));
    // the near team: at the end the match camera looks from (game space)
    let eye = own_camera::shown(g, w.get_resource::<OwnCam>()).1.eye[2];
    let top = if g.players[0].pos[2] * eye > 0.0 { 0 } else { 1 };
    let (Some(up), Some(down)) = (frame(&players, top, ball, aspect, 0.0), frame(&players, 1 - top, ball, aspect, pad)) else { return };
    let k = 1.0 - (-FOLLOW * dt).exp();
    let rigs = match state.shown {
        Some((t, [u, d])) if t == top && u.look.distance(up.look) < CUT && d.look.distance(down.look) < CUT => [u.eased(up, k), d.eased(down, k)],
        _ => [up, down],
    };
    state.shown = Some((top, rigs));
    let Some(target) = target(w) else { return };
    let half = target.y / 2;
    let views = [(UVec2::ZERO, UVec2::new(target.x, half)), (UVec2::new(0, half), UVec2::new(target.x, target.y - half))];
    show(w, state, rigs.map(|r| (r.transform(), 2.0 * r.vertical)), views);
}

/// The main camera's picture size (physical px).
fn target(w: &mut World) -> Option<UVec2> {
    let mut q = w.query_filtered::<&Camera, With<Orbit>>();
    q.single(w).ok()?.physical_target_size()
}

/// Puts the main camera (`Orbit`) and the second one at the halves' eyes, each with its vertical field of view and
/// its part of the picture (position, size; physical px).
fn show(w: &mut World, state: &mut Split, eyes: [(Transform, f32); 2], views: [(UVec2, UVec2); 2]) {
    let cam = *state.cam.get_or_insert_with(|| {
        w.spawn((
            SplitCam,
            Camera3d::default(),
            // the main camera clears the whole picture first (`hud_gamma` leaves a camera that clears nothing as it is)
            Camera { order: 2, clear_color: ClearColorConfig::None, ..default() },
            bevy::core_pipeline::tonemapping::Tonemapping::None,
            Projection::Perspective(PerspectiveProjection { far: 20_000.0, ..default() }),
            Transform::default(),
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
    let mut q = w.query_filtered::<(&mut Transform, &mut Projection, &mut Camera), With<Orbit>>();
    let Ok((mut tf, mut proj, mut main)) = q.single_mut(w) else { return };
    set(eyes[0], views[0], &mut tf, &mut proj, &mut main);
    let mut q = w.query_filtered::<(&mut Transform, &mut Projection, &mut Camera), With<SplitCam>>();
    if let Ok((mut tf, mut proj, mut c)) = q.get_mut(w, cam) {
        set(eyes[1], views[1], &mut tf, &mut proj, &mut c);
    }
}

/// M146: the personal split's state.
#[derive(Resource, Default)]
pub(super) struct Personal {
    /// Chosen (1P's Select).
    on: bool,
    /// A Select press waiting for the next tick.
    press: bool,
    /// While it plays, each player's own match camera and the view it showed the tick before.
    cams: Vec<(sim_cam::Camera, View)>,
}

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

/// Switches on Select in live play (the serve or the rally, nothing else on screen), steps each player's camera,
/// and sets which sticks turn: those whose own picture looks from the other end than the one their stick follows
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
        p.cams.clear();
        *turn = StickTurn::default();
        return;
    }
    let live = matches!(g.phase, Phase::Serve | Phase::Rally) && !d.is_some_and(|d| d.showing()) && !g.instant.replaying;
    if press && live {
        p.on = !p.on;
        save(&args.iso, p.on);
        info!("split view {}", if p.on { "on" } else { "off" });
    }
    if !p.on {
        p.cams.clear();
        *turn = StickTurn::default();
        return;
    }
    let fresh = p.cams.is_empty();
    if fresh {
        p.cams = (0..2).map(|_| (sim_cam::Camera::from_view(g.cam.view), g.cam.view)).collect();
    }
    let players: Vec<_> = g.players.iter().map(|p| p.pos).collect();
    let scene = Scene { players: &players, ball: g.flight.ball.pos };
    for (i, (cam, prev)) in p.cams.iter_mut().enumerate() {
        let turned = g.players[i].end < 0.0;
        // a new point's placement (the shared camera's cut), its end changed, or just switched on: cut straight to it
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

/// M146: each player's own camera on their half: 1P left (or on top), 2P right (or below).
fn place_own(w: &mut World, state: &mut Split, aspect: f32) {
    let a = w.resource::<Time<Fixed>>().overstep_fraction();
    let Some(target) = target(w) else { return };
    let wide = aspect >= 1.0;
    let half = if wide { aspect / 2.0 } else { aspect * 2.0 };
    let views = if wide {
        let x = target.x / 2;
        [(UVec2::ZERO, UVec2::new(x, target.y)), (UVec2::new(x, 0), UVec2::new(target.x - x, target.y))]
    } else {
        let y = target.y / 2;
        [(UVec2::ZERO, UVec2::new(target.x, y)), (UVec2::new(0, y), UVec2::new(target.x, target.y - y))]
    };
    let p = w.resource::<Personal>();
    let eye = |i: usize| {
        let (cam, v0) = &p.cams[i];
        let v = cam.view;
        let mix = |x: [f32; 3], y: [f32; 3]| Vec3::from(x).lerp(Vec3::from(y), a).to_array();
        let (eye, pitch, yaw) = super::orbit_from(mix(v0.eye, v.eye), mix(v0.rot[2], v.rot[2]));
        let fov = v0.fov + (v.fov - v0.fov) * a;
        // as the match camera's: never less than the game's 4:3 picture across
        let fov = 2.0 * (fov.tan() * super::SHOWN_ASPECT.max(1.0 / half)).atan();
        (Transform::from_translation(eye).with_rotation(Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)), fov)
    };
    let eyes = [eye(0), eye(1)];
    show(w, state, eyes, views);
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

/// Game space (Y down) → Bevy's.
fn bevy_space(p: Vec3) -> Vec3 {
    Vec3::new(p.x, -p.y, -p.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Doubles at the baselines: each half looks from behind its own team toward the net, its eye no further back
    /// than the fences, both players and their heads inside the picture.
    #[test]
    fn each_half_frames_its_team_from_behind() {
        // Bevy space: team 0 at +z, team 1 at −z
        let players = [Vec3::new(-3.0, 0.0, 11.5), Vec3::new(2.5, 0.0, -11.0), Vec3::new(2.0, 0.0, 8.0), Vec3::new(-1.0, 0.0, -6.0)];
        let ball = Vec3::new(1.0, 1.0, 4.0);
        for (team, side) in [(0, 1.0), (1, -1.0)] {
            let rig = frame(&players, team, ball, 0.92, 0.0).unwrap();
            assert_eq!(rig.side, side);
            let (eye, tf) = (rig.eye(), rig.transform());
            assert!(eye.z * side > 11.0 && eye.z * side <= BACK + 1e-3, "team {team}: eye {eye}");
            assert!(tf.forward().z * side < 0.0, "team {team} looks toward the net");
            for p in players.iter().skip(team).step_by(2) {
                for q in [*p, *p + Vec3::Y * 1.8] {
                    let d = tf.rotation.inverse() * (q - eye);
                    let (x, y) = (d.x / -d.z, d.y / -d.z);
                    assert!(x.abs() < rig.half.tan() && y.abs() < rig.vertical.tan(), "team {team}: {q} off the picture");
                }
            }
        }
    }

    /// The deepest player's feet stand at `FEET` below the centre, and on a touch screen's bottom half that much of
    /// what the buttons leave uncovered, higher up.
    #[test]
    fn the_players_stand_above_the_buttons() {
        let players = [Vec3::new(0.0, 0.0, 11.0), Vec3::new(0.0, 0.0, -11.0)];
        let height = |covered| {
            let rig = frame(&players, 1, Vec3::ZERO, 0.9, covered).unwrap();
            let tf = rig.transform();
            let d = tf.rotation.inverse() * (players[1] - rig.eye());
            d.y / -d.z / rig.vertical.tan()
        };
        assert!((height(0.0) + FEET).abs() < 0.01, "{}", height(0.0));
        let up = height(0.25);
        assert!(up > height(0.0) + 0.2 && up > -1.0 + 0.5, "{up}");
    }

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
        use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
        if !hst_data::fixture::has_iso() {
            return;
        }
        let app = |split: bool| {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, AssetPlugin::default()));
            app.init_asset::<Mesh>().init_asset::<StandardMaterial>().init_asset::<Image>().init_asset::<SkinnedMeshInverseBindposes>();
            app.insert_resource(Args { iso: hst_data::fixture::ISO_PATH.into(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![6, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 4, pads: None });
            app.insert_resource(Pads { slots: default(), connected: 0, seats: Some([true, true, false, false]), autoplay: false });
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
        };
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
            let (g, p) = (w.resource::<Game>(), w.resource::<super::Personal>());
            assert_eq!(p.cams.len(), 2);
            for i in 0..2 {
                let z = g.players[i].pos[2];
                let eye = p.cams[i].0.view.eye;
                assert!(eye[2] * z > 0.0 && eye[2].abs() > z.abs(), "tick {tick}: player {i}'s eye {eye:?}, at z {z}");
                // turned when its own picture's end isn't the shared camera's
                let shared = g.cam.view.eye[2] * z > 0.0;
                assert_eq!(w.resource::<super::StickTurn>().0[i], !shared, "tick {tick}: player {i}'s stick");
            }
            assert_eq!(w.resource::<super::StickTurn>().0.iter().filter(|t| **t).count(), 1);
        }
    }
}
