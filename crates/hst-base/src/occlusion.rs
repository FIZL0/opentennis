//! Character Shading (Graphics, ours: the PS2 has none): soft contact shade under a chin, a skirt, an arm, baked
//! once per model into its vertex colours, so the GS colour path (`gs.wesl`) darkens every light by it.
//!
//! Each vertex casts cosine-weighted rays over its normal's hemisphere against every triangle of the model in its
//! bind pose (all parts together: a skirt shades the legs); the share that reaches 20 cm is its visibility. A ray
//! whose nearest hit is a face's back has gone into another part (a leg inside its shoe): it counts as open, and a
//! vertex most of whose rays do that is hidden inside, so it stays unshaded (its black would bleed up the triangle).
//! A face's side comes from its vertices' normals (normal + tangent slots, `gs.wesl`'s skinned normal), not its
//! winding. A hit shades less the further it is, so a cap's brim dims a face softly instead of in blotches; then
//! vertices at one spot (seams, part joins) share one value and a few passes smooth it over the mesh's edges. The
//! rays walk a bounding-volume tree of the triangles, so a mod's 50k-triangle model bakes in a blink.
//! Rest pose only: the shade doesn't follow the motion. A face (a part with morph targets: face, teeth, tongue)
//! doesn't shade itself: its lips' and eyelids' crease shade would smear across it as the expression changes; the
//! cap, hair and collar still shade it.
//!
//! The setting takes effect at once on models already loaded: every model is baked whatever the setting, and each
//! part's unshaded mesh is kept (the main world drops it once drawn, `RenderAssetUsages::RENDER_WORLD`) so [`follow`]
//! can put it back with or without the shade when the setting flips. A rig's own copy of a part (a swaying
//! costume's, a CPU-morphed face's) is copied again from the new mesh when [`generation`] moves on.

use crate::character::Store;
use bevy::mesh::{Indices, VertexAttributeValues as V};
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

const RAYS: usize = 32;
/// How far a crease shades, in metres.
const REACH: f32 = 0.2;
/// Smoothing passes over the mesh's edges.
const SMOOTH: usize = 2;

#[derive(Clone, Copy)]
struct Tri {
    a: Vec3,
    e1: Vec3,
    e2: Vec3,
    /// its vertices' normals summed: which side is the front
    n: Vec3,
    lo: Vec3,
    hi: Vec3,
    /// a face's (a part with morph targets)
    face: bool,
}

/// A baked part: its vertices' visibility and its unshaded colours; its whole mesh as built once [`snapshot`] has
/// seen it in the world (with the unshaded colours), and whether the mesh in the world is shaded.
struct Baked {
    ao: Vec<f32>,
    colours: Vec<[f32; 4]>,
    mesh: Option<Mesh>,
    shaded: bool,
}

type Registry = HashMap<AssetId<Mesh>, Baked>;

fn registry() -> MutexGuard<'static, Registry> {
    static R: OnceLock<Mutex<Registry>> = OnceLock::new();
    R.get_or_init(Mutex::default).lock().unwrap_or_else(|e| e.into_inner())
}

static GENERATION: AtomicU32 = AtomicU32::new(0);
/// Parts baked that [`snapshot`] hasn't kept yet.
static WAITING: AtomicBool = AtomicBool::new(false);

/// Moves on whenever baked meshes in the world were rebuilt with or without their shade: a rig's own copies of them
/// are to be copied again.
pub fn generation() -> u32 {
    GENERATION.load(Ordering::Relaxed)
}

fn shading() -> bool {
    crate::graphics::SHADING.load(Ordering::Relaxed)
}

/// Bake `parts`' shade, and shade their vertex colours when Character Shading is on (`graphics::SHADING`).
pub fn bake<'a>(meshes: &mut impl Store<Mesh>, parts: impl IntoIterator<Item = &'a Handle<Mesh>>) {
    bake_into(&mut registry(), meshes, parts, shading());
    WAITING.store(true, Ordering::Relaxed);
}

fn bake_into<'a>(reg: &mut Registry, meshes: &mut impl Store<Mesh>, parts: impl IntoIterator<Item = &'a Handle<Mesh>>, on: bool) {
    let parts: Vec<_> = parts.into_iter().collect();
    // every part's vertices (position, normal) and triangles, in the model's space
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    let mut edges = Vec::new();
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
        let base = verts.len();
        edges.extend(idx.as_chunks::<3>().0.iter().flat_map(|t| [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])]).map(|(a, b)| (base + a, base + b)));
        let face = m.has_morph_targets() || crate::cpu_morph::targets(h.id()).is_some();
        tris.extend(idx.as_chunks::<3>().0.iter().map(|t| {
            let (a, b, c) = (p[t[0]], p[t[1]], p[t[2]]);
            Tri { a, e1: b - a, e2: c - a, n: n[t[0]] + n[t[1]] + n[t[2]], lo: a.min(b).min(c), hi: a.max(b).max(c), face }
        }));
        verts.extend(p.into_iter().zip(n).map(|(p, n)| (p, n, face)));
    }
    let mut ao = visibility(&verts, &Bvh::new(tris));
    smooth(&mut ao, &verts, &edges);
    let mut at = 0;
    for h in parts {
        let Some(m) = meshes.get_mut(h) else { continue };
        let count = m.count_vertices();
        let ao = &ao[at..at + count];
        at += count;
        if let Some(V::Float32x4(col)) = m.attribute_mut(Mesh::ATTRIBUTE_COLOR) {
            reg.insert(h.id(), Baked { ao: ao.to_vec(), colours: col.clone(), mesh: None, shaded: on });
            if on {
                shade(col, ao);
            }
        }
    }
}

fn shade(col: &mut [[f32; 4]], ao: &[f32]) {
    for (c, k) in col.iter_mut().zip(ao) {
        c[0] *= k;
        c[1] *= k;
        c[2] *= k;
    }
}

/// Puts `id`'s mesh as built back in the world, shaded or not, with the usage the world's has (a swaying part's
/// stays readable). True if it did.
fn rebuild(id: AssetId<Mesh>, b: &mut Baked, on: bool, meshes: &mut Assets<Mesh>) -> bool {
    let Some(mut m) = b.mesh.clone() else { return false };
    if let Some(now) = meshes.get(id) {
        m.asset_usage = now.asset_usage;
        // Graphics → Mesh may have reduced the triangles since the copy was kept (lod::live)
        if let Some(i) = now.indices() {
            m.insert_indices(i.clone());
        }
    }
    if on && let Some(V::Float32x4(col)) = m.attribute_mut(Mesh::ATTRIBUTE_COLOR) {
        shade(col, &b.ao);
    }
    b.shaded = on;
    meshes.insert(id, m).is_ok()
}

/// Each frame's end, before the render world takes the new meshes' data: keeps each part baked since, unshaded, and
/// puts the setting right on one baked before it flipped.
pub fn snapshot(meshes: Option<ResMut<Assets<Mesh>>>) {
    let Some(mut meshes) = meshes else { return };
    if !WAITING.swap(false, Ordering::Relaxed) {
        return;
    }
    let mut reg = registry();
    if snapshot_in(&mut reg, &mut meshes, shading()) {
        GENERATION.fetch_add(1, Ordering::Relaxed);
    }
    if reg.values().any(|b| b.mesh.is_none()) {
        WAITING.store(true, Ordering::Relaxed);
    }
}

/// True if a mesh in the world was rebuilt.
fn snapshot_in(reg: &mut Registry, meshes: &mut Assets<Mesh>, on: bool) -> bool {
    // forget the meshes gone (an unloaded character's) and the ones the render world took before they were kept;
    // one not in the world yet (a load still staged) waits
    reg.retain(|id, b| match (&b.mesh, meshes.get(*id)) {
        (Some(_), now) => now.is_some(),
        (None, Some(now)) => now.try_attribute_option(Mesh::ATTRIBUTE_POSITION).is_ok(),
        (None, None) => true,
    });
    let mut any = false;
    for (&id, b) in reg.iter_mut().filter(|(_, b)| b.mesh.is_none()) {
        let Some(m) = meshes.get(id) else { continue };
        let mut m = m.clone();
        if let Some(V::Float32x4(col)) = m.attribute_mut(Mesh::ATTRIBUTE_COLOR) {
            col.clone_from(&b.colours);
        }
        b.colours = Vec::new();
        b.mesh = Some(m);
        if b.shaded != on {
            any |= rebuild(id, b, on, meshes);
        }
    }
    any
}

/// After `graphics::mesh_detail`: when Character Shading flips, every kept part in the world is rebuilt with or
/// without its shade.
pub fn follow(meshes: Option<ResMut<Assets<Mesh>>>, mut last: Local<Option<bool>>) {
    let Some(mut meshes) = meshes else { return };
    let on = shading();
    if last.replace(on) == Some(on) {
        return;
    }
    if follow_in(&mut registry(), &mut meshes, on) {
        GENERATION.fetch_add(1, Ordering::Relaxed);
    }
}

/// True if a mesh in the world was rebuilt.
fn follow_in(reg: &mut Registry, meshes: &mut Assets<Mesh>, on: bool) -> bool {
    reg.retain(|id, b| b.mesh.is_none() || meshes.contains(*id));
    let mut any = false;
    for (&id, b) in reg.iter_mut().filter(|(_, b)| b.mesh.is_some() && b.shaded != on) {
        any |= rebuild(id, b, on, meshes);
    }
    any
}

/// Welds vertices at one spot (to 0.1 mm) to their mean, then averages each with its edge neighbours.
fn smooth(ao: &mut [f32], verts: &[(Vec3, Vec3, bool)], edges: &[(usize, usize)]) {
    let mut spot: std::collections::HashMap<[i32; 3], (f32, f32)> = Default::default();
    let keys: Vec<[i32; 3]> = verts.iter().map(|(p, ..)| (*p * 1e4).round().as_ivec3().to_array()).collect();
    for (k, v) in keys.iter().zip(ao.iter()) {
        let e = spot.entry(*k).or_default();
        *e = (e.0 + v, e.1 + 1.0);
    }
    // the passes run over spots, so a seam's two sides stay one value
    let mut val: std::collections::HashMap<[i32; 3], f32> = spot.into_iter().map(|(k, (s, n))| (k, s / n)).collect();
    for _ in 0..SMOOTH {
        let mut sum: std::collections::HashMap<[i32; 3], (f32, f32)> = val.iter().map(|(k, v)| (*k, (*v, 1.0))).collect();
        for &(a, b) in edges {
            let (ka, kb) = (keys[a], keys[b]);
            let (va, vb) = (val[&ka], val[&kb]);
            sum.entry(ka).and_modify(|e| *e = (e.0 + vb, e.1 + 1.0));
            sum.entry(kb).and_modify(|e| *e = (e.0 + va, e.1 + 1.0));
        }
        val = sum.into_iter().map(|(k, (s, n))| (k, s / n)).collect();
    }
    ao.iter_mut().zip(&keys).for_each(|(a, k)| *a = val[k]);
}

/// The model's triangles in a bounding-volume tree (median splits), so a ray tests only the few it can reach.
struct Bvh {
    tris: Vec<Tri>,
    /// (lo, hi, first, count): a leaf's triangles, or with count 0 an inner node whose children are `first`, `first + 1`
    nodes: Vec<(Vec3, Vec3, usize, usize)>,
}

impl Bvh {
    fn new(mut tris: Vec<Tri>) -> Self {
        let mut nodes = if tris.is_empty() { vec![] } else { vec![(Vec3::ZERO, Vec3::ZERO, 0, tris.len())] };
        let mut todo = vec![0];
        while let Some(i) = todo.pop().filter(|_| !nodes.is_empty()) {
            let (_, _, first, count) = nodes[i];
            let span = &mut tris[first..first + count];
            let (lo, hi) = span.iter().fold((Vec3::MAX, Vec3::MIN), |(l, h), t| (l.min(t.lo), h.max(t.hi)));
            nodes[i] = (lo, hi, first, count);
            if count <= 4 {
                continue;
            }
            let axis = (hi - lo).max_position();
            let half = count / 2;
            span.select_nth_unstable_by(half, |a, b| (a.lo + a.hi)[axis].total_cmp(&(b.lo + b.hi)[axis]));
            let kid = nodes.len();
            nodes[i] = (lo, hi, kid, 0);
            nodes.extend([(lo, hi, first, half), (lo, hi, first + half, count - half)]);
            todo.extend([kid, kid + 1]);
        }
        Bvh { tris, nodes }
    }

    /// The nearest triangle within reach along `d` from `o`: its distance, and whether the ray met its back.
    /// `face`: skip faces' triangles.
    fn cast(&self, o: Vec3, d: Vec3, face: bool) -> Option<(f32, bool)> {
        let inv = d.recip();
        let mut nearest: Option<(f32, bool)> = None;
        // ponytail: fixed stack; median splits keep the depth near log2(triangles / 4), far under 64
        let mut stack = [0usize; 64];
        let mut depth = usize::from(!self.nodes.is_empty());
        while depth > 0 {
            depth -= 1;
            let (lo, hi, first, count) = self.nodes[stack[depth]];
            let reach = nearest.map_or(REACH, |(h, _)| h);
            let (t1, t2) = ((lo - o) * inv, (hi - o) * inv);
            if t1.min(t2).max_element().max(0.0) > t2.max(t1).min_element().min(reach) {
                continue;
            }
            if count == 0 {
                stack[depth] = first;
                stack[depth + 1] = first + 1;
                depth += 2;
                continue;
            }
            for tri in &self.tris[first..first + count] {
                if face && tri.face {
                    continue;
                }
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
                if u >= 0.0 && v >= 0.0 && u + v <= 1.0 && h > 1e-4 && h < nearest.map_or(REACH, |(x, _)| x) {
                    nearest = Some((h, d.dot(tri.n) > 0.0));
                }
            }
        }
        nearest
    }
}

/// Each vertex's share of open rays, spread over the CPU's cores.
fn visibility(verts: &[(Vec3, Vec3, bool)], bvh: &Bvh) -> Vec<f32> {
    #[cfg(target_arch = "wasm32")]
    return verts.iter().map(|&(p, n, face)| vertex(p, n, face, bvh)).collect();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
        let chunk = verts.len().div_ceil(threads).max(1);
        std::thread::scope(|s| {
            let jobs: Vec<_> = verts.chunks(chunk).map(|c| s.spawn(move || c.iter().map(|&(p, n, face)| vertex(p, n, face, bvh)).collect::<Vec<_>>())).collect();
            jobs.into_iter().flat_map(|j| j.join().unwrap()).collect()
        })
    }
}

/// `face`: the vertex is a face's, which its own face doesn't shade.
fn vertex(p: Vec3, n: Vec3, face: bool, bvh: &Bvh) -> f32 {
    let Some(n) = n.try_normalize() else { return 1.0 };
    let o = p + n * 0.003;
    let (t, b) = n.any_orthonormal_pair();
    let (mut open, mut inside) = (0.0, 0);
    for i in 0..RAYS {
        // cosine-weighted: a Fibonacci spiral over the disc, lifted onto the hemisphere
        let r = ((i as f32 + 0.5) / RAYS as f32).sqrt();
        let a = i as f32 * 2.399_963;
        let d = (t * r * a.cos() + b * r * a.sin() + n * (1.0 - r * r).sqrt()).normalize();
        match bvh.cast(o, d, face) {
            None => open += 1.0,
            Some((_, true)) => inside += 1,
            // a far hit shades less
            Some((h, false)) => open += (h / REACH).powi(2),
        }
    }
    if inside * 2 > RAYS {
        return 1.0;
    }
    (open + inside as f32) / RAYS as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri(a: [f32; 3], b: [f32; 3], c: [f32; 3], up: Vec3) -> Tri {
        let (a, b, c) = (Vec3::from(a), Vec3::from(b), Vec3::from(c));
        Tri { a, e1: b - a, e2: c - a, n: up * 3.0, lo: a.min(b).min(c), hi: a.max(b).max(c), face: false }
    }

    #[test]
    fn setting_flips_on_a_loaded_model() {
        // a floor with a low roof over its middle, drawn-only (RENDER_WORLD) like a character's part
        use bevy::asset::RenderAssetUsages as U;
        let mut meshes = Assets::<Mesh>::default();
        let colours = vec![[0.25, 0.5, 0.75, 1.0]; 6];
        let floor = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, U::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[-0.01, 0.0, -0.01], [0.01, 0.0, -0.01], [0.0, 0.0, 0.01], [-1.0, 0.03, -1.0], [0.0, 0.03, 1.0], [1.0, 0.03, -1.0]])
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0]])
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colours.clone())
            .with_inserted_indices(Indices::U32(vec![0, 1, 2, 3, 4, 5]));
        let h = meshes.add(floor);
        let col = |meshes: &Assets<Mesh>| match meshes.get(&h).unwrap().attribute(Mesh::ATTRIBUTE_COLOR) {
            Some(V::Float32x4(c)) => c.clone(),
            _ => panic!("no colours"),
        };
        let mut reg = Registry::new();
        // baked with the setting on, flipped off before the frame's end, then on and off again
        bake_into(&mut reg, &mut meshes, [&h], true);
        let shaded = col(&meshes);
        assert!(shaded[0][0] < colours[0][0], "under the roof: {shaded:?}");
        assert!(snapshot_in(&mut reg, &mut meshes, false));
        assert_eq!(col(&meshes), colours);
        assert_eq!(meshes.get(&h).unwrap().asset_usage, U::RENDER_WORLD);
        assert!(!follow_in(&mut reg, &mut meshes, false), "nothing to do");
        assert!(follow_in(&mut reg, &mut meshes, true));
        assert_eq!(col(&meshes), shaded);
        assert!(follow_in(&mut reg, &mut meshes, false));
        assert_eq!(col(&meshes), colours, "the colours as loaded, exactly");
        // gone from the world: forgotten
        meshes.remove(&h);
        assert!(!follow_in(&mut reg, &mut meshes, true));
        assert!(reg.is_empty());
    }

    #[test]
    fn smooth_welds_and_blends() {
        // two copies of one spot (a seam) and a neighbour along an edge
        let v = |x: f32| (Vec3::new(x, 0.0, 0.0), Vec3::Y, false);
        let mut ao = [0.0, 1.0, 1.0];
        smooth(&mut ao, &[v(0.0), v(0.0), v(1.0)], &[(0, 2)]);
        assert_eq!(ao[0], ao[1], "seam shares one value");
        assert!(ao[0] > 0.5 && ao[2] < 1.0, "{ao:?}");
    }

    #[test]
    fn open_shaded_and_inside() {
        // a 1 m ceiling 5 cm above the origin, facing down
        let roof = Bvh::new(vec![tri([-1.0, 0.05, -1.0], [1.0, 0.05, -1.0], [0.0, 0.05, 1.0], Vec3::NEG_Y)]);
        assert_eq!(vertex(Vec3::ZERO, Vec3::NEG_Y, false, &roof), 1.0, "facing away");
        assert!(vertex(Vec3::ZERO, Vec3::Y, false, &roof) < 0.3, "under it");
        assert_eq!(vertex(Vec3::new(0.0, -0.5, 0.0), Vec3::Y, false, &roof), 1.0, "out of reach");
        // the same roof facing up: the vertex is inside whatever it bounds
        let lid = Bvh::new(vec![tri([-1.0, 0.05, -1.0], [1.0, 0.05, -1.0], [0.0, 0.05, 1.0], Vec3::Y)]);
        assert_eq!(vertex(Vec3::ZERO, Vec3::Y, false, &lid), 1.0, "inside");
    }

    #[test]
    fn tree_finds_what_every_triangle_finds() {
        // a soup of small triangles in a 40 cm box, cast at from points inside it
        let mut seed = 1u32;
        let mut f = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1 << 24) as f32 - 0.5
        };
        let mut v = || Vec3::new(f(), f(), f());
        let tris: Vec<Tri> = (0..3000)
            .map(|i| {
                let a = v() * 0.4;
                let mut t = tri(a.into(), (a + v() * 0.05).into(), (a + v() * 0.05).into(), v());
                t.face = i % 7 == 0;
                t
            })
            .collect();
        let rays: Vec<_> = (0..2000).map(|i| (v() * 0.4, v().normalize(), i % 3 == 0)).collect();
        // one leaf: every triangle tested
        let every = Bvh { tris: tris.clone(), nodes: vec![(Vec3::splat(-1.0), Vec3::splat(1.0), 0, tris.len())] };
        let want: Vec<_> = rays.iter().map(|&(o, d, face)| every.cast(o, d, face)).collect();
        let bvh = Bvh::new(tris);
        let got: Vec<_> = rays.iter().map(|&(o, d, face)| bvh.cast(o, d, face)).collect();
        assert!(want.iter().filter(|w| w.is_some()).count() > 200, "the soup is hit");
        assert_eq!(got, want);
    }
}
