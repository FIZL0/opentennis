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

use bevy::camera::Viewport;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::sync::atomic::{AtomicBool, Ordering};

use super::own_camera::{self, OwnCam};
use super::{Game, Phase};
use crate::Orbit;

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
    app.init_resource::<Split>().add_systems(Update, split.after(super::camera).before(super::draw));
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
    let Some(g) = w.get_resource::<Game>() else { return false };
    aspect < TALL
        && g.players.len() >= 2
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
    let on = live(w, aspect);
    ON.store(on, Ordering::Relaxed);
    let mut state = w.remove_resource::<Split>().unwrap_or_default();
    if on {
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
    let mut q = w.query_filtered::<(&mut Transform, &mut Projection, &mut Camera), With<Orbit>>();
    let Ok((mut tf, mut proj, mut main)) = q.single_mut(w) else { return };
    let Some(target) = main.physical_target_size() else { return };
    let half = target.y / 2;
    let views = [(UVec2::ZERO, UVec2::new(target.x, half)), (UVec2::new(0, half), UVec2::new(target.x, target.y - half))];
    let show = |rig: &Rig, (at, size): (UVec2, UVec2), tf: &mut Transform, proj: &mut Projection, cam: &mut Camera| {
        *tf = rig.transform();
        if let Projection::Perspective(p) = proj {
            p.fov = 2.0 * rig.vertical;
            p.near = 0.1;
        }
        let v = Some(Viewport { physical_position: at, physical_size: size.max(UVec2::ONE), ..default() });
        if cam.viewport.as_ref().map(|v| (v.physical_position, v.physical_size)) != v.as_ref().map(|v| (v.physical_position, v.physical_size)) {
            cam.viewport = v;
        }
    };
    show(&rigs[0], views[0], &mut tf, &mut proj, &mut main);
    let mut q = w.query_filtered::<(&mut Transform, &mut Projection, &mut Camera), With<SplitCam>>();
    if let Ok((mut tf, mut proj, mut c)) = q.get_mut(w, cam) {
        show(&rigs[1], views[1], &mut tf, &mut proj, &mut c);
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
}
