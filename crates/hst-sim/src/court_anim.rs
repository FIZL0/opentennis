//! Court hole animation: the hole model's `.UVA` (texture offset per material or node) and `.MTA` (material alpha)
//! play from the court's start, each on its own clock (speed 1, looping) — water, waterfalls and the like.
//!
//! A `.UVA` track named `@mtluva_<material>` drives every packet of that material; any other name is a node name and
//! drives every packet whose batch palette holds that node. Unmatched tracks never play. The offset (track x, y) is
//! added to the packet's texture coordinates, swapped for packets stored swapped ([`Packet::uv_swap`]); where two
//! tracks reach one packet the later one wins. `.MTA` tracks bind as the effects' do ([`crate::effect`]).
//!
//! A `.MOR` ([`CourtAnim::with_morphs`]) weights the model's morph targets, a track binding the target of its own name
//! (`000shibuki\x01000shibuki001`), on a clock of its own as the faces' ([`crate::face`]): court 2's surf foam slides
//! up and back down the beach. A vertex is drawn at its rest place plus each target's offset times the target's weight.
//!
//! [`Packet::uv_swap`]: hst_data::mdl::Packet::uv_swap
#![deny(clippy::float_arithmetic)]

use hst_data::{mdl::Model, mor::Tracks, mtl::Material};

use crate::{effect::Channel, face, motion::Clock, ps2::F};

/// A `.UVA` track with this name prefix drives a material (the rest of the name); any other names a node.
const MATERIAL_TRACK: &str = "@mtluva_";

struct UvTrack {
    ticks: Vec<i32>,
    values: Vec<[f32; 4]>,
    cursor: usize,
    value: [f32; 2],
}

pub struct CourtAnim {
    uva: Vec<UvTrack>,
    uva_tpf: f32,
    uva_length: f32,
    uva_clock: Clock,
    /// Per material, per packet: the `.UVA` track driving it.
    bind: Vec<Vec<Option<usize>>>,
    swap: Vec<Vec<bool>>,
    alpha: Option<Channel>,
    /// Per material its alpha (1 = the MTL's 0x80).
    pub alphas: Vec<f32>,
    morph: Option<Morphs>,
}

/// A bound `.MOR` track: its morph target, ticks, weights and sampler cursor.
type MorphTrack = (usize, Vec<i32>, Vec<[f32; 4]>, usize);

/// The `.MOR`: each bound track (morph target, ticks, weights, sampler cursor), the clock, and every target's weight
/// this frame.
struct Morphs {
    tracks: Vec<MorphTrack>,
    tpf: f32,
    length: f32,
    clock: Clock,
    weights: Vec<f32>,
}

impl CourtAnim {
    pub fn new(mdl: &Model, uva: Option<&Tracks>, mta: Option<&Tracks>, materials: &[Material]) -> CourtAnim {
        let mut bind: Vec<Vec<Option<usize>>> = mdl.materials.iter().map(|p| vec![None; p.len()]).collect();
        let swap = mdl.materials.iter().map(|p| p.iter().map(|p| p.uv_swap).collect()).collect();
        let mut tracks = Vec::new();
        for tr in uva.iter().flat_map(|u| &u.tracks) {
            let i = tracks.len();
            let mut hit = false;
            if let Some(name) = tr.name.strip_prefix(MATERIAL_TRACK) {
                for (m, _) in materials.iter().enumerate().filter(|(_, m)| m.name == name) {
                    bind.get_mut(m).into_iter().flatten().for_each(|b| *b = Some(i));
                    hit = true;
                }
            } else if let Some(node) = mdl.node_names.iter().position(|n| *n == tr.name) {
                for (m, packets) in mdl.materials.iter().enumerate() {
                    for (p, _) in packets.iter().enumerate().filter(|(_, pk)| pk.palette.contains(&node)) {
                        bind[m][p] = Some(i);
                        hit = true;
                    }
                }
            }
            if hit {
                tracks.push(UvTrack { ticks: tr.ticks.clone(), values: tr.values.clone(), cursor: 0, value: [0.0; 2] });
            }
        }
        let tpf = uva.map_or(1, |u| u.ticks_per_frame);
        let uva_length = face::length(uva.iter().flat_map(|u| &u.tracks).map(|t| &t.ticks[..]), tpf);
        let alpha = mta.map(|t| {
            let mut c = Channel::new(t, |n| materials.iter().position(|m| m.name == n));
            c.clock = Clock::start(1.0, true, None);
            c
        });
        CourtAnim {
            uva: tracks,
            uva_tpf: tpf as f32,
            uva_length,
            uva_clock: Clock::start(1.0, true, None),
            bind,
            swap,
            alpha,
            alphas: materials.iter().map(|m| m.color[3]).collect(),
            morph: None,
        }
    }

    /// With the model's `.MOR` playing from the start (speed 1, looping) over its morph targets; tracks whose target
    /// the model lacks never play, but count toward the clock's length.
    pub fn with_morphs(mut self, mdl: &Model, mor: Option<&Tracks>) -> CourtAnim {
        let Some(mor) = mor else { return self };
        let tracks: Vec<_> = mor
            .tracks
            .iter()
            .filter_map(|tr| {
                let target = mdl.morph_names.iter().position(|n| *n == tr.name)?;
                Some((target, tr.ticks.clone(), tr.values.clone(), 0))
            })
            .collect();
        if !tracks.is_empty() {
            self.morph = Some(Morphs {
                tracks,
                tpf: mor.ticks_per_frame as f32,
                length: face::length(mor.tracks.iter().map(|t| &t.ticks[..]), mor.ticks_per_frame),
                clock: Clock::start(1.0, true, None),
                weights: vec![0.0; mdl.morph_names.len()],
            });
        }
        self
    }

    /// Each morph target's weight this frame (empty without a `.MOR`).
    pub fn morph_weights(&self) -> &[f32] {
        self.morph.as_ref().map_or(&[], |m| &m.weights)
    }

    /// Anything to play.
    pub fn is_empty(&self) -> bool {
        self.uva.is_empty() && self.alpha.as_ref().is_none_or(|a| a.length == 0.0) && self.morph.is_none()
    }

    /// The `.UVA` track driving packet `packet` of material `material`.
    pub fn uv_track(&self, material: usize, packet: usize) -> Option<usize> {
        self.bind.get(material)?.get(packet).copied().flatten()
    }

    /// The texture offset this frame for packet `packet` of material `material`.
    pub fn uv_offset(&self, material: usize, packet: usize) -> [f32; 2] {
        let Some(t) = self.uv_track(material, packet) else { return [0.0; 2] };
        let [x, y] = self.uva[t].value;
        if self.swap[material][packet] { [y, x] } else { [x, y] }
    }

    /// Play the `.UVA` from its start at `speed` frames a frame.
    pub fn restart(&mut self, speed: f32) {
        self.uva_clock = Clock::start(speed, true, None);
    }

    /// One frame.
    pub fn tick(&mut self) {
        if !self.uva.is_empty() {
            self.uva_clock.tick(self.uva_length);
            let t = (F(self.uva_clock.sampled) * self.uva_tpf).0;
            for tr in &mut self.uva {
                let v = face::sample(&tr.ticks, &tr.values, t, &mut tr.cursor, true);
                tr.value = [v[0], v[1]];
            }
        }
        if let Some(a) = &mut self.alpha {
            a.clock.tick(a.length);
            a.apply(&mut self.alphas, 1.0);
        }
        if let Some(m) = &mut self.morph {
            m.clock.tick(m.length);
            let t = (F(m.clock.sampled) * m.tpf).0;
            for (target, ticks, values, cursor) in &mut m.tracks {
                m.weights[*target] = face::sample(ticks, values, t, cursor, false)[0];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hst_data::{mdl::Packet, mor::Track};

    fn mat(name: &str) -> Material {
        Material { texture: None, color: [1.0; 4], attributes: None, two_sided: false, name: name.into(), header: [0; 0x30] }
    }

    #[test]
    fn greece_water() {
        // court 10's hole: water and waterfall offsets, 32 ticks a frame over 1500 frames; RAM after 60 ticks
        // (sampled frame 59) holds 0.07866666 and 0.354
        let track = |name: &str, end: f32| Track { name: name.into(), ticks: vec![0, 48000], values: vec![[0.0; 4], [end, end, 0.0, 0.0]] };
        let uva = Tracks { ticks_per_frame: 32, tracks: vec![track("@mtluva_water", 2.0), track("@mtluva_zfall@add", 9.0), track("@mtluva_none", 1.0)] };
        let mdl = Model {
            materials: vec![vec![Packet::default()], vec![Packet { uv_swap: true, ..Packet::default() }, Packet::default()], vec![Packet::default()]],
            ..Model::default()
        };
        let mut a = CourtAnim::new(&mdl, Some(&uva), None, &[mat("water"), mat("zfall@add"), mat("rock")]);
        assert_eq!(a.uv_track(2, 0), None);
        for _ in 0..60 {
            a.tick();
        }
        assert_eq!(a.uv_offset(0, 0).map(f32::to_bits), [0x3da1_1bfd; 2]);
        assert_eq!(a.uv_offset(1, 1).map(f32::to_bits), [0x3eb5_3f7c; 2]);
        assert_eq!(a.uv_offset(2, 0), [0.0; 2]);
    }

    #[test]
    fn morph_weights_follow_the_mor() {
        // a 100-frame `.MOR` (80 ticks a frame): one target fades in over it, another holds; a track naming no
        // target is left out, and a target with no track stays at 0
        let track = |name: &str, a: f32, b: f32| Track { name: name.into(), ticks: vec![0, 8000], values: vec![[a, 0.0, 0.0, 0.0], [b, 0.0, 0.0, 0.0]] };
        let mor = Tracks { ticks_per_frame: 80, tracks: vec![track("spray1", 0.0, 1.0), track("spray2", 0.7, 0.7), track("gone", 1.0, 1.0)] };
        let mdl = Model { morph_names: vec!["spray1".into(), "spray2".into(), "spray3".into()], ..Model::default() };
        assert!(CourtAnim::new(&mdl, None, None, &[]).with_morphs(&mdl, None).morph_weights().is_empty());
        let mut a = CourtAnim::new(&mdl, None, None, &[]).with_morphs(&mdl, Some(&mor));
        assert!(!a.is_empty());
        for _ in 0..51 {
            a.tick();
        }
        let w = a.morph_weights();
        assert!((w[0] - 0.5).abs() < 0.02, "{w:?}");
        assert_eq!(&w[1..], [0.7, 0.0]);
    }
}
