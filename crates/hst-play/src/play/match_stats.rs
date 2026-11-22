//! The match statistics (`hst_sim::stats`), counted over the match, and the original's stats screen at its end
//! (textures `result_status00`/`01` from `AZUMA/PRIZE/OTHER.XB0`, coordinates on the 640×448 screen): a column
//! per player under a header pill (face, slot label) and a column of row names, the 9 rows' values (the best of
//! each row on the player's pill colour, but net play and forehand rates), the longest rally and the match time
//! on the bar below. The winner's header pulses white.
//!
//! Behind the columns a dark sprite (`i_pause_result_15`), the row names plain; each player's rank (`inpane_dani`,
//! in the rank colour) under the slot label.
//!
//! F1 (ours, user concept 2026-11-21 "End screen"): the original's result screen had two pages (the final board under
//! the "Game, Set, Match!" model, then this one). Ours is one screen over a flat dim: "GAME, SET, MATCH!" in gold, the
//! stats board widget ([`board`]: the final set scores in the board's own frame over this board, "Stats" in its
//! yellow) at full size in the middle, and the what-next pills in a row under it ([`next`], M196); ←→ choose, from
//! tick 120 ✕ picks. Its button prompts are our key legend (M14), not the original's PS2-glyph bar.
//!
//! The match pauses on it; a pill fades to black over 60 ticks and goes back to the menu, as the pause menu's
//! "Return to the Menu" did, which then does what the pill said ([`next::take`]). `HST_STATS=<seconds>` ends the match
//! (as it stands) after that long, for `--shot`; `HST_STATS_PAGE=1` skips the ceremony.
//!
//! Before it, the original's first state: the winners' ceremony ([`ceremony`]), the banner already up.

mod board;
mod ceremony;
pub(super) use ceremony::END_JINGLE;
mod dissolve;
mod hold;
pub mod next;
mod plate;

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use hst_sim::score::{Event, Score};
use hst_sim::serve::{self, Balloon};
use hst_sim::stats::Table;
use std::time::Duration;

use super::panel::{self, Colours};
use super::popups;
use super::{Game, Pads, Phase};
use crate::Args;

/// From `AZUMA/PRIZE/OTHER.XB0`, then `AZUMA/INPANE/INPANE.XB0`.
const TEXTURES: [&str; 5] =
    ["/result_status00.tm2", "/result_status01.tm2", "/keyassign_inpane.tm2", "/result_gameset05.tm2", "/eff/confetti.tm2"];
const INPANE: [&str; 3] = ["/i_pause_result_15.tm2", "/2d/info.tm2", "/inpane_team.tm2"];
/// The confetti piece's index in [`Art`].
const CONFETTI: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tex {
    Frame,
    Text,
    /// The dark sprite behind the columns.
    Back,
    Face(usize),
    Rank,
    /// The ceremony's "NARROW WIN" etc. rows and the team name plate.
    Gameset,
    Team,
    /// Our own pill sheet ([`super::menu_art::pill_layers`]), behind the team's name.
    Pill,
    /// M196: what-next's words, [`next::LABELS`] then [`next::NOTES`]; B764: then the "Game, Set, Match!" banner's
    /// art (the model's own texture).
    Own(u8),
    /// B764: the result board's sheets (`popups::Art`, `result_gameset00`–`03`): frame, stripes and plate ring; the
    /// settled games' green digits; the orange digits; the sets-won digits and "Set".
    Board(u8),
    /// The board's slot labels (`inpane_p`), which the plates' names take the place of.
    Slot,
    /// B764: player `p`'s plate name and character pill ([`plate::Words`]).
    Name(usize),
    Chara(usize),
    /// B764: the pause menu's pointing hand ([`next::Pointer`]).
    Hand,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Quad {
    tex: Tex,
    src: [f32; 4],
    dst: [f32; 4],
    rgb: [f32; 3],
    alpha: f32,
}

const WHITE: [f32; 3] = [128.0; 3];
/// B764: the header is this much taller than the original's 40 (the pause board's plates: ring, name, pill under the
/// face), the rows and the bar this much lower.
const HEAD: i32 = 16;

/// The struck shot's counts: shot number, contact branch, the forehand side, the sweet balloon, the net distance.
pub(super) fn hit(g: &mut Game, who: usize, branch: u8, grade: u8, offset: i32) {
    let sweet = serve::balloon(grade, offset, branch == hst_sim::shot::branch::DIVE) == Some(Balloon::Sweet);
    let (forehand, z) = (!g.players[who].backhand, g.players[who].pos[2]);
    g.match_stats.hit(who, g.shots, g.score.server as usize, branch, forehand, sweet, z);
}

pub(super) fn point(g: &mut Game, call: u8, winner: Option<usize>) {
    let server = g.score.server as usize;
    g.match_stats.point(call, winner, g.shots, server, g.rally.faults);
}

/// A game (or tiebreak) won by `team`: its final points.
pub(super) fn scored(g: &mut Game, before: &Score, event: Option<Event>, team: usize) {
    if matches!(event, Some(Event::Game | Event::Set)) {
        let mut points = before.points;
        points[team] += 1;
        g.match_stats.game(points);
    }
}

pub(super) fn match_over(g: &mut Game) {
    let mut table = g.match_stats.finish(g.post_winner.clamp(0, 1) as usize);
    table.score = g.score.clone();
    g.match_stats = hst_sim::stats::Stats::new();
    g.match_stats.result = Some(table);
    *ceremony::ENDING.lock().unwrap() = Some((g.score.swapped, g.rng.sound.clone()));
}

/// What the stats screen shows.
struct View<'a> {
    table: &'a Table,
    players: usize,
    slots: [usize; 4],
    pill: [[u8; 3]; 4],
    ranks: [Option<usize>; 4],
    rank_rgb: [[u8; 3]; 4],
    /// The winner's header alpha (pulsing).
    pulse: i32,
    /// B764: the rules' sets to win (the score board's length) and each player's plate words' widths (name, pill).
    sets_to_win: i32,
    words: [Option<[f32; 2]>; 4],
}

fn layout(v: &View) -> Vec<Quad> {
    let mut out = Vec::new();
    let mut q = |tex, src: [i32; 4], dst: [i32; 4], rgb: [f32; 3], alpha: i32| {
        out.push(Quad { tex, src: src.map(|c| c as f32), dst: dst.map(|c| c as f32), rgb, alpha: alpha as f32 })
    };
    let (n, t) = (v.players, v.table);
    let singles = n < 3;
    let rgb = |c: [u8; 3]| c.map(f32::from);

    let (x0, w) = if singles { (136, 368) } else { (40, 560) };
    let hh = 40 + HEAD;
    q(Tex::Back, [0, 0, 8, 8], [x0, 80, w, 256 + HEAD], WHITE, board::BACK_ALPHA as i32);
    // the bar below: longest rally, match time (B764: the digits the result board's, as every value)
    let (bar, by) = (352 + HEAD, 359 + HEAD);
    q(Tex::Frame, [0, 472, 16, 40], [40, bar, 16, 40], WHITE, 128);
    q(Tex::Frame, [16, 472, 16, 40], [56, bar, 528, 40], WHITE, 128);
    q(Tex::Frame, [32, 472, 16, 40], [584, bar, 16, 40], WHITE, 128);
    let rally = t.longest_rally.min(999);
    q(Tex::Text, [0, 320, 104, 24], [88, by, 104, 24], WHITE, 128);
    let mut cells = Vec::new();
    for (k, d) in [rally / 100, rally / 10 % 10, rally % 10].into_iter().enumerate() {
        if (k == 0 && rally < 100) || (k == 1 && rally < 10) {
            continue;
        }
        cells.push(cell(d, 216 + 16 * k as i32, by));
    }
    q(Tex::Text, [0, 344, 80, 24], [352, by, 80, 24], WHITE, 128);
    // M176: in the match's own ticks a second
    for (k, part) in hst_sim::stats::hms_at(t.frames, crate::match_speed::current()).into_iter().enumerate() {
        let x = 440 + 40 * k as i32;
        cells.push(cell(part / 10 % 10, x, by));
        cells.push(cell(part % 10, x + 16, by));
        if k < 2 {
            q(Tex::Text, [80, 344, 8, 24], [x + 32, by, 8, 24], WHITE, 128);
        }
    }
    // each player's header pill, in the player's stripe
    for p in 0..n {
        let vv = 0x110 + 0x28 * p as i32;
        let x = [x0, 408][p & 1] + if p >= 2 { 96 } else { 0 };
        if p & 1 == 0 {
            q(Tex::Frame, [0, vv, 8, 40], [x, 80, 24, hh], WHITE, 128);
            q(Tex::Frame, [8, vv, 16, 40], [x + 24, 80, 16, hh], WHITE, 128);
            q(Tex::Frame, [24, vv, 7, 40], [x + 40, 80, 56, hh], WHITE, 128);
        } else {
            q(Tex::Frame, [0, vv, 8, 40], [x, 80, 56, hh], WHITE, 128);
            q(Tex::Frame, [8, vv, 16, 40], [x + 56, 80, 16, hh], WHITE, 128);
            q(Tex::Frame, [24, vv, 7, 40], [x + 72, 80, 24, hh], WHITE, 128);
        }
    }
    let wx = match (t.winner, singles) {
        (0, true) => 0x88,
        (0, false) => 0x28,
        _ => 0x198,
    };
    q(Tex::Frame, [44, 308, 4, 4], [wx, 80, if singles { 0x60 } else { 0xc0 }, hh], WHITE, v.pulse);
    // each row's best (fewest missed shots and double faults, the most of the rest) on its players' colours
    let hx: &[i32] = if singles { &[0x88, 0x198] } else { &[0x28, 0x198, 0x88, 0x1f8] };
    for r in (0..9).filter(|r| !(6..=7).contains(r)) {
        let who = if r == 8 { n.min(2) } else { n };
        let vals: Vec<f32> = (0..who).map(|p| t.rows[p][r]).collect();
        let best = if (2..=3).contains(&r) {
            vals.iter().copied().fold(f32::MAX, f32::min)
        } else {
            vals.iter().copied().fold(f32::MIN, f32::max)
        };
        let ties = (0..n).filter(|&p| t.rows[p][r] == best).count();
        if ties == n {
            continue;
        }
        for p in (0..who).filter(|&p| vals[p] == best) {
            let w = if r == 8 && !singles { 192 } else { 96 };
            q(Tex::Frame, [49, 273, 6, 8], [hx[p], 120 + HEAD + 24 * r as i32, w, 24], rgb(v.pill[p]), 128);
        }
    }
    // the column frames and their dividers; B764: each frame's header part (its first 48 px) taller by `HEAD`, the
    // rows' part as it is, so its row lines stay on the rows
    let fx = if singles { 128 } else { 32 };
    let mut frame = |u: i32, uw: i32, x: i32, w: i32| {
        q(Tex::Frame, [u, 0, uw, 48], [x, 72, w, 48 + HEAD], WHITE, 128);
        q(Tex::Frame, [u, 48, uw, 224], [x, 120 + HEAD, w, 224], WHITE, 128);
    };
    frame(0, 16, fx, 16);
    frame(16, 8, fx + 16, 88);
    if !singles {
        frame(16, 8, fx + 104, 96);
    }
    let i = fx + if singles { 104 } else { 200 };
    frame(29, 6, i, 176);
    frame(16, 8, i + 176, 88);
    if !singles {
        frame(16, 8, i + 264, 96);
    }
    frame(40, 16, i + if singles { 264 } else { 360 }, 16);
    for (k, x) in [136, 232, 406, 504].into_iter().enumerate() {
        if singles && (k == 0 || k == 3) {
            continue;
        }
        let h = if k == 0 || k == 3 { 232 } else { 256 };
        q(Tex::Frame, [40, 273, 2, 14], [x, 80, 2, h + HEAD], WHITE, 128);
    }
    // "Stats" (white, as the original's sheet has it) and the row names
    q(Tex::Text, [0, 64, 160, 40], [240, 80 + HEAD / 2, 160, 40], WHITE, 128);
    for k in 0..9 {
        q(Tex::Text, [0, 104 + 24 * k, 160, 24], [240, 120 + HEAD + 24 * k, 160, 24], WHITE, 128);
    }
    // the values
    let (a, b, d, e): (&[i32], &[i32], &[i32], &[i32]) = if singles {
        (&[0xd2, 0x1e2], &[0x8a, 0x19a], &[0x92, 0x1a2], &[0x92, 0x1a2])
    } else {
        (&[0x70, 0x1e0, 0xd2, 0x242], &[0x28, 0x198, 0x8a, 0x1fa], &[0x30, 0x1a0, 0x92, 0x202], &[0x58, 0x1c8])
    };
    // `digits` digits right-aligned at x, leading zeros hidden
    let number = |out: &mut Vec<Quad>, value: u32, digits: u32, x: i32, y: i32| {
        for k in 0..digits {
            let place = 10u32.pow(digits - 1 - k);
            if k + 1 < digits && value < place {
                continue;
            }
            out.push(cell(value / place % 10, x + 16 * k as i32, y));
        }
    };
    let mut values = Vec::new();
    for r in 0..9 {
        let y = 120 + HEAD + 24 * r as i32;
        for p in 0..n {
            let v = t.rows[p][r];
            match r {
                0 => {
                    values.push(text([0, 24, 48, 24], b[p] + 48, y));
                    number(&mut values, v as u32, 3, b[p], y);
                }
                1..=4 => number(&mut values, v as u32, 4, d[p], y),
                5..=7 => {
                    values.push(text([72, 24, 24, 24], a[p], y));
                    percent(&mut values, v, b[p], y);
                }
                _ if p < 2 => {
                    let at = if singles { a[p] } else { [152, 520][p] };
                    values.push(text([96, 24, 24, 24], at, y));
                    number(&mut values, v as u32, 3, e[p], y);
                }
                _ => {}
            }
        }
    }
    out.extend(plates(v, x0));
    out.extend(values);
    out.extend(cells);
    out
}

/// B764: digit `d` in the stats sheet's 16×24 cell at `x, y`, drawn with the result board's orange outlined digits
/// (`result_gameset02`, the pause board's), 20 px square round the cell's middle.
fn cell(d: u32, x: i32, y: i32) -> Quad {
    let (x, y) = (x as f32, y as f32);
    Quad { tex: Tex::Board(2), src: [(d % 10) as f32 * 32.0, 0.0, 32.0, 32.0], dst: [x - 2.0, y + 2.0, 20.0, 20.0], rgb: WHITE, alpha: 128.0 }
}

/// B764: the header's plates, as the pause board's: each player's face in the board's white ring at the column's
/// outer end, the plate's name ([`plate::place`]) beside it on the stripes, the character's pill under the face; a
/// CPU's rank (in the rank colour) under its "COM". `x0`: the first column's left.
fn plates(v: &View, x0: i32) -> Vec<Quad> {
    let mut out = Vec::new();
    let q = |tex, src: [f32; 4], dst: [f32; 4], rgb: [f32; 3]| Quad { tex, src, dst, rgb, alpha: 128.0 };
    for p in 0..v.players.min(4) {
        let left = p & 1 == 0;
        let cx = ([x0, 408][p & 1] + if p >= 2 { 96 } else { 0 }) as f32;
        let ring = [if left { cx + 2.0 } else { cx + 46.0 }, 82.0];
        out.push(q(Tex::Board(0), [80.0, 0.0, 48.0, 48.0], [ring[0], ring[1], 48.0, 48.0], WHITE));
        out.push(q(Tex::Face(p), [0.0, 0.0, 64.0, 64.0], [ring[0] + 2.0, ring[1] + 2.0, 64.0, 64.0], WHITE));
        let room = if left { [cx + 52.0, cx + 94.0] } else { [cx + 2.0, cx + 44.0] };
        let rank = v.ranks[p];
        let name_y = if rank.is_some() { 94.0 } else { 100.0 };
        if let Some([name, pill]) = v.words[p].map(|w| plate::place(left, ring, name_y, room, [cx, cx + 96.0], w)) {
            out.push(q(Tex::Name(p), [0.0; 4], name, WHITE));
            out.push(q(Tex::Chara(p), [0.0; 4], pill, WHITE));
        }
        if let Some(r) = rank {
            let w = room[1] - room[0];
            out.push(q(Tex::Rank, [0.0, r as f32 * 24.0, 64.0, 24.0], [room[0], 106.0, w, w * 24.0 / 64.0], v.rank_rgb[p].map(f32::from)));
        }
    }
    out
}

fn text(src: [i32; 4], x: i32, y: i32) -> Quad {
    Quad {
        tex: Tex::Text,
        src: src.map(|c| c as f32),
        dst: [x as f32, y as f32, src[2] as f32, src[3] as f32],
        rgb: WHITE,
        alpha: 128.0,
    }
}

/// A rate with one decimal: hundreds only from 100, tens only from 10, the point after the units.
fn percent(out: &mut Vec<Quad>, v: f32, x: i32, y: i32) {
    let tenths = (v * 10.0) as u32;
    let whole = tenths / 10;
    if whole >= 100 {
        out.push(cell(whole / 100 % 10, x, y));
    }
    if whole >= 10 {
        out.push(cell(whole / 10 % 10, x + 16, y));
    }
    out.push(cell(whole % 10, x + 32, y));
    out.push(text([160, 0, 8, 24], x + 48, y));
    out.push(cell(tenths % 10, x + 56, y));
}


/// The winner's header pulse: 0 up to 102 and back over 21 ticks each way.
#[derive(Default)]
struct Pulse {
    down: bool,
    c: i32,
}

impl Pulse {
    fn advance(&mut self) {
        self.c -= 1;
        if self.c < 0 {
            (self.c, self.down) = (0x14, !self.down);
        }
    }

    fn step(&mut self) -> i32 {
        let a = if self.down { self.c * 0x66 / 0x14 } else { 0x66 - self.c * 0x66 / 0x14 };
        self.advance();
        a
    }
}

/// The sheets, and the system sounds' bank (the slide and Continue sounds).
#[derive(Resource)]
struct Art(Vec<Handle<Image>>, Option<std::sync::Arc<crate::audio::SoundBank>>);
#[derive(Component)]
struct Slot(usize);
#[derive(Component)]
pub struct Root;
/// The whole window's fade to black after Continue (a `Root` too, so the pause screen leaves it up with the HUD hidden).
#[derive(Component)]
struct Fade;
/// B630: the fade to black covers the whole window, the safe area's strips and a fixed Aspect Ratio's bars too, over
/// everything but the loading screen. B657: its copy only past the view (`BleedOutside`, so the strips darken as fast as
/// the rest), and over the key legend's strip (z 30) too.
const FADE_BLEED: hst_base::widescreen::Bleed = hst_base::widescreen::Bleed { order: (1 << 20) + 0x1000, on: true };
const POOL: usize = 400;
/// F1: the end screen's flat dim over the match (a `Root` too, hidden with the HUD by the pause screen), the whole
/// window as the fade to black, under the screen.
#[derive(Component)]
struct Dim;
const DIM_BLEED: hst_base::widescreen::Bleed = hst_base::widescreen::Bleed { order: (1 << 20) + 0x0f00, on: true };
/// Its colour (the concept's: a slate blue, 85 %).
const DIM: Color = Color::srgba(0.239, 0.282, 0.337, 0.85);

/// The "Game, Set, Match!" model.
#[derive(Resource)]
struct Banner(hst_sim::effect::Effect, crate::effects::Shown<crate::effects::OnTop>);
/// B756: the banner's space, a child of the match camera scaled as the umpire's calls' (`popups::CallSpace`), so it
/// lands where the overlay camera drew it with no second 3D camera running through the ceremony and the result page.
#[derive(Component)]
struct BannerSpace;
/// The banner's blended draws sort after every scene draw, as the umpire's calls'.
const BANNER_SORT: f32 = 1e5;
/// Its scale and the screen point it stands on (page 0 at x 0).
const BANNER_SCALE: f32 = 1.7;
const BANNER_AT: [f32; 2] = [321.0, 53.0];

#[derive(Resource, Default)]
struct Screen {
    pulse: Pulse,
    /// The winner's header alpha this tick.
    alpha: i32,
    acc: f32,
    open: bool,
    timer: i32,
    /// Ticks into the fade to black after a pill.
    leaving: Option<i32>,
    /// M196: the pill chosen, the left stick's last x and y.
    sel: usize,
    stick_x: f32,
    stick_y: f32,
}

impl Screen {
    fn opened() -> Self {
        Screen { open: true, pulse: Pulse { down: false, c: 0x14 }, ..default() }
    }

    /// A tick.
    fn step(&mut self) {
        self.alpha = self.pulse.step();
        self.timer += 1;
    }

    /// The pills take ✕ (the original's two seconds before its screen took one).
    fn ready(&self) -> bool {
        self.timer >= 120
    }

    /// `by` pills on from the one chosen (clamped); whether it moved.
    fn choose(&mut self, by: i32, n: usize) -> bool {
        let k = (self.sel as i32 + by).clamp(0, n as i32 - 1) as usize;
        std::mem::replace(&mut self.sel, k) != k
    }
}

pub fn plugin(app: &mut App) {
    dissolve::plugin(app);
    hold::plugin(app);
    app.init_resource::<Screen>()
        .init_resource::<Serving>()
        .init_resource::<plate::Words>()
        .init_resource::<next::Hits>()
        .add_systems(Update, plate::refresh.before(draw))
        .add_systems(PostStartup, setup.after(super::setup))
        .add_systems(FixedUpdate, tick.after(super::simulate))
        .add_systems(Update, (ceremony::begin, step, pool, draw, draw_banner).chain())
        .add_systems(Update, banner_space.after(ceremony::show))
        .add_systems(Update, ceremony::warm.before(ceremony::begin))
        .add_systems(Update, legend.after(step))
        .add_systems(Update, ceremony::pose.after(crate::character::animate).after(crate::character::uv_offsets).before(crate::character::texture_faces))
        .add_systems(PostUpdate, ceremony::hide.before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate))
        // in Update after the match camera: written in PostUpdate the fov never reached the camera's clip matrix (B246)
        .add_systems(Update, ceremony::show.after(crate::character::animate));
}

/// M14: the end screen's legend: ↑↓ choose a pill (B764: stacked as the pause menu's), OK from tick 120.
fn legend(screen: Res<Screen>, dissolve: Res<dissolve::Dissolve>, last: Res<super::legend::Last>, mut out: ResMut<super::legend::Legend>) {
    use super::legend::{Act, item};
    if !screen.open || screen.leaving.is_some() || dissolve.fading() {
        return;
    }
    let mut v = vec![item(Act::UpDown, "Choose", &["W", "S"])];
    if screen.ready() {
        v.push(item(Act::Ok, "OK", &["Enter", "J"]));
    }
    out.one(*last, v);
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    args: Res<Args>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<crate::effects::OnTop>>,
    mut bindposes: ResMut<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>,
) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let mut art = Vec::new();
    let mut banner_art = None;
    for (xb, names) in [("AZUMA/PRIZE/OTHER.XB0", &TEXTURES[..]), ("AZUMA/INPANE/INPANE.XB0", &INPANE[..])] {
        let data = iso.read(xb).expect("archive on disc");
        let arc = Archive::parse(&data).expect("xb archive");
        for name in names {
            // the Japanese disc keeps the info sheet in another folder
            let e = super::release_art::entry(&arc, name).unwrap_or_else(|| panic!("{name} in {xb}"));
            art.push(super::menu_art::image(&mut images, &arc.read(e).expect("archive bytes"), name));
        }
        if xb.contains("PRIZE") {
            // game space (y down) → Bevy, as the call models, in front of the match camera (`banner_space`)
            let space = commands.spawn((BannerSpace, Transform::default(), Visibility::default())).id();
            let root = commands.spawn((Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::PI)), Visibility::default(), ChildOf(space))).id();
            let (mut effect, shown) = crate::effects::model(&arc, "azuma/inpane/mdl/i_gameset_00", &mut commands, root, &mut meshes, &mut materials, &mut images, &mut bindposes)
                .expect("i_gameset_00");
            effect.hold = true;
            for h in &shown.materials {
                if let Some(mut m) = materials.get_mut(h) {
                    m.base.depth_bias = BANNER_SORT;
                }
            }
            commands.entity(root).insert_recursive::<Children>((bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver));
            commands.insert_resource(Banner(effect, shown));
            // B764: the model's own art, flat, for the end screen's banner
            let get = |n: &str| arc.entries.iter().find(|e| e.name.replace('\\', "/").to_ascii_lowercase().ends_with(n)).and_then(|e| arc.read(e).ok());
            let mtl = get("i_gameset_00.mtl").and_then(|m| hst_data::mtl::parse(&m, get("i_gameset_00.mti").as_deref()).ok()).expect("i_gameset_00 art");
            banner_art = Some(images.add(crate::character::texture_image(&mtl.textures[0])));
        }
    }
    art.push(super::menu_art::pill_layers(&mut images));
    // M196: the what-next pills' words and notes (`Tex::Own`)
    for s in next::LABELS {
        art.push(super::word_label::label(&mut iso, &mut images, s));
    }
    for s in next::NOTES {
        art.push(super::word_label::label_w(&mut iso, &mut images, s, next::NOTE_W));
    }
    art.extend(banner_art);
    let bank = crate::audio::SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd");
    commands.insert_resource(Art(art, bank.map(std::sync::Arc::new)));
    commands
        // left/top in percent too: a `Bleed` copy needs all four
        .spawn((Node { position_type: PositionType::Absolute, left: Val::Percent(0.0), top: Val::Percent(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(Color::NONE), GlobalZIndex(31), Fade, Root, FADE_BLEED, hst_base::widescreen::BleedOutside));
    // F1: the flat dim under the end screen, the whole window
    commands.spawn((
        Node { position_type: PositionType::Absolute, left: Val::Percent(0.0), top: Val::Percent(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
        BackgroundColor(Color::NONE),
        GlobalZIndex(10),
        Dim,
        Root,
        DIM_BLEED,
        hst_base::widescreen::BleedOutside,
    ));
    commands
        .spawn((super::widescreen::screen_43(), GlobalZIndex(11), Root))
        .with_children(|p| {
            // the quads go in ahead of these while the screen is up (`pool`)
            super::name_tags::pool(p, super::name_tags::RESULT);
        });
}

/// P39: the screen's quads once the match has a result (the ceremony, then the pages), none in play: UI layout walks
/// hidden nodes too.
fn pool(mut commands: Commands, g: Res<Game>, root: Query<Entity, (With<Root>, Without<Fade>, Without<Dim>)>, slots: Query<Entity, With<Slot>>) {
    for r in &root {
        crate::ui_pool::keep(&mut commands, r, g.match_stats.result.is_some(), slots, POOL, |i| (Slot(i), crate::ui_pool::quad()));
    }
}

/// The last tick was in the serve.
#[derive(Resource, Default, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Serving(bool);

/// A frame of the match: its time, a new serve, the serve's speed the frame after it is struck.
fn tick(mut g: ResMut<Game>, mut serving: ResMut<Serving>) {
    let (g, serving) = (&mut *g, &mut serving.0);
    g.match_stats.tick();
    let serve = g.phase == Phase::Serve;
    if serve && !*serving {
        g.match_stats.serve();
    }
    *serving = serve;
    if g.shots == 1 && g.phase == Phase::Rally {
        g.match_stats.serve_speed(g.build.kmh(g.flight.ball.vel));
    }
}

/// Opens the screen when a match has ended (holding the match's fixed clock) and steps it at 60 Hz: → / ← (✕ to the
/// unseen stats page) slide, ✕ from tick 120 with the stats page seen closes it.
#[allow(clippy::too_many_arguments)]
fn step(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    mut fixed: ResMut<Time<Fixed>>,
    mut g: ResMut<Game>,
    mut screen: ResMut<Screen>,
    mut pads: ResMut<Pads>,
    mut banner: Option<ResMut<Banner>>,
    sounds: Option<Res<Art>>,
    sound: Option<Res<crate::audio::Sound>>,
    (gallery, mut exit, mouse, touches, window, np, hits): (
        Option<Res<crate::audio::GalleryBank>>,
        MessageWriter<AppExit>,
        Res<ButtonInput<MouseButton>>,
        Res<Touches>,
        Query<&Window, With<bevy::window::PrimaryWindow>>,
        Option<Res<super::netplay::Netplay>>,
        Res<next::Hits>,
    ),
    mut cer: Option<ResMut<ceremony::Ceremony>>,
    mut dissolve: ResMut<dissolve::Dissolve>,
    mut commands: Commands,
    mut auto: Local<bool>,
) {
    if !*auto
        && std::env::var("HST_STATS").ok().and_then(|s| s.parse::<f32>().ok()).is_some_and(|t| time.elapsed_secs() >= t)
    {
        *auto = true;
        let g = &mut *g;
        let winner = (g.score.games[1] > g.score.games[0]) as i32;
        g.post_winner = winner;
        match_over(g);
        // the ceremony begins next frame
        return;
    }
    if g.match_stats.result.is_none() {
        return;
    }
    if let Some(c) = cer.as_mut().filter(|c| c.running()) {
        if c.t == 0 {
            if let Some(b) = banner.as_mut() {
                b.0.start();
            }
            fixed.set_timestep(Duration::from_secs(1 << 20));
        }
        screen.acc += time.delta_secs() * 60.0;
        let bank = gallery.as_deref().and_then(|b| b.0.as_ref());
        while screen.acc >= 1.0 && c.running() {
            screen.acc -= 1.0;
            ceremony::tick(c, g.players.len() as u32, g.stage, sound.as_deref(), bank);
            if let Some(b) = banner.as_mut() {
                b.0.tick();
            }
        }
        if c.running() {
            return;
        }
    }
    if !screen.open && cer.is_none() && ceremony::pending() {
        return; // the ceremony begins once the match's last frame is held
    }
    if !screen.open {
        // the ceremony's last frame dissolves into the end screen
        if cer.is_some() && !dissolve.cut(&mut commands, dissolve::TO_PAGE, true) {
            return;
        }
        info!("result screen opens");
        *screen = Screen::opened();
        // ponytail: the fixed clock runs on but never reaches a step, as the pause menu
        fixed.set_timestep(Duration::from_secs(1 << 20));
    }
    let any = |b: GamepadButton| gamepads.iter().filter(|g| super::controls::readable(g)).any(|p| p.just_pressed(b));
    // a click or tap picks the pill under it; off them it does nothing
    let point = crate::widescreen::pointer_press(&window, &mouse, &touches).0;
    let confirm = keys.any_just_pressed([KeyCode::Enter, KeyCode::Space, KeyCode::KeyJ]) || any(GamepadButton::South);
    // the left stick as a d-pad: a push past half way
    let x = gamepads.iter().filter(|g| super::controls::readable(g)).filter_map(|p| p.get(GamepadAxis::LeftStickX)).find(|x| x.abs() >= 0.5).unwrap_or(0.0);
    let pushed = |s: f32| x * s >= 0.5 && screen.stick_x * s < 0.5;
    let right = keys.any_just_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) || any(GamepadButton::DPadRight) || pushed(1.0);
    let left = keys.any_just_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) || any(GamepadButton::DPadLeft) || pushed(-1.0);
    screen.stick_x = x;
    let play = |key: u8| {
        if let (Some(s), Some(b)) = (&sound, sounds.as_deref().and_then(|a| a.1.as_ref())) {
            s.play_menu(b, key);
        }
    };
    for s in &mut pads.slots {
        s.shot = None;
        s.serve = false;
    }
    // M196: ←→ (↑↓ too) choose, a press picks; nothing under the dissolve or the fade
    let online = np.is_some();
    let updown = next::updown(&keys, &gamepads, &mut screen.stick_y);
    let n = next::pills(online).len();
    if !dissolve.fading() && screen.leaving.is_none() {
        let by = right as i32 - left as i32 + updown;
        if by != 0 && screen.choose(by, n) {
            play(0);
        }
        let hit = point.and_then(|p| next::pill_at(&hits, p));
        if let Some(k) = hit {
            screen.sel = k;
        }
        if (confirm || hit.is_some()) && screen.ready() {
            let pick = next::pills(online)[screen.sel.min(n - 1)];
            play(2);
            info!("result screen: {pick:?}");
            next::choose(pick);
            screen.leaving = Some(0);
        }
    }
    screen.acc += time.delta_secs() * 60.0;
    while screen.acc >= 1.0 {
        screen.acc -= 1.0;
        screen.step();
        if let Some(t) = &mut screen.leaving {
            *t += 1;
            if *t >= 60 {
                exit.write(AppExit::Success);
            }
        }
        if let Some(b) = banner.as_mut() {
            b.0.tick();
        }
        // the figures and the confetti go on behind the screen
        if let Some(c) = cer.as_mut() {
            ceremony::tick(c, g.players.len() as u32, g.stage, sound.as_deref(), gallery.as_deref().and_then(|b| b.0.as_ref()));
        }
    }
}

/// The ceremony's quads, then the end screen's ([`next::fit`] to the screen); the dim and the fade to black.
#[allow(clippy::too_many_arguments)]
fn draw(
    g: Res<Game>,
    pads: Res<Pads>,
    screen: Res<Screen>,
    cer: Option<Res<ceremony::Ceremony>>,
    art: Option<Res<Art>>,
    panel_art: Option<Res<panel::Art>>,
    colours: Option<Res<Colours>>,
    (names, mut plates): (Option<Res<super::name_tags::Names>>, ResMut<super::name_tags::Plates>),
    mut q: Query<(&Slot, &mut ImageNode, &mut Node, &mut Visibility)>,
    mut fade: Query<&mut BackgroundColor, (With<Fade>, Without<Dim>)>,
    mut dim: Query<&mut BackgroundColor, (With<Dim>, Without<Fade>)>,
    np: Option<Res<super::netplay::Netplay>>,
    (board_art, words, pointer, mut hits, window): (
        Option<Res<popups::Art>>,
        Res<plate::Words>,
        Option<Res<next::Pointer>>,
        ResMut<next::Hits>,
        Query<&Window, With<bevy::window::PrimaryWindow>>,
    ),
) {
    for mut c in &mut fade {
        c.set_if_neq(BackgroundColor(Color::BLACK.with_alpha(screen.leaving.map_or(0.0, |t| (t as f32 / 60.0).min(1.0)))));
    }
    let shown = screen.open && g.match_stats.result.is_some();
    for mut c in &mut dim {
        c.set_if_neq(BackgroundColor(if shown { DIM } else { Color::NONE }));
    }
    let (Some(art), Some(panel_art), Some(colours), Some(board_art)) = (art, panel_art, colours, board_art) else {
        return;
    };
    let n = g.players.len();
    let slot = |i: usize| if i < n { pads.slot_of(i, n).unwrap_or(4) } else { 4 };
    let mut quads = Vec::new();
    if let Some(table) = g.match_stats.result.as_ref().filter(|_| screen.open) {
        quads = board::widget(&View {
            table,
            players: n,
            slots: std::array::from_fn(slot),
            pill: colours.0,
            // COM players show their AI row's rank, as the panel; humans none
            ranks: std::array::from_fn(|i| (i < n && slot(i) == 4).then(|| g.players[i].ai.rank as usize)),
            rank_rgb: colours.1,
            pulse: screen.alpha,
            sets_to_win: g.rules.sets,
            words: std::array::from_fn(|p| if p < n { words.widths(p) } else { None }),
        });
        let (all, mut rects) = next::compose(quads, np.is_some(), screen.sel);
        quads = all;
        next::fit(&mut quads, &mut rects, next::visible(crate::widescreen::window_aspect(&window)));
        hits.0 = rects;
    } else if let Some(c) = cer.filter(|_| g.match_stats.result.is_some()) {
        quads = ceremony::quads(&c.plan, n > 2);
    }
    // B371: named players' slot labels give way to their names
    let label = |q: &Quad| match q.tex {
        Tex::Text if q.src[0] == 0.0 && q.src[2] == 48.0 && q.src[1] >= 392.0 => Some(((q.src[1] - 392.0) / 24.0) as usize),
        _ => None,
    };
    let slots: Vec<usize> = (0..n).map(slot).collect();
    plates.0[super::name_tags::RESULT] = super::name_tags::take(&mut quads, names.as_deref(), &slots, 0.0, label, |q| (q.dst, q.src, q.rgb, q.alpha));
    for (Slot(i), mut img, mut node, mut vis) in &mut q {
        let Some(quad) = quads.get(*i) else {
            vis.set_if_neq(Visibility::Hidden);
            continue;
        };
        img.image = match quad.tex {
            Tex::Frame => art.0[0].clone(),
            Tex::Text => art.0[1].clone(),
            Tex::Gameset => art.0[3].clone(),
            Tex::Back => art.0[5].clone(),
            Tex::Team => art.0[7].clone(),
            Tex::Pill => art.0[8].clone(),
            Tex::Own(k) => art.0[9 + k as usize].clone(),
            Tex::Face(p) => panel_art.0[panel::FACES + p].clone(),
            Tex::Rank => panel_art.0[6].clone(),
            Tex::Board(k) => board_art.0[5 + k as usize].clone(),
            Tex::Slot => panel_art.0[1].clone(),
            Tex::Name(p) => words.name[p].as_ref().map_or_else(Handle::default, |w| w.0.clone()),
            Tex::Chara(p) => words.chara[p].as_ref().map_or_else(Handle::default, |w| w.0.clone()),
            Tex::Hand => match &pointer {
                Some(h) => h.0.clone(),
                None => {
                    vis.set_if_neq(Visibility::Hidden);
                    continue;
                }
            },
        };
        let [u, v, w, h] = quad.src;
        // B160: a face is its whole image (INPANE's 64×64, or a baked one's larger square); our words and the hand too
        img.rect = (!matches!(quad.tex, Tex::Face(_) | Tex::Name(_) | Tex::Chara(_) | Tex::Hand)).then(|| Rect::new(u, v, u + w, v + h));
        let [r, g, b] = quad.rgb.map(|c| c / 128.0);
        img.color = Color::srgba(r, g, b, quad.alpha / 128.0);
        let [x, y, w, h] = quad.dst;
        crate::play::widescreen::put(&mut node, x / 6.4, y / 4.48, w / 6.4, h / 4.48);
        vis.set_if_neq(Visibility::Inherited);
    }
}

/// The banner over the result page, where the page is: a screen point turned into the overlay camera's plane 10 in
/// front of it.
fn draw_banner(
    g: Res<Game>,
    screen: Res<Screen>,
    cer: Option<Res<ceremony::Ceremony>>,
    banner: Option<Res<Banner>>,
    mut q: Query<(&mut Visibility, &mut bevy::mesh::morph::MorphWeights)>,
    mut joints: Query<&mut Transform>,
    mut materials: ResMut<Assets<crate::effects::OnTop>>,
) {
    let Some(banner) = banner else { return };
    let (effect, shown) = (&banner.0, &banner.1);
    // F1: the end screen has its own words over the dim
    let during = cer.is_some() && !screen.open && g.match_stats.result.is_some();
    let x = during.then_some(0);
    // M98: no banner with the HUD off
    let x = x.filter(|_| !super::no_hud::OFF.load(std::sync::atomic::Ordering::Relaxed));
    let Some(x) = x else {
        if let Ok((mut v, _)) = q.get_mut(shown.root) {
            *v = Visibility::Hidden;
        }
        return;
    };
    crate::effects::pose(effect, shown, &mut q, &mut joints, &mut materials);
    let w = 2.0 * popups::CALL_DISTANCE * popups::CALL_FOV_DEG.to_radians().tan();
    let at = Vec3::new(
        (BANNER_AT[0] + x as f32 - 320.0) * w / 640.0,
        (BANNER_AT[1] - 224.0) * w * 0.75 / 448.0,
        popups::CALL_DISTANCE,
    );
    if let Ok(mut t) = joints.get_mut(shown.root) {
        *t = Transform::from_translation(at).with_rotation(Quat::from_rotation_y(std::f32::consts::PI)).with_scale(Vec3::splat(BANNER_SCALE));
    }
}

/// B756: the banner's space follows the match camera as the umpire's calls' does: its child, x and y scaled by the
/// match camera's half-angle tangent over the overlay camera's, z kept, so the banner keeps its pixels and size.
fn banner_space(
    mut commands: Commands,
    window: Query<&Window>,
    camera: Query<(Entity, &Projection), With<crate::Orbit>>,
    mut space: Query<(Entity, &mut Transform, Option<&ChildOf>), With<BannerSpace>>,
) {
    let (Ok((cam, proj)), Ok((e, mut t, parent))) = (camera.single(), space.single_mut()) else { return };
    if parent.is_none_or(|p| p.parent() != cam) {
        commands.entity(cam).add_child(e);
    }
    let Projection::Perspective(p) = proj else { return };
    let aspect = window.single().map_or(4.0 / 3.0, crate::play::widescreen::scene_aspect);
    let k = (p.fov / 2.0).tan() / (popups::CALL_FOV_DEG.to_radians().tan() * super::SHOWN_ASPECT.max(1.0 / aspect));
    let scale = Vec3::new(k, k, 1.0);
    if t.scale != scale {
        t.scale = scale;
    }
}

impl super::snapshot::Wire for Serving {}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn table() -> Table {
        let mut rows = [[0.0; 9]; 4];
        rows[0] = [161.0 * 0.62137, 24.0, 3.0, 1.0, 2.0, 37.5, 0.0, 60.0, 64.0];
        rows[1] = [149.0 * 0.62137, 18.0, 7.0, 1.0, 0.0, 12.25, 10.0, 55.0, 36.0];
        Table { rows, longest_rally: 7, frames: 60 * 754, winner: 0, score: Default::default() }
    }

    /// Singles: the values' digits where the original draws them (B764: the board's outlined digits, 20 px round
    /// each cell's middle; the rows `HEAD` lower), the bests highlighted, ties not.
    #[test]
    fn singles() {
        let t = table();
        let rank_rgb = [[127, 105, 112], [82, 107, 121], [0; 3], [0; 3]];
        let pill = [[98, 46, 61], [33, 74, 97], [0; 3], [0; 3]];
        let v = View { table: &t, players: 2, slots: [0, 4, 4, 4], pill, ranks: [None, Some(5), None, None], rank_rgb, pulse: 51, sets_to_win: 1, words: [Some([30.0, 40.0]); 4] };
        let quads = layout(&v);
        let at = |x: i32, y: i32| quads.iter().filter(|q| q.dst[0] == x as f32 && q.dst[1] == y as f32).map(|q| q.src).collect::<Vec<_>>();
        // the cell at (x, y) holds digit d
        let digit = |x: i32, y: i32, d: u32| at(x - 2, y + HEAD + 2) == vec![[d as f32 * 32.0, 0.0, 32.0, 32.0]];
        // fastest serve: 161 km/h is 100 mph at 0x8a
        assert!(digit(0x8a, 120, 1) && digit(0x8a + 32, 120, 0));
        // points won 24 right-aligned in 4 digits from 0x92: "2" at +32, "4" at +48
        assert!(digit(0x92 + 32, 144, 2) && digit(0x92 + 48, 144, 4));
        assert!(at(0x92 - 2, 144 + HEAD + 2).is_empty());
        // 12.25 %: "1" "2" "." "2" from 0x19a (no hundreds); the point is the stats sheet's
        assert!(digit(0x19a + 16, 240, 1) && digit(0x19a + 56, 240, 2));
        assert_eq!(at(0x19a + 48, 240 + HEAD), vec![[160.0, 0.0, 8.0, 24.0]]);
        // highlights: player 0 for points (row 1), missed (2); double faults tie (3); never net play (6)
        let lit = |r: i32| quads.iter().filter(|q| q.src == [49.0, 273.0, 6.0, 8.0] && q.dst[1] == (120 + HEAD + 24 * r) as f32).map(|q| q.dst[0]).collect::<Vec<_>>();
        assert_eq!(lit(1), vec![136.0]);
        assert_eq!(lit(2), vec![136.0]);
        assert!(lit(3).is_empty());
        assert!(lit(6).is_empty());
        // 12:34 match time: 00:12:34, in the bar
        assert!(digit(480 + 16, 359, 2) && digit(520, 359, 3));
        assert!(quads.len() <= POOL);
        // no digit from the stats sheet
        assert!(!quads.iter().any(|q| q.tex == Tex::Text && q.src[1] == 0.0 && q.src[0] < 160.0));
        // the dark background; the COM's rank under its "COM", in the room beside the face
        assert_eq!((quads[0].tex, quads[0].alpha), (Tex::Back, board::BACK_ALPHA));
        let ranks: Vec<_> = quads.iter().filter(|q| q.tex == Tex::Rank).map(|q| (q.src, q.dst, q.rgb)).collect();
        assert_eq!(ranks, vec![([0.0, 120.0, 64.0, 24.0], [410.0, 106.0, 42.0, 15.75], [82.0, 107.0, 121.0])]);
        // each plate: the ring, the face in it, the name and the pill
        for p in 0..2 {
            assert_eq!(quads.iter().filter(|q| matches!(q.tex, Tex::Name(k) | Tex::Chara(k) | Tex::Face(k) if k == p)).count(), 3);
        }
        assert_eq!(quads.iter().filter(|q| q.tex == Tex::Board(0) && q.src == [80.0, 0.0, 48.0, 48.0]).count(), 2);
        // "Stats" white; the row names plain
        let stats: Vec<_> = quads.iter().filter(|q| q.dst[0] == 240.0 && q.dst[1] < 120.0).map(|q| (q.src, q.dst, q.rgb)).collect();
        assert_eq!(stats, [([0.0, 64.0, 160.0, 40.0], [240.0, 88.0, 160.0, 40.0], WHITE)]);
        assert!(quads.iter().filter(|q| q.dst[0] == 240.0 && q.dst[1] >= 120.0 && q.dst[3] == 24.0).all(|q| q.rgb == WHITE));
    }

    /// The player on the left half: the rank in the room right of the face.
    #[test]
    fn rank_left() {
        let t = table();
        let v = View { table: &t, players: 2, slots: [4, 0, 4, 4], pill: [[0; 3]; 4], ranks: [Some(2), None, None, None], rank_rgb: [[1; 3]; 4], pulse: 0, sets_to_win: 1, words: [None; 4] };
        let q = layout(&v).into_iter().find(|q| q.tex == Tex::Rank).unwrap();
        assert_eq!((q.src, q.dst), ([0.0, 48.0, 64.0, 24.0], [188.0, 106.0, 42.0, 15.75]));
    }

    /// One screen: the pills take ✕ from tick 120; choosing stops at the row's ends.
    #[test]
    fn choose() {
        let mut s = Screen::opened();
        for _ in 0..119 {
            s.step();
        }
        assert!(!s.ready());
        s.step();
        assert!(s.ready());
        assert!(!s.choose(-1, 3));
        assert!(s.choose(1, 3) && s.choose(1, 3));
        assert!(!s.choose(1, 3));
        assert_eq!(s.sel, 2);
    }

    #[test]
    fn doubles_fits() {
        // the whole end screen: the widget (five sets), its banner and the pills
        let mut t = table();
        t.score.set = 5;
        t.score.set_games[0] = [6, 4, 6, 3, 7];
        t.score.set_games[1] = [4, 6, 3, 6, 6];
        let v = View { table: &t, players: 4, slots: [0, 4, 4, 4], pill: [[128; 3]; 4], ranks: [Some(0); 4], rank_rgb: [[128; 3]; 4], pulse: 0, sets_to_win: 3, words: [Some([40.0, 50.0]); 4] };
        let (quads, _) = next::compose(board::widget(&v), true, 0);
        assert!(quads.len() <= POOL, "{}", quads.len());
    }

    #[test]
    fn pulse() {
        let mut p = Pulse { down: false, c: 0x14 };
        let a: Vec<i32> = (0..44).map(|_| p.step()).collect();
        assert_eq!((a[0], a[20], a[21], a[41], a[42]), (0, 102, 102, 0, 0));
    }
}
