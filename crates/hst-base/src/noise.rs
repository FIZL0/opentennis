//! Costume noise (`hst_sim::noise`): a costume's `.NOI` deformers sway its hair and cloth with the wind. The
//! game moves each packet's position entries in their bones' spaces before skinning, so a moved entry carries its
//! offset d through its bone's posed matrix: the vertex moves by Σ R·d. Bevy skins the bind-pose mesh, so each frame
//! the drawn vertex gets the bind-space offset its skin maps onto that: (Σ w·R·bind⁻¹)⁻¹ · Σ R·d, in the rig's own
//! copy of the part's mesh.

use std::sync::{Arc, LazyLock};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use hst_data::{mdl, noi};
use hst_sim::noise::{self, Noise};

use crate::character::{Motion, Rig};
use crate::vertex_patch::{self, Patched, Queued};
use crate::weather::Weather;

static TABLE: LazyLock<[f32; 32]> = LazyLock::new(noise::table);

/// A position entry a deformer moves: its bone-space position, its share of the noise and its bone (joint).
pub struct Entry {
    pub deformer: usize,
    pub p: [f32; 3],
    pub weight: f32,
    pub node: usize,
}

/// A moved drawn vertex: its index in the part's mesh, its bind-pose position, its skin (joints and weights as the
/// mesh carries them) and its position entries.
pub struct Moved {
    pub v: usize,
    pub base: [f32; 3],
    pub skin: [(usize, f32); 4],
    pub entries: Vec<Entry>,
}

/// A costume's deformers, and per swaying part (index into `CharacterData::parts`) its moved drawn vertices.
pub struct Costume {
    pub deformers: Vec<noi::Deformer>,
    pub parts: Vec<(usize, Vec<Moved>)>,
    /// A mod's spring joints (standard §3b, `crate::spring`); none on the disc.
    pub springs: Vec<crate::spring::Chain>,
    /// Their collision ellipsoids (§3c): joint, matrix in its space.
    pub colliders: Vec<(usize, bevy::math::Mat4)>,
}

/// The costume's deformers over the parts `character::skinned_parts` built from `model` (one per material with
/// triangles, in material order). The swaying parts' meshes stay readable, so each rig can copy them.
pub fn costume(model: &mdl::Model, noi: &[u8], parts: &[(Handle<Mesh>, Handle<StandardMaterial>, bool)], meshes: &mut impl crate::character::Store<Mesh>) -> Option<Arc<Costume>> {
    let deformers = noi::parse(noi)?;
    let packets: Vec<_> = model.materials.iter().flatten().zip(model.skinned()).collect();
    let mut drawn: Vec<usize> = packets.iter().filter(|(_, s)| !s.2.is_empty()).map(|(_, s)| s.0).collect();
    drawn.sort();
    drawn.dedup();
    let mut base = vec![0usize; model.materials.len()];
    let mut out: Vec<(usize, Vec<_>)> = Vec::new();
    for (pk, (material, verts, ..)) in packets {
        let first = base[material];
        base[material] += verts.len();
        let (Some(d), Some(part)) = (deformers.iter().position(|d| d.node as i16 == pk.group), drawn.iter().position(|&m| m == material)) else { continue };
        if pk.noise.is_empty() {
            continue;
        }
        let moved = (0..verts.len()).filter_map(|v| {
            let entries: Vec<Entry> = (pk.bones[v] as usize..(pk.bones[v + 1] as usize).min(pk.vertices.len()))
                .filter(|&e| pk.noise.get(e).is_some_and(|&w| w != 0.0))
                .map(|e| {
                    let node = pk.palette.get((pk.entry_flags.get(e).copied().unwrap_or(0) >> 3 & 7) as usize).or(pk.palette.first()).copied().unwrap_or(0);
                    Entry { deformer: d, p: pk.vertices[e].pos, weight: pk.noise[e], node }
                })
                .collect();
            let skin = std::array::from_fn(|k| (verts[v].joints[k] as usize, verts[v].weights[k]));
            (!entries.is_empty()).then_some(Moved { v: first + v, base: verts[v].pos, skin, entries })
        });
        match out.iter_mut().find(|(p, _)| *p == part) {
            Some((_, m)) => m.extend(moved),
            None => out.push((part, moved.collect())),
        }
    }
    for (part, _) in &out {
        if let Some(mesh) = meshes.get_mut(&parts[*part].0) {
            mesh.asset_usage = RenderAssetUsages::default();
        }
    }
    (!out.is_empty()).then(|| Arc::new(Costume { deformers, parts: out, springs: Vec::new(), colliders: Vec::new() }))
}

/// A rig's deformers and its own copies of the swaying parts' meshes; the motion set last seen. B344: the deformers
/// step once a tick, the pose every drawn frame, and the drawn vertices of a strip share position entries, so each
/// distinct entry (deformer, position, weight) is moved once per step: per swaying part, each moved vertex's
/// entries' indices (in order) into `distinct`, and their bone-space offsets as the deformers stood at `offsets_at`.
#[derive(Component)]
pub struct Sway {
    noise: Vec<Noise>,
    /// Each copy's handle (kept alive) and its vertices as the GPU holds them (B357: patched, never `get_mut`).
    meshes: Vec<(Handle<Mesh>, Option<Patched>)>,
    sets: Option<u32>,
    slots: Vec<Vec<u32>>,
    distinct: Vec<(usize, [f32; 3], f32)>,
    offsets: Vec<Vec3>,
    offsets_at: Vec<Noise>,
    /// P40: the motion moved (a tick, or a driver set it) since the sway was last drawn. The noise only steps a
    /// tick, so between ticks (frames drawn above 60 Hz) the drawn vertices stay put: their bind-space offsets
    /// only move with the pose through the blend of a vertex's skin, under a millimetre over one tick's
    /// interpolation.
    stale: bool,
    /// `lod::REBUILT` when the copies' vertices were last all written.
    rebuilt: u32,
}

/// The costume's distinct entries, and per swaying part its moved vertices' entries' indices into them.
fn distinct(c: &Costume) -> (Vec<Vec<u32>>, Vec<(usize, [f32; 3], f32)>) {
    let mut at = std::collections::HashMap::new();
    let mut distinct = Vec::new();
    let slots = c.parts.iter().map(|(_, moved)| {
        moved.iter().flat_map(|m| &m.entries).map(|e| {
            *at.entry((e.deformer, e.p.map(f32::to_bits), e.weight.to_bits())).or_insert_with(|| {
                distinct.push((e.deformer, e.p, e.weight));
                distinct.len() as u32 - 1
            })
        }).collect()
    }).collect();
    (slots, distinct)
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (attach, recopy.after(attach).before(draw), draw.after(crate::character::animate))).add_systems(FixedUpdate, step.after(crate::character::tick));
}

/// A new rig with a costume deformer swaps its swaying parts for copies of its own. Court Life (M157) off sways
/// nothing: switched off, each rig gets its shared parts back; switched on, each rig gets its copies, mid-match too.
pub fn attach(mut commands: Commands, g: Option<Res<crate::graphics::Graphics>>, rigs: Query<(Entity, Ref<Rig>, Option<&Sway>)>, kids: Query<&Children>, mut parts: Query<&mut Mesh3d>, mut meshes: ResMut<Assets<Mesh>>) {
    let on = g.as_ref().is_none_or(|g| g.scenery);
    let toggled = g.as_ref().is_some_and(|g| g.is_changed());
    for (root, rig, sway) in &rigs {
        if !toggled && !rig.is_added() {
            continue;
        }
        let Some(c) = &rig.data.noise else { continue };
        // every draw of the part under the rig (a second GS draw is a sibling with the same mesh, `shade`)
        let mut swap = |from: &Handle<Mesh>, to: &Handle<Mesh>| {
            for e in kids.iter_descendants(root) {
                if let Ok(mut m) = parts.get_mut(e)
                    && m.0 == *from
                {
                    m.0 = to.clone();
                }
            }
        };
        match (on, sway) {
            (false, Some(s)) => {
                for ((part, _), (copy, _)) in c.parts.iter().zip(&s.meshes) {
                    swap(copy, &rig.data.parts[*part].0);
                }
                commands.entity(root).try_remove::<Sway>();
            }
            (true, None) => {
                let mut own = Vec::new();
                for (part, _) in &c.parts {
                    let shared = rig.data.parts[*part].0.clone();
                    let Some(copy) = meshes.get(&shared).cloned() else { continue };
                    let patched = Patched::new(&copy);
                    let copy = meshes.add(copy);
                    crate::lod::copied(shared.id(), copy.id());
                    let patched = patched.map(|p| p.of(copy.id()));
                    swap(&shared, &copy);
                    own.push((copy, patched));
                }
                let noise = c.deformers.iter().map(|d| Noise::new(d.period)).collect();
                // the select may have despawned a preview rig in the same frame (B84)
                let (slots, distinct) = distinct(c);
                commands.entity(root).try_insert(Sway { noise, meshes: own, sets: None, slots, distinct, offsets: Vec::new(), offsets_at: Vec::new(), stale: true, rebuilt: crate::lod::REBUILT.load(std::sync::atomic::Ordering::Relaxed) });
            }
            _ => {}
        }
    }
}

/// Character Shading flipped (`occlusion::generation`): each rig copies its swaying parts again from the rebuilt
/// shared meshes.
fn recopy(mut seen: Local<u32>, mut rigs: Query<(Entity, &Rig, &mut Sway)>, kids: Query<&Children>, mut parts: Query<&mut Mesh3d>, mut meshes: ResMut<Assets<Mesh>>) {
    let generation = crate::occlusion::generation();
    if std::mem::replace(&mut *seen, generation) == generation {
        return;
    }
    for (root, rig, mut s) in &mut rigs {
        let Some(c) = &rig.data.noise else { continue };
        let s = &mut *s;
        for ((part, _), (copy, patched)) in c.parts.iter().zip(&mut s.meshes) {
            let Some(mesh) = meshes.get(&rig.data.parts[*part].0).cloned() else { continue };
            let p = Patched::new(&mesh);
            let new = meshes.add(mesh);
            crate::lod::copied(rig.data.parts[*part].0.id(), new.id());
            for e in kids.iter_descendants(root) {
                if let Ok(mut m) = parts.get_mut(e)
                    && m.0 == *copy
                {
                    m.0 = new.clone();
                }
            }
            *patched = p.map(|p| p.of(new.id()));
            *copy = new;
        }
        s.stale = true;
    }
}

/// Every tick: a motion set (a restart of the same motion too) restarts the deformers, then they step with the
/// wind (1.0 off court, as the game boots).
fn step(weather: Option<Res<Weather>>, mut rigs: Query<(&Rig, &Motion, &mut Sway)>) {
    let wind = weather.map_or(1.0, |w| noise::wind(w.today().speed));
    for (rig, motion, mut s) in &mut rigs {
        let Some(c) = &rig.data.noise else { continue };
        let reset = s.sets != Some(motion.sets);
        s.sets = Some(motion.sets);
        for (n, d) in s.noise.iter_mut().zip(&c.deformers) {
            if reset {
                n.reset(0.0, d.rate, d.amp[0], wind);
            }
            n.step(1.0, d.rate, d.amp[0], wind);
        }
    }
}

/// Every drawn frame, after the pose: each moved vertex from the previous phase, its entries' offsets carried
/// through their bones as posed.
pub fn draw(mut rigs: Query<(&Rig, Ref<Motion>, &mut Sway)>, joints: Query<&Transform>, mut queued: ResMut<Queued>) {
    let mut changed = Vec::new();
    for (rig, motion, mut s) in &mut rigs {
        let Some(c) = &rig.data.noise else { continue };
        // Graphics → Mesh rewrote the copies (`lod::live`): the GPU holds their unswayed vertices again
        let rebuilt = crate::lod::REBUILT.load(std::sync::atomic::Ordering::Relaxed);
        if s.rebuilt != rebuilt {
            s.rebuilt = rebuilt;
            s.meshes.iter().filter_map(|m| m.1.as_ref()).for_each(|p| vertex_patch::send_all(p, &mut queued));
        }
        if motion.is_changed() {
            s.stale = true;
        }
        if !s.stale && s.offsets_at == s.noise {
            continue;
        }
        // the entries' offsets from the previous phase, again only after a step
        if s.offsets_at != s.noise {
            let s = &mut *s;
            s.offsets = s.distinct.iter().map(|&(d, p, weight)| {
                let n = &s.noise[d];
                Vec3::from(noise::deform(&TABLE, p, weight, n.freq, n.prev, n.amp)) - Vec3::from(p)
            }).collect();
            s.offsets_at.clone_from(&s.noise);
        }
        s.stale = false;
        // each joint's posed model matrix (its parents' first)
        let mut pose: Vec<Option<Mat4>> = vec![None; rig.joints.len()];
        fn posed(j: usize, rig: &Rig, joints: &Query<&Transform>, pose: &mut [Option<Mat4>]) -> Mat4 {
            if let Some(m) = pose[j] {
                return m;
            }
            let local = joints.get(rig.joints[j]).map_or(Mat4::IDENTITY, |t| t.to_matrix());
            let m = rig.data.joints[j].parent.map_or(Mat4::IDENTITY, |p| posed(p, rig, joints, pose)) * local;
            pose[j] = Some(m);
            m
        }
        // each joint's posed rotation, and that times its inverse bind's (a skin's term), once per joint
        let mut rots: Vec<Option<(Mat3, Mat3)>> = vec![None; rig.joints.len()];
        let mut rot = |j: usize| {
            *rots[j].get_or_insert_with(|| {
                let r = Mat3::from_mat4(posed(j, rig, &joints, &mut pose));
                (r, r * Mat3::from_mat4(rig.data.joints[j].inverse_bind))
            })
        };
        let s = &mut *s;
        for (((_, moved), (_, patched)), slots) in c.parts.iter().zip(&mut s.meshes).zip(&s.slots) {
            // only the moved vertices go to the GPU, written over the mesh's place in the vertex buffer: `get_mut`
            // re-uploads the whole mesh (B328, B357)
            let Some(patched) = patched else { continue };
            changed.clear();
            let mut slots = slots.iter();
            // the last skin's inverse (a strip's neighbours mostly share their skin), None if singular
            let mut last: Option<([(usize, f32); 4], Option<Mat3>)> = None;
            for m in moved {
                let mut off = Vec3::ZERO;
                for (e, &k) in m.entries.iter().zip(&mut slots) {
                    off += rot(e.node).0 * s.offsets[k as usize];
                }
                let inverse = match last {
                    Some((skin, inverse)) if skin == m.skin => inverse,
                    _ => {
                        let skin: Mat3 = m.skin.iter().map(|&(j, w)| rot(j).1 * w).sum();
                        let inverse = (skin.determinant() != 0.0).then(|| skin.inverse());
                        last = Some((m.skin, inverse));
                        inverse
                    }
                };
                if let Some(inverse) = inverse
                    && patched.set(m.v, (Vec3::from(m.base) + inverse * off).into())
                {
                    changed.push(m.v);
                }
            }
            changed.sort_unstable();
            vertex_patch::send(patched, &changed, &mut queued);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::Graphics;
    use bevy::mesh::skinning::SkinnedMeshInverseBindposes;

    /// Court Life switched mid-match: off gives a swaying rig its shared parts back, on gives it its own copies again.
    #[test]
    fn court_life_switches_the_sway_live() {
        let Ok(mut iso) = hst_data::fixture::iso() else { return };
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>().insert_resource(Graphics::DEFAULT).add_systems(Update, attach);
        let (mut t, mut i, mut b) = (Assets::<StandardMaterial>::default(), Assets::<Image>::default(), Assets::<SkinnedMeshInverseBindposes>::default());
        let data = (0..10).find_map(|n| {
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            crate::character::load_disc_with(&mut iso, n, 0, &mut *meshes, &mut t, &mut i, &mut b, Some(&[])).ok().filter(|c| c.noise.is_some())
        });
        let Some(data) = data.map(Arc::new) else { return eprintln!("no swaying costume, skipped") };
        let swaying = data.noise.as_ref().unwrap().parts.len();
        let root = app.world_mut().spawn(Rig { data: data.clone(), joints: Vec::new() }).id();
        let parts: Vec<Entity> = data.parts.iter().map(|(m, ..)| app.world_mut().spawn((Mesh3d(m.clone()), ChildOf(root))).id()).collect();
        // the parts drawn with a mesh of the rig's own
        let own = |app: &App| parts.iter().filter(|&&e| !data.parts.iter().any(|p| p.0 == app.world().get::<Mesh3d>(e).unwrap().0)).count();
        let mut check = |on: bool| {
            app.world_mut().resource_mut::<Graphics>().scenery = on;
            app.update();
            assert_eq!((app.world().get::<Sway>(root).is_some(), own(&app)), (on, if on { swaying } else { 0 }), "Court Life {on}");
        };
        check(true);
        check(false);
        check(true);
    }
}
