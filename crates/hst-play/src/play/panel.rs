//! The original's player/score panel (textures from `AZUMA/INPANE/INPANE.XB0`), shown from the serve until the
//! rally starts: each player's pill, face, slot label (1P..4P / COM) and rank, both teams' points, the Sets/Games
//! strips, the team banners (doubles) and the pulsing serve ring.
//!
//! It slides in from the screen's edges over the serve's first ticks (offset `cnt·44.8 − 224` for cnt 0..5), the
//! ring pulses once it is in (31 ticks up, 31 down), and when the rally starts everything fades out over 5 ticks
//! (alpha 128·c/5, c = 5..1) except the team banners, which stay solid until the panel goes. Coordinates are the
//! PS2's 640×448 screen at the 4:3 screen's scale, each half pinned to its window edge (`widescreen::pin`);
//! colours are GS colours (128 = the texture's own).

use bevy::prelude::*;
use hst_data::{iso::Iso, tim2, xb::Archive};
use hst_sim::score::{Rules, Score};

use super::{Game, Pads, Phase};
use crate::Args;

/// The panel's textures, in `Tex` order.
const TEXTURES: [&str; 11] = [
    "i_status_00",
    "inpane_p",
    "i_status_02",
    "inpane_kihontokuten00",
    "inpane_tiebreak00",
    "inpane_sen1",
    "inpane_dani",
    "inpane_mini0",
    "inpane_mini1",
    "inpane_team",
    "i_gameinfo_00",
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tex {
    /// White pill behind a player (112×48 at 0,0).
    Pill,
    /// "1P 2P 3P 4P COM", 40×24 cells.
    Slot,
    /// The serve ring, 56×56.
    Ring,
    /// Points 0/15/30/40/Deuce/Advantage, 128×64 cells.
    Points,
    /// Tiebreak points, 64×64 cells.
    Tiebreak,
    /// The Sets/Games strip's background fade.
    Strip,
    /// Rank labels, 64×24 rows.
    Rank,
    /// "Games:" and the games digits.
    Games,
    /// "Sets:" and the sets digits.
    Sets,
    /// The team banners' "A Team" / "B Team" (the menus' rebuilt `inpane_team`, 96×32 cells).
    Team,
    /// Our own pill sheet ([`super::menu_art::pill_layers`]) behind a team banner's words.
    TeamPill,
    /// "Score to win this game!" and the other calls, 256×24 rows.
    Info,
    /// Player `i`'s face, 64×64.
    Face(usize),
}

/// One textured rectangle: source rect in the texture (u, v, w, h), screen rect (x, y, w, h), GS tint and alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Quad {
    tex: Tex,
    src: [f32; 4],
    dst: [f32; 4],
    rgb: [f32; 3],
    alpha: f32,
}

/// What the panel shows this frame.
struct View {
    players: usize,
    /// Per player: 0..3 the controlling slot (1P..4P), 4 COM.
    slots: [usize; 4],
    /// Per player: rank label row (none for humans).
    ranks: [Option<usize>; 4],
    pill: [[u8; 3]; 4],
    rank_rgb: [[u8; 3]; 4],
    points: [i32; 2],
    games: [i32; 2],
    sets: [i32; 2],
    deuce: bool,
    advantage: bool,
    tiebreak: bool,
    server: i32,
    /// 0 deuce court, 1 ad court.
    court: i32,
    /// The first team's panel is at the bottom (its end is nearer the camera).
    first_near: bool,
    /// The "Score to win" line: the team it speaks to and its row (see `game_info`).
    info: Option<(usize, usize)>,
    clock: Clock,
}

/// The panel's own timeline, counted in match ticks.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Clock {
    /// Ticks since the serve phase began (1 on its first tick), 0 when not serving.
    serve: u32,
    /// Ticks since the rally began, while the panel fades out (0: not fading).
    fade: u32,
}

impl Clock {
    fn step(&mut self, phase: &Phase) {
        *self = match phase {
            Phase::Serve => Clock {
                serve: self.serve + 1,
                fade: 0,
            },
            // the fade runs to 6 (alpha 0), then the panel is gone
            Phase::Rally if self.serve > 0 && self.fade < 6 => Clock {
                serve: self.serve,
                fade: self.fade + 1,
            },
            _ => Clock::default(),
        };
    }

    fn shown(&self) -> bool {
        self.serve > 0
    }

    /// Slide-in count, 0..5 (5 = fully in).
    fn cnt(&self) -> i32 {
        (self.serve as i32 - 1).min(5)
    }

    /// The faded textures' alpha (also the strip's base level).
    fn alpha(&self) -> i32 {
        if self.fade == 0 {
            128
        } else {
            (6 - self.fade as i32) * 128 / 5
        }
    }

    /// The serve ring's alpha once the panel is in: up over 31 ticks, down over 31.
    fn ring(&self) -> i32 {
        let k = (self.serve as i32 - 7).rem_euclid(62);
        if k < 31 {
            128 - (30 - k) * 128 / 30
        } else {
            (61 - k) * 128 / 30
        }
    }
}

/// Points cells (column, row) by point index: 0, 15, 30, 40, Ad (→ Deuce/Advantage handled apart).
const POINT_U: [i32; 7] = [0, 1, 0, 1, 0, 1, 0];
const POINT_V: [i32; 7] = [0, 0, 1, 1, 2, 2, 3];
const TIEBREAK_U: [i32; 11] = [0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 0];
const TIEBREAK_V: [i32; 11] = [0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3];

fn layout(v: &View) -> Vec<Quad> {
    let mut out = Vec::new();
    if !v.clock.shown() || v.players < 2 {
        return out;
    }
    let n = v.players;
    let a = v.clock.alpha() as f32;
    let fading = v.clock.fade > 0;
    let mut q =
        |tex, src: [i32; 4], x: i32, y: i32, size: Option<[i32; 2]>, rgb: [f32; 3], alpha: f32| {
            let [w, h] = size.unwrap_or([src[2], src[3]]);
            out.push(Quad {
                tex,
                src: src.map(|s| s as f32),
                dst: [x, y, w, h].map(|s| s as f32),
                rgb,
                alpha,
            })
        };
    const WHITE: [f32; 3] = [128.0; 3];
    let rgb = |c: [u8; 3]| c.map(f32::from);

    let i8 = if n < 3 { 20 } else { 0 };
    let tb_index = |p: i32| (p.max(0) as usize).min(TIEBREAK_U.len() - 1);
    let pt_index = |p: i32| (p.max(0) as usize).min(POINT_U.len() - 1);
    // digit offset and second team's shift: only with deuce, advantage or a tiebreak
    let (i24, mut i26) = match (v.tiebreak, v.deuce, v.advantage) {
        (false, false, false) => (0, 0),
        (false, true, _) | (true, true, _) => (8, -24),
        (false, false, true) => (8, 0),
        (true, false, false) => (16, 32),
        (true, false, true) => (8, 32),
    };
    let off = (v.clock.cnt() as f32 * 44.8 - 224.0) as i32;
    let fully_in = v.clock.cnt() > 4;
    let ys = if v.first_near { [328, 40] } else { [40, 328] };
    let server_team = (v.server & 1) as usize;
    let mirrored = (v.first_near && v.court == 1) || (!v.first_near && v.court == 0);
    let adv_lead = |t: usize| v.advantage && v.points[t] > v.points[1 - t];

    let mut face = [[0; 2]; 4];
    let mut label = [[0; 2]; 4];
    let (mut i27, mut i28) = (0, 0);
    // the team drawn on the left: the second when mirrored
    let left = if mirrored { 1 } else { 0 };
    if adv_lead(1 - left) {
        i26 = -24;
    }
    let right_x = |base: i32| i26 + base - off - i24;
    let (lt, rt) = (left, 1 - left);
    if n < 3 {
        face[lt] = [off + 16, ys[lt] + i8];
        label[lt] = [off + 48, ys[lt] + i8];
        face[rt] = [right_x(424), ys[rt] + i8];
        label[rt] = [right_x(456), ys[rt] + i8];
    } else {
        // the team's first player sits low (y + 40), its partner high, 16 further out
        for (t, x0, x1) in [(lt, off + 32, off + 64), (rt, right_x(440), right_x(472))] {
            face[t] = [x0, ys[t] + 40];
            label[t] = [x1, ys[t] + 40];
            face[t + 2] = [x0 - 16, ys[t] + i8];
            label[t + 2] = [x1 - 16, ys[t] + i8];
        }
        if v.server < 2 {
            (i27, i28) = (16, 40);
        }
    }
    let mut digit_x = [0; 2];
    digit_x[lt] = off + 112 + i24;
    digit_x[rt] = i26 + 520 - off;
    let ring_shift = if server_team == lt { 0 } else { i26 };
    let mut ring_x = if mirrored {
        i27 + (ring_shift + 424 - server_team as i32 * 408) - 8
    } else {
        i27 + (ring_shift + 408) * server_team as i32 + 16 - 8
    };
    if ring_x > 319 {
        ring_x -= i24;
    }
    let ring_y = i28 + i8 + ys[server_team] - 8;
    let mut strip_x = [0; 2];
    strip_x[lt] = off + 16;
    strip_x[rt] = 440 - off;
    let strip_y = if v.first_near { [408, 14] } else { [14, 408] };

    // players: pill, face, slot label, rank
    for i in 0..n {
        let [fx, fy] = face[i];
        let [lx, ly] = label[i];
        q(Tex::Pill, [0, 0, 112, 48], fx, fy, None, rgb(v.pill[i]), a);
        q(Tex::Face(i), [0, 0, 64, 64], fx, fy, None, WHITE, a);
        q(
            Tex::Slot,
            [v.slots[i] as i32 * 40, 0, 40, 24],
            lx,
            ly,
            None,
            rgb(v.pill[i]),
            a,
        );
        if let Some(r) = v.ranks[i] {
            q(
                Tex::Rank,
                [0, r as i32 * 24, 64, 24],
                lx + 8,
                ly + 24,
                None,
                rgb(v.rank_rgb[i]),
                a,
            );
        }
    }
    // points; with advantage the trailing team is drawn at half alpha
    let half = if fading { (a as i32 / 2) as f32 } else { 64.0 };
    for t in 0..2 {
        let (x, y) = (digit_x[t], ys[t] + 8);
        let p = v.points[t];
        let (tex, src, alpha) = match (v.tiebreak, v.deuce, v.advantage) {
            (false, true, _) => (Tex::Points, [0, 128, 128, 64], a),
            (true, true, _) => (Tex::Tiebreak, [0, 128, 128, 64], a),
            (false, false, true) if p == 4 => (Tex::Points, [128, 128, 128, 64], a),
            (true, false, true) if p == 8 => (Tex::Tiebreak, [128, 128, 128, 64], a),
            (false, false, adv) => {
                let k = pt_index(p);
                (
                    Tex::Points,
                    [POINT_U[k] * 128, POINT_V[k] * 64, 128, 64],
                    if adv { half } else { a },
                )
            }
            (true, false, adv) => {
                let k = tb_index(p);
                (
                    Tex::Tiebreak,
                    [TIEBREAK_U[k] * 64, TIEBREAK_V[k] * 64, 64, 64],
                    if adv { half } else { a },
                )
            }
        };
        q(tex, src, x, y, None, WHITE, alpha);
    }
    // the Sets/Games strips, then (doubles) the team banners
    for t in 0..2 {
        let (x, y) = (strip_x[t], strip_y[t]);
        let (bg_y, banner_y) = if y < 224 { (16, 122) } else { (410, 298) };
        let right = x > 319;
        let (bg_x, banner_x) = if right {
            (400 - off, 528 - off)
        } else {
            (off, off)
        };
        let sa = (a * 0.35).trunc();
        if right {
            q(
                Tex::Strip,
                [0, 0, 40, 16],
                bg_x,
                bg_y,
                Some([40, 20]),
                WHITE,
                sa,
            );
            q(
                Tex::Strip,
                [40, 0, 16, 16],
                bg_x + 40,
                bg_y,
                Some([200, 20]),
                WHITE,
                sa,
            );
        } else {
            q(
                Tex::Strip,
                [40, 0, 16, 16],
                bg_x,
                bg_y,
                Some([200, 20]),
                WHITE,
                sa,
            );
            q(
                Tex::Strip,
                [56, 0, 40, 16],
                bg_x + 200,
                bg_y,
                Some([40, 20]),
                WHITE,
                sa,
            );
        }
        q(Tex::Sets, [0, 0, 72, 24], x, y, None, WHITE, a);
        q(
            Tex::Sets,
            [v.sets[t] * 16, 24, 16, 24],
            x + 72,
            y,
            None,
            WHITE,
            a,
        );
        q(Tex::Games, [0, 0, 72, 24], x + 96, y, None, WHITE, a);
        q(
            Tex::Games,
            [v.games[t] * 16, 24, 16, 24],
            x + 168,
            y,
            None,
            WHITE,
            a,
        );
        // the team the "Score to win" line speaks to loses its banner while the line is up
        if n > 2 && !(fully_in && v.info.is_some_and(|(a, _)| a == t)) {
            let tint = if t == 0 {
                [127.0, 77.0, 77.0]
            } else {
                [67.0, 101.0, 127.0]
            };
            // B653: our own pill (rim, white ring, team colour), its flat end run off the screen's edge, and the
            // menus' rebuilt "A Team" / "B Team" (`inpane_team`'s 96×32 cells at ¾ size) in the middle of what shows
            let pill = [if right { banner_x } else { banner_x - 16 }, banner_y, 128, 24].map(|c| c as f32);
            for (i, rgb) in [[15.0; 3], WHITE, tint].into_iter().enumerate() {
                for (src, [x, y, w, h]) in super::menu_art::pill_quads(i, pill) {
                    q(Tex::TeamPill, src.map(|c| c as i32), x as i32, y as i32, Some([w as i32, h as i32]), rgb, 128.0);
                }
            }
            q(
                Tex::Team,
                [0, t as i32 * 32, 96, 32],
                banner_x + 20,
                banner_y,
                Some([72, 24]),
                WHITE,
                128.0,
            );
        }
    }
    if fully_in && !fading {
        q(
            Tex::Ring,
            [0, 0, 56, 56],
            ring_x,
            ring_y,
            None,
            WHITE,
            v.clock.ring() as f32,
        );
    }
    // the "Score to win" line on its team's side, just inside its panel (y 300 bottom, 124 top); it blinks (15 ticks off, 15
    // on, from when the panel is in) and fades out solid with the panel
    if let Some((t, row)) = v.info.filter(|_| fully_in) {
        let x = if t == left { 0 } else { 384 };
        let y = if ys[t] > 200 { 300 } else { 124 };
        let blink = if (v.clock.serve as i32 - 7).rem_euclid(30) < 15 { 0.0 } else { 128.0 };
        q(Tex::Info, [0, row as i32 * 24, 256, 24], x, y, None, WHITE, if fading { a } else { blink });
    }
    out
}

/// The "Score to win" line for a game point (the first team's first): the team it speaks to and the
/// `i_gameinfo_00` row — 0 "…this game!" (the server's), 3 "…for a service break!", 1 "…the set!", 2 "…the
/// match!", or 4 "Score or you lose the match!" to the other team when only it has a human player.
fn game_info(s: &Score, r: &Rules, human: [bool; 2]) -> Option<(usize, usize)> {
    let t = (0..2).find(|&t| s.game_point(r, t))?;
    let lead = (s.games[t] > s.games[t ^ 1]) as i32;
    Some(if !s.tiebreak && s.games[t] < r.games - lead {
        (t, if (s.server & 1) as usize == t { 0 } else { 3 })
    } else if s.sets[t] < r.sets - 1 {
        (t, 1)
    } else if !human[t] && human[t ^ 1] {
        (t ^ 1, 4)
    } else {
        (t, 2)
    })
}

/// The panel's textures: `TEXTURES` in order, our pill sheet, then each player's face.
#[derive(Resource)]
pub struct Art(pub Vec<Handle<Image>>);
/// Where the faces start in `Art`.
pub const FACES: usize = TEXTURES.len() + 1;
/// The pool of screen rectangles, drawn in order.
#[derive(Component)]
pub(super) struct Slot(usize);
/// Per-player colours from the game program: pill/label tint and rank tint.
#[derive(Resource)]
pub struct Colours(pub [[u8; 3]; 4], pub [[u8; 3]; 4]);

const POOL: usize = 48;

pub fn plugin(app: &mut App) {
    app.init_resource::<Clock>()
        .add_systems(PostStartup, setup.after(super::setup))
        .add_systems(FixedUpdate, tick.after(super::simulate))
        .add_systems(Update, draw)
        .add_systems(Update, await_faces.run_if(resource_exists::<WaitingFaces>));
}

/// B649: players on a mod whose own face wasn't there when the match began (the select's drawing of it still under
/// way, or a launch that didn't wait for it): their face slot, and the store's picture of the mod once asked for.
#[derive(Resource, Default)]
struct WaitingFaces(Vec<WaitingFace>);

struct WaitingFace {
    player: usize,
    /// The donor (disc character) and costume the player is on.
    char: usize,
    outfit: usize,
    face: Handle<Image>,
    /// The mod's store picture: asked for (Some), then come (Some(Some(Some(png))) or not (Some(Some(None))).
    store: Option<std::sync::Arc<std::sync::Mutex<Option<Option<Vec<u8>>>>>>,
}

/// A transparent face, INPANE's 64×64.
fn clear_face() -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    Image::new_fill(Extent3d { width: 64, height: 64, depth_or_array_layers: 1 }, TextureDimension::D2, &[0; 4], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD)
}

/// B649: how often a waiting face looks in the select's cache again.
const FACE_RETRY: f32 = 0.5;

/// B649: fills a waiting mod face in place (same handle, so the panel and pop-ups pick it up) with the select's baked
/// close-up once it's there, else the face cut from the mod's store picture (a store mod's listing picture is the
/// select's own drawing of costume 0), asked for once.
fn await_faces(mut waiting: ResMut<WaitingFaces>, args: Res<Args>, mods: Option<Res<crate::mods::MatchMod>>, front: Option<Res<super::front_end::FrontEnd>>, mut images: ResMut<Assets<Image>>, time: Res<Time>, mut next: Local<f32>) {
    if waiting.0.is_empty() || time.elapsed_secs() < *next {
        return;
    }
    *next = time.elapsed_secs() + FACE_RETRY;
    waiting.0.retain_mut(|w| {
        let Some(m) = mods.as_ref().and_then(|m| m.get(w.player)) else { return false };
        let mut face = front.as_ref().and_then(|f| (f.face)(&args.iso, Some(m), w.char, w.outfit));
        if face.is_none() && w.outfit == 0 {
            match &w.store {
                None => {
                    let got = std::sync::Arc::new(std::sync::Mutex::new(None));
                    let to = got.clone();
                    if crate::modstore::installed_preview(&m.dir, &m.id, move |png| *to.lock().unwrap() = Some(png)) {
                        w.store = Some(got);
                    } else {
                        w.store = Some(std::sync::Arc::new(std::sync::Mutex::new(Some(None))));
                    }
                }
                Some(got) => {
                    if let Some(png) = got.lock().unwrap().as_ref() {
                        face = png.as_deref().and_then(|b| crate::inspect::face_icon(b, 40));
                    }
                }
            }
        }
        let Some(img) = face else { return true };
        if let Some(mut slot) = images.get_mut(&w.face) {
            *slot = img;
        }
        false
    });
}

/// An INPANE sheet (alpha authored 0–255: the board's 0x80 centre shows the court through it on the PS2).
pub fn image(images: &mut Assets<Image>, data: &[u8]) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let pic = tim2::decode_alpha8(data).expect("TIM2").remove(0);
    let img = Image::new(
        Extent3d {
            width: pic.width,
            height: pic.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pic.rgba.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    crate::textures::add_tim2(images, img, &pic)
}

fn setup(mut commands: Commands, args: Res<Args>, g: Res<Game>, mut images: ResMut<Assets<Image>>, mods: Option<Res<crate::mods::MatchMod>>, front: Option<Res<super::front_end::FrontEnd>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso
        .read("AZUMA/INPANE/INPANE.XB0")
        .expect("INPANE archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let pill = super::menu_art::pill_layers(&mut images);
    let mut get = |name: String| {
        let e = arc
            .entries
            .iter()
            .find(|e| {
                e.name
                    .to_ascii_lowercase()
                    .replace('\\', "/")
                    .ends_with(&name)
            })
            .unwrap_or_else(|| panic!("{name} in INPANE"));
        super::menu_art::image(&mut images, &arc.read(e).expect("INPANE bytes"), &name)
    };
    let mut art: Vec<_> = TEXTURES
        .iter()
        .map(|t| get(format!("/{}.tm2", t.to_ascii_lowercase())))
        .collect();
    art.push(pill);
    // B133: a mod's own face, cut from its select preview (B649: never the donor's, `await_faces`); B160: a disc one's too
    let own: Vec<_> = g.chars.iter().enumerate().map(|(i, &c)| {
        let o = args.outfits.get(i).copied().unwrap_or(0);
        front.as_ref().and_then(|f| (f.face)(&args.iso, mods.as_ref().and_then(|m| m.get(i)), c as usize, o))
    }).collect();
    // Z2: the disc face is only made where there's no own one (it was made and never drawn)
    let mut drawn = Vec::new();
    let mut waiting = WaitingFaces::default();
    for (i, (&c, own)) in g.chars.iter().zip(own).enumerate() {
        let o = args.outfits.get(i).copied().unwrap_or(0);
        match own {
            Some(img) => {
                drawn.push((art.len(), img));
                art.push(Handle::default());
            }
            // B649: a mod not baked yet never shows its donor's face: a clear one until its own comes
            None if mods.as_ref().and_then(|m| m.get(i)).is_some() => {
                waiting.0.push(WaitingFace { player: i, char: c as usize, outfit: o, face: Handle::default(), store: None });
                drawn.push((art.len(), clear_face()));
                art.push(Handle::default());
            }
            None => art.push(get(format!("face/face_{c:02}_{o:02}.tm2"))),
        }
    }
    for (k, img) in drawn {
        art[k] = images.add(img);
    }
    for w in &mut waiting.0 {
        w.face = art[FACES + w.player].clone();
    }
    commands.insert_resource(waiting);
    commands.insert_resource(Art(art));
    let (cnf, bin) = (
        iso.read("SYSTEM.CNF").expect("SYSTEM.CNF"),
        iso.read("ZZBIN/GAME.BIN").expect("GAME.BIN"),
    );
    let (pill, rank) = hst_data::exe::Game::new(&cnf, &bin)
        .expect("supported disc")
        .hud_colours();
    commands.insert_resource(Colours(pill, rank));
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        })
        .with_children(|p| {
            for i in 0..POOL {
                p.spawn((
                    Slot(i),
                    ImageNode { image_mode: NodeImageMode::Stretch, ..default() },
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    Visibility::Hidden,
                ));
            }
            super::name_tags::pool(p, super::name_tags::PANEL);
        });
}

fn tick(g: Res<Game>, mut clock: ResMut<Clock>) {
    clock.step(&g.phase);
}

fn draw(
    g: Res<Game>,
    pads: Res<Pads>,
    clock: Res<Clock>,
    art: Option<Res<Art>>,
    colours: Option<Res<Colours>>,
    cam: Query<&Transform, With<crate::Orbit>>,
    window: Query<&Window>,
    (names, mut plates): (Option<Res<super::name_tags::Names>>, ResMut<super::name_tags::Plates>),
    mut q: Query<(&Slot, &mut ImageNode, &mut Node, &mut Visibility)>,
) {
    let (Some(art), Some(colours)) = (art, colours) else {
        return;
    };
    let n = g.players.len();
    let s = &g.score;
    // the first team's end is game −z when its `end` is +1, world +z; it is near when the camera is on that side
    let first_near = cam
        .single()
        .map_or(true, |c| c.translation.z * g.players[0].end >= 0.0)
        // B633: split, the near team plays on the top half: its panel goes up there
        != super::split_view::on();
    let slot = |i: usize| pads.slot_of(i, n).unwrap_or(4);
    let view = View {
        players: n,
        slots: std::array::from_fn(|i| if i < n { slot(i) } else { 4 }),
        // COM players show their AI row's rank (AIParam.csv 段位); humans none (the game stores 17 = no label)
        ranks: std::array::from_fn(|i| (i < n && slot(i) == 4).then(|| g.players[i].ai.rank as usize)),
        pill: colours.0,
        rank_rgb: colours.1,
        points: s.points,
        games: s.games,
        sets: s.sets,
        deuce: s.deuce,
        advantage: s.advantage,
        tiebreak: s.tiebreak,
        server: s.server,
        court: s.side,
        first_near,
        info: game_info(s, &g.rules, [0, 1].map(|t| (t..n).step_by(2).any(|i| slot(i) < 4))),
        clock: *clock,
    };
    let mut quads = layout(&view);
    // named players' slot labels give way to their names; the points sit 48 px past a label
    let label = |q: &Quad| matches!(q.tex, Tex::Slot).then(|| (q.src[0] / 40.0) as usize);
    plates.0[super::name_tags::PANEL] = super::name_tags::take(&mut quads, names.as_deref(), &view.slots[..n], 8.0, label, |q| (q.dst, q.src, q.rgb, q.alpha));
    let aspect = window.single().map_or(4.0 / 3.0, crate::play::widescreen::aspect);
    let share = super::widescreen::share(aspect);
    let drop = window.single().map_or(0.0, |w| touch_drop(w, aspect));
    for (Slot(i), mut img, mut node, mut vis) in &mut q {
        let Some(quad) = quads.get(*i) else {
            vis.set_if_neq(Visibility::Hidden);
            continue;
        };
        let tex = match quad.tex {
            Tex::Face(p) => FACES + p,
            Tex::TeamPill => TEXTURES.len(),
            t => [
                Tex::Pill,
                Tex::Slot,
                Tex::Ring,
                Tex::Points,
                Tex::Tiebreak,
                Tex::Strip,
                Tex::Rank,
                Tex::Games,
                Tex::Sets,
                Tex::Team,
                Tex::Info,
            ]
            .iter()
            .position(|&k| k == t)
            .unwrap(),
        };
        let [u, v, w, h] = quad.src;
        // B160: a face is its whole image (INPANE's 64×64, or a baked one's larger square)
        let rect = (!matches!(quad.tex, Tex::Face(_))).then(|| Rect::new(u, v, u + w, v + h));
        let [r, g, b] = quad.rgb.map(|c| c / 128.0);
        crate::ui_pool::paint(&mut img, art.0[tex].clone(), rect, Color::srgba(r, g, b, quad.alpha / 128.0));
        let [x, y, w, h] = quad.dst;
        let (left, width) = super::widescreen::pin(x, w, share);
        let (top, height) = vplace_clear(y, h, aspect, drop);
        super::widescreen::put(&mut node, left, top, width, height);
        vis.set_if_neq(Visibility::Inherited);
    }
}

/// B610: how far (percent of the view) the panel's top half drops so its first row (the Sets/Games strip, y 14)
/// clears the touch pad's shoulder row, a little gap between; 0 without the pad or with room already.
pub(super) fn touch_drop(window: &Window, aspect: f32) -> f32 {
    drop_below(super::touch_pad::top_row(Vec2::new(window.width(), window.height())), aspect)
}

/// `touch_drop` for a shoulder row reaching `row` percent down the view.
pub(super) fn drop_below(row: f32, aspect: f32) -> f32 {
    if row <= 0.0 {
        return 0.0;
    }
    (row + 1.0 - 14.0 / 4.48 * super::widescreen::vshare(aspect)).max(0.0)
}

/// `widescreen::vplace` (`Sides`) for a panel quad, the top half `drop` lower (`touch_drop`).
pub(super) fn vplace_clear(y: f32, h: f32, aspect: f32, drop: f32) -> (f32, f32) {
    let (top, height) = super::widescreen::vplace(y, h, aspect, super::widescreen::Anchor::Sides);
    (if y + h / 2.0 < 224.0 { top + drop } else { top }, height)
}

impl super::snapshot::Wire for Clock {}

#[cfg(test)]
mod tests {
    use super::*;

    fn doubles(clock: Clock) -> View {
        View {
            players: 4,
            slots: [4; 4],
            ranks: [Some(0), Some(0), Some(0), Some(1)],
            pill: [[98, 46, 61], [33, 74, 97], [105, 70, 25], [63, 78, 25]],
            rank_rgb: [[0; 3]; 4],
            points: [0, 0],
            games: [0, 0],
            sets: [0, 0],
            deuce: false,
            advantage: false,
            tiebreak: false,
            server: 0,
            court: 0,
            first_near: true,
            info: None,
            clock,
        }
    }

    /// Slot 5 of the reference saves (doubles, all COM, first team near, deuce court, player 0 serving): the
    /// positions read off the original's draw and matched against its screenshot.
    #[test]
    fn doubles_in() {
        let quads = layout(&doubles(Clock { serve: 40, fade: 0 }));
        let at = |tex: Tex| {
            quads
                .iter()
                .filter(|q| q.tex == tex)
                .map(|q| [q.dst[0] as i32, q.dst[1] as i32])
                .collect::<Vec<_>>()
        };
        assert_eq!(at(Tex::Face(0)), [[32, 368]]);
        assert_eq!(at(Tex::Face(2)), [[16, 328]]);
        assert_eq!(at(Tex::Face(1)), [[440, 80]]);
        assert_eq!(at(Tex::Face(3)), [[424, 40]]);
        assert_eq!(at(Tex::Slot), [[64, 368], [472, 80], [48, 328], [456, 40]]);
        assert_eq!(at(Tex::Points), [[112, 336], [520, 48]]);
        assert_eq!(at(Tex::Ring), [[24, 360]]);
        assert_eq!(
            &at(Tex::Sets)[..],
            &[[16, 408], [88, 408], [440, 14], [512, 14]]
        );
    }

    /// The stretched background pieces (B5): each piece's sub-rect and on-screen size as the original's draw passes
    /// them (left strip at the bottom, right strip at the top), and the banners above/below them.
    #[test]
    fn doubles_pieces() {
        let quads = layout(&doubles(Clock { serve: 40, fade: 0 }));
        let pieces = |tex: Tex| {
            quads
                .iter()
                .filter(|q| q.tex == tex)
                .map(|q| (q.src.map(|s| s as i32), q.dst.map(|s| s as i32)))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            pieces(Tex::Strip),
            [
                ([40, 0, 16, 16], [0, 410, 200, 20]),
                ([56, 0, 40, 16], [200, 410, 40, 20]),
                ([0, 0, 40, 16], [400, 16, 40, 20]),
                ([40, 0, 16, 16], [440, 16, 200, 20]),
            ]
        );
        // B653: each banner is our pill, its flat end past the screen's edge (left team at the bottom, right at the
        // top), the team colour on top, and the rebuilt words at ¾ size over the middle of what shows
        let pill = |y: f32| quads.iter().filter(|q| q.tex == Tex::TeamPill && q.dst[1] == y).map(|q| (q.dst, q.rgb)).collect::<Vec<_>>();
        let left = pill(298.0);
        assert_eq!(left.len(), 9);
        assert_eq!((left[6].0, left[8].0), ([-16.0, 298.0, 12.0, 24.0], [100.0, 298.0, 12.0, 24.0]));
        assert_eq!(left[8].1, [127.0, 77.0, 77.0]);
        let right = pill(122.0);
        assert_eq!((right[6].0, right[8].0), ([528.0, 122.0, 12.0, 24.0], [644.0, 122.0, 12.0, 24.0]));
        assert_eq!(right[8].1, [67.0, 101.0, 127.0]);
        assert_eq!(
            pieces(Tex::Team),
            [
                ([0, 0, 96, 32], [20, 298, 72, 24]),
                ([0, 32, 96, 32], [548, 122, 72, 24]),
            ]
        );
    }

    #[test]
    fn timeline() {
        let mut c = Clock::default();
        let mut cnt = Vec::new();
        for _ in 0..7 {
            c.step(&Phase::Serve);
            cnt.push(c.cnt());
        }
        assert_eq!(cnt, [0, 1, 2, 3, 4, 5, 5]);
        assert_eq!(c.ring(), 0);
        let mut alpha = Vec::new();
        for _ in 0..7 {
            c.step(&Phase::Rally);
            alpha.push((c.shown(), c.alpha()));
        }
        assert_eq!(
            alpha,
            [
                (true, 128),
                (true, 102),
                (true, 76),
                (true, 51),
                (true, 25),
                (true, 0),
                (false, 128)
            ]
        );
    }

    #[test]
    fn score_to_win() {
        let r = Rules { sets: 2, games: 6, no_deuce: false, one_point_games: false, players: 4 };
        let mut s = Score { points: [3, 1], ..default() };
        assert_eq!(game_info(&s, &r, [false; 2]), Some((0, 0)));
        s.server = 1;
        assert_eq!(game_info(&s, &r, [false; 2]), Some((0, 3)));
        s.games = [5, 3];
        assert_eq!(game_info(&s, &r, [false; 2]), Some((0, 1)));
        s.sets = [1, 0];
        assert_eq!(game_info(&s, &r, [false; 2]), Some((0, 2)));
        assert_eq!(game_info(&s, &r, [false, true]), Some((1, 4)));
        s.deuce = true;
        assert_eq!(game_info(&s, &r, [false; 2]), None);

        // the first team near (bottom, left): its line on the left, just above its panel; blinking in once the panel is in
        let mut c = Clock::default();
        let mut v = doubles(c);
        v.info = Some((0, 3));
        let line = |v: &View| layout(v).into_iter().find(|q| q.tex == Tex::Info).map(|q| (q.src, q.dst, q.alpha));
        assert_eq!(line(&v), None);
        let mut alpha = Vec::new();
        for _ in 0..37 {
            c.step(&Phase::Serve);
            v.clock = c;
            alpha.push(line(&v).map(|l| l.2));
        }
        assert_eq!(alpha[..6], [None, None, None, None, None, Some(128.0)]);
        assert_eq!(alpha[6..21], [Some(0.0); 15]);
        assert_eq!(alpha[21..36], [Some(128.0); 15]);
        let l = line(&v).unwrap();
        assert_eq!((l.0, l.1), ([0.0, 72.0, 256.0, 24.0], [0.0, 300.0, 256.0, 24.0]));
        v.info = Some((1, 0));
        assert_eq!(line(&v).unwrap().1, [384.0, 124.0, 256.0, 24.0]);
        // the line's team has no banner (the other's is still there)
        let banners = |v: &View| layout(v).iter().filter(|q| q.tex == Tex::Team && q.rgb == [128.0; 3]).map(|q| q.src[1]).collect::<Vec<_>>();
        assert_eq!(banners(&v), [0.0]);
    }

    /// B649: a mod player's face waits clear (never the donor's) and fills in place, on the same handle the panel and
    /// pop-ups draw, once the select's drawing of it is there; a mod with no store picture doesn't ask the store.
    #[test]
    fn a_mods_face_fills_in_when_its_drawing_lands() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static BAKED: AtomicBool = AtomicBool::new(false);
        fn face(_: &str, m: Option<&crate::mods::Mod>, c: usize, k: usize) -> Option<Image> {
            assert_eq!((m.map(|m| m.id.as_str()), c, k), (Some("x"), 3, 0));
            BAKED.load(Ordering::Relaxed).then(|| {
                let mut img = clear_face();
                img.data.as_mut().unwrap().fill(200);
                img
            })
        }
        let dir = std::env::temp_dir().join(format!("hst-b649-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("model")).unwrap();
        std::fs::write(dir.join("model/c00.glb"), b"").unwrap();
        std::fs::write(dir.join("mod.json"), r#"{"standard":1,"id":"x","name":"X","costumes":["model/c00.glb"],"donor":3,"hand":"right","params":{"base":3,"override":{}},"ai_row":3,"face":"morph"}"#).unwrap();
        let m = crate::mods::read(&dir).unwrap();
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default())).init_asset::<Image>();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f32(FACE_RETRY)));
        app.insert_resource(Args { iso: String::new(), archives: vec![], shot: None, shot_at: 0.5, radius: None, ball: false, court: 0, stage: None, play: true, singles: true, chars: vec![3, 1], outfits: vec![], viewer: None, sound: None, music: false, umpire: 4, sets: 1, games: 1, pads: None });
        let mut mods = crate::mods::MatchMod::default();
        mods.set(0, m);
        app.insert_resource(mods);
        app.insert_resource(super::super::front_end::FrontEnd { paused: |_| false, set_paused: |_, _| {}, face });
        let h = app.world_mut().resource_mut::<Assets<Image>>().add(clear_face());
        app.insert_resource(WaitingFaces(vec![WaitingFace { player: 0, char: 3, outfit: 0, face: h.clone(), store: None }]));
        app.add_systems(Update, await_faces.run_if(resource_exists::<WaitingFaces>));
        let px = |app: &App| app.world().resource::<Assets<Image>>().get(&h).unwrap().data.as_ref().unwrap()[0];
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(px(&app), 0, "a face before its drawing");
        assert_eq!(app.world().resource::<WaitingFaces>().0.len(), 1);
        BAKED.store(true, Ordering::Relaxed);
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(px(&app), 200, "the drawing never came in");
        assert!(app.world().resource::<WaitingFaces>().0.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
