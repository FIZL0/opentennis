//! B693: switches and a log for finding why other-game mod characters' heads show the court through them on iOS
//! (B691's CPU morph didn't fix it in the 1.5.2 IPA). A phone can't take environment variables, so the switches are
//! read once at start-up from `debug.txt` in the game's data folder (on iOS the folder the Files app shows, beside
//! `hst-log.txt`; elsewhere `disc::data_dir`, or `HST_DATA`), one `key = value` a line, `#` starts a comment:
//!
//! - `mod_morph = gpu | cpu | off`: a mod face's morphs on the GPU (Bevy's; every platform but Apple's), on the CPU
//!   (`cpu_morph`; Apple's default), or not at all (the face stays at rest).
//! - `mod_hide = face, mae_hair`: a mod part whose node, mesh or material name contains one of these isn't drawn.
//! - `mod_two_sided = on`: mod materials drawn with both faces (no back-face culling).
//! - `metal_precise_math = on`: Metal compiles the shaders without fast math (our wgpu-hal patch).
//! - `mod_log = on | off`: each mod rig's parts logged once, a few seconds after it appears (on by default on Apple).
//!
//! The log says which switches were read, each mod costume as loaded (its size and a hash, to compare with another
//! device's copy), each part as built, and per rig each part's state as drawn.

use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::morph::{MeshMorphWeights, MorphWeights};
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Morph {
    Gpu,
    Cpu,
    Off,
}

#[derive(Debug, Default, PartialEq)]
pub struct Switches {
    pub morph: Option<Morph>,
    /// Lowercase name pieces of the parts not drawn.
    pub hide: Vec<String>,
    pub two_sided: bool,
    pub precise_math: bool,
    pub log: Option<bool>,
    /// Lines that set nothing (an unknown key or value), for the log.
    pub unknown: Vec<String>,
}

impl Switches {
    pub fn parse(text: &str) -> Switches {
        let mut s = Switches::default();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                s.unknown.push(line.to_string());
                continue;
            };
            let v = v.trim().to_lowercase();
            let on = match v.as_str() {
                "on" | "1" | "true" | "yes" => Some(true),
                "off" | "0" | "false" | "no" => Some(false),
                _ => None,
            };
            let known = match k.trim() {
                "mod_morph" => {
                    s.morph = match v.as_str() {
                        "gpu" => Some(Morph::Gpu),
                        "cpu" => Some(Morph::Cpu),
                        "off" | "none" => Some(Morph::Off),
                        _ => None,
                    };
                    s.morph.is_some()
                }
                "mod_hide" => {
                    s.hide = v.split(',').map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).collect();
                    true
                }
                "mod_two_sided" => on.map(|o| s.two_sided = o).is_some(),
                "metal_precise_math" => on.map(|o| s.precise_math = o).is_some(),
                "mod_log" => on.map(|o| s.log = Some(o)).is_some(),
                _ => false,
            };
            if !known {
                s.unknown.push(line.to_string());
            }
        }
        s
    }
}

/// The switches, read on the first call (`start`, first thing in `main`).
pub fn switches() -> &'static Switches {
    static S: OnceLock<Switches> = OnceLock::new();
    S.get_or_init(|| {
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(text) = std::fs::read_to_string(std::env::var_os("HST_DATA").filter(|d| !d.is_empty()).map_or_else(crate::disc::data_dir, Into::into).join("debug.txt")) {
            return Switches::parse(&text);
        }
        Switches::default()
    })
}

/// Read `debug.txt` and apply what has to be set before the renderer starts; logs the switches (stderr: the logger
/// isn't up yet; on iOS that is `hst-log.txt`).
pub fn start() {
    let s = switches();
    #[cfg(target_vendor = "apple")]
    wgpu::hal::metal::PRECISE_MATH.store(s.precise_math, std::sync::atomic::Ordering::Relaxed);
    if *s != Switches::default() {
        eprintln!("debug.txt: mod_morph {:?}, mod_hide {:?}, mod_two_sided {}, metal_precise_math {}, mod_log {:?}", s.morph, s.hide, s.two_sided, s.precise_math, s.log);
        for l in &s.unknown {
            eprintln!("debug.txt: `{l}` sets nothing (mod_morph = gpu|cpu|off, mod_hide = a, b, mod_two_sided = on, metal_precise_math = on, mod_log = on)");
        }
    }
    if logging() {
        eprintln!("mod faces: morphed on the {}", if crate::cpu_morph::on() { "CPU" } else { "GPU" });
    }
}

/// Whether mod parts are logged: `mod_log`, else on Apple platforms.
pub fn logging() -> bool {
    switches().log.unwrap_or(cfg!(target_vendor = "apple"))
}

/// Mod faces drawn without morphs (`mod_morph = off`).
pub fn no_morph() -> bool {
    switches().morph == Some(Morph::Off)
}

struct Part {
    name: String,
    vertices: usize,
    morph: &'static str,
}

fn parts() -> std::sync::MutexGuard<'static, HashMap<AssetId<Mesh>, Part>> {
    static P: OnceLock<Mutex<HashMap<AssetId<Mesh>, Part>>> = OnceLock::new();
    P.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
}

/// A mod costume's file as read: its size and an FNV-1a hash of its bytes.
pub fn costume(id: &str, file: &std::path::Path, bytes: &[u8]) {
    if logging() {
        let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
        info!("mod {id}: {} ({} bytes, hash {hash:016x})", file.display(), bytes.len());
    }
}

/// A mod part as built: `name` (node/mesh/material), its vertex count and how its face morphs (cpu, gpu, none).
pub fn part(mesh: AssetId<Mesh>, name: String, vertices: usize, morph: &'static str) {
    if logging() {
        info!("mod part {name}: {vertices} vertices, morph {morph}");
    }
    parts().insert(mesh, Part { name, vertices, morph });
}

/// Whether `mesh` (a mod part's, or a rig's copy of one) is hidden by `mod_hide`.
fn hidden(mesh: AssetId<Mesh>) -> bool {
    let hide = &switches().hide;
    !hide.is_empty() && parts().get(&crate::cpu_morph::shared(mesh)).is_some_and(|p| hide.iter().any(|h| p.name.to_lowercase().contains(h)))
}

pub fn plugin(app: &mut App) {
    if !switches().hide.is_empty() {
        app.add_systems(PostUpdate, hide.before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate));
    }
    if logging() {
        app.add_systems(PostUpdate, census.after(bevy::camera::visibility::VisibilitySystems::CheckVisibility));
    }
}

/// Every draw of a hidden part (the GS's second draws, a rig's face copy) kept hidden.
fn hide(mut drawn: Query<(&Mesh3d, &mut Visibility), Changed<Mesh3d>>) {
    for (m, mut v) in &mut drawn {
        if hidden(m.0.id()) {
            *v = Visibility::Hidden;
        }
    }
}

/// Frames a rig is drawn before its parts are logged, and the most rigs logged.
const AFTER: u32 = 180;
const RIGS: usize = 8;

/// Each mod rig's parts, once, `AFTER` frames after it appears: the part, whether it draws its shared mesh or its
/// own copy (and whether that mesh exists), how it morphs, and whether it is visible and in view this frame.
#[allow(clippy::type_complexity)]
fn census(
    mut seen: Local<HashMap<Entity, u32>>,
    mut logged: Local<usize>,
    rigs: Query<(Entity, &Children, Option<&MorphWeights>), With<crate::character::Rig>>,
    drawn: Query<(&Mesh3d, &Visibility, &InheritedVisibility, &ViewVisibility, Has<NoFrustumCulling>, Has<crate::cpu_morph::Face>, Has<MeshMorphWeights>, Option<&MeshMaterial3d<crate::gs::GsMaterial>>)>,
    meshes: Res<Assets<Mesh>>,
    gs: Res<Assets<crate::gs::GsMaterial>>,
    placed: Query<&GlobalTransform>,
) {
    if *logged >= RIGS {
        return;
    }
    for (root, children, weights) in &rigs {
        let n = seen.entry(root).or_default();
        *n += 1;
        if *n != AFTER {
            continue;
        }
        let p = parts();
        let lines: Vec<String> = children
            .iter()
            .filter_map(|c| drawn.get(c).ok())
            .filter_map(|(m, vis, inherited, view, unculled, cpu, gpu, mat)| {
                let shared = crate::cpu_morph::shared(m.0.id());
                let part = p.get(&shared)?;
                let mesh = if shared != m.0.id() { "own copy" } else { "shared" };
                let missing = if meshes.contains(m.0.id()) { "" } else { " (not in the main world)" };
                let now = if cpu { "cpu face" } else if gpu { "gpu weights" } else { "none" };
                let cull = match mat.and_then(|h| gs.get(&h.0)) {
                    Some(g) if !g.key.cull => "both faces",
                    Some(g) if g.key.mirror => "front faces culled (mirrored)",
                    Some(_) => "back faces culled",
                    None => "no GS material",
                };
                Some(format!(
                    "  {}: {} vertices, built morph {}, drawn morph {now}, {mesh} mesh{missing}, {vis:?}, inherited {}, in view {}, unculled {unculled}, {cull}",
                    part.name,
                    part.vertices,
                    part.morph,
                    inherited.get(),
                    view.get()
                ))
            })
            .collect();
        if lines.is_empty() {
            continue;
        }
        let w = weights.map(|w| w.weights()).unwrap_or_default();
        let on = w.iter().filter(|w| **w != 0.0).count();
        let top = w.iter().copied().fold(0.0f32, f32::max);
        let mirrored = placed.get(root).is_ok_and(|t| t.affine().matrix3.determinant() < 0.0);
        info!("mod rig {root} ({}): {} parts, {} morph weights ({on} non-zero, largest {top})\n{}", if mirrored { "mirrored, left-handed" } else { "unmirrored" }, lines.len(), w.len(), lines.join("\n"));
        *logged += 1;
        if *logged >= RIGS {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_switches_and_names_what_it_did_not_take() {
        let s = Switches::parse("# test\nmod_morph = OFF\nmod_hide = Face, mae_hair ,\nmod_two_sided = on # both\nmetal_precise_math=1\nmod_log = no\nmod_morph2 = cpu\nnonsense\n");
        assert_eq!(s.morph, Some(Morph::Off));
        assert_eq!(s.hide, ["face", "mae_hair"]);
        assert!(s.two_sided && s.precise_math);
        assert_eq!(s.log, Some(false));
        assert_eq!(s.unknown, ["mod_morph2 = cpu", "nonsense"]);
        assert_eq!(Switches::parse(""), Switches::default());
        assert_eq!(Switches::parse("mod_morph = maybe").unknown, ["mod_morph = maybe"]);
    }
}
