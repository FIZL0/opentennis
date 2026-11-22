//! The original's pause screen (textures from `AZUMA/INPANE/INPANE.XB0`), coordinates on the PS2's 640×448 screen as
//! the panel. Start opens it (system sound 2) and freezes the match: the court dimmed (`i_pause_result_15` at alpha
//! 64; F2: 104), the top bar (`inpane_pause03`) with "Paused" and the rules ("1 sets 4 games", `inpane_pause02`), and
//! the dialog (the original's `i_dialog_00`/`01`, drawn with our own box, pills and words since B141): "Resume the
//! Match" / "Return to the Menu" on pills, the chosen one bright, the hand
//! (`hsm_yubi`) beside it swinging out 16 px and back. Between them the remaster's "Settings" (B126, ours: the dialog's sheet has no such row, so `word.tm2` writes it) opens the main
//! menu's Settings screens over the paused match (`main_menu/pause.rs`); a click or tap on a pill chooses it.
//!
//! Input waits 5 ticks after it opens. Up/down move with wrap-around (repeat after 32 ticks held, then every 8;
//! sound 0, the hand starts its swing again), ✕ chooses (sound 2), Start closes it (sound 2). ○ does nothing in the
//! original; here (B340) it is Back and closes it too (sound 1), as does a right click.
//! "Return to the Menu" fades to black over 60 ticks to the menu.
//! F2 (ours, the user's concept round): the original's score board at the bottom (`score_board`, kept for the
//! Continue cards) gives way to the end screen's stats board, live (`match_stats::pause_board`): the score so far
//! over the match's numbers, right of the dialog, up the whole time the dialog or its confirm is, all of it fitted
//! between the top bar and the key legend at one scale for any aspect ratio. "Save and Quit" and "Return to the Menu"
//! are one "Quit", which asks "Quit the match?" in the dialog's place (its title over its pills, the chosen one's note
//! under them): "Save and Quit" (where this machine can save), "Quit without Saving" (the fade out), "Cancel" (back
//! to the pause menu on "Quit", as ○ and Start are); up/down wrap, ✕ chooses.
//! B254 (ours): in a local match a fourth pill, "Controllers", opens the main menu's controller assignment over the
//! paused match (`main_menu/pause.rs`) to move devices between the human seats.
//! `HST_PAUSE=<seconds>` opens it after that long (for `--shot`), with `HST_PAUSE_SETTINGS=settings|graphics|controls|assign`
//! on that Settings screen (or the controller assignment), or with `HST_PAUSE_ASK=1` on the Quit the match? confirm.
//! M194 (ours): "Save and Quit" (a local match, or the host's online one; the browser too, M198; F2: in the Quit
//! confirm) saves the match as its point started (`save_game`) and shows "Match saved" in the dialog's style
//! (`main_menu/pause.rs` `SavedBox`): the score where it picks up and how to go on; OK fades out to the menu.
//! `HST_PAUSE_SAVE=1` (with `HST_PAUSE`) saves at once, for `--shot`.

use bevy::prelude::*;
use hst_data::{iso::Iso, xb::Archive};
use std::sync::Arc;
use std::time::Duration;

use super::main_menu::pause::{PauseSettings, Root as PauseRoot, SavedBox, SavedRoot};
use super::match_stats::pause_board::{Build as PauseBoardBuild, PauseBoard};
use super::panel::{self, Colours};
use super::widescreen::Anchor;
use super::{Game, Pads};
use crate::Args;
use crate::audio::{Sound, SoundBank};

mod plate;
mod rules_tab;
/// B750: the score board for the Continue screen's save cards.
pub(super) mod saved_board;

// Z21: not `i_dialog_00`/`01`, which our box, pills and rows (B141) stand in for, nor `hsm_yubi`, whose place the 3D
// hand always takes (`cursor::setup` at Startup)
const TEXTURES: [&str; 8] = [
    "/i_pause_result_15.tm2",
    "/inpane_pause00.tm2",
    "/inpane_pause01.tm2",
    "/inpane_pause02.tm2",
    "/inpane_pause03.tm2",
    "/result_gameset00.tm2",
    "/result_gameset01.tm2",
    "/result_gameset02.tm2",
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tex {
    /// `TEXTURES[k]`.
    Own(usize),
    /// The panel's slot labels.
    Slot,
    Face(usize),
    /// The 3D pointing hand (`cursor.rs`), where the original swung `hsm_yubi`.
    Hand,
    /// B671: player `p`'s name in the tag's place, and their character's pill (`plate.rs`), each its whole image.
    Name(usize),
    Chara(usize),
    /// B671a: the Offbeat rules tab (`rules_tab.rs`), its whole image.
    Rules,
    /// F2: the live stats board's picture `i` (`match_stats::pause_board`), its own image, part and colour.
    Live(usize),
}
const DIM: Tex = Tex::Own(0);
const POINTS: Tex = Tex::Own(1);
const TIEBREAK: Tex = Tex::Own(2);
const TEXT: Tex = Tex::Own(3);
const BAR: Tex = Tex::Own(4);
const BOARD: Tex = Tex::Own(5);
const GAMES_PAST: Tex = Tex::Own(6);
const GAMES_NOW: Tex = Tex::Own(7);
const HAND: Tex = Tex::Hand;
/// "Settings" in `word.tm2` on a row of its own (`pause::label`).
const LABEL: Tex = Tex::Own(8);
/// B141: "Resume the Match" set the same way, in place of `i_dialog_01`'s rows; F2: "Quit", in "Return to the Menu"'s
/// place.
const RESUME: Tex = Tex::Own(9);
const QUIT_ROW: Tex = Tex::Own(10);
/// F2: the quit confirm's rows, set as `RESUME`: "Quit the match?", "Quit without Saving", "Cancel".
const ASK: Tex = Tex::Own(11);
const DROP_ROW: Tex = Tex::Own(12);
const CANCEL_ROW: Tex = Tex::Own(13);
/// B254: "Controllers", set as `RESUME`.
const SEATS: Tex = Tex::Own(14);
/// B141: our pill and box layers (`menu_art::pill_layers`), in place of `i_dialog_00`'s frame and pills; after
/// [`LABELS`] in `setup` (B177: it was left at 14 when B68 added three labels, drawing the pills from "Leave the match?").
const PILL: Tex = Tex::Own(15);
/// B293: the match's court name and umpire (`pause::venue`), set as [`LABEL`], at the right of the top bar.
const COURT: Tex = Tex::Own(16);
const UMPIRE: Tex = Tex::Own(17);
/// The rows set in `setup` after `TEXTURES`, as [`LABEL`] to [`SEATS`].
const LABELS: [&str; 7] = ["Settings", "Resume the Match", "Quit", "Quit the match?", "Quit without Saving", "Cancel", "Controllers"];
/// M194: "Save and Quit", set as [`LABEL`] after [`UMPIRE`].
const SAVE_ROW: Tex = Tex::Own(18);
const SAVE_LABEL: &str = "Save and Quit";
/// F2: the quit confirm's note under its pills for the chosen one (Save and Quit, Quit without Saving, Cancel), set
/// [`NOTE_W`] wide after [`SAVE_ROW`] as the end screen's notes.
const NOTES: [&str; 3] = ["Keep this match to continue later", "Drop this match and go to the menu", "Back to the pause menu"];
const NOTE: usize = 19;
const NOTE_W: usize = 448;
/// The dialog's colours (ours, B141): rims, its fill, a pill's fill chosen and not.
const INK: [f32; 3] = [15.0; 3];
pub(super) const PANEL: [f32; 3] = [52.0, 44.0, 36.0];
pub(super) const CHOSEN: [f32; 3] = [123.0, 105.0, 79.0];
pub(super) const UNCHOSEN: [f32; 3] = [94.0, 83.0, 70.0];
/// B171: a label row's size on its pill: 20 px of the 32-px pill high, at most 160 of its 288 px wide.
const FIT: f32 = 0.625;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Quad {
    tex: Tex,
    src: [f32; 4],
    dst: [f32; 4],
    rgb: [f32; 3],
    alpha: f32,
}

/// The dialog's rows as `i_dialog_01` numbers them: "Resume the Match", (ours) "Settings", F2: "Quit" (in "Return to the
/// Menu"'s place: it asks first).
const OPTIONS: [i32; 3] = [6, SETTINGS, QUIT];
const QUIT: i32 = 1;
/// Not a row of `i_dialog_01`: [`LABEL`].
const SETTINGS: i32 = -1;
/// B254: a local match's rows (M124: or an online one with two local players), with (ours) "Controllers" ([`SEATS`]) before "Quit".
const OPTIONS_LOCAL: [i32; 4] = [6, SETTINGS, CONTROLLERS, QUIT];
const CONTROLLERS: i32 = -2;
/// F2: the quit confirm's choices: M194's "Save and Quit", "Quit without Saving" (what "Return to the Menu" did),
/// "Cancel".
const SAVE: i32 = -3;
const DROP: i32 = -4;
const BACK: i32 = -5;
const ASKS: [i32; 3] = [SAVE, DROP, BACK];

/// The dialog's rows: "Controllers" only when there is something to assign (M124: online, only with two local players).
fn options(local: bool) -> &'static [i32] {
    if local { &OPTIONS_LOCAL } else { &OPTIONS }
}

/// F2: the quit confirm's choices: "Save and Quit" where this machine can save the match (M194).
fn asks(save: bool) -> &'static [i32] {
    if save { &ASKS } else { &ASKS[1..] }
}

/// A row's words and (F2) a confirm choice's note.
fn row_tex(row: i32) -> Tex {
    match row {
        6 => RESUME,
        SETTINGS => LABEL,
        CONTROLLERS => SEATS,
        SAVE => SAVE_ROW,
        DROP => DROP_ROW,
        BACK => CANCEL_ROW,
        _ => QUIT_ROW,
    }
}
fn note_tex(row: i32) -> Tex {
    Tex::Own(NOTE + ASKS.iter().position(|&r| r == row).unwrap_or(2))
}

/// The score board's height in 16 px plate rows: 10 for three sets to win, else 6; (ours) doubles at one or two sets
/// get 8, so each face's character pill clears the face under it.
fn board_rows(sets_to_win: i32, players: usize) -> i32 {
    match (sets_to_win, players) {
        (3, _) => 10,
        (_, 3..) => 8,
        _ => 6,
    }
}
/// B671a: the score frame's top: the board's rows over its foot at 424, and its rim.
fn frame_top(rows: i32) -> f32 {
    (416 - 16 * rows) as f32
}
/// Points cells (column, row) by point index; 4 is Deuce, 5 Advantage (tiebreak: 0–7, 8 Deuce, 9 Advantage).
const POINT_U: [i32; 10] = [0, 1, 0, 1, 0, 1, 0, 1, 0, 1];
const POINT_V: [i32; 10] = [0, 0, 1, 1, 2, 2, 3, 3, 4, 4];
const WHITE: [f32; 3] = [128.0; 3];

/// The hand's swing: out to −16 slowing down, an 11-tick rest, then back speeding up (57 ticks a cycle).
#[derive(Clone, Copy, Default, Debug)]
struct Hand {
    x: f32,
    t: u32,
    stage: u32,
}

impl Hand {
    fn step(&mut self) {
        let t = self.t as f32;
        match self.stage {
            0 => {
                self.x += (t / 3.0).powi(2) / 100.0 + 0.35;
                if self.x >= 0.0 {
                    *self = Hand { x: 0.0, t: 0, stage: 1 };
                }
            }
            1 => {
                self.x -= ((30.0 - t) / 3.0).powi(2) / 100.0 + 0.35;
                if self.x <= -16.0 {
                    *self = Hand { x: -16.0, t: 0, stage: 2 };
                }
            }
            _ if self.t >= 10 => (self.stage, self.t) = (0, 0),
            _ => {}
        }
        self.t += 1;
    }
}

/// The pause state, stepped at 60 Hz.
#[derive(Resource, Default)]
struct Menu {
    open: bool,
    /// Ticks into the fade to black after "Quit without Saving" (or the saved box's OK).
    quitting: Option<i32>,
    /// Ticks left before input is read.
    delay: i32,
    sel: usize,
    hand: Hand,
    /// Ticks up/down held (for the repeat).
    held: [u32; 2],
    /// Presses since the last tick: Start, ✕.
    start: bool,
    confirm: bool,
    acc: f32,
    /// "Settings" chosen: `step` opens them.
    settings: bool,
    /// B254: the dialog has "Controllers" (a local match, or M124 an online one with two local players), set by `step`;
    /// "Controllers" chosen.
    local: bool,
    seats: bool,
    /// F2: the quit confirm is up with its choice `k` chosen ([`asks`]); ○ pressed since the last tick.
    ask: Option<usize>,
    cancel: bool,
    /// M194: the confirm has "Save and Quit" (set by `step`); it was chosen (`save_match` saves); the "Match saved" box
    /// is up, `true` saved (OK fades out), `false` not (OK goes back to the dialog).
    save: bool,
    saving: bool,
    saved: Option<bool>,
}

impl Menu {
    fn opened() -> Self {
        Menu { open: true, delay: 5, ..default() }
    }

    /// One tick of the open menu with up/down held; returns the system sound to play.
    fn tick(&mut self, up: bool, down: bool) -> Option<usize> {
        if self.quitting.is_some() {
            self.quitting = self.quitting.map(|t| t + 1);
            return None;
        }
        if let Some(a) = self.ask {
            return self.tick_ask(a, up, down);
        }
        if let Some(ok) = self.saved {
            return self.tick_saved(ok);
        }
        if self.saving {
            return None;
        }
        // B340 (ours: the original's ○ does nothing here): ○ is Back, closing the menu like Start (with the back sound)
        let cancel = std::mem::take(&mut self.cancel);
        if std::mem::take(&mut self.start) {
            *self = Menu::default();
            return Some(2);
        }
        if cancel {
            *self = Menu::default();
            return Some(1);
        }
        let mut sound = None;
        if self.delay < 1 {
            // first press, then after 32 ticks held every 8; one direction at a time
            let repeat = |on: bool, n: &mut u32| {
                *n = if on { *n + 1 } else { 0 };
                *n == 1 || (*n >= 32 && (*n - 32).is_multiple_of(8))
            };
            let up = self.held[1] == 0 && repeat(up, &mut self.held[0]);
            let down = self.held[0] == 0 && repeat(down, &mut self.held[1]);
            let opts = options(self.local);
            let n = opts.len();
            if down || up {
                self.sel = if down { (self.sel + 1) % n } else { (self.sel + n - 1) % n };
                self.hand = Hand::default();
                sound = Some(0);
            } else if self.confirm {
                sound = Some(2);
                match opts[self.sel] {
                    6 => *self = Menu::default(),
                    SETTINGS => self.settings = true,
                    CONTROLLERS => self.seats = true,
                    // F2: "Quit" always asks, its first choice chosen
                    _ => (self.ask, self.hand) = (Some(0), Hand::default()),
                }
            }
        } else {
            self.delay -= 1;
        }
        self.confirm = false;
        self.hand.step();
        sound
    }

    /// M194: one tick of the "Match saved" box: ✕ (its one pill, OK), ○ or Start fade out to the menu once saved, or
    /// go back to the dialog when it wasn't.
    fn tick_saved(&mut self, ok: bool) -> Option<usize> {
        let (start, cancel, confirm) =
            (std::mem::take(&mut self.start), std::mem::take(&mut self.cancel), std::mem::take(&mut self.confirm));
        let mut sound = None;
        if start || cancel || confirm {
            if ok {
                self.quitting = Some(0);
            } else {
                (self.saved, self.hand) = (None, Hand::default());
            }
            sound = Some(if confirm || ok { 2 } else { 1 });
        }
        self.hand.step();
        sound
    }

    /// F2: one tick of the quit confirm with choice `a` chosen: the menu's up/down repeat (with wrap-around) over its
    /// choices, ✕ chooses ("Save and Quit" saves, "Quit without Saving" fades out, "Cancel" goes back to the pause menu
    /// on "Quit"), ○ or Start is Cancel; a direction held keeps its count across the switch.
    fn tick_ask(&mut self, a: usize, up: bool, down: bool) -> Option<usize> {
        let choices = asks(self.save);
        let n = choices.len();
        let a = a.min(n - 1);
        let (start, cancel, confirm) =
            (std::mem::take(&mut self.start), std::mem::take(&mut self.cancel), std::mem::take(&mut self.confirm));
        let repeat = |on: bool, n: &mut u32| {
            *n = if on { *n + 1 } else { 0 };
            *n == 1 || (*n >= 32 && (*n - 32).is_multiple_of(8))
        };
        let u = self.held[1] == 0 && repeat(up, &mut self.held[0]);
        let d = self.held[0] == 0 && repeat(down, &mut self.held[1]);
        let mut sound = None;
        if u || d {
            (self.ask, self.hand) = (Some(if d { (a + 1) % n } else { (a + n - 1) % n }), Hand::default());
            sound = Some(0);
        } else if start || cancel || (confirm && choices[a] == BACK) {
            (self.ask, self.hand) = (None, Hand::default());
            sound = Some(if confirm { 2 } else { 1 });
        } else if confirm {
            if choices[a] == SAVE {
                (self.ask, self.saving) = (None, true);
            } else {
                self.quitting = Some(0);
            }
            sound = Some(2);
        }
        self.hand.step();
        sound
    }
}

/// What the pause screen shows.
struct View {
    players: usize,
    slots: [usize; 4],
    pill: [[u8; 3]; 4],
    /// Rules: sets to win, games a set.
    sets_to_win: i32,
    games: i32,
    points: [i32; 2],
    deuce: bool,
    advantage: bool,
    tiebreak: bool,
    /// The set in play (0-based) and each team's games per set.
    set: i32,
    set_games: [[i32; 5]; 2],
    sel: usize,
    hand_x: f32,
    fade: Option<i32>,
    /// F2: the quit confirm is up with its choice `k` chosen ([`asks`]).
    ask: Option<usize>,
    /// B254: a local match's dialog, with "Controllers".
    local: bool,
    /// M194: the confirm has "Save and Quit"; the "Match saved" box is up (`pause.rs` draws it over the dimmed screen
    /// in the dialog's place).
    save: bool,
    saved: bool,
    /// B671: each player's name and character pill widths (`plate.rs`), once set: drawn in place of the slot tag.
    words: [Option<[f32; 2]>; 4],
    /// B671a: the rules tab's size (`rules_tab.rs`) while any Offbeat rule is on.
    tab: Option<[f32; 2]>,
}

/// B750: the old pause screen's score board (frame, plates, each set's games, the points), which the Continue screen's
/// save cards draw (`saved_board.rs`); F2's pause screen shows the stats board's in its place.
fn score_board(v: &View) -> Vec<Quad> {
    let mut out = Vec::new();
    let mut q = |tex, src: [i32; 4], dst: [f32; 4], rgb: [f32; 3], alpha: i32| {
        out.push(Quad { tex, src: src.map(|c| c as f32), dst, rgb, alpha: alpha as f32 })
    };
    let r = |x: i32, y: i32, w: i32, h: i32| [x, y, w, h].map(|c| c as f32);
    let rgb = |c: [u8; 3]| c.map(f32::from);
    let board = board_rows(v.sets_to_win, v.players);
    let (centre_h, top) = (16 * board, frame_top(board) as i32);
    let (edge_h, y0) = (centre_h - 16, top + 8);
    let (diag_y, mid) = (y0 + board * 8 - 8, y0 + centre_h / 2);
    let n = v.players;
    for p in 0..n {
        let (count, off) = if n < 3 { (board, 0) } else { (board / 2, if p > 1 { board * 8 } else { 0 }) };
        for k in 0..count {
            q(BOARD, [1 + 16 * p as i32, 40, 14, 16], r([96, 456][p & 1], y0 + off + 16 * k, 88, 16), WHITE, 128);
        }
    }
    if n >= 3 {
        for side in 0..2 {
            q(BOARD, [0, [56, 80][side], 88, 24], r([96, 456][side], diag_y, 88, 24), WHITE, 128);
        }
    }
    let bottom = top + 16 + edge_h;
    for (u, x, w) in [(0, 88, 16), (16, 104, 432), (40, 536, 16)] {
        q(BOARD, [u, 0, 16, 16], r(x, top, w, 16), WHITE, 128);
        q(BOARD, [u, 16, 16, 8], r(x, top + 16, w, edge_h), WHITE, 128);
        q(BOARD, [u, 24, 16, 16], r(x, bottom, w, 16), WHITE, 128);
    }
    q(BOARD, [65, 41, 14, 14], r(184, y0, 272, centre_h), WHITE, 128);
    for x in [248, 390, 184, 454] {
        q(BOARD, [64, 9, 8, 22], r(x, y0, 8, centre_h), WHITE, 128);
    }
    // a face and its pill (54 px) centred on the board, or doubles' on each half
    let plate_y = if n < 3 { [mid - 22; 2] } else { [y0 + centre_h / 4 - 27, y0 + centre_h * 3 / 4 - 27] };
    for p in 0..n {
        let (x, y) = ([100, 496][p & 1], plate_y[p / 2]);
        q(BOARD, [80, 0, 48, 48], r(x, y, 48, 48), WHITE, 128);
        q(Tex::Face(p), [0, 0, 64, 64], r(x + 2, y + 2, 64, 64), WHITE, 128);
        if let Some(w) = v.words[p] {
            let [name, pill] = plate::place(p, x as f32, y as f32, w);
            q(Tex::Name(p), [0; 4], name, WHITE, 128);
            q(Tex::Chara(p), [0; 4], pill, WHITE, 128);
            continue;
        }
        q(Tex::Slot, [v.slots[p] as i32 * 40, 0, 40, 24], r([144, 456][p & 1], y + 6, 40, 24), rgb(v.pill[p]), 128);
    }
    // the set in play on the bright sheet, a past set the team didn't win at half alpha
    let cols = v.sets_to_win * 2 - 1;
    let games_y = mid - 16 * cols;
    for team in 0..2 {
        for col in 0..cols {
            let y = games_y + 32 * col;
            let sheet = if col == v.set { GAMES_NOW } else { GAMES_PAST };
            if col <= v.set {
                let c = col.min(4) as usize;
                let g = v.set_games[team][c];
                let alpha = if col == v.set || v.set_games[team ^ 1][c] < g { 128 } else { 64 };
                q(sheet, [g * 32, 0, 32, 32], r([272, 336][team], y, 32, 32), WHITE, alpha);
            }
            if team == 0 {
                q(sheet, [320, 0, 24, 32], r(308, y, 24, 32), WHITE, 128);
            }
        }
    }
    // the points; with advantage the trailing team at half alpha
    let (tex, last) = if v.tiebreak { (TIEBREAK, 9) } else { (POINTS, 5) };
    for t in 0..2 {
        let p = v.points[t].clamp(0, last);
        let k = match (v.deuce, v.advantage) {
            (true, _) => last - 1,
            (false, true) if p >= last - 1 => last,
            _ => p,
        } as usize;
        let alpha = if v.advantage && k != last as usize { 64 } else { 128 };
        q(tex, [POINT_U[k] << 6, POINT_V[k] * 48, 64, 48], r([184, 390][t], mid - 24, 64, 48), WHITE, alpha);
    }

    out
}

/// F2: the dialog's box and the board around it (the 640×448 screen's px, as the end screen's `next.rs`): its width, a
/// pill's left, width and height and the step between pills, the first pill's top (under the confirm's title), the
/// note's gap and height, the room under the last row.
const BOX_W: f32 = 320.0;
const PILL_X: f32 = 16.0;
const PILL_W: f32 = 288.0;
const PILL_H: f32 = 32.0;
const STEP: f32 = 40.0;
const FIRST: f32 = 20.0;
const ASK_FIRST: f32 = 46.0;
const NOTE_GAP: f32 = 6.0;
const NOTE_H: f32 = 15.0;
const BOTTOM: f32 = 20.0;
/// The gap between the box and the board, and how far left of the box the hand reaches; the rules tab's gap under the
/// box.
const GAP: f32 = 24.0;
const HAND_ROOM: f32 = 44.0;
const TAB_GAP: f32 = 8.0;
/// The room kept under the top bar (64 high) and over the key legend's strip, and at the sides.
const BAR_H: f32 = 64.0;
const MARGIN: [f32; 2] = [6.0, 16.0];
const SIDE: f32 = 8.0;
/// F2 (the concept's): the court under the board and the dialog dimmed this much more than the original's 64.
const DIM_ALPHA: i32 = 104;

/// F2: what the screen is laid out around: the live board's pictures' places (`match_stats::pause_board`, from y 0,
/// centred on x 320) and the visible part of the 640×448 screen.
struct Around {
    board: Vec<[f32; 4]>,
    vis: [f32; 2],
}

/// F2: where the left column (the dialog, the rules tab under it) and the board go: the column left of the board, their
/// middles level, all of it at one scale `k` (no larger than its own size) centred in the room between the top bar and
/// the legend's strip, so it keeps its shape at any aspect ratio.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Place {
    col: [f32; 2],
    l: f32,
    t: f32,
    k: f32,
    x0: f32,
    y0: f32,
}

impl Place {
    fn new(board: &[[f32; 4]], [cw, ch]: [f32; 2], [vw, vh]: [f32; 2]) -> Place {
        let mut b = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &[x, y, w, h] in board {
            b = (b.0.min(x), b.1.min(y), b.2.max(x + w), b.3.max(y + h));
        }
        let col = if board.is_empty() { [320.0 - cw / 2.0, 224.0 - ch / 2.0] } else { [b.0 - GAP - cw, (b.1 + b.3) / 2.0 - ch / 2.0] };
        let (l, t) = ((col[0] - HAND_ROOM).min(b.0), col[1].min(b.1));
        let (r, bottom) = ((col[0] + cw).max(b.2), (col[1] + ch).max(b.3));
        let top = 224.0 - vh / 2.0 + BAR_H + MARGIN[0];
        let room = [vw - 2.0 * SIDE, vh - BAR_H - MARGIN[0] - MARGIN[1]];
        let k = (room[0] / (r - l)).min(room[1] / (bottom - t)).min(1.0);
        Place { col, l, t, k, x0: 320.0 - (r - l) * k / 2.0, y0: top + (room[1] - (bottom - t) * k) / 2.0 }
    }

    /// A rect in the board's (or the whole layout's) units, on the screen.
    fn map(&self, [x, y, w, h]: [f32; 4]) -> [f32; 4] {
        [self.x0 + (x - self.l) * self.k, self.y0 + (y - self.t) * self.k, w * self.k, h * self.k]
    }

    /// A rect in the column's own units, on the screen.
    fn col(&self, [x, y, w, h]: [f32; 4]) -> [f32; 4] {
        self.map([x + self.col[0], y + self.col[1], w, h])
    }
}

/// F2: the dialog in its own units (the box's top-left at 0, 0): the box, a pill per row (bright when chosen), the
/// words, the hand; the quit confirm with its question as the title over its choices and the chosen one's note under
/// them. Its quads, its height and the pills' rects.
fn dialog(v: &View) -> (Vec<Quad>, f32, Vec<[f32; 4]>) {
    let mut out = Vec::new();
    let (rows, sel, first): (Vec<i32>, usize, f32) = match v.ask {
        Some(a) => (asks(v.save).to_vec(), a.min(asks(v.save).len() - 1), ASK_FIRST),
        None => (options(v.local).to_vec(), v.sel, FIRST),
    };
    let pills: Vec<[f32; 4]> = (0..rows.len()).map(|k| [PILL_X, first + STEP * k as f32, PILL_W, PILL_H]).collect();
    let end = first + STEP * rows.len() as f32 - (STEP - PILL_H);
    let h = end + if v.ask.is_some() { NOTE_GAP + NOTE_H } else { 0.0 } + BOTTOM;
    let quad = |tex, src, dst, rgb, alpha| Quad { tex, src, dst, rgb, alpha };
    for (i, rgb) in [INK, WHITE, PANEL].into_iter().enumerate() {
        out.extend(super::menu_art::box_quads(i, [0.0, 0.0, BOX_W, h], 16.0).into_iter().map(|(src, dst)| quad(PILL, src, dst, rgb, 128.0)));
    }
    if v.ask.is_some() {
        // the title as the main menu's popups': 22 px high
        let (w, th) = (256.0 * 22.0 / 32.0, 22.0);
        out.push(quad(ASK, [0.0, 0.0, 256.0, 32.0], [BOX_W / 2.0 - w / 2.0, 14.0, w, th], WHITE, 128.0));
    }
    for (k, &r) in pills.iter().enumerate() {
        let fill = if k == sel { CHOSEN } else { UNCHOSEN };
        for (i, rgb) in [INK, WHITE, fill].into_iter().enumerate() {
            out.extend(super::menu_art::pill_quads(i, r).iter().map(|&(src, dst)| quad(PILL, src, dst, rgb, 128.0)));
        }
    }
    for (k, (&row, r)) in rows.iter().zip(&pills).enumerate() {
        let (rgb, alpha) = if k == sel { (WHITE, 128.0) } else { (UNCHOSEN, 64.0) };
        // B171: the 256×32 row's ink is as tall as the pill; drawn at `FIT` and centred it sits inside the rims
        let (w, rh) = (256.0 * FIT, 32.0 * FIT);
        out.push(quad(row_tex(row), [0.0, 0.0, 256.0, 32.0], [r[0] + (r[2] - w) / 2.0, r[1] + (r[3] - rh) / 2.0, w, rh], rgb, alpha));
    }
    if let (Some(&row), Some(_)) = (rows.get(sel), v.ask) {
        let w = NOTE_W as f32 * NOTE_H / 32.0;
        out.push(quad(note_tex(row), [0.0, 0.0, NOTE_W as f32, 32.0], [BOX_W / 2.0 - w / 2.0, end + NOTE_GAP, w, NOTE_H], UNCHOSEN.map(|c| c + 24.0), 128.0));
    }
    if let Some(r) = pills.get(sel) {
        out.push(quad(HAND, [0.0, 0.0, 64.0, 32.0], [v.hand_x + r[0] - 48.0, r[1] + 8.0, 64.0, 32.0], WHITE, 128.0));
    }
    (out, h, pills)
}

/// B671a: the Offbeat rules tab `[w, h]` with its top-left at `x`, `y`: the board frame's corners, rim and fill around
/// its words (F2: on its own under the dialog, all four sides).
fn rules_tab_quads(out: &mut Vec<Quad>, [x, y]: [f32; 2], [w, h]: [f32; 2]) {
    let quad = |src: [i32; 4], dst| Quad { tex: BOARD, src: src.map(|c| c as f32), dst, rgb: WHITE, alpha: 128.0 };
    out.push(quad([65, 41, 14, 14], [x + 6.0, y + 6.0, w - 12.0, h - 12.0]));
    for (u, px, pw) in [(0, x, 16.0), (16, x + 16.0, w - 32.0), (40, x + w - 16.0, 16.0)] {
        out.push(quad([u, 0, 16, 16], [px, y, pw, 16.0]));
        out.push(quad([u, 16, 16, 8], [px, y + 16.0, pw, h - 32.0]));
        out.push(quad([u, 24, 16, 16], [px, y + h - 16.0, pw, 16.0]));
    }
    out.push(Quad { tex: Tex::Rules, src: [0.0; 4], dst: [x, y, w, h], rgb: WHITE, alpha: 128.0 });
}

/// The pause screen: the court dimmed, the top bar and the rules, (F2) the live board with the dialog left of it (the
/// rules tab under the dialog), all [`Place`]d; the pills' rects on the screen.
fn scene(v: &View, at: &Around) -> (Vec<Quad>, Vec<[f32; 4]>) {
    let mut out = Vec::new();
    let mut q = |tex, src: [i32; 4], dst: [f32; 4], rgb: [f32; 3], alpha: i32| {
        out.push(Quad { tex, src: src.map(|c| c as f32), dst, rgb, alpha: alpha as f32 })
    };
    let r = |x: i32, y: i32, w: i32, h: i32| [x, y, w, h].map(|c| c as f32);
    q(DIM, [0, 0, 8, 8], r(0, 0, 640, 448), WHITE, DIM_ALPHA);
    // the top bar and the rules: best-of sets in red, games in yellow
    q(BAR, [0, 0, 16, 64], r(0, 0, 352, 64), WHITE, 128);
    q(BAR, [16, 0, 32, 64], r(352, 0, 32, 64), WHITE, 128);
    q(BAR, [48, 0, 16, 64], r(384, 0, 256, 64), WHITE, 128);
    q(TEXT, [0, 0, 64, 32], r(24, 16, 64, 32), WHITE, 128);
    q(TEXT, [(v.sets_to_win * 2 - 2) * 16, 40, 16, 24], r(176, 24, 16, 24), [127.0, 59.0, 59.0], 128);
    q(TEXT, [0, 64, 56, 24], r(192, 24, 56, 24), WHITE, 128);
    q(TEXT, [(v.games - 1) * 16, 40, 16, 24], r(248, 24, 16, 24), [127.0, 97.0, 24.0], 128);
    q(TEXT, [0, 88, 104, 24], r(264, 24, 104, 24), WHITE, 128);
    // B293 (ours): the court in the rules' yellow over the umpire, right of the rules, the court on the bar above its rim, the umpire under it
    q(COURT, [0, 0, 256, 32], r(408, 3, 192, 24), [127.0, 110.0, 40.0], 128);
    q(UMPIRE, [0, 0, 256, 32], r(424, 34, 160, 20), WHITE, 128);

    let (dialog, box_h, pills) = dialog(v);
    let tab = v.tab;
    let cw = tab.map_or(BOX_W, |t| t[0].max(BOX_W));
    let ch = box_h + tab.map_or(0.0, |t| TAB_GAP + t[1]);
    let place = Place::new(&at.board, [cw, ch], at.vis);
    out.extend(at.board.iter().enumerate().map(|(i, &dst)| Quad { tex: Tex::Live(i), src: [0.0; 4], dst: place.map(dst), rgb: WHITE, alpha: 128.0 }));
    let dim = |out: &mut Vec<Quad>, alpha: i32| out.push(Quad { tex: DIM, src: [0.0, 0.0, 8.0, 8.0], dst: [0.0, 0.0, 640.0, 448.0], rgb: [0.0; 3], alpha: alpha as f32 });
    // M194: the "Match saved" box is `pause.rs`'s, over the screen dimmed
    if v.saved {
        dim(&mut out, 80);
        if let Some(t) = v.fade {
            dim(&mut out, (128 * t / 60).min(128));
        }
        return (out, Vec::new());
    }
    let dx = (cw - BOX_W) / 2.0;
    out.extend(dialog.into_iter().map(|q| Quad { dst: place.col([q.dst[0] + dx, q.dst[1], q.dst[2], q.dst[3]]), ..q }));
    if let Some(t) = tab {
        let mut quads = Vec::new();
        rules_tab_quads(&mut quads, [(cw - t[0]) / 2.0, box_h + TAB_GAP], t);
        out.extend(quads.into_iter().map(|q| Quad { dst: place.col(q.dst), ..q }));
    }
    if let Some(t) = v.fade {
        dim(&mut out, (128 * t / 60).min(128));
    }
    (out, pills.into_iter().map(|[x, y, w, h]| place.col([x + dx, y, w, h])).collect())
}

/// The match HUD's root nodes, hidden under the pause and result screens. Not B5b's composite node (aimed at its own
/// output camera): it draws the whole frame, and hiding it leaves the clear colour (the "blue screen").
type Hud = (With<Node>, Without<ChildOf>, Without<Root>, Without<super::match_stats::Root>, Without<Slot>, Without<UiTargetCamera>, Without<PauseRoot>, Without<SavedRoot>);

/// B657: the pause screen's layer while it fades to black: over the key legend's strip (30).
const FADE_Z: i32 = 31;

#[derive(Resource)]
struct Art(Vec<Handle<Image>>, Option<Arc<SoundBank>>);
#[derive(Component)]
struct Slot(usize);
#[derive(Component)]
struct Root;
/// F2: with the live board's pictures.
const POOL: usize = 520;

/// F2: the pills' rects on the screen (640×448) as last drawn, for clicks and taps.
#[derive(Resource, Default)]
struct Hits(Vec<[f32; 4]>);

/// The pause screen is open (online play sends it to the peers).
pub(super) fn paused(w: &World) -> bool {
    w.get_resource::<Menu>().is_some_and(|m| m.open)
}

/// A peer opened or closed the pause screen: here too (not while this one is quitting).
pub(super) fn set_paused(w: &mut World, open: bool) {
    let Some(mut m) = w.get_resource_mut::<Menu>() else { return };
    // M194: nor once it's saving or saved (the box's OK leaves the match)
    if m.open == open || m.quitting.is_some() || m.saving || m.saved.is_some() {
        return;
    }
    *m = if open { Menu::opened() } else { Menu::default() };
    if let Some(s) = w.get_resource::<Sound>() {
        s.hold(open);
        if open {
            s.hold_followed();
        }
    }
    let mut fixed = w.resource_mut::<Time<Fixed>>();
    if open {
        fixed.set_timestep(Duration::from_secs(1 << 20));
    } else {
        // M176: back to the match's own rate
        fixed.set_timestep_hz(crate::match_speed::MatchHz::now().hz());
        let over = fixed.overstep();
        fixed.discard_overstep(over);
    }
}

/// The pause screen, with the match (`main.rs` `add_match`).
pub fn plugin(app: &mut App) {
    app.init_resource::<Menu>()
        .insert_resource(super::front_end::FrontEnd { paused, set_paused, face: super::main_menu::face })
        .add_systems(PostStartup, setup.after(super::setup))
        .init_resource::<Hits>()
        .add_systems(Update, (step.after(super::read_input).before(PauseBoardBuild), pool, draw.after(PauseBoardBuild)).chain().before(super::main_menu::pause::Systems))
        .add_systems(Update, save_match.after(step).before(pool))
        .init_resource::<plate::Plates>()
        .add_systems(Update, plate::refresh.after(step).before(draw))
        .init_resource::<rules_tab::RulesTab>()
        .add_systems(Update, rules_tab::refresh.after(step).before(draw))
        // B179: the main menus' 3D hand (`cursor.ani`) points here and on the Settings screens over the match
        .add_systems(Startup, super::main_menu::cursor::setup)
        .add_systems(Update, super::main_menu::cursor::tick.after(draw))
        .add_systems(Last, super::main_menu::cursor::show);
    super::main_menu::pause::plugin(app);
    super::main_menu::legend::plugin(app);
    app.add_systems(Update, legend.after(step));
    app.add_systems(Update, calm.after(draw));
}

/// P137: the pause screen draws only while something moves; offline only (online play keeps the peers' messages
/// flowing at the full rate), never while it's closing into play or the menu.
fn calm(menu: Res<Menu>, net: Option<Res<super::netplay::Netplay>>) {
    if menu.open && menu.quitting.is_none() && net.is_none() {
        hst_base::calm::want();
    }
}

/// M14: the pause menu's legend (its Settings screens fill their own, `pause.rs`): `step`'s keys and buttons.
fn legend(menu: Res<Menu>, settings: Res<PauseSettings>, last: Res<super::main_menu::legend::Last>, mut out: ResMut<super::main_menu::legend::Legend>) {
    use super::main_menu::legend::{Act, item};
    if !menu.open || settings.open || menu.quitting.is_some() {
        return;
    }
    if menu.saved.is_some() {
        return out.one(*last, vec![item(Act::Ok, "OK", &["Enter", "J"])]);
    }
    let mut v = vec![item(Act::UpDown, "Move", &["W", "S"]), item(Act::Ok, "Choose", &["Enter", "J"])];
    if menu.ask.is_some() {
        v.extend([item(Act::Back, "Cancel", &["Backspace", "K"]), item(Act::Start, "Cancel", &["Esc"])]);
    } else {
        v.extend([item(Act::Back, "Resume", &["Backspace", "K"]), item(Act::Start, "Resume", &["Esc"])]);
    }
    out.one(*last, v);
}

fn setup(mut commands: Commands, args: Res<Args>, mut images: ResMut<Assets<Image>>, court_mod: Option<Res<crate::mods_court::MatchCourt>>, umpire_mod: Option<Res<crate::mods_umpire::MatchUmpire>>) {
    let mut iso = Iso::open(&args.iso).expect("open iso");
    let data = iso.read("AZUMA/INPANE/INPANE.XB0").expect("INPANE archive on disc");
    let arc = Archive::parse(&data).expect("xb archive");
    let mut art = TEXTURES
        .iter()
        .map(|name| {
            let e = arc
                .entries
                .iter()
                .find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with(name))
                .unwrap_or_else(|| panic!("{name} in INPANE"));
            super::menu_art::image(&mut images, &arc.read(e).expect("INPANE bytes"), name)
        })
        .collect::<Vec<_>>();
    for s in LABELS {
        art.push(super::main_menu::pause::label(&mut iso, &mut images, s));
    }
    art.push(super::menu_art::pill_layers(&mut images));
    let (mut court, mut umpire) = super::main_menu::pause::venue(&mut iso, args.stage.map_or(args.court, |s| s as usize), args.umpire);
    // B401: a court mod by its own name
    if let Some(c) = &court_mod {
        court = c.0.name.clone();
    }
    // B430: an umpire mod by its own name, never the disc umpire it stands in for
    if let Some(u) = &umpire_mod {
        umpire = super::main_menu::pause::umpire_caption(&u.0.name);
    }
    art.push(super::main_menu::pause::label(&mut iso, &mut images, &court));
    art.push(super::main_menu::pause::label(&mut iso, &mut images, &umpire));
    art.push(super::main_menu::pause::label(&mut iso, &mut images, SAVE_LABEL));
    for s in NOTES {
        art.push(super::word_label::label_w(&mut iso, &mut images, s, NOTE_W));
    }
    let bank = SoundBank::load(&mut iso, "SND/SE/SYS/SYS_SE00.XB", "data/sound/SE/sys/sys_se00.hd").map(Arc::new);
    commands.insert_resource(Art(art, bank));
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            GlobalZIndex(10),
            Root,
        ));
}

/// P39: the menu's quads while it's open (`pool`), none in play: UI layout walks hidden nodes too.
fn pool(mut commands: Commands, menu: Res<Menu>, root: Query<Entity, With<Root>>, slots: Query<Entity, With<Slot>>) {
    for r in &root {
        crate::ui_pool::keep(&mut commands, r, menu.open, slots, POOL, |i| (Slot(i), crate::ui_pool::quad(), super::widescreen::Bleed::default(), super::widescreen::BleedOutside));
    }
}

/// Reads Start/✕/up/down (and clicks or taps on the pills), steps the menu at 60 Hz and holds the match's fixed clock
/// while it is open; with its Settings screens up, they have the input.
/// Presses made in the menu never reach the match: an emote neither (B341: the touch pad's L1/R1 tab Settings).
/// `step` runs after `read_input`, so the frame that resumes the match drops them too.
fn spent(pads: &mut Pads) {
    for s in &mut pads.slots {
        s.shot = None;
        s.serve = false;
        s.emote = None;
    }
}

fn step(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    mut fixed: ResMut<Time<Fixed>>,
    g: Res<Game>,
    mut menu: ResMut<Menu>,
    mut pads: ResMut<Pads>,
    art: Option<Res<Art>>,
    sound: Option<Res<Sound>>,
    mut exit: MessageWriter<AppExit>,
    mut auto: Local<bool>,
    (mouse, touches, window, mut settings, mut saved_box): (Res<ButtonInput<MouseButton>>, Res<Touches>, Query<&Window, With<bevy::window::PrimaryWindow>>, ResMut<PauseSettings>, ResMut<SavedBox>),
    mut hovered: Local<Option<Vec2>>,
    net: Option<Res<super::netplay::Netplay>>,
    bind: Option<Res<super::controls::Bindings>>,
    (hits, mut board): (Res<Hits>, ResMut<PauseBoard>),
) {
    let any = |b: GamepadButton| gamepads.iter().filter(|g| super::controls::readable(g)).any(|g| g.just_pressed(b));
    // pause: Esc / Start, or what Controls binds it to
    let bind = bind.as_deref().cloned().unwrap_or_default();
    let pause = super::controls::Action::Pause;
    let start = bind.key_just_pressed(&keys, pause) || gamepads.iter().filter(|g| super::controls::readable(g)).any(|g| bind.pad_just_pressed(g, pause));
    let confirm = keys.any_just_pressed([KeyCode::Enter, KeyCode::Space, KeyCode::KeyJ]) || any(GamepadButton::South);
    let cancel = keys.any_just_pressed([KeyCode::Backspace, KeyCode::KeyK]) || any(GamepadButton::East);
    // the stick as the original's menus read it: its byte's held direction (|48| of 127, past the driver's deadzone)
    let stick = |g: &Gamepad| -hst_sim::player::pad_held(super::stick_byte(-g.left_stick().y)) as f32 + g.dpad().y;
    let up = keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) || gamepads.iter().filter(|g| super::controls::readable(g)).any(|g| stick(g) > 0.5);
    let down = keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) || gamepads.iter().filter(|g| super::controls::readable(g)).any(|g| stick(g) < -0.5);
    let play = |key: usize| {
        if let (Some(s), Some(Art(_, Some(b)))) = (&sound, art.as_deref()) {
            s.play_menu(b, key as u8);
        }
    };
    let timed = !*auto
        && std::env::var("HST_PAUSE").ok().and_then(|s| s.parse::<f32>().ok()).is_some_and(|t| time.elapsed_secs() >= t);
    if !menu.open {
        board.open = false;
        // the result screen has no pause
        if (start || timed) && g.match_stats.result.is_none() {
            *auto = true;
            *menu = Menu::opened();
            // B77: the whistle and the like stop where they are until the match goes on
            if let Some(s) = &sound {
                s.hold(true);
                s.hold_followed();
            }
            play(2);
            match std::env::var("HST_PAUSE_SETTINGS").as_deref() {
                Ok("assign") if timed => settings.open_seats(),
                Ok(_) if timed => settings.open(),
                _ => {}
            }
            if timed && std::env::var_os("HST_PAUSE_ASK").is_some() {
                // as chosen from "Quit", the last row
                let local = net.as_ref().is_none_or(|np| !np.lock.mine.is_empty());
                (menu.sel, menu.ask) = (options(local).len() - 1, Some(0));
            }
            if timed && std::env::var_os("HST_PAUSE_SAVE").is_some() {
                menu.saving = true;
            }
            // ponytail: the fixed clock runs on but never reaches a step; the overstep is thrown away on closing
            fixed.set_timestep(Duration::from_secs(1 << 20));
        }
        return;
    }
    // F2: the live board beside the dialog the whole time it's up, not under its Settings screens
    board.open = !settings.open;
    if settings.open {
        spent(&mut pads);
        return;
    }
    // M124: online too (concept: with any local player), its own seats only
    menu.local = net.as_ref().is_none_or(|np| !np.lock.mine.is_empty());
    menu.save = can_save(net.as_deref().map(|np| np.me.as_str()));
    let pill = |p: Vec2| hits.0.iter().position(|&[x, y, w, h]| (x..x + w).contains(&p.x) && (y..y + h).contains(&p.y));
    // B221: the mouse over a pill moves the hand there; a right click is Back (a click or tap off the pills does
    // nothing: a phone's swipe for its control centre unpaused the match)
    let (press, right) = super::main_menu::pause::pointer_press(&window, &mouse, &touches);
    let at = super::main_menu::pause::cursor_point(&window);
    // M194: the "Match saved" box takes only its OK (a click or tap on it, `pause.rs`), up until the fade
    let boxed = menu.saved.is_some() || menu.saving;
    if boxed {
        menu.confirm |= std::mem::take(&mut saved_box.tapped);
    }
    if menu.saved.is_none() || menu.quitting.is_some() {
        saved_box.lines.clear();
    }
    if !boxed
        && at != *hovered
        && let Some(k) = at.and_then(pill)
    {
        match menu.ask {
            Some(a) if a != k => (menu.ask, menu.hand) = (Some(k), Hand::default()),
            None if k != menu.sel => (menu.sel, menu.hand) = (k, Hand::default()),
            _ => {}
        }
    }
    *hovered = at;
    menu.cancel |= !boxed && right;
    if let Some(k) = press.filter(|_| !boxed).and_then(pill) {
        match menu.ask {
            Some(a) => {
                if a != k {
                    (menu.ask, menu.hand) = (Some(k), Hand::default());
                }
                menu.confirm = true;
            }
            None => {
                if k != menu.sel {
                    (menu.sel, menu.hand) = (k, Hand::default());
                }
                menu.confirm = true;
            }
        }
    }
    menu.cancel |= cancel;
    menu.start |= start;
    menu.confirm |= confirm;
    menu.acc += time.delta_secs() * 60.0;
    while menu.acc >= 1.0 && menu.open {
        menu.acc -= 1.0;
        if let Some(k) = menu.tick(up, down) {
            play(k);
        }
    }
    spent(&mut pads);
    if std::mem::take(&mut menu.settings) {
        settings.open();
    }
    if std::mem::take(&mut menu.seats) {
        settings.open_seats();
    }
    if menu.quitting.is_some_and(|t| t >= 60) {
        exit.write(AppExit::Success);
    }
    if !menu.open {
        board.open = false;
        if let Some(s) = &sound {
            s.hold(false);
        }
        // M176: back to the match's own rate
        fixed.set_timestep_hz(crate::match_speed::MatchHz::now().hz());
        let over = fixed.overstep();
        fixed.discard_overstep(over);
    }
}

/// M194: whether this machine's pause has "Save and Quit": a local match (`host` none) or the online host's (the
/// browser too since M198: its saves are in the page's storage).
fn can_save(host: Option<&str>) -> bool {
    host.is_none_or(|me| me == "host")
}

/// M194: the score where a save picks up, from the left team's side: `who`, each set's games (`past` the sets played,
/// then the set in play) and the point, as "Carol 6-4 4-2, 30-15.".
fn score_line(who: &str, past: &[[i32; 2]], s: &super::save_game::Summary) -> String {
    let games: Vec<String> = past.iter().chain([&s.games]).map(|g| format!("{}-{}", g[0], g[1])).collect();
    let call = |p: i32| ["0", "15", "30", "40"][p.clamp(0, 3) as usize];
    let [a, b] = s.points;
    let point = if s.tiebreak {
        format!("{a}-{b}")
    } else if s.deuce {
        "40-40".into()
    } else if s.advantage {
        (if a > b { "Ad-40" } else { "40-Ad" }).into()
    } else {
        format!("{}-{}", call(a), call(b))
    };
    format!("{who} {}, {point}.", games.join(" "))
}

/// M194: "Save and Quit" chosen: the match saved as its point started (`save_game`), then the "Match saved" box.
fn save_match(w: &mut World) {
    use super::save_game::{self, Mode, Saves};
    if !w.get_resource::<Menu>().is_some_and(|m| m.saving) {
        return;
    }
    let mode = if w.contains_resource::<super::netplay::Netplay>() { Mode::Online } else { Mode::Local };
    // M198: the flags the match was started with, so Continue can start it again
    let flags = w.get_resource::<save_game::StartFlags>().map_or_else(Vec::new, |f| f.0.clone());
    let save = save_game::capture(w, mode, flags).ok_or_else(|| "The match has no point to pick up from yet.".to_string());
    let save = save.and_then(|s| Saves::user().write(&s).map(|_| s));
    let lines = match &save {
        Ok(s) => {
            // the left team by character (a mod by its own name), each set before the one in play
            let n = w.resource::<Game>().players.len();
            let args = w.resource::<Args>();
            let mods = w.get_resource::<crate::mods::MatchMod>();
            let who: Vec<String> = (0..n).step_by(2).map(|p| plate::character(args, mods, p)).collect();
            let sc = &w.resource::<Game>().score;
            let played = (s.head.summary.sets[0] + s.head.summary.sets[1]).clamp(0, sc.set_games[0].len() as i32) as usize;
            let past: Vec<[i32; 2]> = (0..played).map(|c| [sc.set_games[0][c], sc.set_games[1][c]]).collect();
            let score = score_line(&who.join(" & "), &past, &s.head.summary);
            // M199: an online one is hosted again (its lobby offers it to the same players)
            let how = if mode == Mode::Online { "Host it again from Continue;" } else { "Continue it from the main menu;" };
            vec!["Match saved".into(), score, how.into(), "it picks up at the start of this point.".into()]
        }
        Err(e) => {
            warn!("pause: the match wasn't saved: {e}");
            vec!["Not saved".into(), "The match couldn't be saved:".into(), e.clone()]
        }
    };
    let mut m = w.resource_mut::<Menu>();
    (m.saving, m.saved, m.hand) = (false, Some(save.is_ok()), Hand::default());
    let mut b = w.resource_mut::<SavedBox>();
    (b.lines, b.tapped) = (lines, false);
}

fn draw(
    g: Res<Game>,
    pads: Res<Pads>,
    menu: Res<Menu>,
    art: Option<Res<Art>>,
    panel_art: Option<Res<panel::Art>>,
    colours: Option<Res<Colours>>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut q: Query<(&Slot, &mut ImageNode, &mut Node, &mut Visibility, &mut super::widescreen::Bleed)>,
    mut others: Query<(Entity, &mut Visibility), Hud>,
    mut hidden: Local<Vec<(Entity, Visibility)>>,
    cursor: Option<Res<super::main_menu::cursor::Cursor>>,
    mut root_z: Query<&mut GlobalZIndex, With<Root>>,
    plates: Res<plate::Plates>,
    (rules, board, mut hits): (Res<rules_tab::RulesTab>, Res<PauseBoard>, ResMut<Hits>),
) {
    let (Some(art), Some(panel_art), Some(colours)) = (art, panel_art, colours) else {
        return;
    };
    let n = g.players.len();
    let s = &g.score;
    // the pause and result screens replace the rest of the HUD (panel, pop-ups, the port's text)
    let cover = menu.open || g.match_stats.result.is_some();
    if cover && hidden.is_empty() {
        for (e, mut vis) in &mut others {
            hidden.push((e, *vis));
            vis.set_if_neq(Visibility::Hidden);
        }
    } else if !cover {
        for (e, vis) in hidden.drain(..) {
            if let Ok((_, mut v)) = others.get_mut(e) {
                *v = vis;
            }
        }
    }
    // B657: the fade to black goes over the key legend's strip too (it's over the pause screen until then)
    for mut z in &mut root_z {
        z.set_if_neq(GlobalZIndex(if menu.quitting.is_some() { FADE_Z } else { 10 }));
    }
    let share = super::widescreen::window_share(&window);
    let aspect = super::widescreen::window_aspect(&window);
    let vis = [640.0 / hst_base::widescreen::share(aspect), 448.0 / hst_base::widescreen::vshare(aspect)];
    let quads = if menu.open {
        let view = View {
            players: n,
            slots: std::array::from_fn(|i| if i < n { pads.slot_of(i, n).unwrap_or(4) } else { 4 }),
            pill: colours.0,
            sets_to_win: g.rules.sets.clamp(1, 3),
            games: g.rules.games.clamp(1, 6),
            points: s.points,
            deuce: s.deuce,
            advantage: s.advantage,
            tiebreak: s.tiebreak,
            set: s.set,
            set_games: s.set_games,
            sel: menu.sel,
            // B179: the 3D hand wags in place (`cursor.ani`) as on the main menus; `hsm_yubi` swings
            hand_x: if cursor.is_some() { 0.0 } else { menu.hand.x },
            fade: menu.quitting,
            ask: menu.ask,
            local: menu.local,
            save: menu.save,
            saved: menu.saved.is_some(),
            words: std::array::from_fn(|p| if p < n { plates.widths(p) } else { None }),
            tab: rules.size(),
        };
        let (quads, pills) = scene(&view, &Around { board: board.quads.iter().map(|q| q.dst).collect(), vis });
        hits.0 = pills;
        quads
    } else {
        hits.0.clear();
        Vec::new()
    };
    for (Slot(i), mut img, mut node, mut vis, mut bleed) in &mut q {
        let Some(quad) = quads.get(*i) else {
            vis.set_if_neq(Visibility::Hidden);
            continue;
        };
        let three_d = cursor.as_ref().filter(|_| quad.tex == HAND);
        img.image = match (three_d, quad.tex) {
            (Some(c), _) => c.image.clone(),
            (_, Tex::Own(k)) => art.0[k].clone(),
            (_, Tex::Slot) => panel_art.0[1].clone(),
            (_, Tex::Face(p)) => panel_art.0[panel::FACES + p].clone(),
            (_, Tex::Name(p)) => plates.name[p].as_ref().map_or_else(Handle::default, |w| w.0.clone()),
            (_, Tex::Chara(p)) => plates.chara[p].as_ref().map_or_else(Handle::default, |w| w.0.clone()),
            (_, Tex::Rules) => rules.tab.as_ref().map_or_else(Handle::default, |t| t.0.clone()),
            (_, Tex::Live(k)) => match board.quads.get(k) {
                Some(b) => b.image.clone(),
                None => {
                    vis.set_if_neq(Visibility::Hidden);
                    continue;
                }
            },
            (None, Tex::Hand) => {
                vis.set_if_neq(Visibility::Hidden);
                continue;
            }
        };
        let [u, v, w, h] = quad.src;
        // B160: a face is its whole image (INPANE's 64×64, or a baked one's larger square)
        img.rect = (!matches!(quad.tex, Tex::Face(_) | Tex::Name(_) | Tex::Chara(_) | Tex::Rules) && three_d.is_none()).then(|| Rect::new(u, v, u + w, v + h));
        let [r, g, b] = quad.rgb.map(|c| c / 128.0);
        img.color = Color::srgba(r, g, b, quad.alpha / 128.0);
        // F2: the board's pictures as it set them
        if let Tex::Live(k) = quad.tex
            && let Some(b) = board.quads.get(k)
        {
            (img.rect, img.color) = (b.rect, b.color);
        }
        let [mut x, mut y, mut w, mut h] = quad.dst;
        if three_d.is_some() {
            // where the main menus' rows put it (row x − 60, y + 3), not `hsm_yubi`'s (x − 48, y + 8); F2: scaled with
            // the dialog
            let k = w / 64.0;
            (x, y) = (x - 12.0 * k, y - 5.0 * k);
            // the hand's image covers `PAD` times its quad, around the same centre (B146)
            let pad = super::main_menu::cursor::PAD;
            (x, y, w, h) = (x - w * (pad - 1.0) / 2.0, y - h * (pad - 1.0) / 2.0, w * pad, h * pad);
        }
        // widescreen: the dimmer and the top bar span the window, "Paused" and the rules keep to its left
        // B293: the court and umpire pinned to the window's right, under the bar's right end
        let anchor = if [DIM, BAR, TEXT].contains(&quad.tex) {
            Anchor::Span
        } else if [COURT, UMPIRE].contains(&quad.tex) {
            Anchor::Sides
        } else {
            Anchor::Centre
        };
        let (left, width) = super::widescreen::place(x, w, share, anchor);
        let (top, height) = super::widescreen::vplace(y, h, aspect, anchor);
        crate::play::widescreen::put(&mut node, left, top, width, height);
        vis.set_if_neq(Visibility::Inherited);
        // B630: the dimmer runs on over the whole window (the safe area's strips, a fixed Aspect Ratio's bars), the
        // top bar out to its sides; over the key legend's strip, under the loading screen
        // B657: only past the view (`BleedOutside`): the dimmer and the fade aren't drawn twice over it, so the strips
        // darken with the rest
        bleed.set_if_neq(super::widescreen::Bleed { order: (1 << 20) + 1 + *i as i32, on: [DIM, BAR].contains(&quad.tex) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> View {
        View {
            players: 4,
            slots: [0, 4, 4, 4],
            pill: [[98, 46, 61], [33, 74, 97], [105, 70, 25], [63, 78, 25]],
            sets_to_win: 1,
            games: 4,
            points: [0, 0],
            deuce: false,
            advantage: false,
            tiebreak: false,
            set: 0,
            set_games: [[0; 5]; 2],
            sel: 0,
            hand_x: 0.0,
            fade: None,
            ask: None,
            local: false,
            save: false,
            saved: false,
            words: [None; 4],
            tab: None,
        }
    }

    /// B177: the textures' indices follow `setup`'s order: the disc sheets, the labels, the pill sheet, (F2) the
    /// confirm's notes.
    #[test]
    fn art_order() {
        let n = TEXTURES.len();
        assert_eq!([LABEL, RESUME, QUIT_ROW, ASK, DROP_ROW, CANCEL_ROW, SEATS, PILL, COURT, UMPIRE, SAVE_ROW], std::array::from_fn(|i| Tex::Own(n + i)));
        assert_eq!([SAVE, DROP, BACK].map(note_tex), std::array::from_fn(|i| Tex::Own(n + 11 + i)));
    }

    /// A live board's pictures as the end screen's widget lays them out: the score board over the stats board.
    fn board() -> Vec<[f32; 4]> {
        vec![[96.0, 0.0, 448.0, 140.0], [80.0, 148.0, 480.0, 200.0], [176.0, 356.0, 288.0, 24.0]]
    }
    /// 16:9, 4:3 and 21:9 (wider and taller than the 640×448 screen's 10:7 at the same height).
    const SCREENS: [[f32; 2]; 3] = [[796.4, 448.0], [640.0, 480.0], [1045.3, 448.0]];
    fn around(vis: [f32; 2]) -> Around {
        Around { board: board(), vis }
    }

    /// B341: a press latched while paused, an emote included, is spent before the match resumes.
    #[test]
    fn paused_presses_are_spent() {
        let mut pads = Pads::default();
        pads.slots[1] = super::super::SlotPad { shot: Some(1), serve: true, emote: Some(3), ..default() };
        spent(&mut pads);
        let s = &pads.slots[1];
        assert_eq!((s.shot, s.serve, s.emote), (None, false, None));
    }

    /// P61: the match crate reaches this screen only through `front_end`; a bare world is never paused.
    #[test]
    fn the_match_pauses_through_the_front_end() {
        use super::super::front_end;
        let mut w = World::new();
        front_end::set_paused(&mut w, true);
        assert!(!front_end::paused(&w));
        w.insert_resource(Menu::default());
        w.insert_resource(Time::<Fixed>::from_hz(60.0));
        w.insert_resource(front_end::FrontEnd { paused, set_paused, face: super::super::main_menu::face });
        front_end::set_paused(&mut w, true);
        assert!(front_end::paused(&w) && w.resource::<Menu>().open);
        // the match's clock stops while it's up
        assert_eq!(w.resource::<Time<Fixed>>().timestep(), Duration::from_secs(1 << 20));
        front_end::set_paused(&mut w, false);
        assert!(!front_end::paused(&w));
    }

    /// Slot 3's pause (doubles, one set to 4 games, 0-0): the rules digits on the top bar, the court and umpire; the old
    /// score board (the Continue cards') keeps its points, games and faces.
    #[test]
    fn slot3_pause() {
        let (qs, _) = scene(&view(), &around(SCREENS[0]));
        let at = |qs: &[Quad], tex: Tex, x: f32, y: f32| qs.iter().find(|q| q.tex == tex && q.dst[0] == x && q.dst[1] == y).copied();
        assert_eq!(at(&qs, TEXT, 176.0, 24.0).unwrap().src, [0.0, 40.0, 16.0, 24.0]); // "1"
        assert_eq!(at(&qs, TEXT, 248.0, 24.0).unwrap().src, [48.0, 40.0, 16.0, 24.0]); // "4"
        // B293: the court on the bar's right end, the umpire under its rim
        assert_eq!(qs.iter().find(|q| q.tex == COURT).unwrap().dst, [408.0, 3.0, 192.0, 24.0]);
        assert_eq!(qs.iter().find(|q| q.tex == UMPIRE).unwrap().dst, [424.0, 34.0, 160.0, 20.0]);
        // F2: none of the old board on the pause screen, the live one in its place
        assert!(!qs.iter().any(|q| [POINTS, GAMES_NOW, Tex::Face(0)].contains(&q.tex)));
        assert_eq!(qs.iter().filter(|q| matches!(q.tex, Tex::Live(_))).count(), board().len());
        let qs = score_board(&view());
        // (ours) doubles on an 8-row board, so each face's pill clears the face under it
        assert_eq!(at(&qs, POINTS, 184.0, 336.0).unwrap().src, [0.0, 0.0, 64.0, 48.0]);
        assert!(at(&qs, POINTS, 390.0, 336.0).is_some());
        assert!(at(&qs, GAMES_NOW, 272.0, 344.0).is_some() && at(&qs, GAMES_NOW, 308.0, 344.0).is_some());
        assert!(at(&qs, Tex::Face(2), 102.0, 367.0).is_some() && at(&qs, Tex::Face(1), 498.0, 303.0).is_some());
    }

    /// B671: with the words set, each plate's name takes the slot tag's place (none drawn) and its character's pill sits
    /// under the face, both inside the plate however long (and shrunk to fit); the left team's name against the centre.
    #[test]
    fn plates_carry_names() {
        for players in [2, 4] {
            for w in [[30.0, 40.0], [400.0, 300.0]] {
                let qs = score_board(&View { players, words: [Some(w); 4], ..view() });
                assert!(!qs.iter().any(|q| q.tex == Tex::Slot));
                for p in 0..players {
                    let (left, right) = if p & 1 == 0 { (96.0, 184.0) } else { (456.0, 544.0) };
                    for tex in [Tex::Name(p), Tex::Chara(p)] {
                        let [x, _, w, h] = qs.iter().find(|q| q.tex == tex).unwrap().dst;
                        assert!(x >= left && x + w <= right && h > 0.0, "{tex:?} {x} {w}");
                    }
                }
                let name = |p| qs.iter().find(|q| q.tex == Tex::Name(p)).unwrap().dst;
                assert_eq!(name(0)[0] + name(0)[2], 183.0);
                assert_eq!(name(1)[0], 457.0);
            }
        }
        assert_eq!((plate::who(4, Some("Ryan")), plate::who(0, Some("Ryan")), plate::who(1, None)), ("COM".into(), "Ryan".into(), "2P".into()));
    }

    /// F2: the dialog left of the board, their middles level, the hand's room inside the screen; all of it at one
    /// scale, no larger than its own size, between the top bar and the legend's strip, at 16:9, 4:3 and 21:9, with
    /// or without the rules tab (under the dialog, clear of it) or the confirm; the pills' rects are the pills drawn.
    #[test]
    fn fits_any_screen() {
        for vis in SCREENS {
            for tab in [None, Some([300.0, 21.0]), Some([420.0, 81.0])] {
                for (ask, save, local) in [(None, false, false), (None, true, true), (Some(0), true, false), (Some(1), false, true)] {
                    let v = View { ask, save, local, tab, ..view() };
                    let (qs, pills) = scene(&v, &around(vis));
                    let (l, t) = (320.0 - vis[0] / 2.0, 224.0 - vis[1] / 2.0);
                    let drawn: Vec<_> = qs.iter().filter(|q| !matches!(q.tex, DIM | BAR | TEXT | COURT | UMPIRE)).map(|q| q.dst).collect();
                    for [x, y, w, h] in &drawn {
                        assert!(*x >= l + 7.9 && x + w <= l + vis[0] - 7.9, "{vis:?} {x} {w}");
                        assert!(*y >= t + 64.0 && y + h <= t + vis[1] - 15.9, "{vis:?} {y} {h}");
                    }
                    let live: Vec<_> = qs.iter().filter(|q| matches!(q.tex, Tex::Live(_))).map(|q| q.dst).collect();
                    let k = live[0][2] / board()[0][2];
                    assert!(k <= 1.0 && live.iter().zip(board()).all(|(d, b)| (d[2] / b[2] - k).abs() < 1e-4 && (d[3] / b[3] - k).abs() < 1e-4));
                    let pill = qs.iter().filter(|q| q.tex == PILL).map(|q| q.dst).collect::<Vec<_>>();
                    let right = pill.iter().map(|d| d[0] + d[2]).fold(0.0, f32::max);
                    // the box's right edge (the pills' sheet draws it too) the gap from the board's left
                    let gap = live[1][0].min(live[0][0]) - right;
                    assert!(if tab.is_some_and(|t| t[0] > BOX_W) { gap >= GAP * k - 0.01 } else { (gap - GAP * k).abs() < 0.01 }, "{vis:?} {gap}");
                    assert_eq!(pills.len(), if ask.is_some() { asks(save).len() } else { options(local).len() });
                    let row = qs.iter().find(|q| q.tex == HAND).unwrap().dst;
                    assert!(pills.iter().any(|p| (p[1] + 8.0 * k - row[1]).abs() < 0.01 && (p[3] - PILL_H * k).abs() < 0.01));
                    if let Some(tab) = qs.iter().find(|q| q.tex == Tex::Rules).map(|q| q.dst) {
                        let bottom = pill.iter().map(|d| d[1] + d[3]).fold(0.0, f32::max);
                        assert!(tab[1] >= bottom + TAB_GAP * k - 0.01);
                    }
                }
            }
        }
        // with room to spare the board is drawn at its own size, never larger
        let (qs, _) = scene(&view(), &around([1200.0, 800.0]));
        assert_eq!(qs.iter().find(|q| q.tex == Tex::Live(0)).unwrap().dst[2], 448.0);
        // no board yet: the dialog (and the hand's room) on the middle
        let (_, pills) = scene(&view(), &Around { board: Vec::new(), vis: SCREENS[0] });
        assert!((pills[0][0] + pills[0][2] / 2.0 - 320.0).abs() <= HAND_ROOM / 2.0);
        assert!(!scene(&view(), &around(SCREENS[0])).0.iter().any(|q| q.tex == Tex::Rules));
    }

    /// B254: a local match's dialog has four pills, "Controllers" third, "Quit" last; ✕ on it asks for the controller
    /// assignment.
    #[test]
    fn controllers_row() {
        let (qs, _) = scene(&View { local: true, players: 2, ..view() }, &around([640.0, 1000.0]));
        let y = |t: Tex| qs.iter().find(|q| q.tex == t).unwrap().dst[1];
        assert!(y(RESUME) < y(LABEL) && y(LABEL) < y(SEATS) && y(SEATS) < y(QUIT_ROW));
        assert!((y(QUIT_ROW) - y(SEATS) - (y(SEATS) - y(LABEL))).abs() < 0.01);
        let mut m = Menu { local: true, ..Menu::opened() };
        for _ in 0..5 {
            m.tick(false, false);
        }
        m.tick(false, true);
        m.tick(false, false);
        m.tick(false, true);
        assert_eq!(m.sel, 2);
        m.confirm = true;
        assert_eq!(m.tick(false, false), Some(2));
        assert!(m.open && m.seats && !m.settings);
    }

    /// Advantage: "A" for the leader, the trailing 40 at half alpha; tiebreak deuce uses the red D.
    #[test]
    fn points() {
        let qs = score_board(&View { points: [4, 3], advantage: true, ..view() });
        let pts: Vec<_> = qs.iter().filter(|q| q.tex == POINTS).map(|q| (q.src, q.alpha)).collect();
        assert_eq!(pts, [([64.0, 96.0, 64.0, 48.0], 128.0), ([64.0, 48.0, 64.0, 48.0], 64.0)]);
        let qs = score_board(&View { points: [6, 6], deuce: true, tiebreak: true, ..view() });
        assert!(qs.iter().filter(|q| q.tex == TIEBREAK).all(|q| q.src == [0.0, 192.0, 64.0, 48.0]));
    }

    /// The HUD filter takes the panel's root but neither the frame composite nor the pause screen's own root.
    #[test]
    fn hud_roots() {
        let mut w = World::new();
        let cam = w.spawn_empty().id();
        let panel = w.spawn(Node::default()).id();
        w.spawn((Node::default(), UiTargetCamera(cam)));
        w.spawn((Node::default(), Root));
        let found: Vec<Entity> = w.query_filtered::<Entity, Hud>().iter(&w).collect();
        assert_eq!(found, [panel]);
    }

    /// Down moves and wraps, input waits 5 ticks, the hand swings to −16 and back, ✕ on the second option asks for the
    /// settings, on the third (F2: "Quit") asks whether to quit.
    #[test]
    fn input() {
        let mut m = Menu::opened();
        for _ in 0..5 {
            assert_eq!(m.tick(false, true), None);
        }
        assert_eq!(m.tick(false, true), Some(0));
        assert_eq!(m.sel, 1);
        assert_eq!(m.tick(false, true), None); // held: no repeat until 32
        for _ in 0..29 {
            m.tick(false, true);
        }
        assert_eq!(m.tick(false, true), Some(0)); // 32nd tick held
        assert_eq!(m.sel, 2);
        let mut h = Hand::default();
        let xs: Vec<f32> = (0..60).map(|_| { h.step(); h.x }).collect();
        // out to −16 by tick 20, held 11 ticks, eased back to 0 at tick 58 and out again
        assert!(xs[19] > -16.0 && xs[20] == -16.0 && xs[30] == -16.0 && xs[31] > -16.0);
        assert!(xs[56] < 0.0 && xs[57] == 0.0 && xs[58] < 0.0);
        m.tick(false, false);
        m.tick(true, false);
        assert_eq!(m.sel, 1);
        m.confirm = true;
        assert_eq!(m.tick(false, false), Some(2));
        assert!(m.open && m.settings && m.quitting.is_none());
        m.sel = 2;
        m.confirm = true;
        assert_eq!(m.tick(false, false), Some(2));
        assert_eq!((m.ask, m.quitting), (Some(0), None));
        // no saving here: "Quit without Saving" first
        m.confirm = true;
        m.tick(false, false);
        assert_eq!(m.quitting, Some(0));
        m.start = true;
        m.tick(false, false);
        assert!(m.open && m.quitting == Some(1));
        // B340: ○ resumes the match like Start, even before input is read
        let mut m = Menu::opened();
        m.cancel = true;
        assert_eq!(m.tick(false, false), Some(1));
        assert!(!m.open);
        let mut m = Menu::opened();
        m.start = true;
        assert_eq!(m.tick(false, false), Some(2));
        assert!(!m.open);
    }

    /// F2: "Quit" asks "Quit the match?" every time: Save and Quit saves (what it did on the pause menu), Quit without
    /// Saving fades out to the menu, Cancel (chosen, ○ or Start) goes back to the pause menu on "Quit"; up and down wrap.
    /// Where this machine can't save, only the last two. The board and the top bar stay; the title, the pills, the
    /// chosen one's note under them, the hand by it.
    #[test]
    fn quit_asks() {
        let asked = |save: bool| {
            let mut m = Menu { save, sel: 2, ..Menu::opened() };
            for _ in 0..5 {
                m.tick(false, false);
            }
            m.confirm = true;
            assert_eq!(m.tick(false, false), Some(2));
            assert_eq!((m.ask, m.quitting, m.saving), (Some(0), None, false));
            m
        };
        let choose = |m: &mut Menu, downs: usize| {
            for _ in 0..downs {
                m.tick(false, true);
                m.tick(false, false);
            }
            m.confirm = true;
            m.tick(false, false)
        };
        let mut m = asked(true);
        choose(&mut m, 0);
        assert!(m.saving && m.ask.is_none() && m.quitting.is_none());
        let mut m = asked(true);
        choose(&mut m, 1);
        assert_eq!(m.quitting, Some(0));
        let mut m = asked(true);
        assert_eq!(choose(&mut m, 2), Some(2));
        assert!(m.open && m.ask.is_none() && m.quitting.is_none() && !m.saving && m.sel == 2);
        // ○ and Start are Cancel; up from the first wraps to Cancel
        for back in [true, false] {
            let mut m = asked(true);
            (m.cancel, m.start) = (back, !back);
            assert_eq!(m.tick(false, false), Some(1));
            assert!(m.open && m.ask.is_none() && m.quitting.is_none() && m.sel == 2);
        }
        let mut m = asked(true);
        m.tick(true, false);
        assert_eq!(m.ask, Some(2));
        // no saving here: Quit without Saving first, then Cancel
        let mut m = asked(false);
        choose(&mut m, 0);
        assert_eq!(m.quitting, Some(0));
        let mut m = asked(false);
        choose(&mut m, 1);
        assert!(m.ask.is_none() && m.quitting.is_none() && !m.saving);

        let (qs, pills) = scene(&View { ask: Some(1), save: true, players: 2, ..view() }, &around([640.0, 1000.0]));
        assert_eq!(qs.iter().filter(|q| matches!(q.tex, Tex::Live(_))).count(), board().len());
        assert!(!qs.iter().any(|q| [RESUME, QUIT_ROW, LABEL, SEATS].contains(&q.tex)));
        let at = |t: Tex| qs.iter().find(|q| q.tex == t).unwrap();
        assert!(at(ASK).dst[1] + at(ASK).dst[3] < pills[0][1]);
        assert_eq!([SAVE_ROW, DROP_ROW, CANCEL_ROW].map(|t| at(t).alpha), [64.0, 128.0, 64.0]);
        assert!(at(SAVE_ROW).dst[1] < at(DROP_ROW).dst[1] && at(DROP_ROW).dst[1] < at(CANCEL_ROW).dst[1]);
        assert!(qs.iter().any(|q| q.tex == note_tex(DROP) && q.dst[1] > pills[2][1] + pills[2][3]));
        assert!(!qs.iter().any(|q| [note_tex(SAVE), note_tex(BACK)].contains(&q.tex)));
        assert!((at(HAND).dst[1] - pills[1][1] - 8.0 * pills[1][3] / PILL_H).abs() < 0.01);
        // no extra dim over the board
        assert_eq!(qs.iter().filter(|q| q.tex == DIM).count(), 1);
    }

    /// M194: ✕ on "Save and Quit" saves, nothing moves while it does, and the box's OK fades out once saved or goes
    /// back to the dialog when it wasn't; the screen dimmed under the box, no dialog.
    #[test]
    fn save_and_quit() {
        let chosen = || {
            let mut m = Menu { local: true, save: true, sel: 3, ask: Some(0), ..Menu::opened() };
            m.confirm = true;
            assert_eq!(m.tick(false, false), Some(2));
            assert!(m.saving && m.open && m.quitting.is_none() && m.ask.is_none());
            assert_eq!(m.tick(false, true), None);
            m
        };
        let mut m = chosen();
        (m.saving, m.saved) = (false, Some(true));
        m.tick(false, true);
        assert_eq!(m.sel, 3);
        m.confirm = true;
        assert_eq!(m.tick(false, false), Some(2));
        assert_eq!(m.quitting, Some(0));
        let (qs, pills) = scene(&View { saved: true, fade: Some(30), ..view() }, &around(SCREENS[0]));
        assert!(pills.is_empty() && !qs.iter().any(|q| [PILL, HAND, RESUME, QUIT_ROW].contains(&q.tex)));
        assert_eq!(qs.iter().filter(|q| q.tex == DIM && q.rgb == [0.0; 3]).map(|q| q.alpha).collect::<Vec<_>>(), [80.0, 64.0]);
        let mut m = chosen();
        (m.saving, m.saved) = (false, Some(false));
        m.cancel = true;
        assert_eq!(m.tick(false, false), Some(1));
        assert!(m.open && m.saved.is_none() && m.quitting.is_none());
        // online only the host saves; never the browser
        assert_eq!((can_save(None), can_save(Some("host")), can_save(Some("g"))), (!cfg!(target_arch = "wasm32"), !cfg!(target_arch = "wasm32"), false));
    }

    /// M194: the box's score from the left team's side, each set then the point.
    #[test]
    fn saved_score_line() {
        use super::super::save_game::Summary;
        let s = Summary { games: [4, 2], points: [2, 1], sets: [1, 0], ..default() };
        assert_eq!(score_line("Carol", &[[6, 4]], &s), "Carol 6-4 4-2, 30-15.");
        let s = Summary { points: [3, 3], deuce: true, ..default() };
        assert_eq!(score_line("Carol & Will", &[], &s), "Carol & Will 0-0, 40-40.");
        assert_eq!(score_line("A", &[], &Summary { points: [3, 4], advantage: true, ..default() }), "A 0-0, 40-Ad.");
        assert_eq!(score_line("A", &[], &Summary { games: [6, 6], points: [5, 3], tiebreak: true, ..default() }), "A 6-6, 5-3.");
    }
}
