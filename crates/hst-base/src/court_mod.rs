//! M8f6: a court mod (`mods_court`) seen and heard in place of its layout court. `load` sets the layout court up as
//! ever (collision, judging, weather, the court generators' draws, the people) but spawns its models under a hidden
//! [`DiscCourt`]; here the mod's `court.glb`, `sky.glb` (standing at the eye like the disc skies) and `gimmick.glb`
//! (each animation looping on its own clock) are drawn in the layout court's game space, its manifest light is baked
//! into their vertex colours, its sun casts the shadows, and its BGM and ambience loops play in place of the layout
//! court's music and creature sounds. A `__clear` node isn't drawn while it stands between the eye and the court
//! (M8f5a3, `clear_view`); a gimmick creature standing in the players' room isn't drawn at all (B506,
//! `creatures_in_play`), and a big piece of the court or its gimmicks right in front of the camera isn't drawn
//! (M129, gs.wgsl's `GS_CLEAR`), nor is a whole gimmick one of whose pieces comes right up in front of the lens
//! (B510, [`NearPiece`]); none of it while the intro flies over (B583, `cull::Off`). Nothing here reads or writes the match: replays stay bit-exact.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::image::{CompressedImageFormats, ImageAddressMode, ImageSampler, ImageSamplerDescriptor, ImageType};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use hst_data::mtl;

use crate::audio::{CourtBank, Group, MusicBanks, Sound, SoundBank, Track};
use crate::gs::{GsMaterial, NO_FOG};
use crate::mods_court::{Court, Light, MatchCourt};

/// The layout court's own models, kept but not drawn.
#[derive(Component)]
pub struct DiscCourt;

/// The court mod's BGM bank while its match runs, for the pause menu's music toggle (`bgm`).
static BGM: Mutex<Option<Arc<SoundBank>>> = Mutex::new(None);

/// The match's court mod has no lens flare (`play::flare` draws none).
pub static FLARE_OFF: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The PSP's lighting saturates at full colour (vertex colour 1.0 = ×1 here, as the GS's 0x80).
const MAX_LIGHT: f32 = 1.0;

/// The match's court mod, if any: its systems (the menu's and the command line's matches).
pub fn add(app: &mut App, court: Option<Court>) {
    *BGM.lock().unwrap() = None;
    FLARE_OFF.store(false, std::sync::atomic::Ordering::Relaxed);
    let Some(court) = court else { return };
    app.insert_resource(MatchCourt(court))
        .init_resource::<Kept>()
        .add_systems(PostStartup, setup)
        .add_systems(PreUpdate, sounds)
        .add_systems(Update, (keep.after(crate::weather::apply), animate, slide))
        .add_systems(PostUpdate, (clear_view.in_set(crate::shadow::EyePasses).before(bevy::transform::TransformSystems::Propagate), clear_off))
        .add_systems(PostUpdate, bare.after(bevy::ui::UiSystems::Layout).before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate));
}

/// B401 `HST_COURT_PREVIEW=1`: no 2D on screen (the intro's plate, the HUD, the key legend), for
/// `tools/court_previews.sh`'s `previews/court.png` shots of the intro's flyover (the scene's own nodes, on their
/// cameras, stay).
fn bare(mut ui: Query<&mut Visibility, (With<Node>, Without<ChildOf>, Without<bevy::ui::UiTargetCamera>)>) {
    static ON: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| std::env::var("HST_COURT_PREVIEW").is_ok_and(|v| v == "1"));
    if *ON {
        ui.iter_mut().for_each(|mut v| {
            v.set_if_neq(Visibility::Hidden);
        });
    }
}

/// The pause menu turned the match music back on: the court mod's BGM in place of the layout court's.
pub fn bgm(bgm: &mut Option<(Arc<SoundBank>, Vec<u8>)>) {
    if let Some(b) = BGM.lock().unwrap().clone() {
        *bgm = Some((b, Vec::new()));
    }
}

/// How a court mod material is lit: `Sky` unlit and unfogged (as the disc skies); `Baked` with the manifest's light
/// in its vertex colours and its fog; `Weather` lit and fogged as the layout court is (no manifest light).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Role {
    Sky,
    Baked,
    Weather,
}

/// The court mod's materials and how each is lit, re-applied over each weather change (`keep`).
#[derive(Resource, Default)]
pub struct Kept {
    materials: Vec<(AssetId<GsMaterial>, Role)>,
    fog: Option<(Vec4, Vec4)>,
    /// The manifest's `scroll` speeds by material name, and the materials that slide at them (`slide`).
    speeds: Vec<(String, Vec2)>,
    scroll: Vec<(AssetId<GsMaterial>, Vec2)>,
}

/// A glTF animation looping on its own clock: per channel the node's entity, its times and its values.
#[derive(Component)]
pub struct Looped {
    pub length: f32,
    channels: Vec<Channel>,
}

/// M8f5a3: a `__clear` set piece (standard §2): its box in game space at rest and what to hide while it stands
/// between the eye and the court (its node, and its skinned meshes, which hang elsewhere).
#[derive(Component)]
pub struct ClearView {
    lo: Vec3,
    hi: Vec3,
    hide: Vec<Entity>,
}

/// B510: a placed gimmick (`gimmick.glb`, not `__clear`): its meshes' boxes in game space at rest that stand over
/// 0.5 m up (its ground shadows left out) and what to hide while one of them comes right up in front of the drawn
/// camera (its node, and its skinned meshes, which hang elsewhere), as B303 does for the disc courts' props
/// (`play::net_hide`). Dubno's helicopter stands just past the doubles sideline, where a post-point camera following
/// a player who ran out wide ends up inside its cabin.
#[derive(Component)]
pub struct NearPiece {
    pub boxes: Vec<(Vec3, Vec3)>,
    pub hide: Vec<Entity>,
}

/// M8f7: what `scene` spawned for its caller to drive: the animations by name (not left looping) and the named nodes.
#[derive(Default)]
pub struct Spawned {
    pub clips: Vec<(String, Looped)>,
    pub nodes: Vec<(String, Entity)>,
}

struct Channel {
    node: Entity,
    times: Vec<f32>,
    values: Values,
    step: bool,
}

enum Values {
    Translation(Vec<Vec3>),
    Rotation(Vec<Quat>),
    Scale(Vec<Vec3>),
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    court: Res<MatchCourt>,
    args: Res<crate::Args>,
    root: Query<Entity, With<crate::GameSpace>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<GsMaterial>>,
    mut binds: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut kept: ResMut<Kept>,
    sun: Option<Res<crate::shadow::Sun>>,
) {
    let Ok(root) = root.single() else { return };
    let c = &court.0;
    kept.speeds.clone_from(&c.scroll);
    let mut store = Store { meshes: &mut meshes, images: &mut images, materials: &mut materials, binds: &mut binds, kept: &mut kept, upscaled: None };
    for (file, sky) in [("court.glb", false), ("gimmick.glb", false), ("sky.glb", true)] {
        let path = c.dir.join(file);
        if !crate::modfs::is_file(&path) {
            continue;
        }
        store.upscaled = crate::mods::upscaled_in(&c.dir, &path);
        let parent = if sky {
            // stands at the eye like the disc skies (`backdrops`)
            commands.spawn((crate::Backdrop { yaw: 0.0, lift: 0.0 }, Transform::default(), Visibility::default(), ChildOf(root))).id()
        } else {
            root
        };
        match crate::mods::open(&path).and_then(|(doc, blob)| scene_off(&mut commands, &mut store, &doc, &blob, parent, args.singles, if sky { None } else { c.light.as_ref() }, sky, None, file == "gimmick.glb", !sky)) {
            Ok(n) => info!("court mod {}: {file}, {n} draws", c.id),
            Err(e) => warn!("court mod {}: {}: {e}", c.id, path.display()),
        }
    }
    kept.fog = c.light.as_ref().map(|l| l.fog.as_ref().map_or((NO_FOG, Vec4::ONE), |f| (Vec4::new(255.0 * (1.0 - f.amount[0]), 255.0 * (1.0 - f.amount[1]), f.near, f.far), f.colour.extend(1.0))));
    // the layout court's ball and figure shading comes from its own (unseen) casters: none here
    commands.remove_resource::<crate::shade::Shade>();
    // the shadows fall from the manifest's sun (raised to at least 50°, as the game's)
    if let (Some(l), Some(sun)) = (&c.light, sun) {
        let light = -l.sun;
        let flat = Vec2::new(light.x, light.z);
        let lowest = 50f32.to_radians();
        let dir = if light.y >= lowest.sin() || flat.length() < 1e-6 { light } else { (flat.normalize() * lowest.cos()).extend(lowest.sin()).xzy() };
        commands.insert_resource(crate::shadow::Sun { dir, light, axes: hst_sim::shade::sun_axes([dir.x, dir.y, dir.z, 0.0]), ..*sun });
    }
    // the disc's flare stands for the mod's sprite at the manifest's sun; a court mod without one has none
    FLARE_OFF.store(c.lens_flare.is_none(), std::sync::atomic::Ordering::Relaxed);
}

pub struct Store<'a> {
    pub meshes: &'a mut Assets<Mesh>,
    pub images: &'a mut Assets<Image>,
    pub materials: &'a mut Assets<GsMaterial>,
    pub binds: &'a mut Assets<SkinnedMeshInverseBindposes>,
    pub kept: &'a mut Kept,
    /// M110: the `.glb`'s `textures/upscaled/<stem>/` (`mods::upscaled_in`), drawn in place of its images.
    pub upscaled: Option<std::path::PathBuf>,
}

/// Spawns a court mod `.glb`'s scene under `parent` in the layout court's game space (the scene's `game_space` node
/// undone), leaving out the other match kind's `__singles`/`__doubles` nodes; its animations loop, or with `spawned`
/// go there by name with the named nodes. Returns the draws.
#[allow(clippy::too_many_arguments)]
pub fn scene(commands: &mut Commands, s: &mut Store, doc: &gltf::Document, blob: &[u8], parent: Entity, singles: bool, light: Option<&Light>, sky: bool, spawned: Option<&mut Spawned>) -> Result<usize, String> {
    scene_off(commands, s, doc, blob, parent, singles, light, sky, spawned, false, false)
}

/// [`scene`]; with `off_court` (`gimmick.glb`), the creatures standing in the players' room ([`creatures_in_play`]) aren't
/// spawned (B506) and each placed piece gets its [`NearPiece`] (B510);
/// with `clear`, its big blockers aren't drawn (M129, `GsKey::clear`).
#[allow(clippy::too_many_arguments)]
fn scene_off(commands: &mut Commands, s: &mut Store, doc: &gltf::Document, blob: &[u8], parent: Entity, singles: bool, light: Option<&Light>, sky: bool, mut spawned: Option<&mut Spawned>, off_court: bool, clear: bool) -> Result<usize, String> {
    let worlds = worlds(doc);
    let space = doc.nodes().find(|n| n.name() == Some("game_space")).map_or(Mat4::IDENTITY, |n| worlds[n.index()]);
    let to_game = space.inverse();
    let role = if sky { Role::Sky } else if light.is_some() { Role::Baked } else { Role::Weather };
    let mats = materials(doc, blob, s, role, clear)?;
    let anchor = commands.spawn((Transform::from_matrix(to_game), Visibility::default(), ChildOf(parent))).id();
    let skip = |n: &gltf::Node| n.name().is_some_and(|m| m.ends_with(if singles { "__doubles" } else { "__singles" }));
    // the node tree as entities (an animation or a skin moves them)
    let mut nodes = HashMap::new();
    let off = if off_court { creatures_in_play(doc, blob, to_game, singles) } else { Default::default() };
    // each `__clear` node's subtree: the node it's under (M8f5a3)
    let clear_node = |n: &gltf::Node| n.name().is_some_and(|m| m.trim_end_matches("__singles").trim_end_matches("__doubles").ends_with("__clear"));
    let mut clear_of = HashMap::new();
    let mut stack: Vec<(gltf::Node, Entity, Option<usize>)> = doc.default_scene().or_else(|| doc.scenes().next()).ok_or("no scene")?.nodes().map(|n| (n, anchor, None)).collect();
    while let Some((n, up, clear)) = stack.pop() {
        if skip(&n) {
            continue;
        }
        let clear = clear.or(clear_node(&n).then_some(n.index()));
        if let Some(c) = clear {
            clear_of.insert(n.index(), c);
        }
        let (t, r, sc) = n.transform().decomposed();
        let e = commands.spawn((Transform { translation: Vec3::from(t), rotation: Quat::from_array(r), scale: Vec3::from(sc) }, Visibility::default(), ChildOf(up))).id();
        nodes.insert(n.index(), e);
        if let (Some(sp), Some(name)) = (spawned.as_deref_mut(), n.name()) {
            sp.nodes.push((name.to_string(), e));
        }
        stack.extend(n.children().map(|c| (c, e, clear)));
    }
    let mut clears: HashMap<usize, ClearView> = HashMap::new();
    let piece_of = if off_court { placed_pieces(doc) } else { HashMap::new() };
    let mut near: HashMap<usize, NearPiece> = HashMap::new();
    let mut draws = 0;
    for n in doc.nodes().filter(|n| nodes.contains_key(&n.index())) {
        let Some(mesh) = n.mesh().filter(|_| !off.contains(&n.index())) else { continue };
        let skin = n.skin().filter(|sk| sk.joints().all(|j| nodes.contains_key(&j.index())));
        // the skin's joints in game space at bind (to bake the light), and Bevy's skin
        let (rest, skinned) = match &skin {
            Some(sk) => {
                let ibm = bind_inverses(sk, blob);
                let rest: Vec<Mat4> = sk.joints().zip(&ibm).map(|(j, b)| to_game * worlds[j.index()] * *b).collect();
                let joints = sk.joints().map(|j| nodes[&j.index()]).collect();
                (rest, Some(SkinnedMesh { inverse_bindposes: s.binds.add(SkinnedMeshInverseBindposes::from(ibm)), joints }))
            }
            None => (vec![to_game * worlds[n.index()]], None),
        };
        // the `__clear` piece it belongs to: a rigid mesh's node, or all of a skin's joints, under it
        let clear = match &skin {
            Some(sk) => sk.joints().map(|j| clear_of.get(&j.index()).copied()).reduce(|a, b| a.filter(|&a| Some(a) == b)).flatten(),
            None => clear_of.get(&n.index()).copied(),
        };
        let piece = match &skin {
            Some(sk) => sk.joints().map(|j| piece_of.get(&j.index()).copied()).reduce(|a, b| a.filter(|&a| Some(a) == b)).flatten(),
            None => piece_of.get(&n.index()).copied(),
        };
        for p in mesh.primitives() {
            let Some(m) = primitive(&p, blob, light, &rest, skinned.is_some()) else { continue };
            if let (Some(c), Some(b)) = (clear, bounds(&m, &rest)) {
                let cv = clears.entry(c).or_insert(ClearView { lo: Vec3::INFINITY, hi: Vec3::NEG_INFINITY, hide: vec![nodes[&c]] });
                (cv.lo, cv.hi) = (cv.lo.min(b.0), cv.hi.max(b.1));
            }
            if let (Some(c), Some(b)) = (piece, bounds(&m, &rest)) {
                let np = near.entry(c).or_insert(NearPiece { boxes: Vec::new(), hide: vec![nodes[&c]] });
                // (game up is −y)
                if -b.0.y >= 0.5 {
                    np.boxes.push(b);
                }
            }
            let wide = !sky && bounds(&m, &rest).is_some_and(surrounds_play);
            let flat = !sky && !wide && bounds(&m, &rest).is_some_and(on_ground);
            let m = s.meshes.add(m);
            for mat in &mats[p.material().index().unwrap_or(mats.len() - 1)] {
                let mat = if wide { panorama(s, mat, PANORAMA) } else if flat { panorama(s, mat, GROUND) } else { mat.clone() };
                let mut e = commands.spawn((Mesh3d(m.clone()), MeshMaterial3d(mat), Transform::default(), ChildOf(nodes[&n.index()])));
                if let Some(sk) = &skinned {
                    e.insert((sk.clone(), crate::cull::Cullable));
                    if let Some(cv) = clear.and_then(|c| clears.get_mut(&c)) {
                        cv.hide.push(e.id());
                    }
                    if let Some(np) = piece.and_then(|c| near.get_mut(&c)) {
                        np.hide.push(e.id());
                    }
                }
                draws += 1;
            }
        }
    }
    // a piece under 1.5 m (game up is −y) can't hide a player: always drawn (Dubno's toolboxes behind the baseline)
    for cv in clears.into_values().filter(|cv| -cv.lo.y >= 1.5) {
        commands.spawn(cv);
    }
    for np in near.into_values().filter(|np| !np.boxes.is_empty()) {
        commands.spawn(np);
    }
    for a in doc.animations() {
        let mut channels = Vec::new();
        for ch in a.channels() {
            let Some(&node) = nodes.get(&ch.target().node().index()) else { continue };
            let r = ch.reader(|_| Some(blob));
            let Some(times) = r.read_inputs().map(|i| i.collect::<Vec<f32>>()) else { continue };
            let interp = ch.sampler().interpolation();
            // cubic spline keys are (in-tangent, value, out-tangent): the values, linearly between
            let pick = |k: usize| interp != gltf::animation::Interpolation::CubicSpline || k % 3 == 1;
            let values = match r.read_outputs() {
                Some(gltf::animation::util::ReadOutputs::Translations(v)) => Values::Translation(v.enumerate().filter(|(k, _)| pick(*k)).map(|(_, v)| Vec3::from(v)).collect()),
                Some(gltf::animation::util::ReadOutputs::Rotations(v)) => Values::Rotation(v.into_f32().enumerate().filter(|(k, _)| pick(*k)).map(|(_, q)| Quat::from_array(q).normalize()).collect()),
                Some(gltf::animation::util::ReadOutputs::Scales(v)) => Values::Scale(v.enumerate().filter(|(k, _)| pick(*k)).map(|(_, v)| Vec3::from(v)).collect()),
                _ => continue,
            };
            let len = match &values {
                Values::Translation(v) | Values::Scale(v) => v.len(),
                Values::Rotation(v) => v.len(),
            };
            if len != times.len() || times.is_empty() {
                continue;
            }
            channels.push(Channel { node, times, values, step: interp == gltf::animation::Interpolation::Step });
        }
        let length = channels.iter().filter_map(|c| c.times.last().copied()).fold(0.0, f32::max);
        if !channels.is_empty() && length > 0.0 {
            match spawned.as_deref_mut() {
                Some(sp) => sp.clips.push((a.name().unwrap_or("").to_string(), Looped { length, channels })),
                None => _ = commands.spawn(Looped { length, channels }),
            }
        }
    }
    Ok(draws)
}

/// B433: where the court's own chair seats the umpire in a match of this kind (standard §2's `umpire_seat`, or its
/// `__singles`/`__doubles` one), in the layout court's game space; `None` without one (the layout court's chair).
pub fn umpire_seat(c: &Court, singles: bool) -> Option<[f32; 3]> {
    let (doc, _) = crate::mods::open(&c.dir.join("court.glb")).inspect_err(|e| warn!("court mod {}: {e}", c.id)).ok()?;
    seat(&doc, singles)
}

fn seat(doc: &gltf::Document, singles: bool) -> Option<[f32; 3]> {
    let worlds = worlds(doc);
    let space = doc.nodes().find(|n| n.name() == Some("game_space")).map_or(Mat4::IDENTITY, |n| worlds[n.index()]);
    let kind = if singles { "umpire_seat__singles" } else { "umpire_seat__doubles" };
    let n = doc.nodes().find(|n| n.name() == Some(kind)).or_else(|| doc.nodes().find(|n| n.name() == Some("umpire_seat")))?;
    Some((space.inverse() * worlds[n.index()]).w_axis.truncate().to_array())
}

/// B477: what of `court.glb` stands in the way, as triangles in the layout court's game space: every mesh drawn in a
/// match of this kind at rest, leaving out the light pools and rays (`@add` / `@sub` materials) and other primitives
/// than triangle lists. Empty when the file can't be read.
pub fn solid(c: &Court, singles: bool) -> Vec<[Vec3; 3]> {
    let Ok((doc, blob)) = crate::mods::open(&c.dir.join("court.glb")).inspect_err(|e| warn!("court mod {}: {e}", c.id)) else { return Vec::new() };
    let worlds = worlds(&doc);
    let space = doc.nodes().find(|n| n.name() == Some("game_space")).map_or(Mat4::IDENTITY, |n| worlds[n.index()]);
    let to_game = space.inverse();
    let skip = |n: &gltf::Node| n.name().is_some_and(|m| m.ends_with(if singles { "__doubles" } else { "__singles" }));
    let mut out = Vec::new();
    let Some(scene) = doc.default_scene().or_else(|| doc.scenes().next()) else { return out };
    let mut stack: Vec<gltf::Node> = scene.nodes().collect();
    while let Some(n) = stack.pop() {
        if skip(&n) {
            continue;
        }
        stack.extend(n.children());
        let Some(mesh) = n.mesh() else { continue };
        let m = to_game * worlds[n.index()];
        for p in mesh.primitives().filter(|p| p.mode() == gltf::mesh::Mode::Triangles) {
            if p.material().name().is_some_and(|m| m.ends_with("@add") || m.ends_with("@sub")) {
                continue;
            }
            let r = p.reader(|_| Some(&blob[..]));
            let Some(pos) = r.read_positions() else { continue };
            let pos: Vec<Vec3> = pos.map(|v| m.transform_point3(Vec3::from(v))).collect();
            let idx: Vec<u32> = r.read_indices().map_or_else(|| (0..pos.len() as u32).collect(), |i| i.into_u32().collect());
            out.extend(idx.as_chunks::<3>().0.iter().filter_map(|t| Some([*pos.get(t[0] as usize)?, *pos.get(t[1] as usize)?, *pos.get(t[2] as usize)?])));
        }
    }
    out
}

/// Each node's world matrix in the file.
fn worlds(doc: &gltf::Document) -> Vec<Mat4> {
    let mut parent = vec![None; doc.nodes().len()];
    for n in doc.nodes() {
        n.children().for_each(|c| parent[c.index()] = Some(n.index()));
    }
    let local: Vec<Mat4> = doc.nodes().map(|n| Mat4::from_cols_array_2d(&n.transform().matrix())).collect();
    let mut w: Vec<Option<Mat4>> = vec![None; local.len()];
    for i in 0..local.len() {
        // the chain up to a node already known (the tree is checked at most 1024 deep, `mods::open`)
        let mut chain = vec![i];
        while let Some(p) = parent[*chain.last().unwrap()].filter(|p| w[*p].is_none() && chain.len() <= 1024) {
            chain.push(p);
        }
        for &k in chain.iter().rev() {
            if w[k].is_none() {
                w[k] = Some(parent[k].and_then(|p| w[p]).unwrap_or(Mat4::IDENTITY) * local[k]);
            }
        }
    }
    w.into_iter().map(|m| m.unwrap_or(Mat4::IDENTITY)).collect()
}

/// Sort bias (m nearer) of a court mod's water materials, enough to break the tie with the court's other pieces.
const WATER_LAST: f32 = 5.0;

/// B682: sort offset of a court mod's panorama ([`surrounds_play`]): after the sky's backdrops, before everything else.
const PANORAMA: f32 = crate::BACKDROP / 2.0;

/// B682: a box (game space) that reaches 100 m out from the court on every side: a distant-view ring standing round
/// it (Dubno's city and sea, Canada's mountains, the resort's clouds), far beyond any camera. Sorted by its centre, the
/// court's middle, its blended draws went after anything beyond the net, and covered a far ball's outline (its soft
/// edge writes no Z) wherever the ball stood in front of it. Fences, roofs, water and ground decals reach less far.
fn surrounds_play((lo, hi): (Vec3, Vec3)) -> bool {
    const OUT: f32 = 100.0;
    lo.x < -OUT && hi.x > OUT && lo.z < -OUT && hi.z > OUT
}

/// Sort offset of a court mod's blended ground pieces ([`on_ground`]: decals, dirt, painted lines): after the
/// panorama, before the projected shadows and everything standing on the court, as the disc courts' hole model. Sorted
/// by the court's middle they went after a player beyond it and covered the racket's strings (they write no Z).
const GROUND: f32 = -9.0e5;

/// A box (game space, up is −y) lying flat on the ground.
fn on_ground((lo, hi): (Vec3, Vec3)) -> bool {
    hi.y - lo.y < 0.2 && -lo.y < 0.3 && -hi.y > -0.3
}

/// A copy of `mat` sorted at `order` (blended only), lit and scrolled as `mat` is: a panorama first like the disc
/// courts' backdrops, a ground piece before what stands on it.
fn panorama(s: &mut Store, mat: &Handle<GsMaterial>, order: f32) -> Handle<GsMaterial> {
    let Some(g) = s.materials.get(mat).filter(|g| g.alpha_mode() == AlphaMode::Blend) else { return mat.clone() };
    let h = s.materials.add(GsMaterial { order, ..g.clone() });
    let k = &mut *s.kept;
    let roles: Vec<_> = k.materials.iter().filter(|(id, _)| *id == mat.id()).map(|&(_, r)| (h.id(), r)).collect();
    let speeds: Vec<_> = k.scroll.iter().filter(|(id, _)| *id == mat.id()).map(|&(_, v)| (h.id(), v)).collect();
    k.materials.extend(roles);
    k.scroll.extend(speeds);
    h
}

/// The `.glb`'s materials as GS draws (as a character mod's, `mods::materials_of`; the last one glTF's default), lit
/// as `role` says.
fn materials(doc: &gltf::Document, blob: &[u8], s: &mut Store, role: Role, clear: bool) -> Result<Vec<Vec<Handle<GsMaterial>>>, String> {
    let sampler = || ImageSampler::Descriptor(ImageSamplerDescriptor { address_mode_u: ImageAddressMode::Repeat, address_mode_v: ImageAddressMode::Repeat, ..ImageSamplerDescriptor::linear() });
    let mut tex = Vec::new();
    for im in doc.images() {
        let gltf::image::Source::View { view, mime_type } = im.source() else { return Err("images must be embedded".into()) };
        let bytes = blob.get(view.offset()..view.offset() + view.length()).ok_or("image outside the buffer")?;
        let up = crate::mods::upscaled_image(s.upscaled.as_deref(), &im);
        let (bytes, ty) = match &up {
            Some(b) => (&b[..], ImageType::Extension("png")),
            None => (bytes, ImageType::MimeType(mime_type)),
        };
        let mut img = Image::from_buffer(bytes, ty, CompressedImageFormats::NONE, true, sampler(), RenderAssetUsages::RENDER_WORLD).map_err(|e| format!("image {}: {e}", im.index()))?;
        crate::textures::fit(&mut img);
        // the GS works on the stored values
        img.texture_descriptor.format = bevy::render::render_resource::TextureFormat::Rgba8Unorm;
        tex.push(s.images.add(img));
    }
    let mut out = Vec::new();
    for m in doc.materials().map(Some).chain([None]) {
        let t = m.as_ref().and_then(|m| m.pbr_metallic_roughness().base_color_texture()).map(|i| i.texture().source().index());
        let color = m.as_ref().map_or([1.0; 4], |m| m.pbr_metallic_roughness().base_color_factor());
        let alpha = m.as_ref().map_or(gltf::material::AlphaMode::Opaque, |m| m.alpha_mode());
        let (mask, blend) = (alpha == gltf::material::AlphaMode::Mask, alpha == gltf::material::AlphaMode::Blend);
        let mut header = [0; 0x30];
        header[0x1e..0x20].copy_from_slice(&(if mask { 10i16 } else if blend { 25 } else { 0 }).to_le_bytes());
        let two_sided = m.as_ref().is_none_or(|m| m.double_sided());
        let mtl = mtl::Material { texture: t, color: color.map(|c| c.min(1.0)), attributes: None, two_sided, name: m.as_ref().and_then(|m| m.name()).unwrap_or("").to_string(), header };
        let prim = 0x20 | if t.is_some() { 0x10 } else { 0 } | if blend { 0x40 } else { 0 };
        let mut handles = Vec::new();
        for mut g in GsMaterial::for_batch(&mtl, prim, t.and_then(|t| tex.get(t)).cloned()) {
            style(&mut g, role, None, 0.0);
            // glTF's alpha is texture × vertex alpha: Get a Grip's shadow decals are black texels, their strength in
            // the vertex alpha (HIGHLIGHT2 drew them solid black)
            g.key.modulate = true;
            g.key.clear = clear;
            if role == Role::Sky {
                g.order = crate::BACKDROP;
            } else if mtl.name.to_lowercase().contains("water") {
                // a court's pieces all sit at its origin, so the blended ones tie in the back-to-front sort: the sea
                // goes after them, over what lies under it (Get a Grip c04's corals drew on top of it)
                g.order = WATER_LAST;
            }
            let h = s.materials.add(g);
            s.kept.materials.push((h.id(), role));
            if let Some((_, speed)) = s.kept.speeds.iter().find(|(n, _)| *n == mtl.name) {
                s.kept.scroll.push((h.id(), *speed));
            }
            handles.push(h);
        }
        out.push(handles);
    }
    Ok(out)
}

/// A court mod material's light, fog and shadow for its role (`fog`: the manifest's, `None` the weather's).
fn style(m: &mut GsMaterial, role: Role, fog: Option<(Vec4, Vec4)>, darken: f32) {
    let u = &mut m.uniform;
    if role != Role::Weather {
        // colour = vertex colour × material colour (the light baked in, or none on the sky)
        (u.unlit, u.ambient, u.light_color, u.light2_color, u.glare, u.highlight) = (1.0, Vec4::ONE, Vec4::ZERO, Vec4::ZERO, 0.0, 0.0);
    }
    if role == Role::Sky {
        (u.fog, u.shadow) = (NO_FOG, 0.0);
        return;
    }
    if let Some((f, c)) = fog.filter(|_| role == Role::Baked) {
        (u.fog, u.fog_color) = (f, c);
    }
    u.shadow = darken;
}

/// One triangle-list primitive as a GS mesh: the manifest light baked into its vertex colours (normals turned to
/// game space by `rest`: the node's matrix, or the skin's joints at bind), joints and weights for a skin.
fn primitive(p: &gltf::Primitive, blob: &[u8], light: Option<&Light>, rest: &[Mat4], skinned: bool) -> Option<Mesh> {
    if p.mode() != gltf::mesh::Mode::Triangles {
        return None;
    }
    let r = p.reader(|_| Some(blob));
    let pos: Vec<[f32; 3]> = r.read_positions()?.collect();
    let n = pos.len();
    let nrm: Vec<[f32; 3]> = r.read_normals().map_or_else(|| vec![[0.0, -1.0, 0.0]; n], |i| i.collect());
    let uv: Vec<[f32; 2]> = r.read_tex_coords(0).map_or_else(|| vec![[0.0; 2]; n], |i| i.into_f32().collect());
    let mut col: Vec<[f32; 4]> = r.read_colors(0).map_or_else(|| vec![[1.0; 4]; n], |i| i.into_rgba_f32().collect());
    let joints: Vec<[u16; 4]> = r.read_joints(0).map_or_else(|| vec![[0; 4]; n], |i| i.into_u16().collect());
    let weights: Vec<[f32; 4]> = r.read_weights(0).map_or_else(|| vec![[1.0, 0.0, 0.0, 0.0]; n], |i| i.into_f32().collect());
    let idx: Vec<u32> = r.read_indices().map_or_else(|| (0..n as u32).collect(), |i| i.into_u32().collect());
    if [nrm.len(), uv.len(), col.len(), joints.len(), weights.len()].iter().any(|&l| l != n) || idx.iter().any(|&i| i as usize >= n) {
        return None;
    }
    if let Some(l) = light {
        for (k, c) in col.iter_mut().enumerate() {
            let m = if skinned { (0..4).map(|j| *rest.get(joints[k][j] as usize).unwrap_or(&Mat4::IDENTITY) * weights[k][j]).fold(Mat4::ZERO, |a, b| a + b) } else { rest[0] };
            let w = m.inverse().transpose().transform_vector3(Vec3::from(nrm[k])).normalize_or_zero();
            let lit = (l.ambient + l.sun_colour * w.dot(l.sun).max(0.0) + l.fill_colour * w.dot(l.fill).max(0.0)).min(Vec3::splat(MAX_LIGHT));
            let rgb = Vec3::new(c[0], c[1], c[2]) * lit;
            *c = [rgb.x, rgb.y, rgb.z, c[3]];
        }
    }
    // gs.wgsl draws an untextured colour as the GS does (0x80 = 128 of 255); glTF's vertex colour 1.0 is full white
    // (M8f10a1: Open Tee 2's and Get a Grip's vertex-coloured skies came out at half brightness)
    if p.material().pbr_metallic_roughness().base_color_texture().is_none() {
        for c in &mut col {
            for v in &mut c[..3] {
                *v *= 255.0 / 128.0;
            }
        }
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, crate::cull::usage(RenderAssetUsages::RENDER_WORLD))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_indices(Indices::U32(idx));
    if skinned {
        // gs.wgsl's skinned normal: the first joint's share in the normal slot, the rest in the tangent slot
        mesh = mesh
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm.iter().zip(&weights).map(|(n, w)| (Vec3::from(*n) * w[0]).to_array()).collect::<Vec<_>>())
            .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, nrm.iter().zip(&weights).map(|(n, w)| (Vec3::from(*n) * (1.0 - w[0])).extend(0.0).to_array()).collect::<Vec<_>>())
            .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(joints))
            .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weights);
    } else {
        mesh = mesh.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm);
    }
    Some(mesh)
}

/// B510: each node of a gimmick file's placed pieces (the children where the single-child chain from `game_space`, else
/// the scene's roots, ends: `game_space` → the source's own root → the pieces; skinned meshes hang beside it), mapped
/// to its piece's node; `__clear` pieces left out (`clear_view` has them).
fn placed_pieces(doc: &gltf::Document) -> HashMap<usize, usize> {
    let mut top: Vec<gltf::Node> = match doc.nodes().find(|n| n.name() == Some("game_space")) {
        Some(n) => vec![n],
        None => doc.default_scene().or_else(|| doc.scenes().next()).map_or(Vec::new(), |s| s.nodes().collect()),
    };
    while let [one] = top.as_slice() {
        let next: Vec<_> = one.children().collect();
        if next.is_empty() {
            break;
        }
        top = next;
    }
    let mut out = HashMap::new();
    for piece in top.iter().filter(|n| !n.name().is_some_and(|m| m.trim_end_matches("__singles").trim_end_matches("__doubles").ends_with("__clear"))) {
        let mut stack = vec![piece.clone()];
        while let Some(n) = stack.pop() {
            out.insert(n.index(), piece.index());
            stack.extend(n.children());
        }
    }
    out
}

/// A mesh's box in game space at rest (`rest`: its node's matrix, or its skin's joints at bind, each vertex by its
/// heaviest joint).
fn bounds(m: &Mesh, rest: &[Mat4]) -> Option<(Vec3, Vec3)> {
    let Some(VertexAttributeValues::Float32x3(pos)) = m.attribute(Mesh::ATTRIBUTE_POSITION) else { return None };
    let joints = match (m.attribute(Mesh::ATTRIBUTE_JOINT_INDEX), m.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)) {
        (Some(VertexAttributeValues::Uint16x4(j)), Some(VertexAttributeValues::Float32x4(w))) => Some((j, w)),
        _ => None,
    };
    let mut b = (Vec3::INFINITY, Vec3::NEG_INFINITY);
    for (k, p) in pos.iter().enumerate() {
        let j = joints.map_or(0, |(j, w)| j[k][(0..4).max_by(|&a, &c| w[k][a].total_cmp(&w[k][c])).unwrap_or(0)] as usize);
        let q = rest.get(j).unwrap_or(&Mat4::IDENTITY).transform_point3(Vec3::from(*p));
        b = (b.0.min(q), b.1.max(q));
    }
    (b.0.x <= b.1.x).then_some(b)
}

fn bind_inverses(sk: &gltf::Skin, blob: &[u8]) -> Vec<Mat4> {
    sk.reader(|_| Some(blob)).read_inverse_bind_matrices().map_or_else(|| vec![Mat4::IDENTITY; sk.joints().count()], |i| i.map(|m| Mat4::from_cols_array_2d(&m)).collect())
}

/// B506: the mesh nodes of a court mod's creatures that stand in the players' room ([`in_play`]) at rest: a skinned
/// mesh (Get a Grip's pigeons, crows, sparrows, crab, dog, tiger; its props are rigid) and the rigid meshes that go
/// with it (its shadow: under the highest node over its skeleton that holds no other skin and nothing out of the room). Get a Grip's pigeons sit on and around the court and fly
/// off when play comes near, a reaction that isn't reproduced: here they'd stay in the rally, so they aren't drawn.
fn creatures_in_play(doc: &gltf::Document, blob: &[u8], to_game: Mat4, singles: bool) -> std::collections::HashSet<usize> {
    let worlds = worlds(doc);
    let mut parent = HashMap::new();
    doc.nodes().for_each(|n| n.children().for_each(|c| _ = parent.insert(c.index(), n.index())));
    let skip = |n: &gltf::Node| n.name().is_some_and(|m| m.ends_with(if singles { "__doubles" } else { "__singles" }));
    let in_room = |n: &gltf::Node, rest: &[Mat4], skinned: bool| {
        n.mesh().is_some_and(|m| m.primitives().any(|p| primitive(&p, blob, None, rest, skinned).and_then(|m| bounds(&m, rest)).is_some_and(|(lo, hi)| in_play(lo, hi))))
    };
    let is_under = |mut i: usize, top: usize| loop {
        if i == top {
            return true;
        }
        match parent.get(&i) {
            Some(&p) => i = p,
            None => return false,
        }
    };
    // the rigid meshes standing in the room, and each skin's joints and whether it stands there
    let rigid: Vec<(usize, bool)> = doc.nodes().filter(|n| n.mesh().is_some() && n.skin().is_none() && !skip(n)).map(|n| (n.index(), in_room(&n, &[to_game * worlds[n.index()]], false))).collect();
    let skins: Vec<(usize, Vec<usize>, bool)> = doc
        .nodes()
        .filter(|n| !skip(n))
        .filter_map(|n| {
            let sk = n.skin()?;
            let rest: Vec<Mat4> = sk.joints().zip(bind_inverses(&sk, blob)).map(|(j, b)| to_game * worlds[j.index()] * b).collect();
            Some((n.index(), sk.joints().map(|j| j.index()).collect(), in_room(&n, &rest, true)))
        })
        .collect();
    let mut off = std::collections::HashSet::new();
    for (k, (node, joints, inside)) in skins.iter().enumerate() {
        if !inside {
            continue;
        }
        off.insert(*node);
        // the creature: up from its skeleton while nothing else comes in (another skin, a mesh out of the room)
        let Some(mut root) = joints.first().copied() else { continue };
        while let Some(&p) = parent.get(&root) {
            let other = skins.iter().enumerate().any(|(o, s)| o != k && s.1.iter().any(|&j| is_under(j, p)));
            if other || rigid.iter().any(|&(r, inside)| !inside && is_under(r, p)) {
                break;
            }
            root = p;
        }
        off.extend(rigid.iter().filter(|&&(r, _)| is_under(r, root)).map(|&(r, _)| r));
    }
    off
}

/// B506: whether the box `lo..hi` (game space, y down) stands in the players' room: over the doubles court and 2 m
/// beside / 4 m behind it (as the walkers' room, `play::npcs::court_clear`), from 3 m up to 1.5 m down (a mod's
/// ground can sit off the layout's).
pub fn in_play(lo: Vec3, hi: Vec3) -> bool {
    let (w, l) = (5.485 + 2.0, 11.885 + 4.0);
    lo.x < w && hi.x > -w && lo.z < l && hi.z > -l && lo.y < 1.5 && hi.y > -3.0
}

/// Whether the box `lo..hi` (game space, y down) stands between `eye` and the court: a line from the eye to a point
/// of the playing area (the doubles court and 2 m behind each baseline, where the players stand, on the ground and
/// 2 m up) passes through it, or the eye is inside it.
/// ponytail: sampled (9 across, 9 along, 2 high), so a piece thinner than the gap between two lines can slip through.
pub fn blocks(eye: Vec3, lo: Vec3, hi: Vec3) -> bool {
    // the doubles sideline and the baseline (m from the centre), 2 m further back
    let (w, l) = (5.485, 11.885 + 2.0);
    (0..9).any(|i| {
        (0..9).any(|k| {
            [0.0, -2.0].iter().any(|&y| {
                let p = Vec3::new(w * (i as f32 / 4.0 - 1.0), y, l * (k as f32 / 4.0 - 1.0));
                // the segment eye..p against the box's slabs
                let d = p - eye;
                let (mut t0, mut t1) = (0f32, 1f32);
                for a in 0..3 {
                    if d[a].abs() < 1e-6 {
                        if eye[a] < lo[a] || eye[a] > hi[a] {
                            return false;
                        }
                    } else {
                        let (u, v) = ((lo[a] - eye[a]) / d[a], (hi[a] - eye[a]) / d[a]);
                        (t0, t1) = (t0.max(u.min(v)), t1.min(u.max(v)));
                    }
                }
                t0 <= t1
            })
        })
    })
}

/// M8f5a3: a `__clear` set piece isn't drawn while it stands between the drawn camera (any mode) and the court; all
/// are while `cull::Off` holds (B583: the intro's flyover).
/// B634: with a second view on (`twin_view`), hidden from the half whose eye it stands in front of only (its meshes
/// put on the other half's layer), and from both when it stands in front of both.
/// B683: hidden means put on `shadow::EYE_HIDDEN` (no camera draws it, the sun still does), so its shadow stays put
/// as the camera moves in and out of it.
#[allow(clippy::too_many_arguments)]
fn clear_view(
    mut commands: Commands,
    camera: Query<&Transform, With<crate::Orbit>>,
    off: Option<Res<crate::cull::Off>>,
    second: Option<Res<crate::twin_view::SecondEye>>,
    pieces: Query<&ClearView>,
    children: Query<&Children>,
    meshes: Query<(), With<Mesh3d>>,
    mut shown_in: Local<HashMap<Entity, Option<usize>>>,
) {
    let Ok(t) = camera.single() else { return };
    // game space is Bevy's turned a half-turn about X
    let game = |t: &Transform| t.translation * Vec3::new(1.0, -1.0, -1.0);
    let second = second.as_ref().and_then(|s| s.0.as_ref()).map(|s| game(&s.1));
    for cv in &pieces {
        let blocked = |eye: Vec3| off.is_none() && blocks(eye, cv.lo, cv.hi);
        // the one layer it's drawn on while the other half's eye has it in front
        let (hidden, only) = match (blocked(game(t)), second.map(blocked)) {
            (true, None | Some(true)) => (true, None),
            (true, Some(false)) => (false, Some(crate::twin_view::BOTTOM)),
            (false, Some(true)) => (false, Some(crate::twin_view::TOP)),
            (false, _) => (false, None),
        };
        let only = if hidden { Some(crate::shadow::EYE_HIDDEN) } else { only };
        let Some(&key) = cv.hide.first() else { continue };
        if shown_in.insert(key, only).unwrap_or(None) != only {
            for e in cv.hide.iter().flat_map(|&e| std::iter::once(e).chain(children.iter_descendants(e))).filter(|&e| meshes.contains(e)) {
                match only {
                    Some(l) => _ = commands.entity(e).try_insert(RenderLayers::layer(l)),
                    None => _ = commands.entity(e).try_remove::<RenderLayers>(),
                }
            }
        }
    }
}

/// B583: while `cull::Off` holds (the intro's flyover) the court's big blockers are drawn too (M129's `clear` off on
/// every material that has it), and `clear` is back on its materials once it goes.
fn clear_off(off: Option<Res<crate::cull::Off>>, mut materials: ResMut<Assets<GsMaterial>>, mut held: Local<Vec<AssetId<GsMaterial>>>) {
    if off.is_some() {
        // (each frame: a material that loads meanwhile too)
        let new: Vec<_> = materials.iter().filter(|(_, m)| m.key.clear).map(|(id, _)| id).collect();
        for id in new {
            materials.get_mut(id).unwrap().key.clear = false;
            held.push(id);
        }
    } else {
        for id in std::mem::take(&mut *held) {
            if let Some(mut m) = materials.get_mut(id) {
                m.key.clear = true;
            }
        }
    }
}

/// After each weather change (`weather::apply` relights every material): the court mod's own light and fog back,
/// and the ground shadow's strength the layout court's ground has now.
fn keep(look: Option<Res<crate::weather::CourtLook>>, kept: Res<Kept>, mut materials: ResMut<Assets<GsMaterial>>, mut last: Local<Option<(Option<u8>, Option<(u8, f32)>)>>) {
    let Some(look) = look else { return };
    if *last == Some((look.last, look.mix)) {
        return;
    }
    *last = Some((look.last, look.mix));
    let darken = look.holes.iter().find_map(|id| materials.get(*id)).map_or(0.0, |m| m.uniform.shadow);
    for &(id, role) in &kept.materials {
        if let Some(mut m) = materials.get_mut(id) {
            style(&mut m, role, kept.fog, darken);
        }
    }
}

/// Each gimmick animation on its own loop.
fn animate(time: Res<Time>, clips: Query<&Looped>, mut nodes: Query<&mut Transform>) {
    let now = time.elapsed_secs();
    for clip in &clips {
        clip.pose(now % clip.length, &mut nodes);
    }
}

/// The manifest's `scroll`: each named material's texture sliding at its speed (whole repeats dropped, so the
/// offset stays small).
fn slide(time: Res<Time>, kept: Res<Kept>, mut materials: ResMut<Assets<GsMaterial>>) {
    let now = time.elapsed_secs_f64();
    for &(id, speed) in &kept.scroll {
        if let Some(mut m) = materials.get_mut(id) {
            m.uniform.uv_offset = speed.as_dvec2().map(|v| (v * now).rem_euclid(1.0)).as_vec2();
        }
    }
}

impl Looped {
    /// Its nodes as at `t` s into it.
    pub fn pose<F: bevy::ecs::query::QueryFilter>(&self, t: f32, nodes: &mut Query<&mut Transform, F>) {
        for ch in &self.channels {
            let Ok(mut tf) = nodes.get_mut(ch.node) else { continue };
            let k = ch.times.partition_point(|&x| x <= t).saturating_sub(1);
            let next = (k + 1).min(ch.times.len() - 1);
            let f = if ch.step || next == k { 0.0 } else { ((t - ch.times[k]) / (ch.times[next] - ch.times[k])).clamp(0.0, 1.0) };
            match &ch.values {
                Values::Translation(v) => tf.translation = v[k].lerp(v[next], f),
                Values::Rotation(v) => tf.rotation = v[k].slerp(v[next], f),
                Values::Scale(v) => tf.scale = v[k].lerp(v[next], f),
            }
        }
    }
}

/// Once the match's banks are loaded: the mod's BGM in place of the layout court's (with `--music`), and its
/// ambience layers looping in place of the layout court's creature and emitter sounds (court bank program 7, kept
/// silent: its plays still come and go on the same ticks).
fn sounds(court: Res<MatchCourt>, args: Res<crate::Args>, music: Option<ResMut<MusicBanks>>, bank: Option<ResMut<CourtBank>>, sound: Option<Res<Sound>>, mut done: Local<bool>) {
    let (false, Some(mut music), Some(mut bank), Some(sound)) = (*done, music, bank, sound) else { return };
    *done = true;
    let c = &court.0;
    let read = |l: &crate::mods_court::Loop, rms| crate::modfs::read(&l.path).ok().and_then(|d| Track::read(&d, l.at, rms)).map(Arc::new);
    if let Some(track) = c.bgm.as_ref().and_then(|l| read(l, Track::BGM_RMS)) {
        let b = Arc::new(SoundBank { hd: Vec::new(), bd: Vec::new(), wav: Default::default(), takes: Default::default(), stream: Some(track) });
        *BGM.lock().unwrap() = Some(b.clone());
        if args.music {
            music.bgm = Some((b, Vec::new()));
        }
    }
    let layers: Vec<_> = c.ambience.iter().filter_map(|l| read(l, Track::AMBIENCE_RMS)).collect();
    if layers.is_empty() {
        return;
    }
    if let Some(b) = &bank.0 {
        let mut wav = b.wav.clone();
        wav.insert(7, vec![(Arc::from([0i16]), 48_000)]);
        bank.0 = Some(Arc::new(SoundBank { hd: b.hd.clone(), bd: b.bd.clone(), wav, takes: b.takes.clone(), stream: None }));
    }
    // M111: all the layers together at a stock court's ambience (`Track::AMBIENCE_RMS`), under the crowd and SFX
    let level = 1.0 / (layers.len() as f32).sqrt();
    for t in &layers {
        sound.play_loop(t, Group::Ambience, level);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_piece_between_the_eye_and_the_court_blocks_it() {
        // HST's resting match eye (41 m at 17°, game space y down) and c05's basket goals 7.2 m behind each baseline
        let eye = Vec3::new(0.0, -12.0, -39.2);
        let goal = |z: f32| (Vec3::new(-0.9, -4.8, z - 0.6), Vec3::new(0.9, 0.0, z + 0.6));
        let (lo, hi) = goal(-19.1);
        assert!(blocks(eye, lo, hi), "the near goal");
        let (lo, hi) = goal(19.1);
        assert!(!blocks(eye, lo, hi), "the far goal stands behind the court");
        assert!(blocks(Vec3::new(0.0, -2.0, -19.1), lo - Vec3::Z * 38.2, hi - Vec3::Z * 38.2), "the eye inside it");
        assert!(!blocks(eye, Vec3::new(14.0, -3.0, -20.0), Vec3::new(16.0, 0.0, -18.0)), "off to the side");
        assert!(!blocks(eye, Vec3::new(-1.0, -0.2, -20.0), Vec3::new(1.0, 0.0, -18.0)), "low on the ground");
    }

    /// B433: the umpire's seat in game space through the file's `game_space` and a Get a Grip court's centimetres
    /// (Y up), the match kind's own over a plain one, none without.
    #[test]
    fn the_umpire_seat_is_read_in_game_space() {
        let doc = |seats: &str| {
            let json = format!(
                r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[
                {{"name":"game_space","rotation":[0,0,1,0],"children":[1]}},
                {{"name":"gag_cm","rotation":[1,0,0,0],"scale":[0.01,0.01,0.01],"children":[2,3]}},
                {{"name":"{seats}","translation":[650,172.8,10]}},
                {{"name":"umpire_seat__doubles","translation":[750,172.8,10]}}]}}"#
            );
            gltf::Gltf::from_slice(json.as_bytes()).unwrap().document
        };
        let near = |a: Option<[f32; 3]>, b: [f32; 3]| a.is_some_and(|a| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-5));
        let d = doc("umpire_seat__singles");
        assert!(near(seat(&d, true), [6.5, -1.728, -0.1]), "{:?}", seat(&d, true));
        assert!(near(seat(&d, false), [7.5, -1.728, -0.1]));
        let d = doc("umpire_seat");
        assert!(near(seat(&d, true), [6.5, -1.728, -0.1]), "a plain seat for either kind");
        assert!(near(seat(&d, false), [7.5, -1.728, -0.1]), "the kind's own first");
        assert_eq!(seat(&doc("chair"), true), None);
    }

    /// B476: every court mod under `$HST_MODS` (the mods repo's `out/mods`) has a seat for both match kinds, up on a
    /// stand beside the doubles sideline: one without falls back to the layout court's chair and floats beside its
    /// own stand (Dubno Tower's umpire by the net post).
    #[test]
    fn every_court_mod_seats_the_umpire() {
        let Ok(root) = std::env::var("HST_MODS") else { return eprintln!("no HST_MODS, skipped") };
        let courts = crate::mods_court::list(std::path::Path::new(&root));
        for c in &courts {
            for singles in [true, false] {
                let s = umpire_seat(c, singles).unwrap_or_else(|| panic!("{}: no umpire_seat", c.id));
                assert!((-2.0..-1.5).contains(&s[1]) && s[0].abs() > 5.485, "{} (singles {singles}): seat {s:?}", c.id);
            }
        }
        eprintln!("{} court mods seat the umpire", courts.len());
    }

    /// B510: Dubno's helicopter (`$HST_MODS`, the mods repo's `out/mods`) is one placed piece whose boxes over 0.5 m up
    /// take in its cabin beside the doubles sideline and leave its ground shadows out; the `__clear` flag isn't one.
    #[test]
    fn dubno_helicopter_is_a_piece() {
        let Ok(root) = std::env::var("HST_MODS") else { return eprintln!("no HST_MODS, skipped") };
        let Ok((doc, blob)) = crate::mods::open(&std::path::Path::new(&root).join("gag_c20_city/gimmick.glb")) else { return eprintln!("no Dubno, skipped") };
        let pieces = placed_pieces(&doc);
        let name = |i: usize| doc.nodes().nth(i).and_then(|n| n.name().map(str::to_string)).unwrap_or_default();
        let mut names: Vec<_> = pieces.values().map(|&p| name(p)).collect();
        names.sort();
        names.dedup();
        assert!(names.iter().any(|n| n.contains("Bheli")) && !names.iter().any(|n| n.ends_with("__clear")), "{names:?}");
        let worlds = worlds(&doc);
        let to_game = doc.nodes().find(|n| n.name() == Some("game_space")).map_or(Mat4::IDENTITY, |n| worlds[n.index()]).inverse();
        let boxes: Vec<(Vec3, Vec3)> = doc
            .nodes()
            .filter(|n| n.skin().is_none() && pieces.get(&n.index()).is_some_and(|&p| name(p).contains("Bheli")))
            .filter_map(|n| {
                let rest = [to_game * worlds[n.index()]];
                n.mesh()?.primitives().filter_map(|p| primitive(&p, &blob, None, &rest, false).and_then(|m| bounds(&m, &rest))).reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))
            })
            .filter(|b| -b.0.y >= 0.5)
            .collect();
        // the logged cut-away eye 0.5 m past the cabin's side, 3 m up: in a box's reach
        let cabin = Vec3::new(9.5, -2.0, 1.8);
        assert!(boxes.iter().any(|(lo, hi)| cabin.cmpge(*lo).all() && cabin.cmple(*hi).all()), "{boxes:?}");
        assert!(boxes.iter().all(|(lo, _)| -lo.y >= 0.5));
    }

    /// B682: the court mods' blended pieces that draw first as panoramas: Dubno's distant city is one (its far-side
    /// ball outline was covered), and none is a net, fence, roof or water.
    #[test]
    fn court_mod_panoramas() {
        let Ok(root) = std::env::var("HST_MODS") else { return eprintln!("no HST_MODS, skipped") };
        let courts = crate::mods_court::list(std::path::Path::new(&root));
        let (mut found, mut ground) = (Vec::new(), Vec::new());
        for c in &courts {
            for file in ["court.glb", "gimmick.glb"] {
                let Ok((doc, blob)) = crate::mods::open(&c.dir.join(file)) else { continue };
                let worlds = worlds(&doc);
                let to_game = doc.nodes().find(|n| n.name() == Some("game_space")).map_or(Mat4::IDENTITY, |n| worlds[n.index()]).inverse();
                for n in doc.nodes().filter(|n| n.skin().is_none()) {
                    let rest = [to_game * worlds[n.index()]];
                    for p in n.mesh().iter().flat_map(|m| m.primitives()) {
                        let blend = p.material().alpha_mode() == gltf::material::AlphaMode::Blend;
                        let b = primitive(&p, &blob, None, &rest, false).and_then(|m| bounds(&m, &rest));
                        if blend && b.is_some_and(surrounds_play) {
                            found.push(format!("{}: {}", c.id, p.material().name().unwrap_or("")));
                        } else if blend && b.is_some_and(on_ground) {
                            ground.push(format!("{}: {} {:?}", c.id, p.material().name().unwrap_or(""), b.unwrap()));
                        }
                    }
                }
            }
        }
        assert!(found.iter().any(|f| f.starts_with("gag_c20_city: c20_bg_Enkei")), "{found:?}");
        assert!(!found.iter().any(|f| ["net", "fence", "fance", "roof", "water"].iter().any(|k| f.to_lowercase().contains(k))), "{found:?}");
        eprintln!("{found:#?}");
        // and the ground pieces that draw before the players (decals, shadows, lines; flat by `on_ground`)
        assert!(ground.iter().any(|f| f.starts_with("gag_c01_yamate: c01_yamate_ground07")), "{ground:?}");
        eprintln!("{ground:#?}");
    }

    /// B506: every court mod under `$HST_MODS`: the creatures left out for standing in the players' room are whole (a
    /// skin with its shadow, a collar) and small pieces on the ground; Get a Grip's University court loses its pigeons there.
    #[test]
    fn no_court_mod_creature_stands_in_the_rally() {
        let Ok(root) = std::env::var("HST_MODS") else { return eprintln!("no HST_MODS, skipped") };
        let courts = crate::mods_court::list(std::path::Path::new(&root));
        assert!(!courts.is_empty());
        for c in &courts {
            let Ok((doc, blob)) = crate::mods::open(&c.dir.join("gimmick.glb")) else { continue };
            let worlds = worlds(&doc);
            let to_game = doc.nodes().find(|n| n.name() == Some("game_space")).map_or(Mat4::IDENTITY, |n| worlds[n.index()]).inverse();
            for singles in [true, false] {
                let off = creatures_in_play(&doc, &blob, to_game, singles);
                let mut names: Vec<String> = Vec::new();
                for n in doc.nodes().filter(|n| off.contains(&n.index())) {
                    let skinned = n.skin().is_some();
                    let rest: Vec<Mat4> = match n.skin() {
                        Some(sk) => sk.joints().zip(bind_inverses(&sk, &blob)).map(|(j, b)| to_game * worlds[j.index()] * b).collect(),
                        None => vec![to_game * worlds[n.index()]],
                    };
                    let (lo, hi) = n.mesh().unwrap().primitives().filter_map(|p| primitive(&p, &blob, None, &rest, skinned).and_then(|m| bounds(&m, &rest))).fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |a, b| (a.0.min(b.0), a.1.max(b.1)));
                    assert!((hi - lo).max_element() < 3.0 && lo.y > -2.0 && hi.y > -0.5, "{}: {:?} {lo} {hi}", c.id, n.name());
                    names.push(format!("{}{} {:.1},{:.1}", n.name().unwrap_or("?"), if skinned { "" } else { " (rigid)" }, (lo.x + hi.x) / 2.0, (lo.z + hi.z) / 2.0));
                }
                assert!(names.iter().filter(|n| !n.contains("(rigid)")).count() * 2 <= names.len(), "{}: each skin with its shadow: {names:?}", c.id);
                if singles {
                    eprintln!("{}: {}", c.id, names.join(", "));
                }
                if c.id == "gag_c14_academy" {
                    assert_eq!(names.len(), 8, "University: its four pigeons by the court");
                }
            }
        }
    }
}
