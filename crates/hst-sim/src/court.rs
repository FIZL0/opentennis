//! A disc court's collision world, built as the game builds it at court load (see `world` and `mesh`).
#![deny(clippy::float_arithmetic)]

use crate::ball::Material;
use crate::mesh::{self, World};
use crate::motion::Clock;
use crate::pose::{self, Clip, Nodes, Skeleton};
use crate::ps2::F;
use crate::world::{self, IDENTITY};
use hst_data::iso::Iso;
use hst_data::xb::Archive;
use hst_data::layout::category;
use hst_data::{ani, exe, layout, mdl, mtl};

/// The game's collision material table as ball responses, by material id.
pub fn materials(game: &exe::Game) -> Vec<Material> {
    game.surfaces().iter().zip(0..=255).map(|(s, id)| Material { court: s.court, special: s.special, restitution: s.restitution, spin_loss: s.spin_loss, id, effect: s.effect, sound: s.sound }).collect()
}

/// Every node's drawing matrix: the root's `root`, each child its own placement through its parent's.
// the walk (local · parent) is checked against court 5's props (tests/world.rs); a prop with its own motion (court 5's
// ball-boy cart wheels) starts at its disc pose here, `Motions` turns those nodes
fn nodes(m: &mdl::Model, root: world::M4) -> Vec<world::M4> {
    walk(&m.node_parent, &m.node_local, root)
}

/// `nodes` over the locals `local`.
fn walk(parent: &[Option<usize>], local: &[world::M4], root: world::M4) -> Vec<world::M4> {
    let mut out: Vec<world::M4> = Vec::with_capacity(parent.len());
    for (k, parent) in parent.iter().enumerate() {
        out.push(match parent {
            None => root,
            Some(p) => world::mat_mul(&local[k], &out[*p]),
        });
    }
    out
}

/// A colliding structure's own motion (court 5's ball-boy carts turn their wheels with `ballboy01.ani`): motion 0 of
/// the model's `.ani`, looping at one frame per step from the disc pose. Only structures play theirs; a prop
/// (category 18) with a motion file stays at its disc pose, so it gets none here.
#[derive(Clone)]
pub struct PropMotion {
    /// The prop it moves, in `World::props`.
    pub prop: usize,
    skeleton: Skeleton,
    clip: Clip,
    /// The node locals the motion samples into (the disc's until the first step).
    pub nodes: Nodes,
    /// The prop's root node matrix (its placement; it never moves).
    root: world::M4,
    pub clock: Clock,
    /// Sample on the next step even if the time hasn't changed (set at bind).
    resample: bool,
}

impl PropMotion {
    fn new(prop: usize, m: &mdl::Model, anim: &ani::Anim, root: world::M4) -> PropMotion {
        let skeleton = Skeleton { names: m.node_names.clone(), parent: m.node_parent.clone(), rest: m.node_local.clone() };
        let clip = Clip::new(&skeleton, anim);
        let nodes = Nodes::new(&skeleton);
        PropMotion { prop, skeleton, clip, nodes, root, clock: Clock::start(1.0, true, None), resample: true }
    }

    /// The motion's length in frames.
    pub fn length(&self) -> f32 {
        self.clip.length
    }

    /// Every node's world matrix from the current locals.
    pub fn world_nodes(&self) -> Vec<world::M4> {
        walk(&self.skeleton.parent, &self.nodes.local, self.root)
    }

    /// The locals sampled at `t` frames, as a step at that time leaves them.
    pub fn sample(&mut self, t: f32) {
        self.nodes.sample(&self.clip, t);
    }

    /// One step: the time wrapped into the clip; sampled there unless it already was; then a frame on. Returns
    /// whether the pose changed.
    fn step(&mut self) -> bool {
        let c = &mut self.clock;
        c.time = pose::wrap(c.time, self.clip.length, c.looping);
        let sampled = c.time != c.sampled || self.resample;
        if sampled {
            self.nodes.sample(&self.clip, c.time);
            c.sampled = c.time;
            self.resample = false;
        }
        c.time = (F(c.time) + c.speed).0;
        sampled
    }
}

/// The court's structure motions and the court draw pass that steps them.
#[derive(Clone, Default)]
pub struct Motions {
    pub list: Vec<PropMotion>,
    /// No court draw pass since the court was built: the first one steps nothing.
    pub first: bool,
}

impl Motions {
    /// One court draw pass (one per displayed frame while the court is shown, cut-aways, intro and replays
    /// included). `step` is false while paused and on slow motion's in-between frames; `hidden(prop)` is a prop
    /// the pass's visibility tests hide (it doesn't step). The first pass after the build never steps. A stepped
    /// prop's nodes in `world` take the new pose: a sweep sees the pose sampled in the last pass before it.
    pub fn pass(&mut self, world: &mut World, step: bool, hidden: impl Fn(usize) -> bool) {
        if step && !self.first {
            for m in &mut self.list {
                if !hidden(m.prop) && m.step() {
                    world.props[m.prop].nodes = m.world_nodes();
                }
            }
        }
        self.first = false;
    }
}

/// A disc model as collision geometry.
fn collision_model(m: &mdl::Model, mt: &mtl::Mtl) -> mesh::Model {
    mesh::Model {
        tris: m
            .colliding(mt)
            .map(|t| mesh::Tri { node: t.node, material: t.material, pos: t.pos, uv: t.uv, bounds: t.bounds })
            .collect(),
        materials: mt
            .materials
            .iter()
            .zip(&m.wrap)
            .map(|(mat, &wrap)| mesh::Material {
                two_sided: mat.two_sided,
                textured: mat.texture.is_some(),
                wrap: wrap.map(mesh::Wrap::from_mode),
                map: mat.attributes.map(|a| {
                    let a = &mt.attributes[a];
                    mesh::AttributeMap {
                        width: a.width(),
                        height: a.height(),
                        stride: a.stride(),
                        texels: a.texels.clone(),
                        table: a.table,
                    }
                }),
            })
            .collect(),
        node_from_model: m.node_inverse.clone(),
    }
}

/// The umpire's creature (`npc15`) among the plant records.
const UMPIRE_CREATURE: i16 = 15;

/// Categories placed as collision props, in the order the game creates them; 15 has no model directory, so none of
/// its records places anything.
const COLLIDING: [u8; 5] = [15, category::TREE, category::PROP, category::STRUCTURE, category::BILLBOARD];

/// The umpire's chair on disc court `n` (1..11): her plant record (creature `npc15`), hole 1.
pub fn umpire_chair(iso: &mut Iso, n: u32) -> Option<[f32; 3]> {
    let data = iso.read(&format!("COURT/{n:02}/HOL01.XB")).ok()?;
    let a = Archive::parse(&data).ok()?;
    let e = a.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("plant_c{n:02}_h01_0.dat")))?;
    layout::plants(&a.read(e).ok()?).ok()?.into_iter().find(|p| p.category == category::CREATURE && p.index == UMPIRE_CREATURE).map(|p| p.pos)
}

/// Disc court `n` (1..11), hole 1: the court model at the origin and the colliding props in the game's list order
/// and grid. An error names what on the disc couldn't be read or doesn't fit (a damaged image or pack).
pub fn world(iso: &mut Iso, n: u32) -> Result<World, String> {
    moving(iso, n).map(|(w, _)| w)
}

/// `world`, with the colliding structures' motions bound (at their disc pose, nothing sampled yet).
pub fn moving(iso: &mut Iso, n: u32) -> Result<(World, Motions), String> {
    let mut anims = std::collections::HashMap::new();
    let mut files = std::collections::HashMap::new();
    let (mut list, mut plants, mut court) = (Vec::new(), Vec::new(), None);
    for arc in ["CMN.XB", "GRD01.XB", "HOL01.XB"] {
        let path = format!("COURT/{n:02}/{arc}");
        let data = iso.read(&path).map_err(|e| format!("{path}: {e}"))?;
        let a = Archive::parse(&data).map_err(|e| format!("{path}: {e}"))?;
        // an entry by name, if the archive has it
        let read = |name: &str| match a.entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)) {
            Some(e) => a.read(e).map(Some).map_err(|err| format!("{path} {name}: {err}")),
            None => Ok(None),
        };
        for e in &a.entries {
            let name = e.name.to_ascii_lowercase();
            let bytes = || a.read(e).map_err(|err| format!("{path} {}: {err}", e.name));
            if name.ends_with(&format!("entry_c{n:02}.txt")) {
                list = layout::entries(&String::from_utf8_lossy(&bytes()?));
            } else if name.ends_with(&format!("plant_c{n:02}_h01_0.dat")) {
                plants = layout::plants(&bytes()?).map_err(|err| format!("{path} {}: {err}", e.name))?;
            } else if let Some(stem) = name.strip_suffix(".ani") {
                let anim = ani::parse(&bytes()?).map_err(|err| format!("{path} {}: {}", e.name, err.0))?;
                anims.insert(stem.rsplit(['/', '\\']).next().unwrap_or(stem).to_string(), anim);
            } else if let Some(stem) = name.strip_suffix(".mdl") {
                let base = &e.name[..e.name.len() - 4];
                let m = mdl::parse(&bytes()?).map_err(|err| format!("{path} {}: {err}", e.name))?;
                if m.collision.iter().any(|t| t.node >= m.node_count) {
                    return Err(format!("{path} {}: a collision face on a node the model doesn't have", e.name));
                }
                let mtl_name = format!("{base}.MTL");
                let mti = read(&format!("{base}.MTI"))?;
                let mt = mtl::parse(&read(&mtl_name)?.ok_or_else(|| format!("{path}: no {mtl_name}"))?, mti.as_deref())
                    .map_err(|err| format!("{path} {mtl_name}: {err}"))?;
                if arc == "GRD01.XB" && m.collides(&mt) {
                    if court.is_some() {
                        return Err(format!("{path}: more than one collision model"));
                    }
                    court = Some((m, mt));
                } else {
                    files.insert(stem.rsplit(['/', '\\']).next().unwrap_or(stem).to_string(), (m, mt));
                }
            }
        }
    }
    let (cm, cmt) = court.ok_or_else(|| format!("COURT/{n:02}/GRD01.XB: no collision model"))?;
    let mut models = vec![collision_model(&cm, &cmt)];
    // the court stands at the origin, unscaled
    let court = mesh::Object { model: 0, scale: 1.0, to_model: IDENTITY, nodes: nodes(&cm, world::mat_mul(&cm.node_local[0], &IDENTITY)), center: cm.center, radius: cm.radius };

    let mut created = Vec::new();
    for cat in COLLIDING {
        for p in plants.iter().filter(|p| p.category == cat && p.scale != 0.0) {
            let Some(e) = layout::resolve(&list, p, 0) else { continue };
            let (m, _) = files.get(&e.stem).ok_or_else(|| format!("court {n:02}: no model {} for a placed prop", e.stem))?;
            let (at, _) = world::place(p.category, p.pos, p.yaw, p.code);
            created.push((e.stem.clone(), world::instance(at, p.scale, &m.node_local[0], m.center, m.radius), p.scale, p.category));
        }
    }
    let (mut props, mut stems, mut spheres, mut motions) = (Vec::new(), Vec::<String>::new(), Vec::new(), Vec::new());
    for k in world::list_order(created.len()) {
        let (stem, prop, scale, cat) = &created[k];
        let (m, mt) = &files[stem];
        if !m.collides(mt) {
            continue;
        }
        let model = match stems.iter().position(|s| s == stem) {
            Some(i) => i + 1,
            None => {
                stems.push(stem.clone());
                models.push(collision_model(m, mt));
                models.len() - 1
            }
        };
        spheres.push((props.len(), prop.center, prop.radius));
        if *cat == category::STRUCTURE
            && let Some(anim) = anims.get(stem)
        {
            motions.push(PropMotion::new(props.len(), m, anim, prop.node));
        }
        props.push(mesh::Object { model, scale: *scale, to_model: prop.to_model, nodes: nodes(m, prop.node), center: prop.center, radius: m.radius });
    }
    Ok((World { models, court, props, grid: world::grid(&spheres) }, Motions { list: motions, first: true }))
}

/// A court mod's view of disc court `w` (its layout): the mod draws its own scenery, so the disc's walls, stands,
/// trees and chair would be invisible walls to the ball. Keeps the court model's ground (faces turned up or down,
/// no more than 0.5 m above the court; game space is Y down) and the net, the prop at the centre.
// ponytail: the mod's own scenery doesn't collide (its glb has no collision); add a mod collision mesh if wanted
#[allow(clippy::float_arithmetic)] // load-time filter of a mod's collision faces, not ported math
pub fn open(mut w: World) -> World {
    let court = &mut w.models[w.court.model];
    court.tris.retain(|t| {
        let [a, b, c] = t.pos;
        let (e, f) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
        let n = [e[1] * f[2] - e[2] * f[1], e[2] * f[0] - e[0] * f[2], e[0] * f[1] - e[1] * f[0]];
        let flat = n[1] * n[1] > 0.81 * (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]);
        flat && t.pos.iter().all(|v| v[1] > -0.5)
    });
    w.props.retain(|p| p.center[0].abs() < 1.0 && p.center[2].abs() < 1.0);
    let spheres: Vec<_> = w.props.iter().enumerate().map(|(i, p)| (i, p.center, p.radius * p.scale)).collect();
    w.grid = world::grid(&spheres);
    w
}
