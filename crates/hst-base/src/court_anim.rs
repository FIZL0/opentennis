//! Court hole animation ([`hst_sim::court_anim`]) on the hole's GS materials, a frame per 60 Hz tick.

use bevy::camera::{primitives::Aabb, visibility::NoAutoAabb};
use bevy::prelude::*;

use crate::gs::GsMaterial;
use crate::vertex_patch::{self, Patched, Queued};

#[derive(Component)]
pub struct Playing {
    pub anim: hst_sim::court_anim::CourtAnim,
    /// Each drawn material with the (material, packet) its offset and alpha come from.
    pub parts: Vec<(Handle<GsMaterial>, (usize, usize))>,
}

/// The meshes a [`Playing`]'s `.MOR` moves (the model's shared library meshes: every placed copy moves with them).
#[derive(Component)]
pub struct Morph {
    pub meshes: Vec<MorphMesh>,
}

/// A morphing mesh: its vertices at rest, per morph target the offsets of the vertices it moves (model space), and
/// a box over every place they reach.
pub struct MorphMesh {
    id: AssetId<Mesh>,
    patched: Patched,
    rest: Vec<Vec3>,
    targets: Vec<Vec<(usize, Vec3)>>,
    moving: Vec<usize>,
    aabb: Aabb,
}

impl MorphMesh {
    /// `mesh` (the asset `id`) with `offsets`: per morph target, each vertex's offset at weight 1. None when it moves
    /// nothing or its positions can't be patched.
    pub fn new(mesh: &Mesh, id: AssetId<Mesh>, offsets: &[Vec<[f32; 3]>]) -> Option<MorphMesh> {
        let rest: Vec<Vec3> = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?.iter().map(|&p| Vec3::from(p)).collect();
        let targets: Vec<Vec<(usize, Vec3)>> =
            offsets.iter().map(|t| t.iter().enumerate().filter(|(v, d)| *v < rest.len() && **d != [0.0; 3]).map(|(v, &d)| (v, Vec3::from(d))).collect()).collect();
        let mut moving: Vec<usize> = targets.iter().flatten().map(|&(v, _)| v).collect();
        moving.sort_unstable();
        moving.dedup();
        if moving.is_empty() {
            return None;
        }
        // ponytail: the box holds each target at weight 0 and 1, enough for weights in 0..=1 that sum to at most 1
        // (the disc's tracks cross-fade one target into the next)
        let reach = rest.iter().copied().chain(targets.iter().flatten().map(|&(v, d)| rest[v] + d));
        let (lo, hi) = reach.fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)));
        Some(MorphMesh { id, patched: Patched::new(mesh)?.of(id), rest, targets, moving, aabb: Aabb::from_min_max(lo, hi) })
    }

    pub fn id(&self) -> AssetId<Mesh> {
        self.id
    }
}

pub fn plugin(app: &mut App) {
    app.insert_resource(Time::<Fixed>::from_hz(60.0)).add_systems(FixedUpdate, (tick, morph.after(tick))).add_systems(PostUpdate, bound);
}

/// Each morphing mesh's vertices at this frame's weights, to the GPU where they moved.
fn morph(playing: Query<(&Playing, &mut Morph)>, queued: Option<ResMut<Queued>>) {
    let Some(mut queued) = queued else { return };
    let mut changed = Vec::new();
    let mut pos = Vec::new();
    for (p, mut m) in playing {
        let w = p.anim.morph_weights();
        for mesh in &mut m.meshes {
            pos.clear();
            pos.extend(mesh.moving.iter().map(|&v| mesh.rest[v]));
            for (t, target) in mesh.targets.iter().enumerate() {
                let Some(&w) = w.get(t).filter(|w| **w != 0.0) else { continue };
                for &(v, d) in target {
                    let k = mesh.moving.binary_search(&v).unwrap();
                    pos[k] += w * d;
                }
            }
            changed.clear();
            for (&v, p) in mesh.moving.iter().zip(&pos) {
                if mesh.patched.set(v, p.to_array()) {
                    changed.push(v);
                }
            }
            vertex_patch::send(&mesh.patched, &changed, &mut queued);
        }
    }
}

/// A drawn morphing mesh is bounded by the box its vertices reach, not where they rest.
fn bound(mut commands: Commands, added: Query<(Entity, &Mesh3d), Added<Mesh3d>>, morphs: Query<&Morph>) {
    if morphs.is_empty() {
        return;
    }
    for (e, m) in &added {
        if let Some(mesh) = morphs.iter().flat_map(|m| &m.meshes).find(|x| x.id() == m.id()) {
            commands.entity(e).insert((mesh.aabb, NoAutoAabb));
        }
    }
}

fn tick(mut playing: Query<&mut Playing>, mut materials: ResMut<Assets<GsMaterial>>) {
    for mut p in &mut playing {
        let p = &mut *p;
        p.anim.tick();
        for (h, (m, pk)) in &p.parts {
            let Some(mut g) = materials.get_mut(h) else { continue };
            g.uniform.uv_offset = Vec2::from(p.anim.uv_offset(*m, *pk));
            g.uniform.color.w = p.anim.alphas[*m];
        }
    }
}
