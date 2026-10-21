//! The AI's predicted ball path (`hst_sim::path`) against the game's own, frame by frame. Recordings
//! (`context/live/p11n1/*.bin`, not in git) come from `tools/record_live.py <slot> <frames> <out> --path` (the commands
//! are in the P11n1 journal): each frame's live ball, predictor and path record. Each frame starts from the previous
//! frame's record and predictor, runs what the game ran that frame (the update; a launch, a net touch) and must give
//! the next frame's end, start, active, mode, gap, new entries and predictor bit for bit.
//!
//! - `doubles`: slot 5 (bot doubles, court 10), serves and rallies; `singles`: `bots_singles` (slot 8), no look countdown.
//! - `cord`: a ball poked into the net cord (net touches re-seed the predictor, keeping its counts).
//! - `out_a`, `out_b`, `cap`, `cap2`, `cap4`: the ball poked and the predictor re-seeded from it (`--reseed`, `.seed`):
//!   balls landing long (the gap), balls hit straight up.
//! - `cap_e`, `cap_f`: end/start poked near the 600-entry cap (`--setpath`).
//!
//! Skips when the recordings are absent.

use hst_data::exe::Game;
use hst_sim::ball::{Ball, COURTS, Flight, Params, Shot};
use hst_sim::judge::{Call, Lines};
use hst_sim::path::Path;
use hst_sim::rally;

/// `bots_singles`' court surface (courts 1, 2, 3 and 9 share it; the others diverge at the first bounce).
const SINGLES_COURT: usize = 1;
const MAIN: usize = 4 + 0x290 + 0x290 + 0x40;
const HEAD: usize = 0x130;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn v3(b: &[u8], o: usize) -> [f32; 3] {
    [f(b, o), f(b, o + 4), f(b, o + 8)]
}
fn rows(b: &[u8], o: usize) -> [[f32; 4]; 4] {
    std::array::from_fn(|r| std::array::from_fn(|k| f(b, o + 16 * r + 4 * k)))
}
fn call(b: u8) -> Call {
    [Call::None, Call::In, Call::Out, Call::Net, Call::NetIn, Call::NetOut][b as usize]
}

/// A ball object (0x290 bytes) as a flight and its shot.
fn load(b: &[u8]) -> (Flight, Shot) {
    let ball = Ball { pos: v3(b, 0xe0), vel: v3(b, 0x130), spin: f(b, 0x1a4) };
    let mut fl = Flight::new(ball, rows(b, 0x160), rows(b, 0x1c0));
    fl.frame = i(b, 0xac);
    (fl.bounces, fl.contacts, fl.special_contacts) = (i(b, 0x224), i(b, 0x228), i(b, 0x22c));
    fl.rolling = b[0xa4] == 2;
    fl.slide = f(b, 0x200);
    fl.call = call(b[0xa5]);
    fl.line_distance = f(b, 0x230);
    let shot = Shot {
        params: Params::default(),
        curve: f(b, 0x254),
        bend: f(b, 0x250),
        side: v3(b, 0x90),
        curve_frames: i(b, 0x260),
        wind: v3(b, 0x240),
        first_bounce_spin: f(b, 0x1a8),
        first_bounce_restitution: f(b, 0x1ac),
        class: b[0x58],
        kind: i(b, 0x5c),
        bounce_turn: f(b, 0x1b0),
    };
    (fl, shot)
}

/// One recorded frame: live ball, predictor, record header, and the tip entries (`first`, entries from there).
struct Frame<'a> {
    vsync: i32,
    live: &'a [u8],
    pred: &'a [u8],
    head: &'a [u8],
    first: i32,
    tip: Vec<rally::Ball>,
}

impl Frame<'_> {
    fn end(&self) -> i32 {
        i(self.head, 4)
    }
    fn start(&self) -> i32 {
        i(self.head, 8)
    }
    fn phase(&self) -> u8 {
        self.head[0x105]
    }
    fn look(&self) -> i32 {
        i(self.head, 0xe8)
    }
    /// The record as the port holds it: entries before the tip are stand-ins.
    fn path(&self) -> Option<Path> {
        let (end, first) = (self.end() as usize, self.first);
        if first < 0 || (end > 0 && self.tip.is_empty()) {
            return None;
        }
        let mut entries = vec![rally::Ball::default(); end - self.tip.len()];
        entries.extend_from_slice(&self.tip);
        let mut p = Path::new(load(self.pred).0);
        (p.entries, p.start, p.active) = (entries, self.start(), self.head[0xc] != 0);
        (p.mode, p.gap, p.look) = (call(self.head[0xd]), f(self.head, 0x10), self.look());
        Some(p)
    }
}

fn frames<'a>(main: &'a [u8], path: &'a [u8]) -> Vec<Frame<'a>> {
    let mut out = Vec::new();
    let (mut o, mut k) = (0, 0);
    while o < path.len() && (k + 1) * MAIN <= main.len() {
        let s = &main[k * MAIN..(k + 1) * MAIN];
        let head = &path[o + 4..o + 4 + HEAD];
        let (first, n) = (i(path, o + 4 + HEAD), i(path, o + 8 + HEAD) as usize);
        let at = o + 12 + HEAD;
        let tip = (0..n)
            .map(|j| {
                let e = &path[at + j * 0x30..];
                rally::Ball { pos: std::array::from_fn(|c| f(e, 4 * c)), vel: std::array::from_fn(|c| f(e, 0x10 + 4 * c)), bounces: i(e, 0x20) }
            })
            .collect();
        assert_eq!(i(path, o), i(s, 0), "main and path files out of step");
        out.push(Frame { vsync: i(s, 0), live: &s[4..4 + 0x290], pred: &s[4 + 0x290..4 + 0x520], head, first, tip });
        o = at + n * 0x30;
        k += 1;
    }
    out
}

#[derive(Default, Debug)]
struct Tally {
    frames: usize,
    entries: usize,
    hits: usize,
    nets: usize,
    outs: usize,
    capped: usize,
    failures: Vec<String>,
}

/// What a recording was poked with: the predictor re-seeded after `vsync` (its bytes from 0x58), or end/start set.
enum Poke {
    None,
    Seed(i32, Vec<u8>),
    SetPath(i32, i32, i32),
}

fn run(fs: &[Frame], court: usize, margin: f32, poke: &Poke, t: &mut Tally) {
    let surface = &COURTS[court];
    for w in fs.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if b.vsync != a.vsync + 1 {
            continue;
        }
        // stops (strike, reaction, serve, point end) clear the record: nothing to step
        if b.end() == 0 {
            continue;
        }
        let lines = |fr: &Frame, ball: &[u8]| Lines {
            shots: i(fr.head, 0x120),
            doubles: i(fr.head, 0x128) > 2,
            side: i(fr.head, 0x110),
            hitter_far: f(ball, 0x138) < 0.0,
            margin,
        };
        let calls = b.phase() == rally::phase::RALLY;
        let mut p = match poke {
            Poke::Seed(v, seed) if *v == a.vsync => {
                let mut pred = a.pred.to_vec();
                pred[0x58..0x58 + seed.len()].copy_from_slice(seed);
                let mut p = Path::new(load(&pred).0);
                (p.mode, p.gap, p.look) = (Call::None, f(a.head, 0x10), a.look());
                p.new_path();
                p
            }
            Poke::SetPath(v, end, start) if *v == a.vsync => {
                let Some(mut p) = a.path() else { continue };
                p.entries.resize(*end as usize, rally::Ball::default());
                p.start = *start;
                p
            }
            _ => match a.path() {
                Some(p) => p,
                None => continue,
            },
        };
        let (_, shot) = load(a.pred);
        let old = p.entries.len();
        p.predictor.lines = Some(lines(a, a.pred));
        let swinging = (i(b.head, 0x128) > 2).then_some(b.look() == 15);
        let hit = i(b.live, 0xac) == 0 && v3(b.live, 0x130) != [0.0; 3] && b.end() == 15 && b.start() == 0;
        let net = i(b.live, 0x22c) > i(a.live, 0x22c);
        let kind = if hit {
            p.frame(&shot, surface, calls, swinging);
            let (mut live, shot) = load(b.live);
            live.lines = Some(lines(b, b.live));
            p.hit(&live, &shot, surface, calls, b.phase() > 1);
            t.hits += 1;
            "hit"
        } else if net {
            let (mut live, shot) = load(b.live);
            live.lines = Some(lines(b, b.live));
            p.net(&live);
            p.new_path();
            p.frame(&shot, surface, calls, swinging);
            t.nets += 1;
            "net"
        } else {
            p.frame(&shot, surface, calls, swinging);
            "frame"
        };
        // the record
        let got = (p.entries.len() as i32, p.start, p.active, p.mode as u8, p.gap.to_bits());
        let want = (b.end(), b.start(), b.head[0xc] != 0, b.head[0xd], f(b.head, 0x10).to_bits());
        if got != want {
            t.failures.push(format!("{} ({kind}): end/start/active/mode/gap {got:?}, want {want:?}", b.vsync));
            continue;
        }
        if p.mode == Call::Out && a.head[0xd] != Call::Out as u8 {
            t.outs += 1;
        }
        if want.0 as usize >= hst_sim::path::CAP {
            t.capped += 1;
        }
        if i(b.head, 0x128) > 2 {
            let n = b.look();
            let prev = a.look();
            if n != 15 && n != (prev - 1).max(0) && n != 0 {
                t.failures.push(format!("{}: look {n} after {prev}", b.vsync));
            }
        } else if b.look() != 0 {
            t.failures.push(format!("{}: singles look {}", b.vsync, b.look()));
        }
        // the tip entries, bit for bit
        if b.first >= 0 {
            for (j, want) in b.tip.iter().enumerate() {
                let got = p.entries[b.first as usize + j];
                let bits = |e: &rally::Ball| (e.pos.map(f32::to_bits), e.vel.map(f32::to_bits), e.bounces);
                if b.first as usize + j >= old || kind != "frame" {
                    t.entries += 1;
                    if bits(&got) != bits(want) {
                        t.failures.push(format!("{} ({kind}) entry {}: {got:?}, want {want:?}", b.vsync, b.first as usize + j));
                        break;
                    }
                }
            }
        }
        // the predictor it leaves
        let (want, _) = load(b.pred);
        let q = &p.predictor;
        let bits = |f: &Flight| (f.ball.pos.map(f32::to_bits), f.ball.vel.map(f32::to_bits), f.bounces, f.contacts, f.special_contacts, f.call);
        if bits(q) != bits(&want) {
            t.failures.push(format!("{} ({kind}) predictor {:?}, want {:?}", b.vsync, bits(q), bits(&want)));
        }
        t.frames += 1;
    }
}

#[test]
fn the_path_record_matches_the_game_frame_by_frame() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let dir = format!("{root}/context/live/p11n1");
    let (Ok(cnf), Ok(bin)) = (std::fs::read(format!("{root}/context/iso/SYSTEM.CNF")), std::fs::read(format!("{root}/context/iso/ZZBIN/GAME.BIN"))) else {
        eprintln!("disc missing, skipped");
        return;
    };
    let margin = Game::new(&cnf, &bin).unwrap().line_margin();
    // (recording, court, the (vsync, end, start) it was poked with, if any)
    type Case = (&'static str, usize, Option<(i32, i32, i32)>);
    let cases: [Case; 10] = [
        ("doubles", 10, None),
        ("singles", SINGLES_COURT, None),
        ("cord", 10, None),
        ("out_a", 10, None),
        ("out_b", 10, None),
        ("cap", 10, None),
        ("cap2", 10, None),
        ("cap4", 10, None),
        ("cap_e", 10, Some((7900, 585, 575))),
        ("cap_f", 10, Some((7900, 590, 580))),
    ];
    let mut t = Tally::default();
    let mut found = 0;
    for (name, court, set) in cases {
        let (Ok(main), Ok(path)) = (std::fs::read(format!("{dir}/{name}.bin")), std::fs::read(format!("{dir}/{name}.bin.path"))) else {
            continue;
        };
        found += 1;
        let poke = match (set, std::fs::read_to_string(format!("{dir}/{name}.bin.poke")), std::fs::read(format!("{dir}/{name}.bin.seed"))) {
            (Some((v, e, s)), ..) => Poke::SetPath(v, e, s),
            (None, Ok(v), Ok(seed)) => Poke::Seed(v.trim().parse().unwrap(), seed),
            _ => Poke::None,
        };
        let before = t.failures.len();
        run(&frames(&main, &path), court, margin, &poke, &mut t);
        eprintln!("{name}: {} failures so far", t.failures.len() - before);
    }
    if found == 0 {
        eprintln!("recordings missing, skipped");
        return;
    }
    eprintln!(
        "{} frames, {} entries bit-exact; {} hits, {} net touches, {} outs, {} capped",
        t.frames, t.entries, t.hits, t.nets, t.outs, t.capped
    );
    assert!(t.failures.is_empty(), "{} frames diverged:\n{}", t.failures.len(), t.failures[..t.failures.len().min(20)].join("\n"));
    assert!(t.hits > 10 && t.nets > 0 && t.outs > 1 && t.capped > 1);
}

