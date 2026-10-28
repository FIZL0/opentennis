//! The net isn't drawn when the camera is right at it looking past it, as the original's court-object pass.
//!
//! The game marks one court object for this: the category-17 plant placed within 1 m of the court's centre (the
//! net). Every frame, before drawing it, it skips it when all of these hold for the view (game space, y down):
//! - the eye is less than 7 m from the court's long axis (|x| < 7·k),
//! - the eye's distance to the net plane, along the view's heading, is under 2 m during a post-point cut-away and
//!   0.8 m otherwise (|z| / cos heading < d·k),
//! - the net top (0.91 m up) seen from the eye stands higher than a line 0.8 × the vertical half-angle below the
//!   view's pitch, the half-angle's tangent being 0.7 × the horizontal one's,
//!
//! k = tan 20° / tan(half the horizontal fov), at least 1 (a zoomed-in view widens the zone).
//! Checked frame by frame against the game's own "not drawn" flag over slot 5's matches (`research/journal/2026-10-11-b74-skybox/scripts/b74d_check.py`).
//!
//! The same pass skips any other court object of categories 17–20 the eye is buried in (`buried`): a cut-away
//! camera standing in a flower bed or bush doesn't show it filling the lens.
//!
//! B303: a cut-away follows a player who ended the point off court into the props behind it (Alpine Heights' tables,
//! fences and carved horses), which the test above doesn't hide (thin, partly in front, or no collision mesh), so a
//! tree or prop whose box comes right up in front of the lens is hidden too (`in_front`).

use bevy::prelude::*;
use hst_sim::camera::View;

use super::Game;
use crate::shadow::{EyeHidden, EyePasses};

/// The court object the close-camera test hides (the net).
#[derive(Component)]
#[require(EyeHidden)]
pub struct HideNearCamera;

/// A court object (categories 17–20, its code's third character not '0') the same pass skips while the eye is
/// buried in it: `pos` is its plant's ground position, `prop` its collision object in `Game::world` once found
/// (`None` inside: the model has no collision mesh, so the test never hides it).
#[derive(Component)]
#[require(EyeHidden)]
pub struct HideAroundEye {
    pub pos: [f32; 3],
    pub prop: Option<Option<usize>>,
}

/// A tree or prop model with its box in game space (B303's fallback, `in_front`).
#[derive(Component)]
#[require(EyeHidden)]
pub struct NearBox {
    pub lo: Vec3,
    pub hi: Vec3,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, hide.after(super::camera).in_set(EyePasses))
        .add_systems(PostUpdate, (hide_around_eye, hide_near_pieces).in_set(EyePasses).before(bevy::transform::TransformSystems::Propagate));
}

/// The objects the eye test runs on: the pass's categories with the code character the setup checks.
pub fn tested(category: u8, code: [u8; 4]) -> bool {
    (17..=20).contains(&category) && code[2] != b'0'
}

/// Whether the court-object pass skips `o` for this view: a sphere of radius r = tan 20° / tan(half the horizontal
/// fov), clamped to 1..15, centred r ahead of the eye, overlaps the object's bounding sphere, and sweeping it onto
/// that centre from 100 m above and from (3·object radius + 2r) to either side along x all hit the object's mesh,
/// i.e. the eye is looking into the inside of it.
/// ponytail: plain f32 and the ball's sweep (`mesh::Object::sweep`, which drops texel-0 hits); presentation only.
pub fn buried(v: &View, models: &[hst_sim::mesh::Model], o: &hst_sim::mesh::Object) -> bool {
    buried_at(v, reach(v), models, o)
}

/// The sphere radius `buried` and `in_front` test with: tan 20° / tan(half the horizontal fov), clamped to 1..15.
fn reach(v: &View) -> f32 {
    (20f32.to_radians().tan() / v.fov.tan()).clamp(1.0, 15.0)
}

fn buried_at(v: &View, r: f32, models: &[hst_sim::mesh::Model], o: &hst_sim::mesh::Object) -> bool {
    let f = v.rot[2];
    let c = [v.eye[0] + f[0] * r, v.eye[1] + f[1] * r, v.eye[2] + f[2] * r, 1.0];
    let size = o.scale * o.radius;
    let d2: f32 = (0..3).map(|k| (c[k] - o.center[k]).powi(2)).sum();
    if d2 > (r + size).powi(2) {
        return false;
    }
    let side = size * 3.0 + r * 2.0;
    [[0.0, -100.0, 0.0], [-side, 0.0, 0.0], [side, 0.0, 0.0]].iter().all(|off| {
        let start = [c[0] + off[0], c[1] + off[1], c[2] + off[2], 1.0];
        let mut hit = hst_sim::mesh::Hit::NONE;
        o.sweep(models, &mut hit, start, c, r, &[])
    })
}

/// Whether the box `lo..hi` comes within r/2 of the lens' line out to 3r ahead of the eye (r as `buried`, at least 1):
/// the model would fill most of the view.
/// ponytail: by eye, not the original's (its cut-aways have no obstacle check, so it shows these too); sampled along
/// the line, a box thinner than the step between samples can slip through.
pub fn in_front(v: &View, lo: Vec3, hi: Vec3) -> bool {
    in_front_at(v, reach(v), lo, hi)
}

fn in_front_at(v: &View, r: f32, lo: Vec3, hi: Vec3) -> bool {
    let (e, f) = (Vec3::from(v.eye), Vec3::from(v.rot[2]));
    // P43: a box clear of the line's own box grown by r/2 (and a hair for rounding) can't be near any sample on it
    let (end, reach) = (e + f * (3.0 * r), Vec3::splat(r / 2.0 + 1e-3));
    if (e.min(end) - reach).cmpgt(hi).any() || (e.max(end) + reach).cmplt(lo).any() {
        return false;
    }
    (0..=12).any(|i| {
        let p = e + f * (3.0 * r * i as f32 / 12.0);
        p.clamp(lo, hi).distance(p) <= r / 2.0
    })
}

/// The plant the game picks: category 17 and its ground position within 1 m of the centre.
pub fn picked(category: u8, pos: [f32; 3]) -> bool {
    category == 17 && pos[0].hypot(pos[2]) < 1.0
}

/// Whether the game skips the net's draw for this view.
pub fn hidden(v: &View, cutaway: bool) -> bool {
    let (e, f) = (v.eye, v.rot[2]);
    let k = (20f32.to_radians().tan() / v.fov.tan()).max(1.0);
    let flat = f[0].hypot(f[2]);
    if e[0].abs() >= 7.0 * k || e[2].abs() / (f[2].abs() / flat) >= if cutaway { 2.0 } else { 0.8 } * k {
        return false;
    }
    let top = ((0.91 + e[1]) / e[2].abs()).atan();
    let low = (-f[1]).atan2(flat) - (v.fov.tan() * 0.7).atan() * 0.8;
    low < top
}

fn hide(g: Res<Game>, own: Option<Res<super::own_camera::OwnCam>>, d: Option<Res<super::cutaway::Director>>, off: Option<Res<crate::cull::Off>>, mut q: Query<&mut EyeHidden, With<HideNearCamera>>) {
    let gone = off.is_none() && hidden(&super::own_camera::shown(&g, own.as_deref()).1, d.is_some_and(|d| d.showing()));
    for mut v in &mut q {
        EyeHidden::set(&mut v, EyeHidden::NET, gone);
    }
}

/// Runs on the drawn camera (any mode: match, cut-away, replay, own camera, `HST_EYE`) once it has moved; hides
/// nothing while `cull::Off` holds (the intro's flyover).
/// B634: while the split view is on, a prop that buries either half's eye is hidden (from both).
#[allow(clippy::type_complexity)]
/// M157: skipped while the drawn view, the split view and `cull::Off` stay as they were and no prop was added (the
/// props don't move): each prop's test sweeps its mesh three times per view.
fn hide_around_eye(g: Option<Res<Game>>, off: Option<Res<crate::cull::Off>>, camera: Query<(&Transform, &Projection), With<crate::Orbit>>, window: Query<&Window>, second: Option<Res<crate::twin_view::SecondEye>>, mut last: Local<Option<(View, bool, bool)>>, mut q: Query<(Option<&mut HideAroundEye>, Option<Ref<NearBox>>, &mut EyeHidden), Or<(With<HideAroundEye>, With<NearBox>)>>) {
    let (Some(g), Ok((t, proj))) = (g, camera.single()) else { return };
    let Some(view) = drawn_view(t, proj, &window) else { return };
    let key = (view, off.is_some(), second.is_some());
    let added = q.iter_mut().any(|(h, n, _)| h.is_some_and(|h| h.is_added()) || n.is_some_and(|n| n.is_added()));
    if !added && !second.as_ref().is_some_and(|s| s.is_changed()) && last.as_ref() == Some(&key) {
        return;
    }
    *last = Some(key);
    let views: Vec<(View, f32)> = std::iter::once(view).chain(second_view(second.as_deref())).map(|v| { let r = reach(&v); (v, r) }).collect();
    let world = &g.world.0;
    for (h, near, mut v) in &mut q {
        let buried = off.is_none() && h.is_some_and(|mut h| {
            let pos = h.pos;
            // the prop whose placement takes the plant's ground position to its model origin (looked up once; read
            // through `Mut` without marking it changed after that)
            let prop = if let Some(p) = h.prop { p } else { *h.prop.get_or_insert_with(|| {
                world.props.iter().position(|o| {
                    let m = hst_sim::vu0::transform(&o.to_model, [pos[0], pos[1], pos[2], 1.0]);
                    m[0].abs() + m[1].abs() + m[2].abs() < 1e-3
                })
            }) };
            prop.is_some_and(|i| views.iter().any(|(v, r)| buried_at(v, *r, &world.models, &world.props[i])))
        });
        let in_front = off.is_none() && near.is_some_and(|b| views.iter().any(|(v, r)| in_front_at(v, *r, b.lo, b.hi)));
        EyeHidden::set(&mut v, EyeHidden::BURIED, buried);
        EyeHidden::set(&mut v, EyeHidden::IN_FRONT, in_front);
    }
}

/// The drawn camera's view in game space (Bevy's turned a half-turn about X), with the horizontal fov as `fixed_eye`
/// sets the vertical one; `None` for an orthographic one.
fn drawn_view(t: &Transform, proj: &Projection, window: &Query<&Window>) -> Option<View> {
    let Projection::Perspective(p) = proj else { return None };
    let aspect = window.single().map_or(16.0 / 9.0, crate::play::widescreen::scene_aspect);
    Some(view_at(t, p, aspect))
}

/// A camera's view in game space with the horizontal fov of its picture `aspect` wide.
fn view_at(t: &Transform, p: &PerspectiveProjection, aspect: f32) -> View {
    let (e, f) = (t.translation * Vec3::new(1.0, -1.0, -1.0), *t.forward() * Vec3::new(1.0, -1.0, -1.0));
    View { rot: [[0.0; 3], [0.0; 3], f.to_array()], eye: e.to_array(), fov: ((p.fov / 2.0).tan() / 0.75f32.max(1.0 / aspect)).atan() }
}

/// B634: the split view's bottom half's view (`twin_view`'s second view), while it's on.
fn second_view(second: Option<&crate::twin_view::SecondEye>) -> Option<View> {
    let (_, t, p) = second?.0.as_ref()?;
    Some(view_at(t, p, p.aspect_ratio))
}

/// B510: a court mod's gimmick (`court_mod::NearPiece`) one of whose boxes comes right up in front of the drawn camera
/// is hidden whole: B303's test (`in_front`) at its widest lens' reach whatever the zoom (within 0.5 m of the lens'
/// line out to 3 m), since a telephoto's longer reach also takes the pieces in frame far behind the player it
/// follows (Dubno's caution flags); nothing while `cull::Off` holds (the intro's flyover).
#[allow(clippy::type_complexity)]
fn hide_near_pieces(mut commands: Commands, off: Option<Res<crate::cull::Off>>, camera: Query<(&Transform, &Projection), With<crate::Orbit>>, window: Query<&Window>, second: Option<Res<crate::twin_view::SecondEye>>, pieces: Query<&crate::court_mod::NearPiece>, mut vis: Query<Option<&mut EyeHidden>, (Without<HideAroundEye>, Without<NearBox>)>) {
    let Ok((t, proj)) = camera.single() else { return };
    let Some(view) = drawn_view(t, proj, &window) else { return };
    // B634: in front of either half's eye while the split view is on
    let views: Vec<View> = std::iter::once(view).chain(second_view(second.as_deref())).collect();
    for np in &pieces {
        let hidden = off.is_none() && np.boxes.iter().any(|&(lo, hi)| views.iter().any(|v| in_front_at(v, 1.0, lo, hi)));
        for &e in &np.hide {
            match vis.get_mut(e) {
                Ok(Some(mut v)) => EyeHidden::set(&mut v, EyeHidden::PIECE, hidden),
                Ok(None) if hidden => _ = commands.entity(e).try_insert(EyeHidden::of(EyeHidden::PIECE)),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(eye: [f32; 3], fwd: [f32; 3], fov_deg: f32) -> View {
        View { rot: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], fwd], eye, fov: fov_deg.to_radians() / 2.0 }
    }

    /// Views read from the game (slot 5, court 10) with its flag: the close cut-away hides it, the match camera
    /// doesn't.
    #[test]
    fn slot5_cutaway() {
        assert!(hidden(&view([-2.926, -0.517, -0.329], [-0.207, -0.128, 0.97], 51.41), true));
        assert!(hidden(&view([-3.247, -0.5165, -0.3792], [-0.099, -0.128, 0.987], 51.41), true));
        assert!(!hidden(&view([0.0, -11.89, -39.238], [0.0, 0.292, 0.956], 20.0), false));
        // match over, the camera 2.6 m past the net: drawn (the cut-away distance would hide it)
        let end = view([-0.001, -0.807, 2.611], [0.0, 0.019, 1.0], 30.93);
        assert!(!hidden(&end, false) && hidden(&end, true));
        // B654: the cut-away from beside a net post, right at the net plane: the game draws the net (its top is below
        // the view's lower line), both ends
        assert!(!hidden(&view([6.285, -1.187, 0.219], [-0.747, -0.013, -0.665], 60.0), true));
        assert!(!hidden(&view([-6.285, -1.187, 0.219], [0.958, -0.007, 0.285], 60.0), true));
    }

    #[test]
    fn picks_the_net() {
        assert!(picked(17, [0.0, 0.0, 0.0]));
        assert!(!picked(17, [0.0, 0.0, 5.0]));
        assert!(!picked(18, [0.0, 0.0, 0.0]));
    }

    /// Cut-away views the port's own bots reached on Alpine Heights (disc court 6) and Wild Green (8), with the eye
    /// in a flower bed (91) and a red flower bush (44): hidden; the match camera hides nothing in frame.
    #[test]
    fn buried_props() {
        let Ok(mut iso) = hst_data::fixture::iso() else { return };
        let match_cam = view([0.0, -11.89, -39.238], [0.0, 0.292, 0.956], 20.0);
        for (court, prop, eye, fwd, t) in [
            (6, 91, [-2.4966, -1.63983, -16.0944], [0.162865, 0.036274, 0.985981], 0.0436609f32),
            (8, 44, [5.77494, -0.114015, -19.7535], [-0.273811, -0.108348, 0.955661], 0.176327),
        ] {
            let w = hst_sim::court::world(&mut iso, court).unwrap();
            let v = View { rot: [[0.0; 3], [0.0; 3], fwd], eye, fov: t.atan() };
            assert!(buried(&v, &w.models, &w.props[prop]), "court {court} prop {prop}");
            let gone: Vec<_> = (0..w.props.len()).filter(|&i| buried(&match_cam, &w.models, &w.props[i])).collect();
            // Wild Green's match camera stands in the forked tree behind the baseline (out of frame)
            assert_eq!(gone, if court == 8 { vec![54] } else { vec![] }, "court {court} match camera");
        }
        assert!(tested(17, [0, 0, 0, 0]) && tested(20, *b"ab1\0") && !tested(18, *b"ab0\0") && !tested(16, [0; 4]));
    }

    /// B303: a table 1.5 m ahead of a cut-away's lens is hidden, one 5 m ahead or off to the side isn't; the match
    /// camera's narrower view reaches further.
    #[test]
    fn props_in_front() {
        let cut = view([0.0, -1.0, 0.0], [0.0, 0.0, 1.0], 51.4);
        let table = |z: f32, x: f32| (Vec3::new(x - 0.4, -0.8, z - 0.4), Vec3::new(x + 0.4, 0.0, z + 0.4));
        let hit = |v: &View, (lo, hi): (Vec3, Vec3)| in_front(v, lo, hi);
        assert!(hit(&cut, table(1.5, 0.0)) && hit(&cut, table(2.0, 0.6)));
        assert!(!hit(&cut, table(5.0, 0.0)) && !hit(&cut, table(1.5, 2.0)) && !hit(&cut, table(-1.0, 0.0)));
        assert!(hit(&view([0.0, -1.0, 0.0], [0.0, 0.0, 1.0], 20.0), table(5.0, 0.0)));
    }

    /// B510: the gimmick test at the widest lens' reach (`hide_near_pieces`): Dubno's helicopter cabin (its box at rest)
    /// fills a logged doubles cut-away from 0.5 m past its side; a telephoto replay down the court, whose zoomed reach
    /// would take in the far caution flag behind the player, keeps a box 20 m out on its line.
    #[test]
    fn gimmick_in_front() {
        let cabin = (Vec3::new(7.06, -3.7, -6.85), Vec3::new(9.96, -0.57, 9.03));
        let cut = view([10.427, -2.983, 1.839], [-0.959, 0.224, 0.175], 2.0 * 0.4363f32.to_degrees());
        assert!(in_front_at(&cut, 1.0, cabin.0, cabin.1));
        let tele = view([2.535, -1.507, 23.806], [-0.135, 0.026, -0.990], 2.0 * 0.04363f32.to_degrees());
        let far = (Vec3::new(-0.5, -2.5, 2.0), Vec3::new(0.5, -1.0, 3.0));
        assert!(!in_front_at(&tele, 1.0, far.0, far.1) && in_front(&tele, far.0, far.1));
    }

    /// P43: the quick box rejection in `in_front` never changes its answer from the sampled test alone.
    #[test]
    fn in_front_rejection_is_exact() {
        let mut seed = 0x2545_f491_u32;
        let mut rnd = |lo: f32, hi: f32| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            lo + (hi - lo) * (seed as f32 / u32::MAX as f32)
        };
        let mut hits = 0;
        for _ in 0..20_000 {
            let fwd = Vec3::new(rnd(-1.0, 1.0), rnd(-1.0, 1.0), rnd(-1.0, 1.0)).normalize_or(Vec3::Z);
            let v = view([rnd(-3.0, 3.0), rnd(-3.0, 3.0), rnd(-3.0, 3.0)], fwd.to_array(), rnd(10.0, 70.0));
            let c = Vec3::new(rnd(-12.0, 12.0), rnd(-12.0, 12.0), rnd(-12.0, 12.0));
            let h = Vec3::new(rnd(0.0, 2.0), rnd(0.0, 2.0), rnd(0.0, 2.0));
            let (lo, hi) = (c - h, c + h);
            let r = (20f32.to_radians().tan() / v.fov.tan()).clamp(1.0, 15.0);
            let (e, f) = (Vec3::from(v.eye), Vec3::from(v.rot[2]));
            let sampled = (0..=12).any(|i| {
                let p = e + f * (3.0 * r * i as f32 / 12.0);
                p.clamp(lo, hi).distance(p) <= r / 2.0
            });
            assert_eq!(in_front(&v, lo, hi), sampled, "{v:?} {lo} {hi}");
            hits += sampled as u32;
        }
        assert!(hits > 100, "{hits}");
    }
}
