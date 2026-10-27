//! Character Shading (Graphics, ours: the PS2 has none): soft contact shade under a chin, a skirt, an arm, baked
//! once per model into its vertex colours, so the GS colour path (`gs.wgsl`) darkens every light by it.
//!
//! Each vertex casts cosine-weighted rays over its normal's hemisphere against every triangle of the model in its
//! bind pose (all parts together: a skirt shades the legs); the share that reaches 20 cm is its visibility. A ray
//! whose nearest hit is a face's back has gone into another part (a leg inside its shoe): it counts as open, and a
//! vertex most of whose rays do that is hidden inside, so it stays unshaded (its black would bleed up the triangle).
//! A face's side comes from its vertices' normals (normal + tangent slots, `gs.wgsl`'s skinned normal), not its
//! winding. Rest pose only: the shade doesn't follow the motion.

use crate::character::Store;
use bevy::mesh::{Indices, VertexAttributeValues as V};
use bevy::prelude::*;
use std::sync::atomic::Ordering;

const RAYS: usize = 32;
/// How far a crease shades, in metres.
const REACH: f32 = 0.2;

struct Tri {
    a: Vec3,
    e1: Vec3,
    e2: Vec3,
    /// its vertices' normals summed: which side is the front
    n: Vec3,
    lo: Vec3,
    hi: Vec3,
}

/// Shade `parts`' vertex colours when Character Shading is on (`graphics::SHADING`).
pub fn bake<'a>(meshes: &mut impl Store<Mesh>, parts: impl IntoIterator<Item = &'a Handle<Mesh>>) {
    if !crate::graphics::SHADING.load(Ordering::Relaxed) {
        return;
    }
    let parts: Vec<_> = parts.into_iter().collect();
    // every part's vertices (position, normal) and triangles, in the model's space
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    for h in &parts {
        let Some(m) = meshes.get_mut(*h) else { continue };
        let (Some(V::Float32x3(pos)), Some(V::Float32x3(nrm))) = (m.attribute(Mesh::ATTRIBUTE_POSITION), m.attribute(Mesh::ATTRIBUTE_NORMAL)) else { continue };
        let n: Vec<Vec3> = match m.attribute(Mesh::ATTRIBUTE_TANGENT) {
            Some(V::Float32x4(t)) => nrm.iter().zip(t).map(|(n, t)| Vec3::from(*n) + Vec4::from(*t).truncate()).collect(),
            _ => nrm.iter().map(|n| Vec3::from(*n)).collect(),
        };
        let p: Vec<Vec3> = pos.iter().map(|p| Vec3::from(*p)).collect();
        let idx: Vec<usize> = match m.indices() {
            Some(Indices::U32(i)) => i.iter().map(|&i| i as usize).collect(),
            Some(Indices::U16(i)) => i.iter().map(|&i| i as usize).collect(),
            None => continue,
        };
        tris.extend(idx.chunks_exact(3).map(|t| {
            let (a, b, c) = (p[t[0]], p[t[1]], p[t[2]]);
            Tri { a, e1: b - a, e2: c - a, n: n[t[0]] + n[t[1]] + n[t[2]], lo: a.min(b).min(c), hi: a.max(b).max(c) }
        }));
        verts.extend(p.into_iter().zip(n));
    }
    let ao = visibility(&verts, &tris);
    let mut at = 0;
    for h in parts {
        let Some(m) = meshes.get_mut(h) else { continue };
        let count = m.count_vertices();
        if let Some(V::Float32x4(col)) = m.attribute_mut(Mesh::ATTRIBUTE_COLOR) {
            for (c, k) in col.iter_mut().zip(&ao[at..at + count]) {
                c[0] *= k;
                c[1] *= k;
                c[2] *= k;
            }
        }
        at += count;
    }
}

/// Each vertex's share of open rays, spread over the CPU's cores.
fn visibility(verts: &[(Vec3, Vec3)], tris: &[Tri]) -> Vec<f32> {
    #[cfg(target_arch = "wasm32")]
    return verts.iter().map(|&(p, n)| vertex(p, n, tris)).collect();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
        let chunk = verts.len().div_ceil(threads).max(1);
        std::thread::scope(|s| {
            let jobs: Vec<_> = verts.chunks(chunk).map(|c| s.spawn(move || c.iter().map(|&(p, n)| vertex(p, n, tris)).collect::<Vec<_>>())).collect();
            jobs.into_iter().flat_map(|j| j.join().unwrap()).collect()
        })
    }
}

fn vertex(p: Vec3, n: Vec3, tris: &[Tri]) -> f32 {
    let Some(n) = n.try_normalize() else { return 1.0 };
    let o = p + n * 0.003;
    // only the triangles within reach
    let near: Vec<&Tri> = tris.iter().filter(|t| (o.clamp(t.lo, t.hi) - o).length_squared() < REACH * REACH).collect();
    let (t, b) = n.any_orthonormal_pair();
    let (mut open, mut inside) = (0, 0);
    for i in 0..RAYS {
        // cosine-weighted: a Fibonacci spiral over the disc, lifted onto the hemisphere
        let r = ((i as f32 + 0.5) / RAYS as f32).sqrt();
        let a = i as f32 * 2.399_963;
        let d = (t * r * a.cos() + b * r * a.sin() + n * (1.0 - r * r).sqrt()).normalize();
        let mut nearest: Option<(f32, bool)> = None;
        for tri in &near {
            let q = d.cross(tri.e2);
            let det = tri.e1.dot(q);
            if det.abs() < 1e-9 {
                continue;
            }
            let s = o - tri.a;
            let u = s.dot(q) / det;
            let r = s.cross(tri.e1);
            let v = d.dot(r) / det;
            let h = tri.e2.dot(r) / det;
            if u >= 0.0 && v >= 0.0 && u + v <= 1.0 && h > 1e-4 && h < REACH && nearest.is_none_or(|(x, _)| h < x) {
                nearest = Some((h, d.dot(tri.n) > 0.0));
            }
        }
        match nearest {
            None => open += 1,
            Some((_, true)) => inside += 1,
            Some((_, false)) => {}
        }
    }
    if inside * 2 > RAYS {
        return 1.0;
    }
    (open + inside) as f32 / RAYS as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri(a: [f32; 3], b: [f32; 3], c: [f32; 3], up: Vec3) -> Tri {
        let (a, b, c) = (Vec3::from(a), Vec3::from(b), Vec3::from(c));
        Tri { a, e1: b - a, e2: c - a, n: up * 3.0, lo: a.min(b).min(c), hi: a.max(b).max(c) }
    }

    #[test]
    fn open_shaded_and_inside() {
        // a 1 m ceiling 5 cm above the origin, facing down
        let roof = [tri([-1.0, 0.05, -1.0], [1.0, 0.05, -1.0], [0.0, 0.05, 1.0], Vec3::NEG_Y)];
        assert_eq!(vertex(Vec3::ZERO, Vec3::NEG_Y, &roof), 1.0, "facing away");
        assert!(vertex(Vec3::ZERO, Vec3::Y, &roof) < 0.2, "under it");
        assert_eq!(vertex(Vec3::new(0.0, -0.5, 0.0), Vec3::Y, &roof), 1.0, "out of reach");
        // the same roof facing up: the vertex is inside whatever it bounds
        let lid = [tri([-1.0, 0.05, -1.0], [1.0, 0.05, -1.0], [0.0, 0.05, 1.0], Vec3::Y)];
        assert_eq!(vertex(Vec3::ZERO, Vec3::Y, &lid), 1.0, "inside");
    }
}
