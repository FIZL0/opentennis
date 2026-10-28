//! B691: a mod character's face morphed on the CPU on Apple GPUs. On the iPhone build (Metal) the other-game
//! characters' faces came out swollen, over their fringe and necklace, so the court showed through their heads where
//! those thin layers should be: Bevy's storage-buffer morphs read the wrong targets or weights there, as B137a found
//! for the effects in Safari on the iPhone. Here the face part's mesh carries no morph targets; each rig draws its own
//! copy, whose moving vertices (base + Σ weight · target) are written into the vertex buffer when the root's weights
//! change (`vertex_patch`, B357: no re-upload). Disc characters keep Bevy's morphs.

use crate::vertex_patch::{self, Patched, Queued};
use bevy::mesh::morph::MorphWeights;
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Whether mod faces morph here: on Apple platforms, or as `HST_CPU_MORPH` (1/0) says.
pub fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| match std::env::var("HST_CPU_MORPH").ok().as_deref() {
        Some("1") => true,
        Some("0") => false,
        _ => cfg!(target_vendor = "apple"),
    })
}

/// A face mesh's morph targets: the vertices any target moves, and per target their offsets (in that order).
#[derive(Debug, PartialEq)]
pub struct Targets {
    moving: Vec<u32>,
    offsets: Vec<Vec<Vec3>>,
}

impl Targets {
    /// From each target's offset of every vertex (as the morph weights order them).
    pub fn new(targets: &[Vec<Vec3>]) -> Targets {
        let count = targets.iter().map(Vec::len).max().unwrap_or(0);
        let moving: Vec<u32> = (0..count).filter(|&v| targets.iter().any(|t| t.get(v).is_some_and(|d| *d != Vec3::ZERO))).map(|v| v as u32).collect();
        let offsets = targets.iter().map(|t| moving.iter().map(|&v| t.get(v as usize).copied().unwrap_or(Vec3::ZERO)).collect()).collect();
        Targets { moving, offsets }
    }

    /// Each moving vertex (index, position) at `weights`, from the unmorphed positions `base`.
    pub fn morph<'a>(&'a self, base: &'a [Vec3], weights: &'a [f32]) -> impl Iterator<Item = (usize, Vec3)> + 'a {
        self.moving.iter().enumerate().map(move |(k, &v)| {
            let p = self.offsets.iter().zip(weights).filter(|(_, w)| **w != 0.0).fold(base.get(v as usize).copied().unwrap_or(Vec3::ZERO), |p, (t, &w)| p + t[k] * w);
            (v as usize, p)
        })
    }
}

#[derive(Default)]
struct Registry {
    /// The shared face meshes (a mod's, as loaded) and their targets.
    faces: HashMap<AssetId<Mesh>, Arc<Targets>>,
    /// Each rig's copy → the shared mesh it copies.
    copies: HashMap<AssetId<Mesh>, AssetId<Mesh>>,
}

fn registry() -> std::sync::MutexGuard<'static, Registry> {
    static R: OnceLock<Mutex<Registry>> = OnceLock::new();
    R.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
}

/// Keep `mesh`'s targets (the mesh itself carries none).
pub fn keep(mesh: AssetId<Mesh>, targets: Targets) {
    registry().faces.insert(mesh, Arc::new(targets));
}

/// The targets kept for `mesh`, if it is a face morphed here.
pub fn targets(mesh: AssetId<Mesh>) -> Option<Arc<Targets>> {
    if !on() {
        return None;
    }
    registry().faces.get(&mesh).cloned()
}

/// The mesh a part draws as the character's data holds it: a rig's own face copy is its shared face's.
pub fn shared(mesh: AssetId<Mesh>) -> AssetId<Mesh> {
    if !on() {
        return mesh;
    }
    registry().copies.get(&mesh).copied().unwrap_or(mesh)
}

/// A face part morphed here, from its rig root's `MorphWeights`.
#[derive(Component)]
pub struct Face {
    root: Entity,
    shared: AssetId<Mesh>,
    targets: Arc<Targets>,
    /// The rig's own copy (kept alive), its unmorphed positions and its vertices as the GPU holds them.
    copy: Option<(Handle<Mesh>, Vec<Vec3>, Patched)>,
    last: Vec<f32>,
}

impl Face {
    pub fn new(root: Entity, shared: AssetId<Mesh>, targets: Arc<Targets>) -> Face {
        Face { root, shared, targets, copy: None, last: Vec::new() }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(PostUpdate, draw);
}

/// Give each face its rig's own copy (on every draw of it: the GS's second draws are siblings with the same mesh),
/// then write the moving vertices when the weights changed.
fn draw(mut faces: Query<&mut Face>, weights: Query<&MorphWeights>, children: Query<&Children>, mut parts: Query<&mut Mesh3d>, mut meshes: ResMut<Assets<Mesh>>, queued: Option<ResMut<Queued>>) {
    let Some(mut queued) = queued else { return };
    let mut changed = Vec::new();
    for mut f in &mut faces {
        if f.copy.is_none() {
            let Some(mesh) = meshes.get(f.shared).cloned() else { continue };
            let base = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|p| p.as_float3()).unwrap_or_default().iter().map(|p| Vec3::from(*p)).collect();
            let Some(patched) = Patched::new(&mesh) else { continue };
            let h = meshes.add(mesh);
            let mut r = registry();
            // forget the meshes gone since (an unloaded mod's, a despawned rig's copy)
            r.faces.retain(|id, _| meshes.contains(*id));
            r.copies.retain(|id, _| meshes.contains(*id));
            r.copies.insert(h.id(), f.shared);
            drop(r);
            f.copy = Some((h.clone(), base, patched.of(h.id())));
        }
        let f = &mut *f;
        let Some((copy, base, patched)) = &mut f.copy else { continue };
        for &c in children.get(f.root).into_iter().flatten() {
            if parts.get(c).is_ok_and(|m| m.0.id() == f.shared)
                && let Ok(mut m) = parts.get_mut(c)
            {
                m.0 = copy.clone();
            }
        }
        let Ok(w) = weights.get(f.root) else { continue };
        if f.last == w.weights() {
            continue;
        }
        f.last.clear();
        f.last.extend_from_slice(w.weights());
        changed.clear();
        for (v, p) in f.targets.morph(base, w.weights()) {
            if patched.set(v, p.to_array()) {
                changed.push(v);
            }
        }
        vertex_patch::send(patched, &changed, &mut queued);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_only_the_targets_vertices_by_their_weights() {
        let t = Targets::new(&[vec![Vec3::ZERO, Vec3::X, Vec3::ZERO], vec![Vec3::ZERO, Vec3::Y, Vec3::Z]]);
        assert_eq!(t.moving, [1, 2]);
        let base = [Vec3::ONE, Vec3::ONE * 2.0, Vec3::ONE * 3.0];
        let got: Vec<_> = t.morph(&base, &[0.5, 2.0]).collect();
        assert_eq!(got, [(1, Vec3::new(2.5, 4.0, 2.0)), (2, Vec3::new(3.0, 3.0, 5.0))]);
        let rest: Vec<_> = t.morph(&base, &[0.0, 0.0]).collect();
        assert_eq!(rest, [(1, base[1]), (2, base[2])]);
    }
}
