//! P137: the menus and the pause screen stop redrawing at the full frame rate while nothing on them moves (a phone at
//! 120 fps drained 5% of its battery every 4 minutes sitting on a menu). A front-end screen asks for it each frame
//! with [`want`]; once nothing has moved for [`SETTLE`] the app waits for input instead of drawing on, waking
//! [`IDLE_HZ`] times a second to read the pads (which don't wake it) and play what keeps going on its own. Any input,
//! animation or screen change puts it back to the full rate that same frame. A match never asks (only its pause screen,
//! offline, does), so play and its timing are untouched; the simulation's 60 Hz doesn't depend on the drawing rate.
//!
//! "Moved" is any change to what's drawn: a UI node's size or place, its image, tint or outline, text, visibility, a
//! transform that went somewhere (3D previews, cameras), a sprite or mesh, or a modified image or mesh asset. Values
//! are compared, not change marks: the menus rewrite their quads and camera every frame whether they moved or not. What loops forever by design (the menus' wagging
//! hand on an [`ambient_layer`], the home screen's breathing row's quads marked [`Ambient`]) doesn't count: it plays
//! on at the idle rate.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

/// Still this long (s, real time) before the drawing slows: past one 60 Hz tick drawn at 120 fps, and past the
/// pauses between a menu's animation steps.
pub const SETTLE: f32 = 0.5;
/// Frames a second while still: often enough for the pads' presses (polled, they don't wake the app) and the ambient
/// hand, a quarter of a 120 Hz phone's.
pub const IDLE_HZ: u64 = 30;

/// A transform this close to where it was (each of its matrix's numbers, metres) hasn't moved: what eases toward a
/// target on frame time (a head turning, a sway settling) creeps on by fractions of a millimetre for ever.
const NUDGE: f32 = 1e-4;

/// A front-end screen wants the slower drawing this frame (cleared each frame by `pace`).
static WANT: AtomicBool = AtomicBool::new(false);
/// Render layers (bit per layer, 0–63) whose movement doesn't count ([`ambient_layer`]).
static AMBIENT: AtomicU64 = AtomicU64::new(0);

/// Asks for the slower drawing this frame: call it every frame a menu or the (offline) pause screen is up.
pub fn want() {
    WANT.store(true, Ordering::Relaxed);
}

/// A UI node whose movement loops by design (the home screen's breathing row): it alone keeps the full rate off.
#[derive(Component)]
pub struct Ambient;

/// What's drawn on render layer `l` loops by design: its movement alone keeps the full rate off.
pub fn ambient_layer(l: usize) {
    if l < 64 {
        AMBIENT.fetch_or(1 << l, Ordering::Relaxed);
    }
}

fn ambient(layers: Option<&RenderLayers>) -> bool {
    let mask = AMBIENT.load(Ordering::Relaxed);
    layers.is_some_and(|l| {
        let mut it = l.iter().peekable();
        it.peek().is_some() && it.all(|i| i < 64 && mask & (1 << i) != 0)
    })
}

/// Changes Bevy only marks when the value differs: UI layout (a node's size, place, turn), visibility, and what
/// the menus rarely touch.
type Moved = Or<(
    Changed<ComputedNode>,
    Changed<UiGlobalTransform>,
    Changed<InheritedVisibility>,
    Changed<BackgroundColor>,
    Changed<Text>,
    Changed<TextSpan>,
    Changed<TextColor>,
    Changed<Sprite>,
    Changed<Mesh3d>,
)>;

type Shown<'w, 's> = Query<'w, 's, (&'static ImageNode, Option<&'static BorderColor>, &'static InheritedVisibility), Without<Ambient>>;

/// A fingerprint of what the shown images look like (image, part, tint, mode) and the outlines' colours: the menus
/// rewrite every quad each frame, moved or not, so their change marks say nothing.
fn looks(nodes: &Shown) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::hash::DefaultHasher::new();
    for (img, rim, vis) in nodes {
        if !vis.get() {
            continue;
        }
        img.image.id().hash(&mut h);
        img.rect.map(|r| [r.min.x, r.min.y, r.max.x, r.max.y].map(f32::to_bits)).hash(&mut h);
        img.color.to_linear().to_f32_array().map(f32::to_bits).hash(&mut h);
        (img.flip_x, img.flip_y, std::mem::discriminant(&img.image_mode)).hash(&mut h);
        rim.map(|b| [b.top, b.right, b.bottom, b.left].map(|c| c.to_linear().to_f32_array().map(f32::to_bits))).hash(&mut h);
    }
    h.finish()
}

/// `HST_CALM=1`: slows a `--shot` or unattended run too (to check this in one).
static FORCE: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| std::env::var_os("HST_CALM").is_some_and(|v| v != "0"));
/// `HST_CALM=0`: never slows (to tell whether something seen on a menu comes from this).
static OFF: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| std::env::var_os("HST_CALM").is_some_and(|v| v == "0"));

/// Whether the drawing slows now: wanted, still for [`SETTLE`] s, and not a `--shot` or unattended run (those count
/// frames) unless [`FORCE`]d.
fn slow(wanted: bool, moved: bool, still: &mut f32, dt: f32) -> bool {
    let off = crate::graphics::NO_AUTO.load(Ordering::Relaxed) && !*FORCE || *OFF;
    if !wanted || moved || off || cfg!(target_arch = "wasm32") {
        *still = 0.0;
        return false;
    }
    *still += dt;
    *still >= SETTLE
}

#[allow(clippy::too_many_arguments)]
fn pace(
    moved: Query<Entity, (Moved, Without<Ambient>)>,
    nodes: Shown,
    placed: Query<(Entity, &GlobalTransform, Option<&RenderLayers>), Changed<GlobalTransform>>,
    mut images: MessageReader<AssetEvent<Image>>,
    mut meshes: MessageReader<AssetEvent<Mesh>>,
    time: Res<Time<Real>>,
    winit: Option<ResMut<WinitSettings>>,
    mut seen: Local<(u64, bevy::platform::collections::HashMap<Entity, GlobalTransform>)>,
    mut still: Local<f32>,
) {
    let wanted = WANT.swap(false, Ordering::Relaxed);
    let assets = images.read().any(|e| matches!(e, AssetEvent::Modified { .. })) | meshes.read().any(|e| matches!(e, AssetEvent::Modified { .. }));
    // only looked at while wanted: a match never pays for it
    let moving = wanted && {
        let look = looks(&nodes);
        // a transform rewritten with the place it had (a camera set each frame), or crept a hair, isn't a move
        let mut went = None;
        for (e, g, l) in &placed {
            // hidden pieces can sit at NaN, which never equals itself
            if ambient(l) || !g.affine().is_finite() {
                continue;
            }
            let was = seen.1.insert(e, *g);
            if !was.is_some_and(|w| w.affine().abs_diff_eq(g.affine(), NUDGE)) {
                went = Some(e);
            }
        }
        let differs = std::mem::replace(&mut seen.0, look) != look;
        let moved = moved.iter().next();
        // `HST_CALM=1`: what keeps it going, once a second
        if *FORCE && (time.elapsed_secs() + time.delta_secs()) as u32 != time.elapsed_secs() as u32 && (assets || went.is_some() || differs || moved.is_some()) {
            info!("menus moving: assets {assets}, transform {went:?}, looks {differs}, ui/text {moved:?}");
        }
        assets | went.is_some() | differs | moved.is_some()
    };
    if !wanted {
        seen.1.clear();
    }
    let mode = if slow(wanted, moving, &mut still, time.delta_secs()) { UpdateMode::reactive(Duration::from_micros(1_000_000 / IDLE_HZ)) } else { UpdateMode::Continuous };
    if let Some(mut w) = winit.filter(|w| w.focused_mode != mode) {
        if crate::fps_log() || *FORCE {
            info!("{:.2} s: menus drawing {}", time.elapsed_secs(), if mode == UpdateMode::Continuous { "at the full rate" } else { "only on change (still)" });
        }
        w.focused_mode = mode;
    }
}

/// In the base app (`graphics::plugin`), for every mode: a mode that stops asking (the menu → a match) is back at the
/// full rate the next frame.
pub fn plugin(app: &mut App) {
    app.add_systems(Last, pace);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Slows only after SETTLE s of wanted stillness; a move, or a frame not wanted (a match), is the full rate at once.
    #[test]
    fn settles_then_wakes() {
        let mut still = 0.0;
        // exact in binary, so SETTLE is reached on a known frame
        let dt = 1.0 / 128.0;
        let frames = (SETTLE / dt) as usize;
        assert!((0..frames - 1).all(|_| !slow(true, false, &mut still, dt)));
        slow(true, false, &mut still, dt);
        assert!((0..3).all(|_| slow(true, false, &mut still, dt)));
        assert!(!slow(true, true, &mut still, dt));
        assert!(!slow(true, false, &mut still, dt));
        // a 60 Hz tick drawn at 120 fps: every other frame moves, never slow
        still = 0.0;
        assert!((0..240).all(|i| !slow(true, i % 2 == 0, &mut still, dt)));
        still = 1.0;
        assert!(!slow(false, false, &mut still, dt));
    }

    #[test]
    fn ambient_layers() {
        ambient_layer(40);
        assert!(ambient(Some(&RenderLayers::layer(40))));
        assert!(!ambient(Some(&RenderLayers::layer(39))));
        assert!(!ambient(Some(&RenderLayers::from_layers(&[39, 40]))));
        assert!(!ambient(None));
    }
}
