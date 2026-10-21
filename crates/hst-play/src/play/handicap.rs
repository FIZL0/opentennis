//! B40g1: Set Handicap. Each player in the match plays with no handicap, one weight or two (`--handicap 0,1,2,0` by
//! match slot, the confirm screen's per-player setting). The handicap slows the run to 80 % (one weight) or 70 % (two)
//! for humans and computers alike (`hst_sim::player::handicap_size`, read by every run speed), and the player wears
//! the weights on the calves for the whole match: the first on the right calf, the second on the left. Looks only
//! past the run speed: no shot, stamina, AI choice or score reads it.

use std::sync::Arc;

use bevy::prelude::*;
use hst_data::iso::Iso;
use hst_data::xb::Archive;

use super::Game;
use crate::character::{self, CharacterData, Rig};

/// `--handicap 0,1,2,…` by match slot: 0 none, 1 one weight, 2 two.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq)]
pub struct Handicaps(pub [u8; 4]);

impl Handicaps {
    pub fn parse(s: &str) -> Self {
        let mut out = [0; 4];
        for (o, v) in out.iter_mut().zip(s.split(',')) {
            *o = v.trim().parse::<u8>().ok().filter(|&h| h <= 2).unwrap_or(0);
        }
        Handicaps(out)
    }

    /// The flag's value for `levels` (match order), or None with nobody handicapped.
    pub fn flag(levels: &[u8]) -> Option<String> {
        levels.iter().any(|&h| h > 0).then(|| levels.iter().map(u8::to_string).collect::<Vec<_>>().join(","))
    }
}

/// Player `i`'s handicap for the match: its body runs at the size it gives from now on (kept through every point's
/// new body, `reset_positions`).
pub(super) fn apply(g: &mut Game, i: usize, h: Option<&Handicaps>) {
    g.players[i].body.handicap = h.map_or(0, |h| h.0[i]);
}

/// Player `i`'s run size (%) for the match: 100, or 80 / 70 handicapped.
pub fn size(g: &Game, i: usize) -> i32 {
    g.players[i].body.size()
}

/// The weight model a character wears (`AZUMA/C_EFF/EFFCT.XB0`, `chara_eff/weight/`): JJ, Big Chief and Kent in his
/// costumes 5–9 have their own, everyone else the default.
fn model(character: usize, costume: usize) -> &'static str {
    match (character, costume) {
        (5, _) => "weight_pc05",
        (7, _) => "weight_pc07",
        (9, 5..) => "weight_pc09_c",
        _ => "weight",
    }
}

/// Where a weight sits on its calf: the offset back along the bone's own axes for a body drawn as modelled, the
/// offset for a mirrored (left-handed) body, and the model's uniform scale.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Place {
    normal: [f32; 3],
    mirrored: [f32; 3],
    scale: f32,
}

/// The placement for a character (menu order) in a costume: every character's own, and Kent's in costumes 5–9.
fn place(character: usize, costume: usize) -> Place {
    let p = |x: f32, mx: f32, scale: f32| Place { normal: [x, 0.0, 0.0], mirrored: [mx, 0.0, 0.0], scale };
    match character {
        5 => p(0.0, -0.15, 1.0),
        7 => p(0.14, -0.25, 1.3),
        9 if costume >= 5 => p(0.275, -0.385, 1.0),
        9 => p(0.2, -0.35, 1.15),
        11 => Place { normal: [0.25, 0.0, -0.015], mirrored: [-0.4, 0.0, -0.015], scale: 1.0 },
        13 => p(0.02, -0.18, 1.0),
        3 | 10 | 12 => p(0.2, -0.35, 1.0),
        _ => p(0.15, -0.3, 1.0),
    }
}

/// The calf joints the weights hang from (slot 0, slot 1) and each one's placement on it. A mirrored body swaps the
/// calves and turns the weight round the bone's x axis (the original's, which leaves the weight on the leg the
/// mirrored body shows there, unmirrored); the offset then runs along that turned axis.
fn hangs(character: usize, costume: usize, mirrored: bool) -> [(&'static str, Transform); 2] {
    let p = place(character, costume);
    let [a, b] = if mirrored { ["Bip01LCalf", "Bip01RCalf"] } else { ["Bip01RCalf", "Bip01LCalf"] };
    let t = if mirrored {
        let [x, y, z] = p.mirrored;
        Transform { translation: Vec3::new(x, -y, -z), scale: Vec3::new(-p.scale, p.scale, p.scale), ..default() }
    } else {
        let [x, y, z] = p.normal;
        Transform { translation: -Vec3::new(x, y, z), scale: Vec3::splat(p.scale), ..default() }
    };
    [(a, t), (b, t)]
}

/// Hang player's weights (`level` of them, 0..2) on the rig `root` just spawned for `data`: disc character (a mod's
/// donor) `character` in `costume`, `mirrored` as the match draws it. Each is a child of its calf joint, so it shows
/// whenever the body does and is lit as the body is (`character::WornGs`). A rig without the calf bone wears none.
#[allow(clippy::too_many_arguments)]
pub(super) fn wear(
    commands: &mut Commands,
    iso: &mut Iso,
    root: Entity,
    data: &Arc<CharacterData>,
    (character, costume, level, mirrored): (usize, usize, u8, bool),
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    if level == 0 {
        return;
    }
    let parts = iso.read("AZUMA/C_EFF/EFFCT.XB0").map_err(|e| e.to_string()).and_then(|bytes| {
        let arc = Archive::parse(&bytes).map_err(|e| e.0)?;
        character::worn(&arc, model(character, costume), meshes, materials, images)
    });
    let parts = match parts {
        Ok(p) => p,
        Err(e) => {
            warn!("handicap weights: {e}");
            return;
        }
    };
    let hangs: Vec<(usize, Transform)> = hangs(character, costume, mirrored)
        .into_iter()
        .take(level.min(2) as usize)
        .filter_map(|(bone, t)| Some((data.joint(bone)?, t)))
        .collect();
    // the joints from each calf up carry a visibility, so the weight inherits the rig's (as the racket's chain does)
    let chains: Vec<Vec<usize>> = hangs
        .iter()
        .map(|&(j, _)| std::iter::successors(Some(j), |&k| data.joints[k].parent).collect())
        .collect();
    commands.queue(move |w: &mut World| {
        let Some(joints) = w.get::<Rig>(root).map(|r| r.joints.clone()) else { return };
        for ((j, t), chain) in hangs.into_iter().zip(chains) {
            for k in chain {
                if let Ok(mut e) = w.get_entity_mut(joints[k])
                    && !e.contains::<Visibility>()
                {
                    e.insert(Visibility::default());
                }
            }
            let weight = w.spawn((t, Visibility::default())).id();
            for (mesh, material, gs) in &parts {
                let part = w.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), gs.clone(), Transform::default(), crate::cull::Cullable)).id();
                w.entity_mut(weight).add_child(part);
            }
            w.entity_mut(joints[j]).add_child(weight);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_reads_and_writes_per_player_levels() {
        assert_eq!(Handicaps::parse("0,1,2"), Handicaps([0, 1, 2, 0]));
        assert_eq!(Handicaps::parse("3,x,2,1"), Handicaps([0, 0, 2, 1]), "anything else is none");
        assert_eq!(Handicaps::flag(&[0, 0]), None);
        assert_eq!(Handicaps::flag(&[0, 2, 1, 0]).as_deref(), Some("0,2,1,0"));
    }

    /// The models and placements by character and costume (the spec's table).
    #[test]
    fn models_and_rows() {
        assert_eq!([model(5, 0), model(7, 9), model(9, 4), model(9, 5), model(0, 7)], ["weight_pc05", "weight_pc07", "weight", "weight_pc09_c", "weight"]);
        for c in [0, 1, 2, 4, 6, 8] {
            assert_eq!(place(c, 0), place(c, 7));
            assert_eq!(place(c, 0), Place { normal: [0.15, 0.0, 0.0], mirrored: [-0.3, 0.0, 0.0], scale: 1.0 });
        }
        assert_eq!(place(9, 4).scale, 1.15);
        assert_eq!(place(9, 5), Place { normal: [0.275, 0.0, 0.0], mirrored: [-0.385, 0.0, 0.0], scale: 1.0 });
        assert_eq!(place(11, 6).mirrored, [-0.4, 0.0, -0.015]);
    }

    /// A mirrored body's weight covers the same stretch of its calf as an unmirrored one's (the default model runs
    /// 0.15 m back along −x from its origin): the mirrored offset along the turned axis lands it where the normal one
    /// does, on the other calf.
    #[test]
    fn a_mirrored_weight_sits_where_the_normal_one_does() {
        let span = |t: Transform| {
            let (a, b) = (t.transform_point(Vec3::ZERO), t.transform_point(Vec3::new(-0.15, 0.0, 0.0)));
            (a.min(b), a.max(b))
        };
        for c in [0, 1, 2, 3, 4, 5, 6, 8, 10, 11, 12] {
            let [(r, n), (l, _)] = hangs(c, 0, false);
            let [(m, t), _] = hangs(c, 0, true);
            assert_eq!((r, l, m), ("Bip01RCalf", "Bip01LCalf", "Bip01LCalf"));
            let ((a, b), (x, y)) = (span(n), span(t));
            assert!(a.abs_diff_eq(x, 1e-6) && b.abs_diff_eq(y, 1e-6), "{c}: {:?} vs {:?}", (a, b), (x, y));
        }
    }
}
