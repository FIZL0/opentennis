//! Face morph weights against the five save-state RAM dumps (`fixtures/ram/s0N.bin`): every live morph player
//! (ticks per frame 80, sampled at least once) holds, per bound track, the weight `face::sample` gives at its last
//! sampled time, and the key cursor. Also every `.MOR` on the disc parses and binds to its model's targets.

use hst_data::fixture;
use hst_sim::face::{length, sample};

fn u(d: &[u8], a: u32) -> u32 {
    let a = (a & 0x1ff_ffff) as usize;
    u32::from_le_bytes(d[a..a + 4].try_into().unwrap())
}
fn f(d: &[u8], a: u32) -> f32 {
    f32::from_bits(u(d, a))
}

#[test]
fn morph_weights_ram() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let (mut players, mut weights) = (0, 0);
    for s in ["s03", "s04", "s05", "s08", "s09"] {
        let Ok(ram) = fixture::read(format!("{root}fixtures/ram/{s}.bin")) else { return };
        // the morph player: +8 count, +0xc bindings, +0x10 ticks per frame, +0x14 length, +0x1c last sampled time,
        // +0x28 weight; a binding: +0x14 track (+0 header, +4 keys; +0xc ticks, +0x10 values), +0x20 cursor, +0x24 weight
        for a in (0..ram.len() as u32 - 0x30).step_by(4) {
            let (n, list) = (u(&ram, a + 8), u(&ram, a + 0xc));
            if ram[a as usize] != 1 || u(&ram, a + 0x10) != 0x42a0_0000 || f(&ram, a + 0x28) != 1.0 || !(1..40).contains(&n) || !(0x10_0000..0x200_0000).contains(&list) {
                continue;
            }
            let mut tracks = Vec::new();
            for i in 0..n {
                let b = u(&ram, list + 4 * i);
                let tr = u(&ram, b + 0x14);
                let keys = u(&ram, u(&ram, tr) + 4);
                let ticks: Vec<i32> = (0..keys).map(|k| u(&ram, u(&ram, tr + 0xc) + 4 * k) as i32).collect();
                let values: Vec<[f32; 4]> = (0..keys).map(|k| [f(&ram, u(&ram, tr + 0x10) + 4 * k), 0.0, 0.0, 0.0]).collect();
                tracks.push((b, ticks, values));
            }
            assert_eq!(length(tracks.iter().map(|t| &t.1[..]), 80), f(&ram, a + 0x14), "{s} {a:#x} length");
            let last = f(&ram, a + 0x1c);
            if last < 0.0 {
                continue; // never sampled
            }
            for (b, ticks, values) in &tracks {
                let mut cur = u(&ram, b + 0x20) as usize;
                let w = sample(ticks, values, hst_sim::ps2::mul(last, 80.0), &mut cur, false)[0];
                assert_eq!((w.to_bits(), cur), (f(&ram, b + 0x24).to_bits(), u(&ram, b + 0x20) as usize), "{s} {a:#x} binding {b:#x} t {last}");
                weights += 1;
            }
            players += 1;
        }
    }
    eprintln!("{players} morph players, {weights} weights bit-exact");
    assert!(weights > 10);
}

#[test]
fn every_morph_binds() {
    use hst_data::{mdl, mor, xb::Archive};
    let Ok(mut iso) = fixture::iso() else { return };
    let (mut files, mut tracks, mut unbound) = (0, 0, 0);
    for c in 0..10 {
        let data = iso.read(&format!("PC/PC{c:02}C00.XB")).unwrap();
        let arc = Archive::parse(&data).unwrap();
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().contains(&format!("pc{c:02}_t")) && e.name.to_ascii_lowercase().ends_with("_c00.mdl")).unwrap();
        let m = mdl::parse(&arc.read(e).unwrap()).unwrap();
        // every packet's morph blocks: one per target, entries inside the packet
        for pk in m.materials.iter().flatten().filter(|p| !p.morphs.is_empty()) {
            assert_eq!(pk.morphs.len(), m.morph_names.len());
            assert!(pk.morphs.iter().flatten().all(|&(i, _)| i < pk.vertices.len()));
        }
        let data = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
        let arc = Archive::parse(&data).unwrap();
        for e in arc.entries.iter().filter(|e| e.name.to_ascii_uppercase().ends_with(".MOR")) {
            let t = mor::parse(&arc.read(e).unwrap(), 1).unwrap_or_else(|err| panic!("{}: {err}", e.name));
            assert_eq!(t.ticks_per_frame, 80, "{}", e.name);
            for tr in &t.tracks {
                // the game binds by exact name: a few tracks name targets the model lacks or carry trailing spaces
                if !m.morph_names.iter().any(|n| n == &tr.name) {
                    unbound += 1;
                    continue;
                }
                assert!(tr.ticks.windows(2).all(|w| w[0] < w[1]) && tr.ticks.len() == tr.values.len(), "{}", e.name);
                tracks += 1;
            }
            files += 1;
        }
    }
    eprintln!("{files} MOR files, {tracks} tracks bound, {unbound} not");
    assert!(files > 300 && unbound * 20 < tracks);
}

/// B673: each player's face through 9000 frames of the slot-5 doubles match, post-point reactions included
/// (`fixtures/b673/face_s05.bin`, `research/face_rec.py`): the face playing is the motion's (the soft follow-throughs
/// keep the previous one; a team reaction's losers its `_2` take five on, the whole team alike, the winners the base
/// one), with the motion's speed and looping; restarted with the motion, the face clock ticks as the motion clock
/// does (the motion player's countdown counts the ticks, the crossfade hold aside) over the face's own length.
#[test]
fn face_s05_reactions() {
    use hst_sim::motion::Clock;
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let Ok(data) = fixture::read(format!("{root}fixtures/b673/face_s05.bin")) else { return };
    const S: usize = 8 + 4 * 0xb4;
    let i = |b: &[u8], o: usize| i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let g = |b: &[u8], o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    // (vsync, motion, clip, motion speed, motion loop, countdown, face, face clock, face length)
    let read = |k: usize, p: usize| {
        let b = &data[k * S + 8 + p * 0xb4..];
        let m = &b[0x80..];
        let face = Clock { time: g(m, 0x20), sampled: g(m, 0x1c), speed: g(m, 0x24), looping: m[0x2c] != 0, hold: None };
        (i(&data[k * S..], 0), i(b, 0x20), i(b, 0x24), g(b, 0x40), b[0x78] != 0, i(b, 0x60), i(b, 0xb0), face, g(m, 0x14))
    };
    let (mut ticks, mut sets, mut reactions, mut losers) = (0, 0, 0, 0);
    for k in 1..data.len() / S {
        let now: Vec<_> = (0..4).map(|p| read(k, p)).collect();
        // a team reaction: one team (players 0/2 or 1/3) on its `_2` faces, the other on the base ones
        let team = |t: usize| [t, t + 2].map(|p| (now[p].1 >= 0x30).then_some(now[p].6 == now[p].1 + 5));
        if let ([Some(a), Some(b)], [Some(c), Some(d)]) = (team(0), team(1)) {
            assert!(a == b && c == d && a != c, "k={k} {now:x?}");
            losers += 1;
        }
        for (p, &b) in now.iter().enumerate() {
            let a = read(k - 1, p);
            let ctx = format!("k={k} p={p} {a:?} -> {b:?}");
            let face = match b.1 {
                0x1c | 0x1d => a.6,
                m if m >= 0x30 && b.6 == m + 5 => m + 5,
                m => m,
            };
            assert_eq!(b.6, face, "{ctx}");
            assert_eq!((b.7.speed.to_bits(), b.7.looping), (b.3.to_bits(), b.4), "{ctx}");
            reactions += (0x2b..0x35).contains(&b.1) as i32;
            if b.0 != a.0 + 1 {
                continue; // a torn read skipped
            }
            let n = a.5 - b.5;
            let mut c = a.7;
            if a.1 == b.1 && a.2 == b.2 && a.6 == b.6 && (0..=2).contains(&n) {
                (0..n).for_each(|_| c.tick(a.8));
                ticks += n;
            } else {
                // set this frame: the face restarts at 0 with the motion, then up to three ticks (a reaction set on the
                // frame after the phase change's skipped one)
                let fits = (0..=3).any(|j| {
                    c = Clock::start(b.7.speed, b.7.looping, None);
                    (0..j).for_each(|_| c.tick(b.8));
                    (c.sampled, c.time) == (b.7.sampled, b.7.time)
                });
                assert!(fits || b.6 == a.6 && b.7 == a.7, "set {ctx}");
                sets += 1;
                continue;
            }
            assert_eq!((c.sampled.to_bits(), c.time.to_bits()), (b.7.sampled.to_bits(), b.7.time.to_bits()), "{ctx}");
        }
    }
    eprintln!("{ticks} face ticks, {sets} sets, {reactions} reaction frames, {losers} team-reaction frames");
    assert!(ticks > 30000 && sets > 100 && reactions > 1000 && losers > 100);
}
