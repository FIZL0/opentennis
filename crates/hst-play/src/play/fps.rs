//! First-person view for player 0 (`HST_FIRST_PERSON=1`, a fun branch): the mouse looks, the stick and keys move relative to
//! where it looks, in any direction. The player's own head isn't drawn, but still casts its shadow: the camera's copy
//! of each body part has the head's joints shrunk to a point, and an untouched copy on render layer 1 (seen only by
//! the sun) casts the full shadow. ponytail: player 0 only and local only (netplay would desync on the turned stick).

use std::sync::LazyLock;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use super::{Figure, Game};
use crate::Orbit;

pub fn on() -> bool {
    static ON: LazyLock<bool> = LazyLock::new(|| std::env::var_os("HST_FIRST_PERSON").is_some());
    *ON
}

/// The look's Bevy yaw (0 looks along −z) and pitch, as f32 bits: `pad_run` reads the yaw from the sim's tick.
static YAW: AtomicU32 = AtomicU32::new(0);
static PITCH: AtomicU32 = AtomicU32::new(0);

const SENSITIVITY: f32 = 0.0025;
const FOV: f32 = 105.0;
/// The right stick's full-tilt turn, radians per second.
const STICK_TURN: f32 = 3.5;
/// The eye over the head joint, and ahead of it along the look.
const EYE_UP: f32 = 0.08;
const AHEAD: f32 = 0.12;
/// While serving, the eye goes this far back along the look and up.
const SERVE_BACK: Vec2 = Vec2::new(2.0, 0.5);

pub fn plugin(app: &mut App) {
    if on() {
        app.add_systems(
            Update,
            (
                look.before(super::camera),
                view.after(super::camera),
                headless,
                sun_sees_shadows,
            ),
        );
    }
}

/// The stick's (right, forward) turned to the look, in game space (x, z).
pub fn turned(right: f32, fwd: f32) -> Vec2 {
    let y = f32::from_bits(YAW.load(Relaxed));
    let (s, c) = y.sin_cos();
    Vec2::new(c * right - s * fwd, s * right + c * fwd)
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

/// Mouse or right-stick look, the cursor held in the window; the look starts toward the other end and again after a change of ends.
fn look(
    g: Res<Game>,
    mouse: Res<AccumulatedMouseMotion>,
    pads: Query<&Gamepad>,
    time: Res<Time>,
    mut cursor: Query<&mut CursorOptions>,
    mut end: Local<f32>,
) {
    if *end != g.players[0].end {
        *end = g.players[0].end;
        // facing game +z is Bevy −z
        YAW.store(
            if *end > 0.0 {
                0.0f32
            } else {
                std::f32::consts::PI
            }
            .to_bits(),
            Relaxed,
        );
        PITCH.store(0.0f32.to_bits(), Relaxed);
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
    let yaw = f32::from_bits(YAW.load(Relaxed)) - d.x;
    let pitch = (f32::from_bits(PITCH.load(Relaxed)) - d.y).clamp(-1.5, 1.5);
    YAW.store(yaw.to_bits(), Relaxed);
    PITCH.store(pitch.to_bits(), Relaxed);
}

/// The eye: on player 0's head as this frame's pose leaves it (after the transforms propagate, so it never trails the
/// body), a little ahead along the look; pulled back over the shoulder while it serves, to see the toss.
fn view(
    g: Res<Game>,
    time: Res<Time>,
    head: Option<Res<Head>>,
    joints: Query<&GlobalTransform, Without<Camera3d>>,
    mut q: Query<
        (
            Entity,
            &mut Orbit,
            &mut Projection,
            &mut Transform,
            &mut GlobalTransform,
        ),
        With<Camera3d>,
    >,
    mut back: Local<f32>,
    mut commands: Commands,
) {
    let Ok((cam, mut o, mut proj, mut tf, mut global)) = q.single_mut() else {
        return;
    };
    let Some(at) = head
        .and_then(|h| joints.get(h.0).ok())
        .map(|t| t.translation())
    else {
        return;
    };
    let serving = g.phase == super::Phase::Serve && g.score.server == 0;
    *back += ((serving as u8 as f32) - *back) * (4.0 * time.delta_secs()).min(1.0);
    o.yaw = f32::from_bits(YAW.load(Relaxed));
    o.pitch = f32::from_bits(PITCH.load(Relaxed));
    let flat = Vec3::new(-o.yaw.sin(), 0.0, -o.yaw.cos());
    let eye = at
        + Vec3::Y * EYE_UP
        + flat * (AHEAD - *back * SERVE_BACK.x)
        + Vec3::Y * *back * SERVE_BACK.y;
    o.focus = eye - Quat::from_euler(EulerRot::YXZ, o.yaw, o.pitch, 0.0) * Vec3::Z * o.radius;
    *tf = o.transform();
    *global = GlobalTransform::from(*tf);
    // pulled back, the whole body shows (the shadow twins have the head)
    let layers = if *back > 0.3 {
        RenderLayers::from_layers(&[0, 1])
    } else {
        RenderLayers::layer(0)
    };
    commands.entity(cam).insert(layers);
    if let Projection::Perspective(p) = &mut *proj {
        p.fov = FOV.to_radians();
        p.near = 0.05;
    }
}

/// Player 0's head joint (the eye's height).
#[derive(Resource)]
struct Head(Entity);

#[derive(Component)]
struct Headless;

/// Once player 0's rig is spawned: each skinned part gets a shadow-only twin, and its own head joints (the head and
/// everything under it: face, eyes) swapped for a point at the head.
fn headless(
    mut commands: Commands,
    rigs: Query<(Entity, &Figure, &crate::character::Rig, &Children), Without<Headless>>,
    parts: Query<&SkinnedMesh>,
) {
    for (root, f, rig, parts_of) in &rigs {
        if f.0 != 0 {
            continue;
        }
        commands.entity(root).insert(Headless);
        let joints = &rig.data.joints;
        let Some(head) = ["Bip01Head", "head", "Head"]
            .into_iter()
            .find_map(|n| rig.data.joint(n))
        else {
            continue;
        };
        commands.insert_resource(Head(rig.joints[head]));
        let under_head = |mut j: usize| loop {
            if j == head {
                return true;
            }
            match joints[j].parent {
                Some(p) => j = p,
                None => return false,
            }
        };
        let point = commands
            .spawn((
                Transform::from_scale(Vec3::splat(1e-4)),
                ChildOf(rig.joints[head]),
            ))
            .id();
        let hidden: Vec<Entity> = (0..joints.len())
            .filter(|&j| under_head(j))
            .map(|j| rig.joints[j])
            .collect();
        for &part in parts_of {
            let Ok(skin) = parts.get(part) else { continue };
            commands
                .entity(part)
                .clone_and_spawn()
                .insert((RenderLayers::layer(1), NoFrustumCulling));
            let mut skin = skin.clone();
            for j in &mut skin.joints {
                if hidden.contains(j) {
                    *j = point;
                }
            }
            commands.entity(part).insert(skin);
        }
    }
}

/// The sun casts from layer 1 (the shadow twins) as well as the usual layer 0.
fn sun_sees_shadows(
    mut commands: Commands,
    lights: Query<Entity, (With<DirectionalLight>, Without<RenderLayers>)>,
) {
    for e in &lights {
        commands
            .entity(e)
            .insert(RenderLayers::from_layers(&[0, 1]));
    }
}
