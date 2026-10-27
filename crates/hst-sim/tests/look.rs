//! B675: the head look against the game. `research/b675_turn_rec.py 5 2400` (slot 5, the bot-only doubles game)
//! dumps, every frame, each player's motion player (with its look state), its head node and the head's parent node
//! (world matrix, local matrix, sampled rotation and position), the live ball and the match globals.

use hst_data::{ani, fixture, iso::Iso, mdl, xb::Archive};
use hst_sim::look::{Look, Pose, Target};
use hst_sim::pose::{Clip, M4, Skeleton, q_matrix};

const PLAYER: usize = 0xc0 + 0x240 + 0x10 + 0x50 + 8;
const HEAD: usize = 0x64;
const LEN: usize = HEAD + 4 * PLAYER;

fn f(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn i(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn m4(b: &[u8], o: usize) -> M4 {
    std::array::from_fn(|r| std::array::from_fn(|c| f(b, o + 16 * r + 4 * c)))
}
fn v4(b: &[u8], o: usize) -> [f32; 4] {
    std::array::from_fn(|c| f(b, o + 4 * c))
}

struct Frame<'a>(&'a [u8]);

impl<'a> Frame<'a> {
    fn v(&self) -> u32 {
        i(self.0, 0) as u32
    }
    fn ball(&self) -> [f32; 4] {
        v4(self.0, 0x44)
    }
    /// The shared contact hold.
    fn hold(&self) -> i32 {
        i(self.0, 0x54 + 8)
    }
    fn phase(&self) -> u8 {
        self.0[4 + 5]
    }
    fn p(&self, k: usize) -> Player<'a> {
        Player(&self.0[HEAD + k * PLAYER..HEAD + (k + 1) * PLAYER])
    }
}

struct Player<'a>(&'a [u8]);

impl Player<'_> {
    fn motion(&self) -> i32 {
        i(self.0, 0x20)
    }
    fn time(&self) -> f32 {
        f(self.0, 0x38)
    }
    fn look(&self) -> Look {
        let a = |o: usize| f(self.0, o);
        Look {
            ready: self.0[0xa0] != 0,
            reseed: self.0[0xa1] == 0,
            snap: self.0[0xa2] != 0,
            applied: [a(0xa4), a(0xa8)],
            rest: [a(0xac), a(0xb0)],
            last: [a(0xb4), a(0xb8)],
            at_end: i(self.0, 0xbc),
        }
    }
    fn head(&self) -> &[u8] {
        &self.0[0xc0..0xc0 + 0x120]
    }
    fn neck(&self) -> &[u8] {
        &self.0[0x1e0..0x1e0 + 0x120]
    }
    fn z(&self) -> f32 {
        f(self.0, 0x308)
    }
    fn latch(&self) -> bool {
        self.0[0x360] != 0
    }
    /// The head's local as the clip posed it: the sampled rotation's rows and the sampled position.
    fn clip(&self) -> M4 {
        let h = self.head();
        let mut m = q_matrix(v4(h, 0xe0));
        m[3] = v4(h, 0xf0);
        m
    }
}

fn bits(m: &M4) -> Vec<u32> {
    m.iter().flatten().map(|x| x.to_bits()).collect()
}

/// The characters in slot 5's seats.
const CHARS: [usize; 4] = [0, 2, 1, 5];

/// Character `c`'s first costume skeleton.
fn skeleton(iso: &mut Iso, c: usize) -> Skeleton {
    let data = iso.read(&format!("PC/PC{c:02}C00.XB")).unwrap();
    let arc = Archive::parse(&data).unwrap();
    let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(".mdl")).unwrap();
    let m = mdl::parse(&arc.read(e).unwrap()).unwrap();
    Skeleton { names: m.node_names.clone(), parent: m.node_parent.clone(), rest: m.node_local.clone() }
}

/// The head's rest local.
fn rest(sk: &Skeleton) -> M4 {
    sk.rest[sk.names.iter().position(|n| n == "Bip01Head").unwrap()]
}

/// Per character and motion the clip's length (0 without a clip).
struct Lengths(std::collections::HashMap<(usize, i32), f32>, Vec<Skeleton>);

impl Lengths {
    fn new(iso: &mut Iso) -> Lengths {
        Lengths(Default::default(), CHARS.map(|c| skeleton(iso, c)).to_vec())
    }
    fn of(&mut self, iso: &mut Iso, k: usize, m: i32) -> f32 {
        let c = CHARS[k];
        let sk = &self.1[k];
        *self.0.entry((c, m)).or_insert_with(|| {
            let data = iso.read(&format!("PCANI/PC{c:02}ANI.XB")).unwrap();
            let arc = Archive::parse(&data).unwrap();
            let Some(stem) = ani::motion_name(m as usize, c) else { return 0.0 };
            let stem = stem.to_ascii_lowercase();
            arc.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&format!("{stem}.ani2"))).map_or(0.0, |e| Clip::new(sk, &ani::parse(&arc.read(e).unwrap()).unwrap()).length)
        })
    }
}

/// The match phase once the point is over.
const POINT_OVER: u8 = 4;

/// What the game does for player `k` on frame `b` with its latch on: track the ball, or (doubles, point live) let go
/// when this player is the nearer of the pair to the net and the ball is on its side, deeper than it.
fn latched_target(b: &Frame, k: usize) -> Target {
    let (z, partner, ball) = (b.p(k).z(), b.p((k + 2) % 4).z(), b.ball());
    let side = |v: f32| v >= 0.0;
    if b.phase() != POINT_OVER && z.abs() < partner.abs() && side(ball[2]) == side(z) && z.abs() < ball[2].abs() {
        Target::Release
    } else {
        Target::Ball(ball)
    }
}

/// Every settled tick of the four players (the motion unchanged for 30 frames, so the dumped sampled rotation is
/// the posed head; no contact hold): a player whose latch is on calls the look toward the ball or, in the doubles
/// case, releases, and the state and the head's new local matrix come out bit for bit as the game's; a player whose
/// latch is off in a motion that isn't a stroke doesn't call it (state and head as the clip left them).
#[test]
fn turns_the_head_like_the_game() {
    let Ok(mut iso) = fixture::iso() else { return };
    let Ok(data) = fixture::read("fixtures/b675/run1.bin") else { return fixture::skip("b675/run1.bin") };
    let mut lengths = Lengths::new(&mut iso);
    let rests: Vec<M4> = lengths.1.iter().map(rest).collect();
    let all: Vec<Frame> = data.chunks_exact(LEN).map(Frame).collect();
    let stroke = |m: i32| (0x10..=0x1f).contains(&m) || (0x25..=0x2a).contains(&m);
    let (mut since, mut tracked, mut released, mut idle) = ([0usize; 4], 0, 0, 0);
    for w in all.windows(3) {
        let (a, b, c) = (&w[0], &w[1], &w[2]);
        for k in 0..4 {
            let (pa, pb) = (a.p(k), b.p(k));
            since[k] = if pa.motion() != pb.motion() { 0 } else { since[k] + 1 };
            if b.v() != a.v() + 1 || since[k] < 30 || b.hold() > 0 || a.hold() > 0 || c.p(k).motion() != pb.motion() {
                continue;
            }
            // the head's position track has one key: its local row 3 is what the look wrote last tick
            let mut clip = pb.clip();
            clip[3] = m4(pa.head(), 0x80)[3];
            let game = m4(pb.head(), 0x80);
            let at = format!("v{} p{k} m{:#x}", b.v(), pb.motion());
            let target = match (pa.latch() || pb.latch(), stroke(pb.motion())) {
                (true, _) => latched_target(b, k),
                (false, false) => {
                    assert_eq!(pa.look(), pb.look(), "{at}: no call");
                    assert_eq!(bits(&clip), bits(&game), "{at}: the clip's head");
                    idle += 1;
                    continue;
                }
                // ponytail: a stroke with the latch off (just hit, dive, whiff, smash): the gate's own test covers it
                (false, true) => continue,
            };
            let clip_done = lengths.of(&mut iso, k, pb.motion()) <= pb.time();
            if !clip_done && pa.time() == pb.time() {
                // the players' tick didn't run this frame (the game paused between points)
                continue;
            }
            let pose = Pose { clip, rest: rests[k], parent: m4(pb.neck(), 0), mirrored: false, clip_done, factor: 1.0 };
            let mut look = pa.look();
            let head = look.turn(&pose, target);
            assert_eq!(look, pb.look(), "{at}: state ({target:?})");
            assert_eq!(head.map(|h| bits(&h)), Some(bits(&game)), "{at}: head");
            match target {
                Target::Ball(_) => tracked += 1,
                Target::Release => released += 1,
            }
        }
    }
    eprintln!("{tracked} tracking, {released} releasing, {idle} idle");
    assert!(tracked > 1500 && released > 200 && idle > 1000);
}
