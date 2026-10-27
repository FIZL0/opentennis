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
use bevy::light::NotShadowCaster;
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
    .add_systems(PostUpdate, top_halves.before(VisibilitySystems::VisibilityPropagate));
}

/// The second view's camera, where it is and its lens, while it's on (its owner sets it each frame).
#[derive(Resource, Default)]
pub struct SecondEye(pub Option<(Entity, Transform, PerspectiveProjection)>);

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
