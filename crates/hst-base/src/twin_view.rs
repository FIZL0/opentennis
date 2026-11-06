//! B634: a second view of the scene (the split view's bottom half, `hst-play`'s `split_view`) sees the
//! camera-facing quads (name tags, markers, pop-ups, sparks, dust, rain…) turned to the main camera: from the
//! other end they read backwards or squashed. While a second view is on, each such quad (a [`Billboard`]) is drawn
//! by the main camera only ([`TOP`]) and gets a copy drawn by the second view only ([`BOTTOM`]), turned to it: the
//! quad's own system poses the copy ([`Second::pose`]) or, for quads built into a mesh each frame, fills the copy's
//! own mesh ([`Second::mesh`]). Copies take the original's mesh, material and visibility each frame; they go with
//! the second view.
//!
//! Presentation only: nothing here reads or writes the match.

use bevy::camera::visibility::{NoFrustumCulling, RenderLayers, VisibilitySystems};
use bevy::ecs::system::SystemParam;
use bevy::light::cascade::{Cascade, Cascades};
use bevy::light::{DirectionalLightShadowMap, NotShadowCaster, SimulationLightSystems};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::transform::TransformSystems;

use crate::effects::{Emptied, Fx};
use crate::gs::GsMaterial;

/// The main camera's own layer while a second view is on, and the second view's (clear of the menus' and
/// `hud_gamma`'s layers).
pub const TOP: usize = 28;
pub const BOTTOM: usize = 29;

pub fn plugin(app: &mut App) {
    app.init_resource::<SecondEye>().init_resource::<Twins>().add_systems(
        PostUpdate,
        (sync, material::<StandardMaterial>, material::<GsMaterial>, material::<Fx>)
            .chain()
            .after(crate::effects::hide_empty)
            .before(TransformSystems::Propagate)
            .before(VisibilitySystems::VisibilityPropagate),
    )
    .add_systems(PostUpdate, top_halves.before(VisibilitySystems::VisibilityPropagate))
    .add_systems(
        PostUpdate,
        shared_cascades.after(SimulationLightSystems::UpdateDirectionalLightCascades).before(SimulationLightSystems::UpdateLightFrusta),
    );
}

/// The second view's camera, where it is and its lens, while it's on (its owner sets it each frame).
#[derive(Resource, Default)]
pub struct SecondEye(pub Option<(Entity, Transform, PerspectiveProjection)>);

/// The split view is on, on a phone or tablet (the apps, or the browser played by touch): each half draws the whole
/// scene again, so the scene image drops to [`PHONE_SCALE`] % and MSAA goes off while it lasts (`hud_gamma`,
/// `graphics::msaa`); the settings themselves don't change.
pub fn phone_split(eye: Option<&SecondEye>, dev: Option<&crate::device::Device>) -> bool {
    use crate::device::{Input, Platform};
    eye.is_some_and(|e| e.0.is_some()) && dev.is_some_and(|d| matches!(d.platform, Platform::Android | Platform::Ios) || d.input == Input::Touch)
}

/// The render scale cap while [`phone_split`] (a half of a phone's dense screen hides it).
pub const PHONE_SCALE: u32 = 50;

/// A camera-facing quad that the second view gets its own copy of; `own_mesh`: its vertices face the camera (built
/// each frame), so the copy has a mesh of its own for its system to fill.
#[derive(Component, Clone, Copy, Default)]
pub struct Billboard {
    pub own_mesh: bool,
}

impl Billboard {
    pub const TURNED: Billboard = Billboard { own_mesh: false };
    pub const BUILT: Billboard = Billboard { own_mesh: true };
}

/// B639: a mesh drawn by the main camera only while a second view is on, for things whose system keeps a whole
/// copy of its own on [`BOTTOM`] (a court mod's card people: scenes, maybe skinned, that one mesh copy can't turn).
#[derive(Component)]
pub struct TopHalf;

/// A copy, for the second view, of this original.
#[derive(Component)]
pub struct TwinOf(pub Entity);

#[derive(Resource, Default)]
pub struct Twins {
    /// Original → copy.
    copies: HashMap<Entity, Entity>,
    /// An original's mesh → the copies' own mesh.
    meshes: HashMap<AssetId<Mesh>, Handle<Mesh>>,
    /// This frame's poses of the copies, by original.
    poses: HashMap<Entity, Transform>,
}

/// What a billboard's system needs to turn its quad to the second view too.
#[derive(SystemParam)]
pub struct Second<'w> {
    eye: Option<Res<'w, SecondEye>>,
    twins: Option<ResMut<'w, Twins>>,
}

impl Second<'_> {
    /// The second view's camera, while it's on.
    pub fn eye(&self) -> Option<&Transform> {
        self.on().map(|e| &e.1)
    }

    fn on(&self) -> Option<&(Entity, Transform, PerspectiveProjection)> {
        self.eye.as_ref()?.0.as_ref()
    }

    /// Its lens.
    pub fn projection(&self) -> Option<&PerspectiveProjection> {
        self.on().map(|e| &e.2)
    }

    /// Its camera entity.
    pub fn camera(&self) -> Option<Entity> {
        self.on().map(|e| e.0)
    }

    /// The pose of `original`'s copy this frame (its own transform's, in the original's parent's space).
    pub fn pose(&mut self, original: Entity, t: Transform) {
        if self.on().is_some()
            && let Some(twins) = self.twins.as_mut()
        {
            twins.poses.insert(original, t);
        }
    }

    /// The copies' mesh to fill for `original`'s mesh (a [`Billboard::BUILT`]), while the second view is on.
    pub fn mesh(&self, original: &Handle<Mesh>) -> Option<&Handle<Mesh>> {
        self.on()?;
        self.twins.as_ref()?.meshes.get(&original.id())
    }
}

#[allow(clippy::type_complexity)]
pub fn sync(
    mut commands: Commands,
    eye: Option<Res<SecondEye>>,
    twins: Option<ResMut<Twins>>,
    originals: Query<(Entity, &Billboard, &Mesh3d, &Transform, &Visibility, Option<&ChildOf>, Option<&RenderLayers>, Has<Emptied>), Without<TwinOf>>,
    mut copies: Query<(&TwinOf, &mut Mesh3d, &mut Transform, &mut Visibility), Without<Billboard>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // (a mode's teardown drops the game's resources)
    let Some(mut twins) = twins else { return };
    let twins = &mut *twins;
    let top = RenderLayers::layer(TOP);
    if eye.is_none_or(|e| e.0.is_none()) {
        if twins.copies.is_empty() {
            return;
        }
        for (_, copy) in twins.copies.drain() {
            commands.entity(copy).try_despawn();
        }
        twins.meshes.clear();
        twins.poses.clear();
        for (e, .., layers, _) in &originals {
            if layers == Some(&top) {
                commands.entity(e).remove::<RenderLayers>();
            }
        }
        return;
    }
    // copies whose original has gone
    twins.copies.retain(|o, c| {
        let kept = originals.contains(*o);
        if !kept {
            commands.entity(*c).try_despawn();
        }
        kept
    });
    for (e, b, mesh, t, vis, parent, layers, emptied) in &originals {
        // an emptied effect stays on no layer (`effects::hide_empty`)
        if !emptied && layers != Some(&top) {
            commands.entity(e).insert(top.clone());
        }
        let m = if b.own_mesh {
            twins
                .meshes
                .entry(mesh.id())
                .or_insert_with(|| {
                    let m = meshes.get(&mesh.0).cloned().unwrap_or_else(|| Mesh::from(Rectangle::new(0.0, 0.0)));
                    meshes.add(m)
                })
                .clone()
        } else {
            mesh.0.clone()
        };
        let pose = twins.poses.remove(&e).unwrap_or(*t);
        match twins.copies.get(&e).and_then(|c| copies.get_mut(*c).ok()) {
            Some((_, mut cm, mut ct, mut cv)) => {
                if cm.0 != m {
                    cm.0 = m;
                }
                ct.set_if_neq(pose);
                cv.set_if_neq(*vis);
            }
            None => {
                let mut c = commands.spawn((TwinOf(e), Mesh3d(m), pose, *vis, RenderLayers::layer(BOTTOM), NoFrustumCulling, NotShadowCaster));
                if let Some(p) = parent {
                    c.insert(ChildOf(p.parent()));
                }
                twins.copies.insert(e, c.id());
            }
        }
    }
    twins.poses.clear();
}

/// The copies wear their originals' material.
fn material<M: Material>(mut commands: Commands, twins: Option<Res<Twins>>, originals: Query<&MeshMaterial3d<M>, Without<TwinOf>>, copies: Query<Option<&MeshMaterial3d<M>>, With<TwinOf>>) {
    let Some(twins) = twins else { return };
    for (o, c) in &twins.copies {
        let (Ok(m), Ok(cm)) = (originals.get(*o), copies.get(*c)) else { continue };
        if cm.is_none_or(|cm| cm.0 != m.0) {
            commands.entity(*c).insert(MeshMaterial3d(m.0.clone()));
        }
    }
}

/// [`TopHalf`] meshes go on [`TOP`] while a second view is on, back to the default layer when it goes.
fn top_halves(mut commands: Commands, eye: Option<Res<SecondEye>>, q: Query<(Entity, Option<&RenderLayers>), With<TopHalf>>) {
    let on = eye.is_some_and(|e| e.0.is_some());
    let top = RenderLayers::layer(TOP);
    for (e, layers) in &q {
        match (on, layers == Some(&top)) {
            (true, false) => _ = commands.entity(e).insert(top.clone()),
            (false, true) => _ = commands.entity(e).remove::<RenderLayers>(),
            _ => {}
        }
    }
}

/// The second view draws no sun shadow pass of its own (`bevy_pbr::ReusesShadowMaps`, our patch): it samples the
/// maps the main camera drew just before it. Each of the sun's cascades becomes one box holding both views' (the
/// same for both), so the one pass covers what either half sees. Its size goes up in whole steps so the texels don't
/// swim as the halves' eyes move.
// ponytail: the union of two views' boxes is coarser than either's; split screen only
// (no shadow map resource, as in the headless tests: no shadows to share)
fn shared_cascades(eye: Option<Res<SecondEye>>, main: Query<Entity, With<crate::Orbit>>, map: Option<Res<DirectionalLightShadowMap>>, mut lights: Query<&mut Cascades>) {
    let (Some((second, ..)), Ok(main), Some(map)) = (eye.as_ref().and_then(|e| e.0.as_ref()), main.single(), map) else { return };
    for mut c in &mut lights {
        let (Some(a), Some(b)) = (c.cascades.get(&main), c.cascades.get(second)) else { continue };
        if a.len() != b.len() {
            continue;
        }
        let both: Vec<Cascade> = a.iter().zip(b).map(|(a, b)| union(a, b, map.size as f32)).collect();
        c.cascades.insert(main, both.clone());
        c.cascades.insert(*second, both);
    }
}

/// Size steps of a shared cascade (m).
const STEP: f32 = 8.0;

/// One cascade holding both boxes (in the light's space, as Bevy's `calculate_cascade` builds them).
fn union(a: &Cascade, b: &Cascade, texels: f32) -> Cascade {
    // light space: Bevy's cascades turn by the light alone, so a's axes are b's
    let world_from_light = Mat4::from_mat3(Mat3::from_mat4(a.world_from_cascade));
    let light_from_world = world_from_light.transpose();
    let bounds = |c: &Cascade| {
        let centre = light_from_world.transform_point3(c.world_from_cascade.w_axis.truncate());
        let half = 1.0 / c.clip_from_cascade.x_axis.x;
        let depth = 1.0 / c.clip_from_cascade.z_axis.z;
        (centre - Vec3::new(half, half, depth), centre + Vec3::new(half, half, 0.0))
    };
    let ((a0, a1), (b0, b1)) = (bounds(a), bounds(b));
    let (min, max) = (a0.min(b0), a1.max(b1));
    let diameter = ((max.x - min.x).max(max.y - min.y) / STEP).ceil() * STEP + STEP;
    let texel = diameter / texels;
    let near = Vec3::new((0.5 * (min.x + max.x) / texel).floor() * texel, (0.5 * (min.y + max.y) / texel).floor() * texel, max.z);
    let t = light_from_world;
    let cascade_from_world = Mat4::from_cols(t.x_axis, t.y_axis, t.z_axis, (-near).extend(1.0));
    let world_from_cascade = Mat4::from_cols(world_from_light.x_axis, world_from_light.y_axis, world_from_light.z_axis, world_from_light * near.extend(1.0));
    let clip_from_cascade = Mat4::from_cols(
        Vec4::new(2.0 / diameter, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 2.0 / diameter, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0 / (max.z - min.z), 0.0),
        Vec4::new(0.0, 0.0, 1.0, 1.0),
    );
    Cascade { world_from_cascade, clip_from_cascade, clip_from_world: clip_from_cascade * cascade_from_world, texel_size: texel }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cascade the way Bevy builds one: light-space box centred at `c` (near plane at c.z), `d` across, `depth` deep.
    fn cascade(rot: Quat, c: Vec3, d: f32, depth: f32) -> Cascade {
        let w = Mat4::from_quat(rot);
        let wt = w.transpose();
        let cfw = Mat4::from_cols(wt.x_axis, wt.y_axis, wt.z_axis, (-c).extend(1.0));
        let wfc = Mat4::from_cols(w.x_axis, w.y_axis, w.z_axis, w * c.extend(1.0));
        let p = Mat4::from_cols(Vec4::new(2.0 / d, 0.0, 0.0, 0.0), Vec4::new(0.0, 2.0 / d, 0.0, 0.0), Vec4::new(0.0, 0.0, 1.0 / depth, 0.0), Vec4::new(0.0, 0.0, 1.0, 1.0));
        Cascade { world_from_cascade: wfc, clip_from_cascade: p, clip_from_world: p * cfw, texel_size: d / 2048.0 }
    }

    /// Every corner of either box lands inside the shared cascade's clip volume (x, y in −1..1, reverse z in 0..1).
    #[test]
    fn the_union_holds_both_boxes() {
        let rot = Quat::from_euler(EulerRot::YXZ, 0.7, -1.0, 0.2);
        let (a, b) = (cascade(rot, Vec3::new(-30.0, 5.0, 40.0), 90.0, 150.0), cascade(rot, Vec3::new(25.0, -10.0, 30.0), 70.0, 120.0));
        let u = union(&a, &b, 2048.0);
        let w = Mat4::from_quat(rot);
        for (c, d, depth) in [(Vec3::new(-30.0, 5.0, 40.0), 90.0, 150.0), (Vec3::new(25.0, -10.0, 30.0), 70.0, 120.0)] {
            for i in 0..8 {
                let k = |bit: i32| if i & bit != 0 { 1.0 } else { -1.0 };
                let corner = c + Vec3::new(k(1) * d / 2.0, k(2) * d / 2.0, if i & 4 != 0 { -depth } else { 0.0 });
                let clip = u.clip_from_world.project_point3(w.transform_point3(corner));
                assert!(clip.x.abs() <= 1.0 && clip.y.abs() <= 1.0 && (-1e-4..=1.0001).contains(&clip.z), "{clip}");
            }
        }
        assert!(u.clip_from_world.abs_diff_eq(u.clip_from_cascade * u.world_from_cascade.inverse(), 1e-4), "{u:?}");
    }
}
