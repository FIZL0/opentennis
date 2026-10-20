//! Third-person ball cam for player 0 (`HST_THIRD_PERSON=1`, a fun branch): the camera follows behind the player and
//! turns toward the ball; the mouse or right stick looks, the stick and keys move relative to the camera, in any
//! direction. ponytail: player 0 only and local only (netplay would desync on the turned stick).

use std::f32::consts::{PI, TAU};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use super::{Figure, Game};
use crate::Orbit;

pub fn on() -> bool {
    static ON: LazyLock<bool> = LazyLock::new(|| std::env::var_os("HST_THIRD_PERSON").is_some());
    *ON
}

/// The look's Bevy yaw (0 looks along −z) and pitch, as f32 bits: `pad_run` reads the yaw from the sim's tick.
static YAW: AtomicU32 = AtomicU32::new(0);
static PITCH: AtomicU32 = AtomicU32::new(0);
/// Shift held: the keys walk.
static WALK: AtomicBool = AtomicBool::new(false);

const SENSITIVITY: f32 = 0.0025;
/// The left stick's round deadzone and the tilt past which it runs flat out; a lighter push walks. The keys walk at
/// `WALK_SPEED` with Shift held.
const STICK_DEAD: f32 = 0.15;
const STICK_FULL: f32 = 0.85;
const WALK_SPEED: f32 = 0.35;
const FOV: f32 = 75.0;
/// The right stick's full-tilt turn, radians per second.
const STICK_TURN: f32 = 3.5;
/// The camera: pivot height over the feet, distance behind, resting pitch (looking down), and how much farther
/// back it goes per metre the ball is over the pivot and when the ball is straight behind the player.
const HEIGHT: f32 = 1.3;
const DIST: f32 = 4.5;
const PITCH_REST: f32 = -0.3;
const HIGH_ZOOM: f32 = 0.8;
const BEHIND_ZOOM: f32 = 4.0;
/// The focus leads this fraction of the way toward the ball, at most `LEAD_MAX` metres.
const LEAD: f32 = 0.25;
const LEAD_MAX: f32 = 3.0;
/// Eases (per second): the focus, the zoom, and the ball cam's turn.
const FOCUS_EASE: f32 = 5.0;
const ZOOM_EASE: f32 = 2.0;
const LOCK_EASE: f32 = 3.0;
/// The ball cam lets the ball wander this far (yaw, pitch radians) before it turns, and doesn't turn for a ball
/// within this many metres (across the court) of the player.
const LOCK_FREE: Vec2 = Vec2::new(0.15, 0.1);
const LOCK_NEAR: f32 = 1.5;

pub fn plugin(app: &mut App) {
    if on() {
        app.add_systems(Update, look.before(super::camera))
            .add_systems(
                PostUpdate,
                view.after(bevy::transform::TransformSystems::Propagate),
            )
            .init_resource::<Pivot>();
    }
}

/// The left stick with a round deadzone instead of the original's per-axis one (which snaps a near-axis push onto the
/// axis), its tilt kept as the speed: any angle, and a light push walks.
pub fn stick(v: Vec2) -> Vec2 {
    let len = v.length();
    if len <= STICK_DEAD {
        return Vec2::ZERO;
    }
    v / len * ((len - STICK_DEAD) / (STICK_FULL - STICK_DEAD)).min(1.0)
}

/// Player 0's run: the stick (right, forward), else the keys' `keys`, turned to the look, in game space (x, z); its
/// length (≤ 1) is the speed.
pub fn run(stick: Vec2, keys: Vec2) -> Vec2 {
    let v = if stick != Vec2::ZERO {
        stick
    } else if WALK.load(Relaxed) {
        keys * WALK_SPEED
    } else {
        keys
    };
    let (s, c) = f32::from_bits(YAW.load(Relaxed)).sin_cos();
    Vec2::new(c * v.x - s * v.y, s * v.x + c * v.y)
}

/// First person's d-pad just pressed on pad `g`: its emote (up, down, left, right as the right stick's flicks).
pub fn dpad_emote(g: &Gamepad) -> Option<u8> {
    use GamepadButton::*;
    if !on() {
        return None;
    }
    [DPadUp, DPadDown, DPadLeft, DPadRight]
        .into_iter()
        .position(|b| g.just_pressed(b))
        .map(|d| d as u8)
}

/// Mouse or right-stick look, the cursor held in the window; the look starts toward the other end and again after a
/// change of ends. Ball cam (on by default; right-stick click or middle mouse toggles it) turns the camera from behind
/// the player toward the ball, easing, and lets the ball wander a little on the screen rather than pinning it.
fn look(
    g: Res<Game>,
    mouse: Res<AccumulatedMouseMotion>,
    pads: Query<&Gamepad>,
    time: Res<Time>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    pivot: Res<Pivot>,
    mut cursor: Query<&mut CursorOptions>,
    mut end: Local<f32>,
    mut free: Local<bool>,
) {
    WALK.store(
        keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        Relaxed,
    );
    if buttons.just_pressed(MouseButton::Middle)
        || pads
            .iter()
            .any(|p| p.just_pressed(GamepadButton::RightThumb))
    {
        *free = !*free;
    }
    if *end != g.players[0].end {
        *end = g.players[0].end;
        // facing game +z is Bevy −z
        YAW.store(if *end > 0.0 { 0.0f32 } else { PI }.to_bits(), Relaxed);
        PITCH.store(PITCH_REST.to_bits(), Relaxed);
    }
    for mut c in &mut cursor {
        c.grab_mode = CursorGrabMode::Locked;
        c.visible = false;
    }
    let stick: Vec2 = pads
        .iter()
        .map(|p| p.right_stick())
        .filter(|v| v.length() > 0.15)
        .sum();
    let d =
        mouse.delta * SENSITIVITY + Vec2::new(stick.x, -stick.y) * STICK_TURN * time.delta_secs();
    let mut yaw = f32::from_bits(YAW.load(Relaxed)) - d.x;
    let mut pitch = f32::from_bits(PITCH.load(Relaxed)) - d.y;
    if !*free {
        let [x, y, z] = g.flight.ball.pos;
        let to = Vec3::new(x, -y, -z) - pivot.0;
        let ease = |e: f32, free: f32| {
            e.signum() * (e.abs() - free).max(0.0) * (LOCK_EASE * time.delta_secs()).min(1.0)
        };
        // the ball right over the player (a toss, a lob coming down on it): no direction to turn to
        if to.xz().length() > LOCK_NEAR {
            let want = (-to.x).atan2(-to.z);
            yaw += ease((want - yaw + PI).rem_euclid(TAU) - PI, LOCK_FREE.x);
        }
        // tilt up only part of the way to a high ball: the zoom out does the rest
        let want = PITCH_REST + to.y.atan2(to.xz().length()).max(0.0) * 0.4;
        pitch += ease(want - pitch, LOCK_FREE.y);
    }
    YAW.store(yaw.to_bits(), Relaxed);
    PITCH.store(pitch.clamp(-1.3, 0.6).to_bits(), Relaxed);
}

/// The camera: behind player 0 along the look, a little above, its focus eased toward the player and led a little
/// toward the ball; it pulls back when the ball goes high or gets behind the player, and never dips under the court.
fn view(
    g: Res<Game>,
    time: Res<Time>,
    figures: Query<(&Figure, &GlobalTransform), Without<Camera3d>>,
    mut q: Query<
        (
            &mut Orbit,
            &mut Projection,
            &mut Transform,
            &mut GlobalTransform,
        ),
        With<Camera3d>,
    >,
    mut pivot: ResMut<Pivot>,
    mut zoom: Local<Option<(Vec3, f32)>>,
) {
    let Ok((mut o, mut proj, mut tf, mut global)) = q.single_mut() else {
        return;
    };
    let Some(player) = figures
        .iter()
        .find(|(f, _)| f.0 == 0)
        .map(|(_, t)| t.translation() + Vec3::Y * HEIGHT)
    else {
        return;
    };
    pivot.0 = player;
    let dt = time.delta_secs();
    let [x, y, z] = g.flight.ball.pos;
    let to = Vec3::new(x, -y, -z) - player;
    o.yaw = f32::from_bits(YAW.load(Relaxed));
    let fwd = Vec3::new(-o.yaw.sin(), 0.0, -o.yaw.cos());
    let behind = (-fwd.dot(to.with_y(0.0).normalize_or_zero())).max(0.0);
    let high = (to.y - 1.0).max(0.0);
    let (want_focus, want_radius) = (
        player + to.clamp_length_max(LEAD_MAX) * LEAD,
        DIST + high * HIGH_ZOOM + behind * BEHIND_ZOOM,
    );
    let (focus, radius) = zoom.get_or_insert((want_focus, want_radius));
    *focus = focus.lerp(want_focus, (FOCUS_EASE * dt).min(1.0));
    *radius += (want_radius - *radius) * (ZOOM_EASE * dt).min(1.0);
    // looking up lowers the camera behind the focus: keep it over the court
    let lowest = ((focus.y - 0.4) / *radius).clamp(-1.0, 1.0).asin();
    o.pitch = f32::from_bits(PITCH.load(Relaxed)).min(lowest);
    (o.focus, o.radius) = (*focus, *radius);
    *tf = o.transform();
    *global = GlobalTransform::from(*tf);
    if let Projection::Perspective(p) = &mut *proj {
        p.fov = FOV.to_radians();
    }
}

/// Player 0's middle, where the camera pivots (the ball cam aims from it).
#[derive(Resource, Default)]
struct Pivot(Vec3);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_keeps_any_angle_and_walks() {
        // 20° off forward: the original's per-axis deadzone would snap this to straight ahead
        let v = stick(Vec2::new(
            20f32.to_radians().sin(),
            20f32.to_radians().cos(),
        ));
        assert!(
            (v.x.atan2(v.y).to_degrees() - 20.0).abs() < 1e-3 && (v.length() - 1.0).abs() < 1e-6
        );
        assert!((stick(Vec2::Y * 0.5).length() - 0.5).abs() < 1e-6);
        assert_eq!(stick(Vec2::X * 0.1), Vec2::ZERO);
    }
}
