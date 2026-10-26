//! B676: the drawn pose against the game's posed nodes. The journal's `scripts/b676_nodes.py` dumps every player's node array
//! (sampled local rotation and position, world matrix) beside its motion player each frame; the fixtures are the
//! slot-4 (Carol, right-handed, character 4, 3, 11) and slot-5 (characters 0, 2, 1, 5) matches, 300 samples each.

use hst_data::{ani, fixture, iso::Iso, mdl, xb::Archive};
use hst_sim::pose::{Clip, Skeleton, M4};

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

/// One player in one sample: character, motion player (0x80 bytes) and nodes (rotation, position, world rows).
struct Seen<'a> {
    v: u32,
    c: usize,
    anim: &'a [u8],
    nodes: &'a [u8],
}

impl Seen<'_> {
    fn count(&self) -> usize {
        self.nodes.len() / 0x60
    }
    fn motion(&self) -> usize {
        u(self.anim, 0x20) as usize
    }
    fn sampled(&self) -> f32 {
        f(self.anim, 0x38)
    }
    fn rotation(&self, n: usize) -> [f32; 4] {
        [0, 4, 8, 12].map(|j| f(self.nodes, n * 0x60 + j))
    }
    fn world(&self, n: usize) -> M4 {
        std::array::from_fn(|r| std::array::from_fn(|c| f(self.nodes, n * 0x60 + 0x20 + 16 * r + 4 * c)))
    }
}

/// Every sample of a dump, per player.
fn samples(data: &[u8]) -> Vec<[Seen<'_>; 4]> {
    let mut out = vec![];
    let mut o = 0;
    while o < data.len() {
        let v = u(data, o);
        o += 4;
        out.push(std::array::from_fn(|_| {
            let (c, k) = (u(data, o) as usize, u(data, o + 4) as usize);
            let anim = &data[o + 8..o + 8 + 0x80];
            let nodes = &data[o + 8 + 0x80..o + 8 + 0x80 + k * 0x60];
            o += 8 + 0x80 + k * 0x60;
            Seen { v, c, anim, nodes }
        }));
    }
    out
}

fn costume(iso: &mut Iso, c: usize, n: usize) -> Option<Skeleton> {
    let mdata = iso.read(&format!("PC/PC{c:02}C{n:02}.XB")).ok()?;
    let marc = Archive::parse(&mdata).ok()?;
    let e = marc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(".mdl"))?;
    let m = mdl::parse(&marc.read(e).ok()?).ok()?;
    Some(Skeleton { names: m.node_names.clone(), parent: m.node_parent.clone(), rest: m.node_local.clone() })
}

/// The skeleton of the character's first costume with `k` nodes (the game's node array is the body model's).
fn skeleton(iso: &mut Iso, c: usize, k: usize) -> Skeleton {
    (0..10).filter_map(|n| costume(iso, c, n)).find(|s| s.names.len() == k).unwrap_or_else(|| panic!("no costume of {c} has {k} nodes"))
}

fn clip(iso: &mut Iso, sk: &Skeleton, c: usize, m: usize) -> Option<Clip> {
    let data = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).ok()?;
    let arc = Archive::parse(&data).ok()?;
    let stem = ani::motion_name(m, c)?.to_ascii_lowercase();
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2")))?;
    Some(Clip::new(sk, &ani::parse(&arc.read(e).ok()?).ok()?))
}

/// Point `p`'s coordinates in the frame `m` (rows: axes then origin; orthogonal axes).
fn in_frame(m: &M4, p: [f32; 3]) -> [f32; 3] {
    let d = [0, 1, 2].map(|c| p[c] - m[3][c]);
    [0, 1, 2].map(|r| (0..3).map(|c| d[c] * m[r][c]).sum::<f32>() / (0..3).map(|c| m[r][c] * m[r][c]).sum::<f32>())
}

/// The stances, runs and strokes as the game samples them are the disc clips at the motion player's sampled time
/// (`Clip::sample_drawn`, what `character::animate` draws, within 1e-4 per quaternion component), and the off hand
/// sits on the racket where our skeleton chain puts it (within 5 mm in the racket joint's frame: the square stance at
/// the head's rim, the turned one at the throat). Skipped: the 8 frames after a motion change (the crossfade) and the
/// shared team reactions (0x30..).
#[test]
fn stance_poses_match_the_game() {
    let Ok(mut iso) = fixture::iso() else { return };
    let (mut quats, mut hands) = (0, 0);
    for file in ["b676/s4.bin", "b676/s5.bin"] {
        let Ok(data) = fixture::read(format!("fixtures/{file}")) else { return fixture::skip(file) };
        let all = samples(&data);
        let mut sks = std::collections::HashMap::new();
        let mut clips = std::collections::HashMap::new();
        for (i, s) in all.iter().enumerate() {
            for (p, seen) in s.iter().enumerate() {
                let (c, m, t) = (seen.c, seen.motion(), seen.sampled());
                // the crossfade's frames: the motion changed within the last 8 frames (every 2nd frame dumped)
                let settled = (1..=4).all(|k| i >= k && all[i - k][p].motion() == m && seen.v - all[i - k][p].v == 2 * k as u32);
                if m >= 0x30 || !settled {
                    continue;
                }
                let sk = sks.entry(c).or_insert_with(|| skeleton(&mut iso, c, seen.count()));
                let Some(cl) = clips.entry((c, m)).or_insert_with(|| clip(&mut iso, sk, c, m)) else { continue };
                for k in 0..cl.tracks.len() {
                    let n = cl.tracks[k].node;
                    let (Some(q), true) = (cl.sample_drawn(k, t).0, n < seen.count()) else { continue };
                    let g = seen.rotation(n);
                    let d = |s: f32| (0..4).map(|j| (q[j] - s * g[j]).abs()).fold(0.0, f32::max);
                    assert!(d(1.0).min(d(-1.0)) < 1e-4, "{file} v{} p{p} char {c} motion {m:#x} t {t} {}: {q:?} vs {g:?}", seen.v, sk.names[n]);
                    quats += 1;
                }
                if m <= 2 {
                    let ix = |name: &str| sk.names.iter().position(|x| x == name).unwrap();
                    let (r, h) = (ix("Racket"), ix("Bip01LHand"));
                    let game = in_frame(&seen.world(r), [0, 1, 2].map(|k| seen.world(h)[3][k]));
                    let w = hst_sim::pose::model(sk, &cl.locals(sk, t));
                    let ours = in_frame(&w[r], [0, 1, 2].map(|k| w[h][3][k]));
                    assert!((0..3).all(|k| (game[k] - ours[k]).abs() < 0.005), "{file} v{} p{p} char {c} motion {m:#x}: off hand {game:?} vs {ours:?}", seen.v);
                    hands += 1;
                }
            }
        }
    }
    eprintln!("{quats} rotations, {hands} off hands");
    assert!(quats > 50_000 && hands > 500);
}

/// Research (B676 follow-ups): the game's extra turn of a few joints over their sampled local, in degrees, from its
/// world matrices: it turns the head (watching the ball) and bends each thigh and calf alike, which the drawn pose
/// doesn't yet. `B676=b676/s4.bin tools/check.sh -p hst-sim --test drawn_pose extra_turns -- --ignored --nocapture`.
#[test]
#[ignore]
fn extra_turns() {
    let Ok(mut iso) = fixture::iso() else { return };
    let file = std::env::var("B676").unwrap_or("b676/s4.bin".into());
    let Ok(data) = fixture::read(format!("fixtures/{file}")) else { return };
    let names = ["Bip01", "Bip01Pelvis", "Bip01Spine", "Bip01Neck", "Bip01Head", "Bip01LThigh", "Bip01LCalf", "Bip01LFoot", "Bip01RThigh", "Bip01RCalf", "Bip01RFoot", "Bip01LUpperArm", "Bip01RUpperArm"];
    let mut sks = std::collections::HashMap::new();
    for s in samples(&data).iter().filter(|s| s[0].v % 20 == 0) {
        let mut line = format!("v{}", s[0].v);
        for seen in s {
            let sk = sks.entry(seen.c).or_insert_with(|| skeleton(&mut iso, seen.c, seen.count()));
            line += &format!(" | c{} m{:#x}", seen.c, seen.motion());
            for name in names {
                let Some(n) = sk.names.iter().position(|x| x == name) else { continue };
                let Some(par) = sk.parent[n] else { continue };
                // the game's local rotation rows (world rows in the parent's frame) against the sampled rotation's
                let (w, pw) = (seen.world(n), seen.world(par));
                let norm = |r: [f32; 4]| (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
                let rows: [[f32; 3]; 3] = std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|j| w[r][j] * pw[c][j] / norm(pw[c])).sum::<f32>() / norm(w[r])));
                let q = hst_sim::pose::q_matrix(seen.rotation(n));
                let tr: f32 = (0..3).map(|r| (0..3).map(|j| rows[r][j] * q[r][j]).sum::<f32>()).sum();
                let deg = ((tr - 1.0) / 2.0).clamp(-1.0, 1.0).acos().to_degrees();
                if deg > 1.0 {
                    line += &format!(" {}:{deg:.0}", name.trim_start_matches("Bip01"));
                }
            }
        }
        eprintln!("{line}");
    }
}

/// B676b: each drawn thigh is the sampled local with its roll blended toward the rest roll (`pose::untwist_thigh`,
/// the character's `pose::thigh_twist`) times the pelvis's drawn world, and the calf hangs off the plain sampled
/// thigh, within 2e-5 per rotation entry. Skipped: the crossfade's frames (the calf doesn't hang off the dumped
/// thigh) and one frame in slot 4 where the game drew both players' thighs plain (the frame the serve stance starts). In slot 4 Carol (character 6) wears costume 9 (the dump doesn't store it); the others' factor is the same
/// for every costume.
#[test]
fn thighs_untwist_like_the_game() {
    let Ok(mut iso) = fixture::iso() else { return };
    let (mut halved, mut plain, mut skipped) = (0, 0, 0);
    for file in ["b676/s4.bin", "b676/s5.bin"] {
        let Ok(data) = fixture::read(format!("fixtures/{file}")) else { return fixture::skip(file) };
        let mut sks = std::collections::HashMap::new();
        for s in samples(&data) {
            for (p, seen) in s.iter().enumerate() {
                let sk = sks.entry(seen.c).or_insert_with(|| skeleton(&mut iso, seen.c, seen.count()));
                let twist = hst_sim::pose::thigh_twist(seen.c, if seen.c == 6 { 9 } else { 0 });
                for side in ["R", "L"] {
                    let ix = |name: String| sk.names.iter().position(|x| *x == name).unwrap();
                    let (n, calf) = (ix(format!("Bip01{side}Thigh")), ix(format!("Bip01{side}Calf")));
                    let local = |n: usize| {
                        let mut m = hst_sim::pose::q_matrix(seen.rotation(n));
                        m[3] = [f(seen.nodes, n * 0x60 + 0x10), f(seen.nodes, n * 0x60 + 0x14), f(seen.nodes, n * 0x60 + 0x18), 1.0];
                        m
                    };
                    let world = |m: &M4, n: usize| hst_sim::pose::vmul(m, &seen.world(sk.parent[n].unwrap()));
                    let thigh = if twist == 1.0 { local(n) } else { hst_sim::pose::untwist_thigh(&local(n), &sk.rest[n], twist) };
                    let calf_over = hst_sim::pose::vmul(&local(calf), &world(&local(n), n));
                    let err = |a: &M4, b: &M4| (0..3).flat_map(|r| (0..3).map(move |k| (a[r][k] - b[r][k]).abs())).fold(0.0f32, f32::max);
                    // a crossfade's frames: the dump's sampled rotation isn't the local the world was built from
                    if err(&calf_over, &seen.world(calf)) > 1e-5 {
                        continue;
                    }
                    let game = seen.world(n);
                    let ours = world(&thigh, n);
                    if err(&ours, &game) > 2e-5 && twist != 1.0 && err(&world(&local(n), n), &game) == 0.0 {
                        skipped += 1;
                        continue;
                    }
                    assert!(err(&ours, &game) <= 2e-5, "{file} v{} p{p} char {} {side} thigh: {ours:?} vs {game:?}", seen.v, seen.c);
                    if twist == 1.0 { plain += 1 } else { halved += 1 }
                }
            }
        }
    }
    eprintln!("{halved} untwisted thighs, {plain} plain, {skipped} drawn plain by the game");
    assert!(halved > 1000 && plain > 1000 && skipped <= 4);
}
