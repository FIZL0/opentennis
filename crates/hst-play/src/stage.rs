//! P61: the match's court from the disc that `main.rs`' loader and the match share (moved from `main.rs`): layout
//! records, models as Bevy meshes, the sky's cloud draws on each new point; the debug orbit camera.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use hst_data::{iso::Iso, layout, mdl, mtl, xb::Archive};

use crate::{Orbit, gs, shade, textures};

/// Court `n`'s entry list and hole-01 placement records.
pub fn court_layout(iso: &mut Iso, n: usize) -> Option<(Vec<layout::Entry>, Vec<layout::Placement>)> {
    let find = |data: &[u8], suffix: &str| {
        let arc = Archive::parse(data).ok()?;
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(suffix))?;
        arc.read(e).ok()
    };
    let (cmn, hol) = (iso.read(&format!("COURT/{n:02}/CMN.XB")).ok()?, iso.read(&format!("COURT/{n:02}/HOL01.XB")).ok()?);
    let list = layout::entries(&String::from_utf8_lossy(&find(&cmn, &format!("entry_c{n:02}.txt"))?));
    Some((list, layout::plants(&find(&hol, &format!("plant_c{n:02}_h01_0.dat"))?).ok()?))
}

/// Every model in an archive whose lower-cased name ends in `.mdl` and passes `keep`, keyed by its
/// lower-case file stem, as one Bevy mesh + material per original material.
pub fn models(data: &[u8], keep: impl Fn(&str) -> bool, images: &mut Assets<Image>) -> Vec<(String, Vec<(Mesh, StandardMaterial)>)> {
    let mut out = Vec::new();
    for_models(data, keep, |model_stem, model, mats| {
        let _pre = textures::prefetch(&mats);
        let tex: Vec<Handle<Image>> = mats.textures.iter().map(|t| textures::add_mtl(images, image(t), t)).collect();
        let mut parts = Vec::new();
        {
            for (mi, packets) in model.materials.iter().enumerate() {
                let Some(mesh) = mesh(packets.iter(), &model) else { continue };
                let mat = mats.materials.get(mi);
                let texture = mat.and_then(|m| m.texture).map(|t| tex[t].clone());
                if let (Some(m), Some(t), Some(h)) = (mat, mat.and_then(|m| m.texture), &texture) {
                    let mut raw = image(&mats.textures[t]);
                    raw.texture_descriptor.format = TextureFormat::Rgba8Unorm;
                    let mut draws = gs::GsMaterial::for_batch(m, packets.first().map_or(0x10, |p| p.prim), Some(textures::add_mtl(images, raw, &mats.textures[t])));
                    draws.iter_mut().for_each(|g| g.uniform.lod_k = model.lod_k.get(mi).copied().unwrap_or(0.0));
                    shade::BALL_GS.lock().unwrap().insert(h.id(), draws);
                }
                let [r, g, b, a] = mat.map_or([1.0; 4], |m| m.color);
                parts.push((
                    mesh,
                    StandardMaterial {
                        base_color: Color::linear_rgba(r, g, b, a),
                        base_color_texture: texture,
                        unlit: true,
                        double_sided: true,
                        cull_mode: None,
                        alpha_mode: AlphaMode::Mask(0.5),
                        ..default()
                    },
                ));
            }
        }
        out.push((model_stem, parts));
    });
    out
}

/// Every model in an archive whose lower-cased name ends in `.mdl` and passes `keep`, with its lower-case file
/// stem and parsed MTL.
pub fn for_models(data: &[u8], keep: impl Fn(&str) -> bool, mut f: impl FnMut(String, mdl::Model, mtl::Mtl)) {
    let arc = Archive::parse(data).expect("xb archive");
    for e in arc.entries.iter().filter(|e| { let n = e.name.to_ascii_lowercase(); n.ends_with(".mdl") && keep(&n) }) {
        let stem = &e.name[..e.name.len() - 4];
        let sib = |ext: &str| arc.find(&format!("{stem}.{ext}")).and_then(|x| arc.read(x).ok());
        let (Ok(model), Some(mtl_bytes)) = (mdl::parse(&arc.read(e).unwrap()), sib("MTL")) else {
            warn!("skipping {}", e.name);
            continue;
        };
        let mats = match mtl::parse(&mtl_bytes, sib("MTI").as_deref()) {
            Ok(m) => m,
            Err(err) => { warn!("{}: {err}", e.name); continue }
        };
        f(stem.rsplit(['\\', '/']).next().unwrap_or(stem).to_ascii_lowercase(), model, mats);
    }
}

/// One triangle-list mesh of these packets; vertex colours stay in PS2 units (0x80 = 1.0).
/// B211: packets are stored in their nodes' spaces and drawn at the model's bind pose: a packet bound to one node by
/// its bind matrix, a weighted one skinned as [`mdl::Model::skinned`] (Queen's ball machine's launcher wheels and
/// balls sat under the grass at the origin; court 6's horses were torn apart).
pub fn mesh<'a>(packets: impl Iterator<Item = &'a mdl::Packet>, model: &mdl::Model) -> Option<Mesh> {
    let (mut pos, mut nrm, mut uv, mut col, mut idx) = (vec![], vec![], vec![], vec![], vec![]);
    for pk in packets {
        let base = pos.len() as u32;
        if pk.palette.len() > 1 {
            let (verts, tris, _) = model.skin_packet(pk);
            for v in &verts {
                pos.push(v.pos);
                nrm.push((Vec3::from(v.normals[0]) + Vec3::from(v.normals[1])).to_array());
                uv.push(v.uv);
                col.push(v.color.map(|c| c as f32 / 128.0));
            }
            idx.extend(tris.iter().flatten().map(|i| i + base));
            continue;
        }
        let m = pk.palette.first().and_then(|&n| model.node_bind.get(n)).map_or(Mat4::IDENTITY, Mat4::from_cols_array_2d);
        for v in &pk.vertices {
            pos.push(m.transform_point3(Vec3::from(v.pos)).to_array());
            nrm.push(m.transform_vector3(Vec3::from(v.normal)).to_array());
            uv.push(v.uv);
            col.push(v.color.map(|c| c as f32 / 128.0)); // PS2 modulate: 0x80 = 1.0
        }
        idx.extend(pk.triangles.iter().flatten().map(|i| i + base));
    }
    if idx.is_empty() {
        return None;
    }
    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_indices(Indices::U32(idx));
    Some(mesh)
}

pub fn image(t: &mtl::Texture) -> Image {
    let mut img = Image::new(
        Extent3d { width: t.width.max(1), height: t.height.max(1), depth_or_array_layers: 1 },
        TextureDimension::D2,
        if t.rgba.is_empty() { vec![255; 4 * (t.width.max(1) * t.height.max(1)) as usize] } else { t.rgba.clone() },
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // PS2 UVs tile freely; Bevy's default sampler clamps
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    img
}

pub fn orbit(
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut q: Query<(&mut Orbit, &mut Transform)>,
) {
    for (mut o, mut t) in &mut q {
        if buttons.pressed(MouseButton::Left) {
            o.yaw -= motion.delta.x * 0.005;
            o.pitch = (o.pitch - motion.delta.y * 0.005).clamp(-1.5, 1.5);
        }
        o.radius *= 1.0 - scroll.delta.y * 0.1;
        *t = o.transform();
    }
}

/// The match's cloud spawner between the setup and the points: the schedule's weathers (`HST_WEATHER` fixes one),
/// the weather the clouds were last made for, the court's count and the heavy-rain one, and the last layout's
/// weather and `rand()` draws until `clouds` lays them out. None when the court has no in-season cloud models.
pub struct Sky {
    pub weathers: [u8; hst_sim::weather::GAMES],
    pub fixed: Option<u8>,
    pub made: u8,
    pub count: usize,
    pub rain: usize,
    pub draws: Option<(u8, Vec<u32>)>,
}

impl Sky {
    pub fn made_count(&self, weather: u8) -> usize {
        if matches!(weather, 3 | 5) { self.rain } else { self.count }
    }
}

pub static SKY: std::sync::Mutex<Option<Sky>> = std::sync::Mutex::new(None);

/// A new point on game `game` of the schedule (`Rngs::new_point`): a weather change remakes the clouds
/// (`weather::clouds_remade`), their draws between the shared and the court generator's seeds, in doubles too.
pub fn point_clouds(rng: &mut hst_sim::rng::Rngs, game: usize) {
    let mut sky = SKY.lock().unwrap();
    let Some(s) = sky.as_mut().filter(|s| {
        let w = s.fixed.unwrap_or(s.weathers[game % hst_sim::weather::GAMES]);
        hst_sim::weather::clouds_remade(s.made, w)
    }) else {
        return rng.new_point();
    };
    let w = s.fixed.unwrap_or(s.weathers[game % hst_sim::weather::GAMES]);
    s.made = w;
    s.draws = Some((w, rng.new_point_clouds(s.made_count(w))));
}

/// What sorts a material's packets into one drawn mesh of a court model: the primitive's shading bits, the `.UVA`
/// track, the UV swap and the unlit flag (`main.rs` builds one mesh per key, [`mesh`] over its packets in order).
pub fn batch_key(p: &mdl::Packet, uv_track: Option<usize>) -> (u8, Option<usize>, bool, u8) {
    (p.prim & 0x70, uv_track, p.uv_swap, p.flags & 2)
}

/// One vertex of a [`mesh`] as its entries: a node (None: the model's own space), the position in that node's space
/// and the weight the node's translation gets (1 for a rigid packet's vertex).
type Source = Vec<(Option<usize>, Vec3, f32)>;

/// [`mesh`]'s vertices over these packets, as their sources (same order).
fn sources<'a>(packets: impl Iterator<Item = &'a mdl::Packet>, model: &mdl::Model) -> Vec<Source> {
    let mut out = Vec::new();
    for pk in packets {
        if pk.palette.len() > 1 {
            out.extend(model.skin_entries(pk).into_iter().map(|v| v.into_iter().map(|(n, p, w)| (Some(n), Vec3::from(p), w)).collect()));
            continue;
        }
        let node = pk.palette.first().copied().filter(|&n| n < model.node_bind.len());
        out.extend(pk.vertices.iter().map(|v| vec![(node, Vec3::from(v.pos), 1.0)]));
    }
    out
}

/// A source posed by node matrices `nodes` (model space).
fn posed(source: &Source, nodes: &[Mat4]) -> Vec3 {
    source.iter().fold(Vec3::ZERO, |acc, &(n, p, w)| match n.and_then(|n| nodes.get(n)) {
        Some(m) => acc + m.transform_vector3(p) + w * m.w_axis.truncate(),
        None => acc + p,
    })
}

/// P135: a placed structure that plays its own motion (court 5's ball-boy carts turn their wheels, court 6's horses):
/// its own copies of the model's meshes, their vertices re-posed from the motion's nodes whenever a step changes them.
#[derive(Component)]
pub struct PropPose {
    motion: hst_sim::court::Motions,
    meshes: Vec<(crate::vertex_patch::Patched, Vec<Source>)>,
}

/// A posed copy of a court model's drawn `parts` (`tags`: each draw's material and first packet, `uv_track` its
/// packets' `.UVA` tracks), spawned under `root` at `t` (as `main.rs` spawns any prop) with `motion` (one of
/// `court::drawn`'s). Each copy is bounded by the box its vertices fill over every frame of the motion. None when the
/// parts don't match the model's packets.
#[allow(clippy::too_many_arguments)]
pub fn spawn_posed(
    commands: &mut Commands,
    root: Entity,
    t: Transform,
    motion: hst_sim::court::Motions,
    model: &mdl::Model,
    uv_track: impl Fn(usize, usize) -> Option<usize>,
    tags: &[(usize, usize)],
    parts: &[(Handle<Mesh>, Handle<gs::GsMaterial>)],
    meshes: &mut Assets<Mesh>,
) -> Option<(Entity, Vec<(Handle<Mesh>, Handle<gs::GsMaterial>)>)> {
    let m = motion.list.first()?;
    // the node matrices at every frame the clock can show
    let frames: Vec<Vec<Mat4>> = (0..m.length().max(1.0) as i32)
        .map(|f| {
            let mut m = m.clone();
            m.sample(f as f32);
            m.world_nodes().iter().map(Mat4::from_cols_array_2d).collect()
        })
        .collect();
    let mut own: Vec<(Handle<Mesh>, Handle<Mesh>, crate::vertex_patch::Patched, Vec<Source>, bevy::camera::primitives::Aabb)> = Vec::new();
    let mut out = Vec::new();
    for ((shared, mat), &(mi, first)) in parts.iter().zip(tags) {
        if let Some((_, copy, ..)) = own.iter().find(|o| &o.0 == shared) {
            out.push((copy.clone(), mat.clone()));
            continue;
        }
        let packets = model.materials.get(mi)?;
        let key = |pi: usize| batch_key(&packets[pi], uv_track(mi, pi));
        let k = key(first);
        let src = sources((0..packets.len()).filter(|&pi| key(pi) == k).map(|pi| &packets[pi]), model);
        let mesh = meshes.get(shared)?.clone();
        if mesh.count_vertices() != src.len() {
            return None;
        }
        let (lo, hi) = frames.iter().flat_map(|f| src.iter().map(|s| posed(s, f))).fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)));
        let patched = crate::vertex_patch::Patched::new(&mesh)?;
        let copy = meshes.add(mesh);
        let patched = patched.of(copy.id());
        out.push((copy.clone(), mat.clone()));
        own.push((shared.clone(), copy, patched, src, bevy::camera::primitives::Aabb::from_min_max(lo, hi)));
    }
    let e = commands.spawn((t, Visibility::default())).id();
    for (m, mat) in &out {
        let aabb = own.iter().find(|o| &o.1 == m).map(|o| o.4).unwrap_or_default();
        commands.entity(e).with_child((Mesh3d(m.clone()), MeshMaterial3d(mat.clone()), aabb, bevy::camera::visibility::NoAutoAabb));
    }
    commands.entity(root).add_child(e);
    commands.entity(e).insert(PropPose { motion, meshes: own.into_iter().map(|o| (o.2, o.3)).collect() });
    Some((e, out))
}

pub fn plugin(app: &mut App) {
    app.add_systems(FixedUpdate, step_props.run_if(|w: &World| !crate::play::front_end::paused(w)));
}

/// One court draw pass a match tick (pauses and slow motion's in-between frames, which run no tick, don't step): each
/// posed structure the eye tests haven't hidden steps its motion, and the vertices it moved go to the GPU.
fn step_props(mut props: Query<(&mut PropPose, &Visibility)>, queued: Option<ResMut<crate::vertex_patch::Queued>>) {
    let Some(mut queued) = queued else { return };
    let mut changed = Vec::new();
    for (mut p, vis) in &mut props {
        let mut nodes = None;
        p.motion.draw_pass(true, |_| *vis == Visibility::Hidden, |m| nodes = Some(m.world_nodes()));
        let Some(nodes) = nodes else { continue };
        let nodes: Vec<Mat4> = nodes.iter().map(Mat4::from_cols_array_2d).collect();
        for (patched, src) in &mut p.meshes {
            changed.clear();
            changed.extend(src.iter().enumerate().filter_map(|(v, s)| patched.set(v, posed(s, &nodes).to_array()).then_some(v)));
            crate::vertex_patch::send(patched, &changed, &mut queued);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hst_data::{fixture, xb::Archive};

    /// P135: court 5's four ball-boy carts and court 6's five horses (all structures) play their motions; each
    /// posed vertex at the motion's first sample-free pose is where [`mesh`] put it, and the motion moves some.
    #[test]
    fn posed_props_start_at_the_drawn_mesh_and_move() {
        let Ok(mut iso) = fixture::iso() else {
            fixture::skip(format_args!("disc missing, skipped"));
            return;
        };
        for (n, stems, count) in [(5, &["ballboy01"][..], 4), (6, &["k-horse", "k-horse2"][..], 5)] {
            let drawn = hst_sim::court::drawn(&mut iso, n, 0);
            assert_eq!(drawn.len(), count, "court {n}");
            let data = iso.read(&format!("COURT/{n:02}/HOL01.XB")).unwrap();
            assert!(Archive::parse(&data).is_ok());
            let mut seen = 0;
            for_models(&data, |s| stems.iter().any(|t| s.rsplit(['\\', '/']).next() == Some(&format!("{t}.mdl"))), |stem, model, _| {
                seen += 1;
                let (_, m) = drawn.iter().find(|(_, m)| {
                    let w = m.list[0].world_nodes();
                    w.len() == model.node_count && w.iter().zip(&model.node_bind).all(|(a, b)| a.as_flattened().iter().zip(b.as_flattened()).all(|(x, y)| (x - y).abs() < 1e-5))
                }).expect("the model's motion");
                let mut m = m.list[0].clone();
                let bind: Vec<Mat4> = m.world_nodes().iter().map(Mat4::from_cols_array_2d).collect();
                m.sample(m.length() / 2.0);
                let half: Vec<Mat4> = m.world_nodes().iter().map(Mat4::from_cols_array_2d).collect();
                let mut moved = 0;
                for packets in &model.materials {
                    let Some(mesh) = mesh(packets.iter(), &model) else { continue };
                    let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { panic!() };
                    let src = sources(packets.iter(), &model);
                    assert_eq!(src.len(), pos.len(), "{stem}");
                    for (s, p) in src.iter().zip(pos) {
                        assert!(posed(s, &bind).distance(Vec3::from(*p)) < 1e-4, "{stem}: {:?} vs {p:?}", posed(s, &bind));
                        moved += (posed(s, &half).distance(Vec3::from(*p)) > 1e-3) as usize;
                    }
                }
                assert!(moved > 0, "{stem} moves");
            });
            assert_eq!(seen, stems.len(), "court {n}");
        }
    }
}
