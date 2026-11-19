//! M195 (concept, *Main menu and Continue*; ours, menus aren't 1:1): saved matches (`save_game`) on the main menu.
//! With a save, a tall Continue card heads the main list (`home.rs`); with one save it resumes it, with several it
//! opens this screen over the main one: a card per save in the lobby's player cards' style (a header in the mode's
//! colour with a chip, the faces, names, a mini board, the court and when it was saved), under them the navy bar of
//! the chosen save's court, umpire, length and server and the yellow Resume (an online save's Open lobby, greyed
//! Can't resume with the reason). △ / L / Del deletes the chosen save after a confirm. Laid out across the whole view
//! (`Grid::extra`), the cards in as many rows as the width needs and scaled as one, so any aspect ratio fits.

use super::frame::{self, Cell, Geo, Thumb};
use super::{CHAR_NAMES, COURTS, Dev, Draw, Effect, Menu, Press, Screen, Tex, Text, UMPIRES, WHITE, Who, face_frame, pointer};
use crate::play::save_game::{self, Head, MatchSave, Mode, Saves};
use bevy::prelude::Entity;
use std::path::PathBuf;

/// The saved matches the menu shows, newest first; whether the Continue screen is open over the main one, and its
/// delete confirm (`Some(0)` Delete, `Some(1)` Keep).
#[derive(Clone, Debug, Default)]
pub(super) struct Saved {
    pub(super) list: Vec<Card>,
    /// Read since the menu came up (or back from a match).
    loaded: bool,
    pub(super) open: bool,
    pub(super) ask: Option<usize>,
    /// The flags of the save being resumed, from its choice to the match's start (`launch_flags`).
    launch: Option<Vec<String>>,
}

/// One save: its file, what it says about itself and, when it can't be resumed, why.
#[derive(Clone, Debug)]
pub(super) struct Card {
    pub(super) path: PathBuf,
    pub(super) head: Head,
    pub(super) refusal: Option<String>,
}

impl Card {
    pub(super) fn new(path: PathBuf, head: Head) -> Self {
        let refusal = match head.check() {
            Err(r) => Some(r.to_string()),
            // the menus' match start gives the flags; a save without them can't be set up again
            // M199: an online one is hosted again from its lobby
            Ok(()) if head.mode == Mode::Online && head.setup.lobby.is_none() => Some("Saved without its lobby".into()),
            Ok(()) if head.mode == Mode::Local && head.setup.flags.is_empty() => Some("Saved without its setup".into()),
            Ok(()) => None,
        };
        Card { path, head, refusal }
    }

    fn online(&self) -> bool {
        self.head.mode == Mode::Online
    }
}

/// The save waiting for its match to start (`started` hands it to the match).
static PENDING: std::sync::Mutex<Option<MatchSave>> = std::sync::Mutex::new(None);

impl Menu {
    /// Reads the saves folder once after the menu comes up or a match comes back (`after_result`, every frame).
    pub(super) fn saved_refresh(&mut self) {
        if self.saved.loaded || cfg!(test) {
            return;
        }
        self.saved.loaded = true;
        self.saved.list = Saves::user().list().into_iter().map(|e| Card::new(e.path, e.head)).collect();
        let n = self.saved.list.len();
        if self.saved.open && n < 2 {
            self.saved.open = false;
        }
        if self.screen == Screen::Main && !self.saved.open {
            self.sel = self.sel.min(self.main_rows().len() - 1);
        }
    }

    /// Back from a match: read the saves again (it may have saved or finished one).
    pub(super) fn saved_stale(&mut self) {
        (self.saved.loaded, self.saved.open, self.saved.ask, self.saved.launch) = (false, false, None, None);
    }

    /// The main list's Continue card chosen: one save resumes, several open the screen.
    pub(super) fn continue_chosen(&mut self, dev: Dev, out: &mut Vec<Effect>) {
        if self.saved.list.len() == 1 && self.saved.list[0].refusal.is_none() {
            self.resume(0, dev, out);
        } else if !self.saved.list.is_empty() {
            // one that can't be resumed opens too: its card says why and it can be deleted
            (self.saved.open, self.saved.ask, self.sel) = (true, None, 0);
            out.push(Effect::Sound(2));
        }
    }

    /// Resume save `i`: a local one starts its match from its own setup (the players at today's devices), an online
    /// one goes to Online with Host chosen (it's hosted again from the lobby).
    fn resume(&mut self, i: usize, dev: Dev, out: &mut Vec<Effect>) {
        let Some(c) = self.saved.list.get(i) else { return };
        if c.refusal.is_some() {
            out.push(Effect::Sound(1));
            return;
        }
        if c.online() {
            // M199: its lobby goes with it, the next lobby hosted offers it (Resume / New match)
            let read = Saves::user().read(&c.path).inspect_err(|e| bevy::log::warn!("{}: {e}", c.path.display()));
            if !read.is_ok_and(crate::play::netplay::host_again) {
                self.saved.list[i].refusal = Some("Can't read it".into());
                out.push(Effect::Sound(1));
                return;
            }
            let doubles = !c.head.setup.singles;
            self.saved.open = false;
            self.go_online(dev, out);
            self.net.host_save = true;
            (self.doubles, self.sel) = (doubles, 1);
            return;
        }
        match Saves::user().read(&c.path) {
            Ok(save) => {
                self.saved.launch = Some(save.head.setup.flags.clone());
                *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(save);
                self.saved.open = false;
                self.screen = Screen::Playing;
                out.extend([Effect::Sound(2), Effect::Launch]);
            }
            Err(e) => {
                bevy::log::warn!("{}: {e}", c.path.display());
                self.saved.list[i].refusal = Some("Can't read it".into());
                out.push(Effect::Sound(1));
            }
        }
    }

    /// One press while the Continue screen is open (it takes them all); returns whether it was.
    pub(super) fn continue_step(&mut self, dev: Dev, k: Press, out: &mut Vec<Effect>) -> bool {
        // a press on the main screen: no save is on its way into a match (its Rematch plays the menu's setup)
        self.saved.launch = None;
        if !self.saved.open {
            return false;
        }
        let n = self.saved.list.len();
        if let Some(a) = self.saved.ask {
            // the confirm: any direction swaps Delete / Keep (the pointer's buttons press ← or → with ✕)
            let a = if k.left { 0 } else if k.right { 1 } else if k.up || k.down { 1 - a } else { a };
            if Some(a) != self.saved.ask {
                self.saved.ask = Some(a);
                out.push(Effect::Sound(0));
            }
            if k.ok && a == 0 {
                self.delete_save(self.sel.min(n.saturating_sub(1)));
                out.push(Effect::Sound(2));
            } else if k.ok || k.back {
                self.saved.ask = None;
                out.push(Effect::Sound(1));
            }
            return true;
        }
        if k.back {
            self.close_continue();
            out.push(Effect::Sound(1));
            return true;
        }
        if n == 0 {
            return true;
        }
        // the cards in reading order: ← ↑ the one before, → ↓ the next, round the ends
        let d = (k.right || k.down) as i32 - (k.left || k.up) as i32;
        if d != 0 && n > 1 {
            self.sel = (self.sel.min(n - 1) as i32 + d).rem_euclid(n as i32) as usize;
            out.push(Effect::Sound(0));
        }
        if k.card {
            self.saved.ask = Some(1);
            out.push(Effect::Sound(2));
        } else if k.ok {
            self.resume(self.sel.min(n - 1), dev, out);
        }
        true
    }

    /// The Continue screen closed: the main list's Continue card chosen.
    fn close_continue(&mut self) {
        self.saved.open = false;
        self.sel = self.main_at(super::home::Row::Continue);
    }

    /// Deletes save `i` (its file and card); none left, back to the main list.
    fn delete_save(&mut self, i: usize) {
        self.saved.ask = None;
        if i >= self.saved.list.len() {
            return;
        }
        let c = self.saved.list.remove(i);
        if !cfg!(test) {
            Saves::user().delete(&c.path);
        }
        if self.saved.list.is_empty() {
            self.saved.open = false;
            self.sel = self.main_at(super::home::Row::Local);
        } else {
            self.sel = i.min(self.saved.list.len() - 1);
        }
    }

    /// The main list's newest save (the Continue card's).
    pub(super) fn newest_save(&self) -> Option<&Card> {
        self.saved.list.first()
    }

    /// Save `c`'s match player `p` as the menus show it: a disc character or an installed mod.
    pub(super) fn save_who(&self, h: &Head, p: usize) -> Who {
        let pin = h.setup.mods.get(p).and_then(|m| m.as_ref());
        match pin.and_then(|pin| self.mods.iter().position(|m| m.dir == pin.dir)) {
            Some(i) => Who::Mod(i),
            None => Who::Disc(h.setup.chars.get(p).copied().unwrap_or(0).min(CHAR_NAMES.len() - 1)),
        }
    }

    /// Match player `p`'s name in save `h`.
    pub(super) fn save_name(&self, h: &Head, p: usize) -> String {
        let pin = h.setup.mods.get(p).and_then(|m| m.as_ref());
        match (self.save_who(h, p), pin) {
            (Who::Mod(i), _) => self.mods[i].name.clone(),
            (_, Some(pin)) => pin.folder.clone(),
            (Who::Disc(c), None) => CHAR_NAMES[c].to_string(),
        }
    }

    /// The match players of each side in save `h`: team 0 (1P's), team 1.
    pub(super) fn save_teams(h: &Head) -> [Vec<usize>; 2] {
        if h.setup.singles { [vec![0], vec![1]] } else { [vec![0, 2], vec![1, 3]] }
    }

    /// "Carol vs Will", "Carol & Kent vs Jun & Lola".
    pub(super) fn save_players(&self, h: &Head) -> String {
        let [a, b] = Self::save_teams(h).map(|t| t.iter().map(|&p| self.save_name(h, p)).collect::<Vec<_>>().join(" & "));
        format!("{a} vs {b}")
    }

    /// The main card's line: "Carol vs Will · 6-4 4-2 · 30-15".
    pub(super) fn save_line(&self, h: &Head) -> String {
        format!("{} · {} · {}", self.save_players(h), h.summary.games_line(), h.summary.points_line())
    }

    /// The serving player's name.
    pub(super) fn save_server(&self, h: &Head) -> String {
        self.save_name(h, h.summary.server.clamp(0, 3) as usize)
    }

    /// The save's court as a menu court value (a mod court's, else the disc's).
    pub(super) fn save_court(&self, h: &Head) -> usize {
        let m = h.setup.court_mod.as_ref().and_then(|pin| self.court_mods.iter().position(|c| c.dir == pin.dir));
        m.map_or(h.setup.court.saturating_sub(1).min(COURTS - 1), |i| COURTS + 1 + i)
    }

    /// The save's umpire as a menu umpire value.
    pub(super) fn save_umpire(&self, h: &Head) -> usize {
        let m = h.setup.umpire_mod.as_ref().and_then(|pin| self.umpire_mods.iter().position(|u| u.dir == pin.dir));
        m.map_or((h.setup.umpire as usize).min(UMPIRES - 1), |i| UMPIRES + 1 + i)
    }

    /// Every face the saves' cards show, for `previews.rs` to bake.
    pub(super) fn save_faces(&self) -> Vec<Who> {
        let n = if self.saved.open { self.saved.list.len() } else { self.saved.list.len().min(1) };
        self.saved.list[..n].iter().flat_map(|c| (0..c.head.setup.chars.len()).map(|p| self.save_who(&c.head, p))).collect()
    }
}

/// "Local, singles" / "Online, doubles".
pub(super) fn mode_words(h: &Head) -> String {
    format!("{}, {}", if h.mode == Mode::Online { "Online" } else { "Local" }, if h.setup.singles { "singles" } else { "doubles" })
}

/// "1 set, 6 games" / "Best of 3, 4 games" (the flags' sets are the sets to win).
pub(super) fn length_words(h: &Head) -> String {
    let sets = (2 * h.setup.sets.max(1) - 1) as usize;
    let s = if sets == 1 { "1 set".to_string() } else { format!("Best of {sets}") };
    format!("{s}, {} games", h.setup.games)
}

/// When `saved` was, seen at `now` (seconds since 1970): "Just now", "5 min ago", "3 hours ago", "Yesterday",
/// "4 days ago".
pub(super) fn ago(saved: u64, now: u64) -> String {
    let s = now.saturating_sub(saved);
    match s {
        0..60 => "Just now".into(),
        60..3600 => format!("{} min ago", s / 60),
        3600..7200 => "1 hour ago".into(),
        7200..86400 => format!("{} hours ago", s / 3600),
        86400..172800 => "Yesterday".into(),
        _ => format!("{} days ago", s / 86400),
    }
}

/// The clock now (seconds since 1970; M198: the browser's too, it has saves now).
pub(super) fn now() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// The match's flags when a save is being resumed: its own setup's, with `--pads` given to today's devices (the menu's
/// seats where they're still human seats, then the keyboard and the pads in order; a human seat with no device left
/// plays as the computer).
pub(super) fn launch_flags(m: &Menu, pads: &[Entity]) -> Option<Vec<String>> {
    let mut a = m.saved.launch.clone()?;
    if let Some(i) = a.iter().position(|f| f == "--pads").filter(|i| i + 1 < a.len()) {
        let seats: Vec<Option<Dev>> = m.seats.to_vec();
        a[i + 1] = remap(&a[i + 1], &seats, pads);
    }
    Some(a)
}

/// `--pads` value `saved` with its human seats on today's devices (`seats` the menu's, `pads` the sorted gamepads).
fn remap(saved: &str, seats: &[Option<Dev>], pads: &[Entity]) -> String {
    use super::controls::Seat;
    let human: Vec<bool> = saved.split(',').map(|s| Seat::parse(s).is_some()).collect();
    let as_seat = |d: Option<Dev>| match d {
        Some(Dev::Keys) => Some(Seat::Keys),
        Some(Dev::Pad(e)) => pads.iter().position(|x| *x == e).map(Seat::Pad),
        _ => None,
    };
    let mut free: Vec<Seat> = std::iter::once(Seat::Keys).chain((0..pads.len()).map(Seat::Pad)).collect();
    let mut out: Vec<Option<Seat>> = vec![None; human.len()];
    for (p, _) in human.iter().enumerate().filter(|(_, h)| **h) {
        if let Some(k) = as_seat(seats.get(p).copied().flatten()).and_then(|s| free.iter().position(|x| *x == s)) {
            out[p] = Some(free.remove(k));
        }
    }
    for (p, _) in human.iter().enumerate().filter(|(_, h)| **h) {
        if out[p].is_none() && !free.is_empty() {
            out[p] = Some(free.remove(0));
        }
    }
    Seat::flag(&out)
}

/// The match started from a save (after "starting the match"): it goes into the match once it's set up.
pub(super) fn started(m: &mut Menu) {
    if m.saved.launch.take().is_some()
        && let Some(save) = PENDING.lock().unwrap_or_else(|e| e.into_inner()).take()
    {
        save_game::resume_in_next(save);
    }
}

/// The match start called off before it began: nothing resumes.
pub(super) fn cancelled(m: &mut Menu) {
    m.saved.launch = None;
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).take();
}

/// The mode headers' colours: local red, online blue, grey when it can't be resumed.
const LOCAL: [f32; 3] = frame::hex(0xc8413b);
const ONLINE: [f32; 3] = frame::hex(0x2f6fc0);
const DEAD: [f32; 3] = frame::hex(0x5d6475);
const READY: [f32; 3] = frame::hex(0x2fa860);
const SERVE: [f32; 3] = frame::GOLD;

/// A card's design size (units, scaled as one to fit) and the least width it's shown at before the cards wrap.
const CARD_W: f32 = 132.0;
const CARD_H: f32 = 184.0;
const CARD_MIN: f32 = 104.0;
const GAP: f32 = 12.0;
/// The foot: the navy bar's and Resume's height, Resume's width.
const FOOT_H: f32 = 40.0;
const START_W: f32 = 190.0;

/// The colour of save `c`'s header.
fn header_fill(c: &Card) -> [f32; 3] {
    match (c.refusal.is_some(), c.online()) {
        (true, _) => DEAD,
        (_, true) => ONLINE,
        _ => LOCAL,
    }
}

/// Save `c`'s chip: its words and fill.
fn chip(c: &Card) -> (String, [f32; 3]) {
    match &c.refusal {
        Some(r) => (r.clone(), DEAD),
        None => ("Ready".into(), READY),
    }
}

/// The cards' grid in the room `[x, y, w, h]` for `n` cards: how many across, the scale and the first card's corner.
pub(super) fn grid(n: usize, [x, y, w, h]: [f32; 4]) -> (usize, f32, [f32; 2]) {
    let n = n.max(1);
    let cols = (((w + GAP) / (CARD_MIN + GAP)) as usize).clamp(1, n);
    let rows = n.div_ceil(cols);
    let s = ((w - GAP * (cols - 1) as f32) / cols as f32 / CARD_W).min((h - GAP * (rows - 1) as f32) / rows as f32 / CARD_H).min(1.25);
    let (cw, ch) = (CARD_W * s, CARD_H * s);
    let used = [cols as f32 * cw + (cols - 1) as f32 * GAP, rows as f32 * ch + (rows - 1) as f32 * GAP];
    (cols, s, [x + (w - used[0]) / 2.0, y + (h - used[1]) / 2.0])
}

/// The Continue screen over the main one.
pub(super) fn layout(d: &mut Draw, m: &Menu, text: &Text, hand_x: f32) {
    let (top, foot) = super::home::span(&m.grid);
    let e = m.grid.extra;
    // the wall and a gold band with the Continue tab (the tab is Back for the pointer; the main screen's draw puts
    // its "<" on it)
    d.wall(super::WALL);
    tab(d);
    let n = m.saved.list.len();
    let sel = m.sel.min(n.saturating_sub(1));
    let (l, r) = (-e + 16.0, 640.0 + e - 16.0);
    let sub = format!("{n} saved · newest first · keeps {}", save_game::KEEP);
    d.outlined(|d| frame::dotted(d, &sub, l + 4.0, top - 4.0, 14.0, r - l - 8.0, WHITE, false));
    // the foot: the navy bar and Resume (Resume alone across on a narrow view, the bar over it)
    let narrow = r - l < 420.0;
    let fy = foot - FOOT_H - 2.0;
    let (bar, start, room_foot) = if narrow {
        ([l, fy - FOOT_H - 8.0, r - l, FOOT_H], [l, fy, r - l, FOOT_H], fy - FOOT_H - 8.0)
    } else {
        let sw = START_W.min((r - l) * 0.38);
        ([l, fy, r - l - sw - 10.0, FOOT_H], [r - sw, fy, sw, FOOT_H], fy)
    };
    let room = [l, top + 16.0, r - l, room_foot - 10.0 - (top + 16.0)];
    let (cols, s, [x0, y0]) = grid(n, room);
    let mut hand = [x0 - 60.0, y0];
    for (i, c) in m.saved.list.iter().enumerate() {
        let at = [x0 + (i % cols) as f32 * (CARD_W * s + GAP), y0 + (i / cols) as f32 * (CARD_H * s + GAP)];
        let mark = d.mark();
        card(d, m, text, c, i == sel, [at[0], at[1]]);
        d.scale_since(mark, at, s);
        // a tap picks it; on the chosen one, resumes it
        let press = if i == sel { pointer::OK } else { Press::default() };
        d.hit([at[0], at[1], CARD_W * s, CARD_H * s], pointer::Act::Row(i, press));
        if i == sel {
            hand = [at[0] - 56.0, at[1] + CARD_H * s / 2.0 - 16.0];
        }
    }
    if let Some(c) = m.saved.list.get(sel) {
        foot_bar(d, m, text, c, sel, bar, start);
    }
    if hand[0] + hand_x > -e - 8.0 {
        d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [hand[0] + hand_x, hand[1], 64.0, 32.0], WHITE, 128.0);
    }
    if m.saved.ask.is_some() {
        ask_layout(d, m, hand_x);
    }
}

/// The gold Continue tab and band, as `Draw::frame` draws a screen's.
fn tab(d: &mut Draw) {
    let words = "Continue";
    let band = [128.0, 105.0, 31.0];
    d.anchor = super::Anchor::Span;
    d.q(Tex::Solid, [0.0; 4], [0.0, 0.0, 640.0, 14.0], band, 128.0);
    d.q(Tex::Solid, [0.0; 4], [0.0, 14.0, 640.0, 2.5], WHITE, 128.0);
    d.q(Tex::Solid, [0.0; 4], [0.0, 16.5, 640.0, 1.5], [15.0; 3], 128.0);
    let w = d.width(words, 30.0);
    d.title_tab(w, band);
    let start = d.out.len();
    d.text(words, 42.0, 7.0, 30.0, [0.0; 3]);
    d.out[start..].iter_mut().for_each(|q| q.alpha = 40.0);
    d.outlined(|d| d.text(words, 40.0, 5.0, 30.0, WHITE));
    d.hit([-30.0, -30.0, w + 110.0, 70.0], pointer::press(|k| k.back = true));
    d.anchor = super::Anchor::Centre;
}

/// Save `c`'s card at its design size from `[x, y]`.
fn card(d: &mut Draw, m: &Menu, text: &Text, c: &Card, on: bool, [x, y]: [f32; 2]) {
    let h = &c.head;
    let rect = [x, y, CARD_W, CARD_H];
    frame::ringed(d, rect, 10.0, 2.0, if on { WHITE } else { frame::RULE }, Some(frame::PANEL));
    if on {
        frame::focus(d, rect, 10.0, 3.0, 2.5);
    }
    // the header in the mode's colour: "LOCAL · SINGLES"
    let head = format!("{} · {}", if c.online() { "ONLINE" } else { "LOCAL" }, if h.setup.singles { "SINGLES" } else { "DOUBLES" });
    let hr = [x + 2.0, y + 2.0, CARD_W - 4.0, 20.0];
    frame::rounded(d, hr, 8.0, header_fill(c), 128.0);
    d.q(Tex::Solid, [0.0; 4], [hr[0], hr[1] + 10.0, hr[2], 10.0], header_fill(c), 128.0);
    frame::dotted(d, &head, x + CARD_W / 2.0, y + 6.0, 11.0, CARD_W - 16.0, WHITE, true);
    // the chip
    let (words, fill) = chip(c);
    let cw = (d.width(&words, 10.0) + 14.0).min(CARD_W - 16.0);
    let cr = [x + (CARD_W - cw) / 2.0, y + 28.0, cw, 15.0];
    frame::rounded(d, cr, 7.5, fill, 128.0);
    d.label(&words, cr[0] + cw / 2.0, cr[1] + 2.5, 10.0, cw - 8.0, WHITE);
    // the faces: a side each, the teams' two side by side
    let [a, b] = Menu::save_teams(h);
    let size = if h.setup.singles { 40.0 } else { 28.0 };
    for (side, team) in [a, b].iter().enumerate() {
        let cx = x + CARD_W * if side == 0 { 0.27 } else { 0.73 };
        for (k, &p) in team.iter().enumerate() {
            let dx = if team.len() == 1 { 0.0 } else { (k as f32 - 0.5) * (size - 4.0) };
            face_frame(d, (Tex::Face(m.save_who(h, p)), [0.0, 0.0, 64.0, 64.0]), cx + dx, y + 70.0, size, WHITE);
        }
    }
    d.label("vs", x + CARD_W / 2.0, y + 64.0, 11.0, 20.0, frame::CAPTION);
    // the names, the mini board
    super::home::fit_text(d, &m.save_players(h), x + 8.0, y + 94.0, 11.0, CARD_W - 16.0, WHITE);
    mini_board(d, m, h, [x + 8.0, y + 111.0, CARD_W - 16.0, 34.0]);
    // the court and the date at the foot
    let court = m.court_short(text, m.save_court(h));
    super::home::fit_text(d, &court, x + 8.0, y + 152.0, 10.0, CARD_W - 16.0, frame::CAPTION);
    super::home::fit_text(d, &ago(h.saved, now()), x + 8.0, y + 166.0, 9.0, CARD_W - 16.0, frame::CAPTION);
}

/// The score in `[x, y, w, h]` on navy: a row a side, its first player's name, the serve dot, each set's games and
/// the points.
pub(super) fn mini_board(d: &mut Draw, m: &Menu, h: &Head, [x, y, w, bh]: [f32; 4]) {
    frame::rounded(d, [x, y, w, bh], 5.0, frame::NAVY, 128.0);
    let s = &h.summary;
    let serving = (s.server.clamp(0, 3) % 2) as usize;
    let rh = bh / 2.0;
    let size = rh * 0.62;
    let sets: Vec<[i32; 2]> = s.set_games.iter().copied().chain([s.games]).collect();
    let pw = (size * 1.9).max(d.width("AD", size) + 6.0);
    let gw = size * 0.95;
    for (t, team) in Menu::save_teams(h).iter().enumerate() {
        let ry = y + rh * t as f32;
        let ty = ry + (rh - size) / 2.0;
        if t == serving {
            let r = size * 0.22;
            d.q(Tex::Solid, [0.0; 4], [x + 5.0, ry + rh / 2.0 - r, 2.0 * r, 2.0 * r], SERVE, 128.0);
            d.out.last_mut().unwrap().round = [r; 4];
        }
        let name = m.save_name(h, team[0]);
        let room = w - 14.0 - pw - gw * sets.len() as f32 - 4.0;
        let name = frame::ellipsis(d, &name, size, room.max(10.0));
        d.text(&name, x + 12.0, ty, size, WHITE);
        for (k, g) in sets.iter().enumerate() {
            let gx = x + w - pw - gw * (sets.len() - k) as f32;
            let last = k + 1 == sets.len();
            d.centred(&g[t].to_string(), gx + gw / 2.0, ty, size, if last { WHITE } else { frame::CAPTION });
        }
        frame::rounded(d, [x + w - pw, ry + 1.5, pw - 2.0, rh - 3.0], 3.0, frame::BUTTON, 128.0);
        d.centred(&s.call(t), x + w - pw / 2.0 - 1.0, ty, size, frame::GOLD);
    }
}

/// The chosen save's navy bar (court, umpire, length, server) in `bar` and its Resume button in `start`.
fn foot_bar(d: &mut Draw, m: &Menu, text: &Text, c: &Card, sel: usize, bar: [f32; 4], start: [f32; 4]) {
    let h = &c.head;
    let mut g: Geo = frame::foot(bar, true);
    let court = m.save_court(h);
    let umpire = m.save_umpire(h);
    let cells = [
        Cell { label: "COURT", value: m.court_short(text, court), arrows: false, sel: false, thumb: Thumb::Court(court) },
        Cell { label: "UMPIRE", value: m.umpire_name(text, umpire), arrows: false, sel: false, thumb: Thumb::Umpire(umpire) },
        Cell { label: "LENGTH", value: length_words(h), arrows: false, sel: false, thumb: Thumb::None },
        Cell { label: "SERVING", value: m.save_server(h), arrows: false, sel: false, thumb: Thumb::None },
    ];
    // the bar's own cells: Court and Umpire wider (their pictures), the rest even
    let shares = [1.5, 1.2, 1.0, 1.0];
    let total: f32 = shares.iter().sum();
    let mut at = bar[0];
    g.cells = [[bar[0], bar[1], 0.0, bar[3]]; 6];
    for (k, sh) in shares.iter().enumerate() {
        let w = bar[2] * sh / total;
        g.cells[k] = [at, bar[1], w, bar[3]];
        at += w;
    }
    g.panels = vec![bar];
    g.strip = bar;
    g.start = start;
    g.k = bar[3] / (62.0 * super::roster::PY);
    // a narrow bar drops the pictures (the names keep their room)
    let cells: Vec<Cell> = cells.into_iter().map(|c| if bar[2] < 380.0 { Cell { thumb: Thumb::None, ..c } } else { c }).collect();
    frame::strip(d, m, &g, &cells);
    let (big, small) = match &c.refusal {
        Some(r) => ("Can't resume".to_string(), r.clone()),
        None if c.online() => ("Open lobby".into(), "host it again".into()),
        None => ("Resume".into(), format!("from {}, {} to serve", h.summary.points_line(), m.save_server(h))),
    };
    let mark = d.out.len();
    frame::start(d, &g, &big, &small, c.refusal.is_none(), c.refusal.is_none());
    if c.refusal.is_some() {
        // greyed: it can't be pressed
        d.out[mark..].iter_mut().for_each(|q| q.rgb = q.rgb.map(|v| v * 0.55 + 20.0));
    } else {
        d.hit(start, pointer::Act::Row(sel, pointer::OK));
    }
}

/// The delete confirm over the dimmed screen, in the newer-version popup's style.
fn ask_layout(d: &mut Draw, m: &Menu, hand_x: f32) {
    use super::super::menu::{CHOSEN, PANEL, UNCHOSEN};
    let Some(a) = m.saved.ask else { return };
    if let Some(i) = d.out.iter().rposition(|q| q.tex == Tex::Cursor) {
        d.out.remove(i);
    }
    d.popup(Some(pointer::Act::Row(0, Press { ok: false, back: true, ..pointer::OK })));
    d.anchor = super::Anchor::Span;
    d.q(Tex::Solid, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 80.0);
    d.anchor = super::Anchor::Centre;
    let top = 156.0;
    d.box3([160.0, top, 320.0, 136.0], 16.0, [Some(super::INK), Some(WHITE), Some(PANEL)], 128.0);
    let who = m.saved.list.get(m.sel).map(|c| m.save_players(&c.head)).unwrap_or_default();
    d.outlined(|d| d.label("Delete this saved match?", 320.0, top + 12.0, 22.0, 288.0, WHITE));
    d.outlined(|d| d.label(&who, 320.0, top + 38.0, 16.0, 288.0, WHITE));
    for (k, word) in ["Delete", "Keep"].into_iter().enumerate() {
        let r = [176.0 + 148.0 * k as f32, top + 84.0, 140.0, 32.0];
        let on = k == a;
        d.pill_at(r, [super::INK, WHITE, if on { CHOSEN } else { UNCHOSEN }], 128.0);
        let press = if k == 0 { Press { left: true, ..pointer::OK } } else { Press { right: true, ..pointer::OK } };
        d.hit(r, pointer::Act::Press(press));
        d.label(word, r[0] + 70.0, r[1] + 6.0, 20.0, 120.0, if on { WHITE } else { UNCHOSEN.map(|c| c * 1.4) });
    }
    d.q(Tex::Cursor, [0.0, 0.0, 64.0, 32.0], [116.0 + 148.0 * a as f32 + hand_x, top + 84.0, 64.0, 32.0], WHITE, 128.0);
}

/// `HST_MENU` shots: `continue` the main list with three saves (and a Rematch), `continue-one` one save,
/// `continue-screen` the Continue screen open, `continue-delete` its delete confirm, `continue-dead` its chosen save
/// unable to load.
pub(super) fn sample(m: &mut Menu, screen: &str) {
    use crate::play::save_game::{Setup, Summary};
    let t = now();
    let local = |chars: Vec<usize>, games, points, server, sets: Vec<[i32; 2]>, ago: u64, mode| {
        let singles = chars.len() == 2;
        let pads = if singles { "k,-" } else { "k,-,-,-" };
        let setup = Setup { singles, outfits: vec![0; chars.len()], chars, humans: [true, false, false, false], court: 4, umpire: 2, sets: 2, games: 6, flags: vec!["--play".into(), "--pads".into(), pads.into()], rules: crate::rules_choice::chosen(), hz: crate::match_speed::MatchHz::now().0, ..Default::default() };
        let summary = Summary { games, points, server, set_games: sets, ..Default::default() };
        // M199: an online save keeps its lobby
        let setup = Setup { lobby: (mode == Mode::Online).then(Default::default), ..setup };
        Head { build: crate::handshake::build().into(), mode, saved: t.saturating_sub(ago), setup, summary }
    };
    let heads = [
        local(vec![6, 11], [4, 2], [2, 1], 0, vec![[6, 4]], 300, Mode::Local),
        local(vec![0, 9, 4, 2], [3, 3], [3, 3], 1, vec![], 3 * 3600, Mode::Online),
        local(vec![7, 12], [5, 6], [0, 2], 1, vec![[3, 6], [7, 5]], 2 * 86400, Mode::Local),
    ];
    let n = if screen == "continue-one" { 1 } else { 3 };
    m.saved.list = heads.into_iter().take(n).enumerate().map(|(i, h)| Card::new(PathBuf::from(format!("sample-{i}.save")), h)).collect();
    m.saved.loaded = true;
    if screen == "continue" {
        (m.rematch, m.seats[0]) = (true, Some(Dev::Keys));
        m.players[1].cursor = 9;
    }
    if screen == "continue-dead" {
        m.saved.list[0].refusal = Some("Mods changed".into());
    }
    m.saved.open = matches!(screen, "continue-screen" | "continue-delete" | "continue-dead");
    m.saved.ask = (screen == "continue-delete").then_some(1);
    m.sel = 0;
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::play::save_game::{Setup, Summary};

    /// A save's head: Carol (6) vs Will (11), 6-4 4-2 30-15, Carol serving, saved at `saved`.
    pub(in crate::play) fn head(mode: Mode, saved: u64) -> Head {
        let setup = Setup { singles: true, chars: vec![6, 11], outfits: vec![0, 0], humans: [true, false, false, false], court: 3, umpire: 1, sets: 2, games: 6, flags: vec!["--play".into(), "--pads".into(), "k,-".into()], ..Default::default() };
        let summary = Summary { games: [4, 2], points: [2, 1], server: 0, set_games: vec![[6, 4]], ..Default::default() };
        Head { build: crate::handshake::build().into(), mode, saved, setup, summary }
    }

    pub(in crate::play) fn with_saves(n: usize) -> Menu {
        let mut m = Menu { screen: Screen::Main, seats: [Some(Dev::Keys), None, None, None], ..Default::default() };
        m.saved.loaded = true;
        m.saved.list = (0..n).map(|i| Card { path: format!("/nonexistent/{i}.save").into(), head: head(if i == 2 { Mode::Online } else { Mode::Local }, 1000 - i as u64), refusal: None }).collect();
        m
    }

    #[test]
    fn lines() {
        let m = Menu::default();
        let h = head(Mode::Local, 0);
        assert_eq!(m.save_line(&h), "Carol vs Will · 6-4 4-2 · 30-15");
        assert_eq!((m.save_server(&h), length_words(&h), mode_words(&h)), ("Carol".into(), "Best of 3, 6 games".into(), "Local, singles".into()));
        let mut d = h.clone();
        d.setup.singles = false;
        d.setup.chars = vec![6, 11, 0, 9];
        assert_eq!(m.save_players(&d), "Carol & Ashley vs Will & Kent");
        assert_eq!([ago(100, 130), ago(0, 300), ago(0, 3700), ago(0, 3 * 3600), ago(0, 90000), ago(0, 5 * 86400)], ["Just now", "5 min ago", "1 hour ago", "3 hours ago", "Yesterday", "5 days ago"]);
    }

    /// The menu's seats keep their devices on the save's human seats, the rest go to the keyboard then the pads in
    /// order; a human seat with nothing left plays as the computer.
    #[test]
    fn pads_follow_todays_devices() {
        let (p0, p1) = (Entity::from_raw_u32(5).unwrap(), Entity::from_raw_u32(7).unwrap());
        assert_eq!(remap("k,-", &[Some(Dev::Pad(p1)), None], &[p0, p1]), "1,-");
        assert_eq!(remap("0,-,k,-", &[None; 4], &[p0]), "k,-,0,-");
        assert_eq!(remap("k,1", &[Some(Dev::Keys), None], &[]), "k,-");
        assert_eq!(remap("-,-", &[Some(Dev::Keys), None], &[p0]), "-,-");
    }

    /// One save resumes from the main card; several open the screen, where ← → move round, △ asks and Delete deletes,
    /// Keep / ○ don't, ○ closes back onto the Continue card; an online save goes to Online with Host chosen.
    #[test]
    fn continue_screen_steps() {
        use super::super::home::Row;
        let mut m = with_saves(3);
        let mut fx = Vec::new();
        assert_eq!(m.main_rows()[0], Row::Continue);
        m.continue_chosen(Dev::Keys, &mut fx);
        assert!(m.saved.open && m.sel == 0);
        m.continue_step(Dev::Keys, Press { left: true, ..Default::default() }, &mut fx);
        assert_eq!(m.sel, 2);
        m.continue_step(Dev::Keys, Press { card: true, ..Default::default() }, &mut fx);
        assert_eq!(m.saved.ask, Some(1));
        m.continue_step(Dev::Keys, Press { ok: true, ..Default::default() }, &mut fx);
        assert_eq!((m.saved.ask, m.saved.list.len()), (None, 3));
        m.continue_step(Dev::Keys, Press { card: true, ..Default::default() }, &mut fx);
        m.continue_step(Dev::Keys, Press { left: true, ok: true, ..Default::default() }, &mut fx);
        assert_eq!((m.saved.ask, m.saved.list.len(), m.sel), (None, 2, 1));
        m.continue_step(Dev::Keys, Press { back: true, ..Default::default() }, &mut fx);
        assert_eq!((m.saved.open, m.main_rows()[m.sel]), (false, Row::Continue));
        // a save that can't be read can't be resumed
        m.continue_chosen(Dev::Keys, &mut fx);
        fx.clear();
        m.continue_step(Dev::Keys, Press { ok: true, ..Default::default() }, &mut fx);
        assert_eq!((m.screen, m.saved.list[0].refusal.as_deref()), (Screen::Main, Some("Can't read it")));
        // online, unreadable: it stays
        let mut m = with_saves(3);
        m.continue_chosen(Dev::Keys, &mut fx);
        m.sel = 2;
        m.continue_step(Dev::Keys, Press { ok: true, ..Default::default() }, &mut fx);
        assert_eq!((m.screen, m.saved.list[2].refusal.as_deref()), (Screen::Main, Some("Can't read it")));
        // M199: online with its lobby: Host, and the next lobby hosted offers it
        let dir = std::env::temp_dir().join(format!("hst-continue-online-{}", std::process::id()));
        let mut h = head(Mode::Online, 998);
        let mut room = crate::net::Room { doubles: false, seats: vec![None, None], ..Default::default() };
        room.ids.insert("g1".into(), "id-g".into());
        h.setup.lobby = Some(crate::play::save_game::Lobby::of(&room));
        let path = crate::play::save_game::Saves::new(&dir).write(&crate::play::save_game::MatchSave { head: h, state: vec![1, 2, 3] }).unwrap();
        let mut m = with_saves(3);
        m.saved.list[2].path = path;
        m.continue_chosen(Dev::Keys, &mut fx);
        m.sel = 2;
        m.continue_step(Dev::Keys, Press { ok: true, ..Default::default() }, &mut fx);
        assert_eq!((m.screen, m.sel, m.saved.open), (Screen::Online, 1, false));
        let offer = crate::play::netplay::saved_offer().expect("offered");
        *crate::play::netplay::SAVED.lock().unwrap() = None;
        assert_eq!((offer.score.as_str(), offer.on, offer.saved.ids.get("g1").map(String::as_str)), ("4-2", true, Some("id-g")));
        // Host: straight to the lobby, the save's picks
        assert!(m.net.host_save);
        m.net.go = Some(super::super::online::Go::Host);
        fx.clear();
        m.online_open(&mut fx);
        assert_eq!(m.screen, Screen::Lobby);
        assert!(fx.iter().any(|e| matches!(e, Effect::Online(super::super::online::Fx::Net(crate::net::Cmd::Host(_))))));
        let _ = std::fs::remove_dir_all(&dir);
        // one save: straight in (here unreadable, so it stays)
        let mut m = with_saves(1);
        m.continue_chosen(Dev::Keys, &mut fx);
        assert!(!m.saved.open && m.saved.list[0].refusal.is_some());
    }

    /// The cards fit their room at every aspect ratio: inside it, never wider than it, scaled as one.
    #[test]
    fn cards_fit_each_aspect() {
        let extra = |a: f32| (640.0 / super::super::super::widescreen::share(a) - 640.0) / 2.0;
        for a in [9.0 / 19.5, 3.0 / 4.0, 4.0 / 3.0, 16.0 / 9.0, 21.0 / 9.0, 32.0 / 9.0] {
            let e = extra(a);
            let room = [-e + 16.0, 66.0, 640.0 + 2.0 * e - 32.0, 300.0];
            for n in 1..=save_game::KEEP {
                let (cols, s, [x, y]) = grid(n, room);
                let rows = n.div_ceil(cols);
                let w = cols as f32 * CARD_W * s + (cols - 1) as f32 * GAP;
                let h = rows as f32 * CARD_H * s + (rows - 1) as f32 * GAP;
                assert!(x >= room[0] - 0.01 && x + w <= room[0] + room[2] + 0.01 && y >= room[1] - 0.01 && y + h <= room[1] + room[3] + 0.01, "{a} {n}");
                assert!(s > 0.4, "{a} {n}: {s}");
            }
        }
    }
}
