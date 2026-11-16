//! The Japanese release (SCPS-15113, task M163a1a): its own km/h routines against `hst_sim::build::Build::Japan`,
//! run from a RAM image of a bot match on that disc (`fixtures/ram/jp-match.bin`: `7z e -so <state.p2s>
//! eeMemory.bin`, the state saved mid-match on `fixtures/discs/SCPS-15113.iso`). Skipped without either, even under
//! `HST_REQUIRE_DISC`: it isn't the user's disc.

use hst_oracle::{Ee, addr};
use hst_sim::build::Build;

fn rng(seed: &mut u64) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 32) as u32
}
fn uni(seed: &mut u64, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * (rng(seed) as f32 / u32::MAX as f32)
}
/// An integer in lo..hi.
fn int(seed: &mut u64, lo: i32, hi: i32) -> i32 {
    lo + (rng(seed) % (hi - lo) as u32) as i32
}

/// The Japanese match's RAM, its GAME overlay checked against the Japanese disc's.
fn japan() -> Option<Ee> {
    let root = hst_oracle::root();
    let (iso, ram) = (root.join("fixtures/discs/SCPS-15113.iso"), root.join("fixtures/ram/jp-match.bin"));
    if !iso.exists() || !ram.exists() {
        eprintln!("skipped: no fixtures/discs/SCPS-15113.iso or fixtures/ram/jp-match.bin");
        return None;
    }
    let game = hst_data::iso::Iso::open(&iso).ok()?.read("ZZBIN/GAME.BIN").ok()?;
    let ram = std::fs::read(ram).ok()?;
    let load = u32::from_le_bytes(game[8..12].try_into().unwrap()) as usize;
    let (at, n) = (load + 0x100, 0x1000);
    assert_eq!(ram[at..at + n], game[0x100..0x100 + n], "jp-match.bin holds another GAME overlay");
    Some(Ee::with_ram(ram))
}

/// Both of the Japanese program's km/h routines (the ball's last move, and the vector the other callers pass)
/// against the Japanese formula, over the speeds a ball reaches and well past them.
#[test]
fn japanese_kmh() {
    let Some(mut ee) = japan() else { return };
    let obj = 0x01e0_0000u32;
    for (name, at) in [("jp_ball_kmh", 0x150u32), ("jp_vec_kmh", 0x130)] {
        let f = addr(name);
        let (mut seed, mut bad, mut us_differs, n) = (0x2545_f491_4f6c_dd1du64, vec![], 0, 20_000);
        for _ in 0..n {
            let s = &mut seed;
            let v: [f32; 3] = std::array::from_fn(|_| uni(s, -1.2, 1.2) * [1.0, 0.01, 0.3][rng(s) as usize % 3]);
            for (i, c) in v.iter().enumerate() {
                ee.write(obj + at + 4 * i as u32, 4, c.to_bits() as u128);
            }
            ee.call(f, &[obj as u64], &[], 100_000);
            let (want, got) = (ee.f0(), Build::Japan.ball_kmh(v));
            if want.to_bits() != got.to_bits() {
                bad.push(format!("{v:?}: game {want}, port {got}"));
            }
            us_differs += (Build::Us.ball_kmh(v).to_bits() != want.to_bits()) as u32;
        }
        assert!(bad.is_empty(), "{name}: {} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(5)].join("\n"));
        // the check can tell the builds apart: the US formula misses the last bit on some speeds
        assert!(us_differs > 0, "{name}: the US formula matched every speed");
    }
}

/// The Japanese player record's fields the aim reads and writes (M163a1a1).
mod jp_player {
    pub const END: u32 = 0x12b0;
    pub const CON: u32 = 0x1300; // stroke, volley, serve (i32 degrees)
    pub const BODY: u32 = 0x130c; // stroke, volley (%)
    pub const RISING: u32 = 0x1314;
    pub const UNDER: u32 = 0x1380;
    pub const LAST_SHOT: u32 = 0x13cc; // → hitter +0x1b0, branch +0x1b4, kind +0x1b8, sweet +0x1bc
    pub const BRANCH: u32 = 0x3c31;
    pub const NUDGE: u32 = 0x3c44;
    pub const HELD: u32 = 0x3c4c;
    pub const KIND: u32 = 0x3c54; // 1 topspin, 2 slice, 8 flat, 4 lob, 0x10 drop
    pub const FROM: u32 = 0x3c60; // x, y, z
    pub const HEIGHT: u32 = 0x3ca4;
    pub const FLAGS: u32 = 0x3cb0; // 4: a body shot
    pub const OFFSET: u32 = 0x3d00;
}

/// The Japanese aim (its own routine from the base point on, with the deep button, the drop's halving and the
/// centred stick's nudge and the plain smash's held depth) against `shot::aim_from` and `shot::aim` with
/// `Build::Japan`, over 20k hitters, sticks and incoming shots: smashes over the whole court depth (the 6.4..11.885
/// narrowing by 0.3), ground slices and lobs (no 90° widest angle), body shots, rising balls, incoming slices.
#[test]
fn japanese_aim() {
    use hst_sim::build::Build;
    use hst_sim::lines::sideline;
    use hst_sim::shot::{AimStats, Hitter, aim, aim_from, branch, kind};
    use jp_player::*;
    let Some(mut ee) = japan() else { return };
    let f = addr("jp_aim_from");
    let (pl, last, out, base_at, stick_at) = (0x01d0_0000u32, 0x01d0_8000u32, 0x01d0_9000u32, 0x01d0_9010u32, 0x01d0_9020u32);
    let (count, shots) = (0x4248a4u32, 0x424960u32); // player count, shots this rally
    let code = [1u32, 2, 8, 4, 0x10]; // our kinds 0..4 as the game's
    let (mut seed, n) = (0x9e37_79b9_7f4a_7c15u64, 20_000);
    let (mut bad, mut us_differs, mut narrowed, mut wide, mut incoming_n, mut nudges, mut helds) = (vec![], 0, 0, 0, 0, 0, 0);
    for o in (0..0x4000).step_by(16) {
        ee.write(pl + o, 16, 0);
    }
    for case in 0..n {
        let s = &mut seed;
        let end = if rng(s) & 1 == 0 { 1.0f32 } else { -1.0 };
        let b = (rng(s) % 5) as u8;
        // smashes and ground slices/lobs (the two Japanese differences) take half the cases
        let k = match rng(s) % 4 {
            0 => kind::SLICE,
            1 => kind::LOB,
            _ => (rng(s) % 5) as i32,
        };
        let b = if case % 3 == 0 { branch::SMASH } else if case % 3 == 1 { branch::GROUND } else { b };
        let offset = int(s, 0, 25) - 12;
        let body = rng(s).is_multiple_of(4);
        let from = [uni(s, -5.0, 5.0), -end * uni(s, 0.3, 14.0)];
        let height = uni(s, 0.0, 2.2);
        let four = rng(s) & 1 == 0;
        let button = rng(s).is_multiple_of(10);
        let stats = AimStats {
            con: [int(s, 10, 60), int(s, 10, 60), int(s, 10, 60)],
            body: [int(s, 0, 100), int(s, 0, 100)],
            rising: int(s, 0, 100),
            under: uni(s, 0.3, 1.2),
            build: Build::Japan,
        };
        let stick = match rng(s) % 4 {
            0 => [[1.0, -1.0], [-1.0, -1.0], [1.0, 1.0], [-1.0, 1.0], [0.0, -1.0], [1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, 0.0]][rng(s) as usize % 9],
            1 => [0.0, uni(s, -1.0, 1.0)],
            _ => [uni(s, -1.0, 1.0), uni(s, -1.0, 1.0)],
        };
        // the last shot: a rally slice off another player sometimes, struck sweet or not
        let (shot_branch, shot_kind, sweet) = (int(s, 0, 5), int(s, 0, 5), rng(s) & 1 != 0);
        let rally_on = !rng(s).is_multiple_of(3);
        let incoming = (rally_on && shot_branch < 4 && shot_kind == kind::SLICE).then_some(sweet);
        incoming_n += incoming.is_some() as usize;
        let h = Hitter { end, branch: b, kind: k, offset, body, from, height };
        // base, width and half as the rally aim makes them; serves (branch 0) take the same core
        let drop = branch::rally(b) && k == kind::DROP;
        let sweet_hit = b != branch::DIVE && !body && offset.abs() < 2;
        let (long, near) = if drop { (9.885f32, 2.0f32) } else { (8.885, 3.0) };
        let long = if sweet_hit { long } else { long - if branch::rally(b) && k == kind::LOB { 0.5 } else { 1.0 } };
        let half = long / 2.0;
        let base = [0.0f32, (near + half) * end];
        let width = if sweet_hit { sideline(four) } else { sideline(four) - 0.5 };

        // the Japanese routine's inputs
        ee.set_f32(pl + END, end);
        for (i, c) in stats.con.iter().enumerate() {
            ee.set_u32(pl + CON + 4 * i as u32, *c as u32);
        }
        ee.set_u32(pl + BODY, stats.body[0] as u32);
        ee.set_u32(pl + BODY + 4, stats.body[1] as u32);
        ee.set_u32(pl + RISING, stats.rising as u32);
        ee.set_f32(pl + UNDER, stats.under);
        ee.set_u32(pl + LAST_SHOT, last);
        ee.write(pl + BRANCH, 1, b as u128);
        ee.set_u32(pl + KIND, code[k as usize]);
        ee.set_f32(pl + FROM, from[0]);
        ee.set_f32(pl + FROM + 8, from[1]);
        ee.set_f32(pl + HEIGHT, height);
        ee.set_u32(pl + FLAGS, if body { 4 } else { 0 });
        ee.set_u32(pl + OFFSET, offset as u32);
        ee.set_u32(last + 0x1b0, 1);
        ee.set_u32(last + 0x1b4, shot_branch as u32);
        ee.set_u32(last + 0x1b8, shot_kind as u32);
        ee.write(last + 0x1bc, 1, sweet as u128);
        ee.set_u32(count, if four { 4 } else { 2 });
        ee.set_u32(shots, if rally_on { 1 + rng(s) % 6 } else { 0 });
        for (i, v) in [base[0], 0.0, base[1], 0.0].iter().enumerate() {
            ee.set_f32(base_at + 4 * i as u32, *v);
        }
        for (i, v) in [stick[0], 0.0, stick[1], 0.0].iter().enumerate() {
            ee.set_f32(stick_at + 4 * i as u32, *v);
        }
        ee.call(f, &[out as u64, pl as u64, base_at as u64, stick_at as u64, button as u64, drop as u64], &[width, half], 400_000);
        let want = [ee.f32_at(out), ee.f32_at(out + 8)];
        let got = aim_from(&h, &stats, stick, four, button, incoming, base, width, half);
        let us = aim_from(&h, &AimStats { build: Build::Us, ..stats }, stick, four, button, incoming, base, width, half);
        let bits = |v: [f32; 2]| v.map(f32::to_bits);
        if bits([got[0], got[2]]) != bits(want) {
            bad.push(format!("{h:?} {stats:?} stick {stick:?} four {four} button {button} incoming {incoming:?}: port {got:?}, game {want:?}"));
        }
        us_differs += (bits([us[0], us[2]]) != bits(want)) as usize;
        narrowed += (b == branch::SMASH && k != kind::LOB && offset.abs() > 1 && from[1].abs() > 6.4) as usize;
        wide += (incoming.is_none() && (b == branch::GROUND && (k == kind::SLICE || k == kind::LOB) || branch::rally(b) && k == kind::LOB)) as usize;
        // the rally aim's extras: the centred stick's nudge (its two coins read back from what it gave) and the
        // plain smash's held depth
        if b != branch::SERVE {
            let nudge = ee.i32_at(pl + NUDGE);
            let mut coins = [(nudge.abs() == 5) as u32, (nudge > 0) as u32].into_iter().map(|c| c << 16);
            let a = aim(&h, &stats, stick, four, button, incoming, &mut || coins.next().unwrap_or(0));
            let held = ee.f32_at(pl + HELD);
            if (a.nudge, a.held.to_bits(), bits([a.target[0], a.target[2]])) != (nudge, held.to_bits(), bits(want)) {
                bad.push(format!("{h:?} stick {stick:?}: nudge/held/target {:?} vs game {:?}", (a.nudge, a.held, a.target), (nudge, held, want)));
            }
            nudges += (nudge != 0) as usize;
            helds += (held != 0.0) as usize;
        }
    }
    eprintln!("{n} aims: {narrowed} narrowed smashes, {wide} slices/lobs without the 90°, {incoming_n} off incoming slices, {nudges} nudged, {helds} held; the US rules miss {us_differs}");
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(5)].join("\n"));
    assert!(us_differs > n / 20, "the US rules matched the Japanese aim almost everywhere ({us_differs})");
    assert!(narrowed > 1000 && wide > 1000 && incoming_n > 1000 && nudges > 100 && helds > 100);
}

/// The Japanese disc's 14 `TParam.csv` rows as the app reads them (widened to the US layout).
fn japanese_tparam() -> Option<Vec<String>> {
    use hst_data::xb::Archive;
    let mut iso = hst_data::iso::Iso::open(hst_oracle::root().join("fixtures/discs/SCPS-15113.iso")).ok()?;
    let data = iso.read("PCDATA/PCDATA.XB").ok()?;
    let arc = Archive::parse(&data).ok()?;
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("tparam.csv"))?;
    let csv = String::from_utf8_lossy(&arc.read(e).ok()?).into_owned();
    Some((0..14).map(|n| csv.lines().find(|l| l.starts_with(&format!("{n},"))).unwrap().to_owned()).collect())
}

/// The Japanese timing tables (its one setup, which every serve grades with) against the app's serve tables for
/// both tosses (`serve::timing` over the rally counts the Japanese rows are widened with) and the rally table
/// (`swing::timing`): grades, length and depth bias, for the disc's 14 characters and 10k random counts.
#[test]
fn japanese_timing_tables() {
    use hst_sim::player::ReachStats;
    use hst_sim::{serve, swing};
    let Some(mut ee) = japan() else { return };
    let Some(rows) = japanese_tparam() else { return };
    let f = addr("jp_timing_tables");
    // its two memsets clear the tables only (zeroed below): the interpreter lacks memset's halfword splat (pcpyh)
    let memset = addr("jp_memset");
    ee.set_u32(memset, 0x03e0_0008); // jr ra
    ee.set_u32(memset + 4, 0);
    let pl = 0x01d0_0000u32;
    let (after_at, before_at, grades_at, len_at, bias_at) = (0x1318u32, 0x1324u32, 0x14e0u32, 0x151cu32, 0x13f0u32);
    let mut seed = 0x0123_4567_89ab_cdefu64;
    // rally counts after/before, the serve's after/before
    type Counts = ([i32; 3], [i32; 3], [i32; 3], [i32; 3]);
    let mut cases: Vec<Counts> = rows
        .iter()
        .map(|r| {
            let s = ReachStats::from_tparam(r);
            let cell = |i: usize| -> [i32; 3] {
                let v: Vec<i32> = r.split(',').nth(i).unwrap().split('/').map(|v| v.trim().parse().unwrap()).collect();
                [v[0], v[1], v[2]]
            };
            // the widened row's strong and weak toss counts are the rally ones
            assert_eq!((cell(30), cell(31), cell(32), cell(33)), (cell(28), cell(29), cell(28), cell(29)), "{r}");
            (s.after, s.before, cell(30), cell(31))
        })
        .collect();
    for _ in 0..10_000 {
        let s = &mut seed;
        let after = [int(s, 0, 7), int(s, 0, 7), int(s, 0, 7)];
        let before = [int(s, 0, 7), int(s, 0, 7), int(s, 0, 7)];
        cases.push((after, before, after, before));
    }
    let mut bad = vec![];
    for (after, before, toss_after, toss_before) in &cases {
        for o in (0..0x240).step_by(16) {
            ee.write(pl + 0x1300 + o, 16, 0);
        }
        for i in 0..3 {
            ee.set_u32(pl + after_at + 4 * i as u32, after[i] as u32);
            ee.set_u32(pl + before_at + 4 * i as u32, before[i] as u32);
        }
        ee.call(f, &[pl as u64], &[], 200_000);
        let len = ee.i32_at(pl + len_at) as usize;
        let want: (Vec<u8>, Vec<i32>) = ((0..len).map(|k| ee.u8_at(pl + grades_at + k as u32)).collect(), (0..len).map(|k| ee.i32_at(pl + bias_at + 4 * k as u32)).collect());
        let stroke = swing::timing(*after, *before);
        let toss = serve::timing(*toss_after, *toss_before, &stroke);
        if stroke != want || toss != want {
            bad.push(format!("{after:?}/{before:?}: game {want:?}, rally {stroke:?}, serve {toss:?}"));
        }
    }
    assert!(bad.is_empty(), "{} of {} differ:\n{}", bad.len(), cases.len(), bad[..bad.len().min(5)].join("\n"));
}

/// Every serve in a recorded Japanese bot match (`tools/record_jp.py`, fixtures/m163a1a1/jp-bot.bin) graded by its
/// timing table at the frame it locked: grade and depth bias are that table's entry, and the table is the app's
/// serve table for the toss (`serve::timing` over the widened row, i.e. the rally counts) and its rally table. The
/// recorded counts are the disc row's. Serves only (the server's branch 0); rally locks are in the file too.
#[test]
fn japanese_serves_grade_with_the_rally_table() {
    use hst_sim::player::ReachStats;
    use hst_sim::{serve, swing};
    // the Japanese disc is optional (skip quietly); with it, the recording is required under HST_REQUIRE_DISC
    let Some(rows) = japanese_tparam() else { return };
    let Ok(rec) = hst_data::fixture::read("fixtures/m163a1a1/jp-bot.bin") else { return };
    let count = u32::from_le_bytes(rec[..4].try_into().unwrap()) as usize;
    let (globals, head, player) = (0x180usize, 0x100usize, 0x290 + 0x180);
    let size = 4 + globals + head + count * player;
    let frames: Vec<&[u8]> = rec[4..].chunks_exact(size).collect();
    let word = |f: &[u8], at: usize| i32::from_le_bytes(f[at..at + 4].try_into().unwrap());
    // a player field: the first block holds +0x12a0.., the second +0x3bc0..
    let field = |p: usize, off: usize| {
        let base = 4 + globals + head + p * player;
        if off >= 0x3bc0 { base + 0x290 + off - 0x3bc0 } else { base + off - 0x12a0 }
    };
    let (mut serves, mut tosses, mut prev) = (0, [0; 3], vec![-1i32; count]);
    for f in &frames {
        let server = word(f, 4 + 0xcc) as usize;
        for p in 0..count {
            let idx = word(f, field(p, 0x3c34));
            let locked = idx >= 0 && prev[p] < 0;
            prev[p] = idx;
            if !locked || p != server || f[field(p, 0x3c31)] != 0 {
                continue;
            }
            let at = format!("vsync {} player {p}", word(f, 0));
            let ch = word(f, 4 + 0x28 + 4 * p) as usize; // the match's characters
            let counts = |off: usize| [0, 1, 2].map(|i| word(f, field(p, off) + 4 * i));
            let (after, before) = (counts(0x1318), counts(0x1324));
            let s = ReachStats::from_tparam(&rows[ch]);
            assert_eq!((after, before), (s.after, s.before), "{at}: character {ch}'s counts");
            let len = word(f, field(p, 0x151c)) as usize;
            let table: (Vec<u8>, Vec<i32>) =
                ((0..len).map(|k| f[field(p, 0x14e0) + k]).collect(), (0..len).map(|k| word(f, field(p, 0x13f0) + 4 * k)).collect());
            let stroke = swing::timing(after, before);
            assert_eq!(table, stroke, "{at}: the recorded table");
            assert_eq!(serve::timing(after, before, &stroke), stroke, "{at}: the serve table");
            let (grade, bias) = (f[field(p, 0x3c58)], word(f, field(p, 0x3cf8)));
            assert!((idx as usize) < len, "{at}: index {idx} past {len}");
            assert_eq!((grade, bias), (table.0[idx as usize], table.1[idx as usize]), "{at}: index {idx}");
            tosses[(word(f, field(p, 0x3c10)) as usize).min(2)] += 1;
            serves += 1;
        }
    }
    eprintln!("{} frames, {serves} serves (toss kinds {tosses:?})", frames.len());
    assert!(serves >= 20, "only {serves} serves");
}

/// The CPU reaction draws in a recorded Japanese bot match (`tools/record_jp.py … ai`, fixtures/m163a1a1/jp-bot-ai.bin;
/// the two CPUs' AI records beside it in jp-bot-ai.rows, from the same state's RAM), the AI generator in step: every
/// time a CPU's reaction wait is drawn, `AiParams::reaction` over the AIParam row the app parses from the Japanese disc,
/// fed the recorded generator, gives the recorded wait in one draw, with the Japanese rules (no lob or special-serve
/// half). The draw sits at the same place for an incoming lob as for any other shot: the lob draws nothing more.
/// A serve's return is replayed whole from the AI record (seen shots, recorded timing errors): timing errors,
/// shot-choice picks, reaction and guess (ヤマ張り) must use exactly the frame's draws and give the recorded wait,
/// which a guess replaces with its move frames, and the recorded guess.
#[test]
fn japanese_cpu_reactions() {
    use hst_sim::ai::{AiParams, Guess, RECORD_BYTES, Reacting, Seen, Timing};
    use hst_sim::build::Build;
    use hst_sim::rng::Mt;
    use hst_sim::shot::branch;
    let Some(mut iso) = hst_data::iso::Iso::open(hst_oracle::root().join("fixtures/discs/SCPS-15113.iso")).ok() else { return };
    let (Ok(rec), Ok(rows)) =
        (hst_data::fixture::read("fixtures/m163a1a1/jp-bot-ai.bin"), hst_data::fixture::read("fixtures/m163a1a1/jp-bot-ai.rows"))
    else {
        return;
    };
    let xb = iso.read("PCDATA/PCDATA.XB").unwrap();
    let arc = hst_data::xb::Archive::parse(&xb).unwrap();
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with("aiparam.csv")).unwrap();
    let table = AiParams::table(&hst_data::columns::us_layout(&e.name, arc.read(e).unwrap()));
    let head = u32::from_le_bytes(rec[..4].try_into().unwrap());
    assert_eq!(head & 0x100, 0x100, "not an `ai` recording");
    let count = (head & 0xff) as usize;
    let mt_at = 4 + 0x180 + 0x100 + count * (0x290 + 0x180);
    let ai_at = |p: usize| mt_at + 0x9d0 + p * 0x260;
    let size = ai_at(count);
    let frames: Vec<&[u8]> = rec[4..].chunks_exact(size).collect();
    let word = |f: &[u8], at: usize| i32::from_le_bytes(f[at..at + 4].try_into().unwrap());
    let mt = |f: &[u8]| {
        let mut g = Mt::new(0);
        g.0.0 = std::array::from_fn(|i| word(f, mt_at + 4 + 4 * i) as u32);
        g.0.1 = word(f, mt_at + 0x9c4) as usize;
        g
    };
    // each CPU's row: the table row whose in-memory record is the recorded one
    let params: Vec<AiParams> = (0..count)
        .map(|p| {
            // the Japanese record is the US one without the lob and special-serve reaction words (8 bytes at +0x68)
            let want = &rows[p * RECORD_BYTES..(p + 1) * RECORD_BYTES];
            let jp = |a: &AiParams| [&a.bytes()[..0x68], &a.bytes()[0x70..]].concat();
            *table.iter().find(|a| jp(a)[..] == want[..RECORD_BYTES - 8] && a.react_lob == 0 && a.react_special_serve == 0).expect("no AIParam row")
        })
        .collect();
    let (mut seen, mut lobs, mut us_differs, mut fast, mut odd) = ([0usize; 4], 0, 0, 0, vec![]);
    let mut guesses = 0;
    for w in frames.windows(2) {
        let (prev, f) = (w[0], w[1]);
        let mut ai_rng = mt(prev);
        let n = (word(f, mt_at + 0x9c4) - word(prev, mt_at + 0x9c4)).rem_euclid(624) as usize;
        if n == 0 {
            continue;
        }
        let draws: Vec<u32> = (0..n).map(|_| ai_rng.next()).collect();
        for (p, a) in params.iter().enumerate() {
            let (wait, before) = (word(f, ai_at(p) + 0x238), word(prev, ai_at(p) + 0x238));
            if wait == (before - 1).max(0) || wait == 0 {
                continue;
            }
            let at = format!("vsync {} player {p}", word(f, 0));
            // the AI's far/near bases are the row's singles ones
            assert_eq!((word(f, ai_at(p) + 0x20), word(f, ai_at(p) + 0x24)), (a.react_singles_base, a.react_singles_net), "{at}");
            let (kind, shot) = (word(f, ai_at(p) + 0x124), word(f, ai_at(p) + 0x128));
            let tick = (p == 1) as i32;
            if kind == branch::SERVE as i32 {
                // the receiver's draws end the frame's, or player 1's 15 follow them when player 0 receives
                let end = if p == 0 { n - 15 } else { n };
                match serve_return(a, f, ai_at(p), mt(prev), end) {
                    Some((want, guess)) if want == wait + tick && f[ai_at(p) + 0x23f] == guess.map_or(0, |g| 1 + (g == Guess::Other) as u8) => {
                        guesses += guess.is_some() as usize;
                        seen[0] += 1;
                    }
                    _ => odd.push(format!("{at}: serve return, wait {wait}, guess byte {}", f[ai_at(p) + 0x23f])),
                }
                continue;
            }
            let lob = (branch::GROUND as i32..=branch::DIVE as i32).contains(&kind) && shot == hst_sim::shot::kind::LOB;
            // where the draw falls in the frame: last in the receiver's own draws, which player 1's 15 follow when
            // player 0 receives (it updates first). Player 1's wait has already counted down once by the end of the
            // frame
            let backs: &[usize] = if p == 0 { &[16] } else { &[1] };
            // the wait and the draws it took
            let answer = |back: usize, fast_ball: i32, near_net: bool, build: Build| {
                let mut it = draws[n - back..].iter().copied().chain(std::iter::repeat(0));
                let mut took = 0;
                let t = Timing { fast_ball, ..Timing::default() };
                let on = Reacting { lob: lob && build.lob_reaction(), special_serve: false, beside_human: false, near_net };
                let wait = a.reaction(&t, &Seen { kind, ..Seen::default() }, on, &mut || {
                    took += 1;
                    it.next().unwrap()
                });
                (wait, took)
            };
            // the fast-ball frames, or none
            let tries = backs.iter().flat_map(|&b| [0, a.react_speed[1]].map(move |s| (b, s))).flat_map(|(b, s)| [(b, s, false), (b, s, true)]);
            let hit = tries.clone().find(|&(b, s, near)| answer(b, s, near, Build::Japan) == (wait + tick, 1));
            let Some((b, s, near)) = hit else {
                odd.push(format!("{at}: wait {wait}, kind {kind}/{shot}"));
                continue;
            };
            fast += (s != 0) as usize;
            seen[(kind.clamp(0, 4) as usize).min(3)] += 1;
            if lob {
                lobs += 1;
                // the US rule draws the lob's half too (nothing to add on these rows, but one draw more: every later
                // draw would fall out of step)
                us_differs += (answer(b, s, near, Build::Us) != (wait + tick, 1)) as usize;
            }
        }
    }
    eprintln!(
        "{} frames: reactions to serves/ground/volley-dive/smash {seen:?} ({fast} fast balls, {guesses} serve guesses), {lobs} lobs, the US rule misses {us_differs}",
        frames.len()
    );
    assert!(odd.is_empty(), "{} reactions no draw gives:\n{}", odd.len(), odd.join("\n"));
    assert!(lobs >= 20 && seen.iter().sum::<usize>() >= 200 && guesses >= 1, "too few reactions");
    assert_eq!(us_differs, lobs, "the US lob half should take a draw more on every lob");
}

/// A recorded serve's return replayed through the port from the receiver's AI record (at `ai` in frame `f`; the
/// Japanese record is the US one 0x10 lower from the seen shots on): the timing draw whose errors are the recorded
/// ones, starting anywhere in the frame's draws on generator `g`, then the picks, the reaction and the guess, which
/// must use the draws up to `end` exactly. The frames it waits (a guess's move frames in place of the reaction) and
/// the guess.
fn serve_return(a: &hst_sim::ai::AiParams, f: &[u8], ai: usize, mut g: hst_sim::rng::Mt, end: usize) -> Option<(i32, Option<hst_sim::ai::Guess>)> {
    use hst_sim::ai::{Reacting, Seen, Shots};
    let word = |o: usize| i32::from_le_bytes(f[ai + o..ai + o + 4].try_into().unwrap());
    let fl = |o: usize| f32::from_bits(word(o) as u32);
    let seen = |r: usize| Seen { kind: word(r + 4), vel: [fl(r + 0x30), fl(r + 0x34), fl(r + 0x38)] };
    let shots = Shots {
        last: seen(0x120),
        before: (word(0x160) >= 0).then(|| seen(0x160)),
        serve_before: (word(0x1e0) >= 0).then(|| seen(0x1e0).vel),
    };
    let errs = [0x224, 0x228, 0x22c, 0x230].map(word);
    let draws: Vec<u32> = (0..end + 64).map(|_| g.next()).collect();
    (0..end).find_map(|j| {
        let mut it = draws[j..].iter().copied();
        let mut used = j;
        let mut roll = || {
            used += 1;
            it.next().unwrap()
        };
        let t = a.timing(Some(&shots), true, f[ai + 0x220] != 0, false, &mut roll);
        if [t.stroke, t.volley, t.smash, t.serve] != errs {
            return None;
        }
        a.picks(false, &mut roll);
        // a receiver stands at its baseline
        let reaction = a.reaction(&t, &shots.last, Reacting::default(), &mut roll);
        let guess = a.guess(&t, true, true, &mut roll);
        (used == end).then(|| (guess.map_or(reaction, |_| a.guess[1]), guess))
    })
}
