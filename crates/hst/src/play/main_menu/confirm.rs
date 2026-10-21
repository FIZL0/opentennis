//! B449: the confirm screen, v4 (ours, the approved board "Concept v4: Ready to play?", the lobby's frame,
//! `frame.rs`). The title and the Singles / Doubles button (□ switches, 1P's device or a tap) along the top; the
//! players' plates on the court, a team each side of the net (singles: near side, far side; online, no side names, B490); the scoreboard strip
//! under it: Court (its picture; ✕ opens the court select), Umpire (her face; ✕ opens the umpire select), Sets and
//! Games (✕, then ←→, as before, B212); the handicap is △ on a local plate (`handicap.rs`, B40g1), online the host's L2 / R2 in the lobby (B40g3); along the bottom
//! ‹ Back and the big Start the match!. The arrows go about the screen as its parts lie; ✕ on a plate sends that
//! player back to the character select. The court and umpire selects (`court_screen`, `umpire_screen`) are as before.

use super::frame::{self, Cell, Chip, Plate, Spec, Thumb};
use super::{CHAR_NAMES, CONFIRM_ROWS, Draw, Grid, MARKS_ROW, Menu, Press, RANK_NAMES, SETS, Text, UMPIRES, Who};
use super::pointer;
use super::super::controls::Bindings;
use bevy::prelude::{Entity, default};

impl Menu {
    pub(super) fn confirm_geo(&self) -> frame::Geo {
        if !self.online() {
            // concept: local, the confirm screen is the select's columns with the lobby's strip and Start under them
            // B616: its cells laid out to fit their words (`frame::foot`)
            let mut g = self.lobby_foot();
            let c = self.columns();
            g.plates = (0..c.n).map(|p| c.parts(p).card).collect();
            return g;
        }
        let mut g = frame::layout(self.grid.reach(), Spec { n: self.players(), code: false, public: false, chat: false });
        // Handicap is hidden until handicaps work (B482; B40g1 brings it back)
        frame::spread(&mut g, 5);
        g
    }

    /// B449: where the arrows go from the confirm screen's chosen row (`Menu::sel`), as its parts lie.
    pub(super) fn confirm_step(&self, k: Press) -> usize {
        super::online::pill_step(&rects(self), self.sel, k)
    }

    /// Whether ↑↓ and ←→ reach another row from the chosen one.
    pub(super) fn confirm_axes(&self) -> (bool, bool) {
        let to = |f: fn(&mut Press) -> &mut bool| {
            let mut k = Press::default();
            *f(&mut k) = true;
            self.confirm_step(k) != self.sel
        };
        (to(|k| &mut k.up) || to(|k| &mut k.down), to(|k| &mut k.left) || to(|k| &mut k.right))
    }
}

/// The confirm screen's rows (`Menu::sel`): Start, the court, Sets, Games, the umpire, Markers (`CONFIRM_ROWS`), then a plate a
/// player.
pub(super) fn rects(m: &Menu) -> Vec<[f32; 4]> {
    let g = m.confirm_geo();
    let mut v = vec![g.start, g.cells[0], g.cells[2], g.cells[3], g.cells[1], g.cells[4]];
    v.extend(g.plates);
    // B616: local, the Rules cell after the plates (`Menu::rules_row`)
    if !m.online() {
        v.push(g.cells[5]);
    }
    v
}

/// The confirm screen (the court and umpire selects open over it as their own screens).
pub(super) fn draw(d: &mut Draw, m: &Menu, text: &Text, bind: &Bindings, pads: &[Entity]) {
    if !m.online() {
        return super::roster::chars(d, m, text, bind, pads);
    }
    frame::ground(d);
    let g = m.confirm_geo();
    let picking = m.picking.is_some();
    let on = |row: usize| !picking && m.sel == row;
    // the top bar: the title (Back for the pointer), Singles / Doubles (1P's □ or a tap)
    frame::bar(d, &g, "Ready to play?", m.doubles, false, frame::page_glyph(m, Some(0)).filter(|_| !m.online()), !m.online(), false);
    d.hit(g.title, pointer::press(|k| k.back = true));
    // online singles names no sides: each client keeps its own player on the near side (B490)
    // M120: online, no court behind the plates
    let stage = if m.online() { frame::sides } else { frame::stage };
    stage(d, &g, if m.doubles { ["TEAM 1", "TEAM 2"] } else if m.online() { ["", ""] } else { ["NEAR SIDE", "FAR SIDE"] });
    let s = g.k * g.s;
    for (p, &rect) in g.plates.iter().enumerate() {
        plate(d, m, text, pads, p, rect, s, on(CONFIRM_ROWS + p));
        d.hit(rect, pointer::Act::Row(CONFIRM_ROWS + p, pointer::OK));
    }
    // the strip: Court, Umpire, Sets, Games (Handicap hidden, B482)
    frame::strip(d, m, &g, &cells(m, text, &on));
    for (row, cell) in [(1, 0), (4, 1), (2, 2), (3, 3), (MARKS_ROW, 4)] {
        d.hit(g.cells[cell], pointer::Act::Row(row, pointer::OK));
    }
    // the bottom row: ‹ Back, Start
    frame::back(d, &g, "< Back", false);
    d.hit(g.back, pointer::press(|k| k.back = true));
    frame::start(d, &g, "Start the match!", "Everyone ready", false, on(0));
    d.hit(g.start, pointer::Act::Row(0, pointer::OK));
    if picking {
        picked_cell(d, m, &g);
    }
}

/// Concept: the local select's foot, as the online lobby's: Court, Umpire, Sets, Games inline with Start. On the
/// select it shows what's set and how many are ready; once everyone is (the confirm screen) the arrows reach it.
pub(super) fn local_foot(d: &mut Draw, m: &Menu, text: &Text, bind: &Bindings) {
    let confirm = m.screen == super::Screen::Confirm;
    let g = m.confirm_geo();
    let on = |row: usize| confirm && m.picking.is_none() && m.sel == row;
    if confirm && m.picking.is_none() && m.sel >= CONFIRM_ROWS && let Some(&rect) = g.plates.get(m.sel - CONFIRM_ROWS) {
        frame::focus(d, rect, 14.0 * super::roster::PX, 4.0 * super::roster::PX, 3.0 * super::roster::PX);
    }
    if confirm {
        for (p, &rect) in g.plates.iter().enumerate() {
            d.hit(rect, pointer::Act::Row(CONFIRM_ROWS + p, pointer::OK));
        }
    }
    let mut all_cells = Vec::from(cells(m, text, &on));
    if g.merged {
        // B616: Sets, Games and Markers in one Match cell (its panel: `match_panel.rs`)
        all_cells[2] = Cell { label: "MATCH", value: super::match_panel::summary(m), arrows: false, sel: on(2) || on(3) || on(MARKS_ROW), thumb: Thumb::None };
    }
    all_cells.push(Cell { label: "OFFBEAT RULES", value: rules_words(m), arrows: false, sel: on(m.rules_row()), thumb: Thumb::None });
    frame::strip(d, m, &g, &all_cells);
    rules_cell(d, m, bind, g.cells[5], g.k);
    let n = m.players();
    let ready = (0..n).filter(|&p| m.players[p].ready).count();
    // concept: the strip takes a tap while the cards are still picked too
    let rows: &[(usize, usize)] = if g.merged { &[(1, 0), (4, 1), (2, 2)] } else { &[(1, 0), (4, 1), (2, 2), (3, 3), (MARKS_ROW, 4)] };
    for &(row, cell) in rows {
        d.hit(g.cells[cell], pointer::Act::Row(row, pointer::OK));
    }
    if confirm {
        d.hit(g.cells[5], pointer::Act::Row(m.rules_row(), pointer::OK));
    }
    let all = ready == n;
    let note = if all { "Everyone ready".to_string() } else { format!("{ready} of {n} ready") };
    frame::start(d, &g, "Start", &note, all, on(0));
    if confirm {
        d.hit(g.start, pointer::Act::Row(0, pointer::OK));
        if m.picking.is_some() {
            picked_cell(d, m, &g);
        }
    }
}

/// B616: the Rules cell's value: how many rules are on.
fn rules_words(m: &Menu) -> String {
    match m.rules_on().len() {
        0 => "None on".into(),
        n => format!("{n} on"),
    }
}

/// B616: over the Rules cell `[x, y, w, h]` (as `frame::strip` drew it): the button that opens the box (Select / C)
/// right of its caption, the rules on as chips after its value (as many as fit, then "+N").
fn rules_cell(d: &mut Draw, m: &Menu, bind: &Bindings, [x, y, w, h]: [f32; 4], k: f32) {
    use super::roster::{PX, PY, chip, chip_w, key, key_w};
    if w <= 0.0 {
        return;
    }
    let dev = match m.lead() {
        Some(super::Dev::Pad(e)) => Some(e),
        _ => None,
    };
    let (ls, vs) = (11.0 * k * PY, 20.0 * k * PY);
    let top = y + (h - ls - 2.0 * k * PY - vs) / 2.0;
    let (l, r) = (x + 14.0 * k * PX, x + w - 14.0 * k * PX);
    let name = super::legend::button_names(dev, super::legend::Act::Hand, bind).first().cloned().unwrap_or_default();
    let gh = 15.0 * k * PY;
    let kx = l + d.width("OFFBEAT RULES", ls) + 6.0 * k * PX;
    // the key only where it opens the box (on the select it's Hand)
    let opens = matches!(m.screen, super::Screen::Confirm | super::Screen::Playing);
    if opens && kx + key_w(d, dev, bevy::prelude::GamepadButton::Select, &name, gh) <= r {
        key(d, dev, bevy::prelude::GamepadButton::Select, &name, [kx, top + (ls - gh) / 2.0], gh);
    }
    let on = m.rules_on();
    let vy = top + ls + 2.0 * k * PY;
    let mut cx = l + d.width(&rules_words(m), vs) + 8.0 * k * PX;
    let ch = 16.0 * k * PY;
    let cy = vy + (vs - ch) / 2.0;
    for (i, words) in on.iter().enumerate() {
        let more = format!("+{}", on.len() - i - 1);
        let tail = if i + 1 < on.len() { chip_w(d, &more, ch) + 4.0 * k * PX } else { 0.0 };
        if cx + chip_w(d, words, ch) + tail > r {
            chip(d, &format!("+{}", on.len() - i), [cx, cy], ch, CAPTION_CHIP);
            break;
        }
        cx += chip(d, words, [cx, cy], ch, frame::GOLD) + 4.0 * k * PX;
    }
}

/// The Rules cell's "+N" chip.
const CAPTION_CHIP: [f32; 3] = frame::CAPTION;

/// The strip: Court, Umpire, Sets, Games, Markers (rows 1, 4, 2, 3, `MARKS_ROW`).
fn cells(m: &Menu, text: &Text, on: &dyn Fn(usize) -> bool) -> [Cell; 5] {
    let u = m.umpire as usize;
    let umpire = match (u, m.umpire_mod_of(u)) {
        (UMPIRES, _) => "Random".to_string(),
        (_, Some(x)) => x.name.clone(),
        _ => m.umpire_name(text, u),
    };
    [
        Cell { label: "COURT", value: m.court_title(text, m.court), arrows: false, sel: on(1), thumb: Thumb::Court(m.court) },
        Cell { label: "UMPIRE", value: umpire, arrows: false, sel: on(4), thumb: Thumb::Umpire(u) },
        Cell { label: "SETS", value: SETS[m.sets].to_string(), arrows: true, sel: on(2), thumb: Thumb::None },
        Cell { label: "GAMES", value: m.games.to_string(), arrows: true, sel: on(3), thumb: Thumb::None },
        Cell { label: "MARKERS", value: if m.no_marks { "Off" } else { "On" }.into(), arrows: false, sel: on(MARKS_ROW), thumb: Thumb::None },
    ]
}

/// Sets or Games picked (B212): its cell lit blue with ‹ › (each gone at its end, as the original's); the cell's
/// thirds step it, the middle keeps it, a tap off puts it back (as ○).
fn picked_cell(d: &mut Draw, m: &Menu, g: &frame::Geo) {
    if !matches!(m.sel, 2 | 3) {
        return;
    }
    let (at, n) = if m.sel == 2 { (m.sets, SETS.len()) } else { (super::GAMES.iter().position(|&n| n >= m.games).unwrap_or(1), super::GAMES.len()) };
    let (cap, value) = if m.sel == 2 { ("SETS", SETS[m.sets].to_string()) } else { ("GAMES", m.games.to_string()) };
    // the cells lie Court, Umpire, Sets, Games; the rows Start, Court, Sets, Games, Umpire
    let [x, y, w, h] = g.cells[m.sel];
    let lit = [x, y + g.y(2.0), w, h - g.y(4.0)];
    frame::rounded(d, lit, g.x(6.0), frame::hex(0x2f5f9d), 128.0);
    frame::ringed(d, lit, g.x(6.0), g.x(3.0), super::WHITE, None);
    let (ls, vs) = (g.y(11.0), g.y(20.0));
    let top = y + (h - ls - g.y(2.0) - vs) / 2.0;
    let (l, r) = (x + g.x(14.0), x + w - g.x(14.0));
    super::left(d, cap, l, top, ls, r - l, frame::CAPTION);
    let vy = top + ls + g.y(2.0);
    if at > 0 {
        d.text("<", l, vy, vs, super::WHITE);
    }
    if at + 1 < n {
        d.text(">", r - d.width(">", vs), vy, vs, super::WHITE);
    }
    d.label(&value, (l + r) / 2.0, vy, vs, r - l - g.x(40.0), super::WHITE);
    d.popup(Some(pointer::press(|k| k.back = true)));
    d.hit([x, y, w, h], pointer::Act::Row(m.sel, pointer::OK));
    d.hit([x, y, w / 3.0, h], pointer::Act::Row(m.sel, Press { left: true, ..default() }));
    d.hit([x + w * 2.0 / 3.0, y, w / 3.0, h], pointer::Act::Row(m.sel, Press { right: true, ..default() }));
}

/// Player `p`'s plate: who plays it (the device, a computer, online the player's name), the character, its style and
/// hand, Ready or the computer's COM · rank.
#[allow(clippy::too_many_arguments)]
fn plate(d: &mut Draw, m: &Menu, text: &Text, pads: &[Entity], p: usize, rect: [f32; 4], s: f32, focus: bool) {
    let pl = m.players[p];
    let (who, costume) = m.pick(p);
    let remote = m.remote(p);
    let human = m.seats[p].is_some() || remote.is_some_and(|s| s.human());
    let donor = match who {
        Who::Mod(i) => m.mods[i].donor,
        Who::Disc(c) => c,
    };
    let chip_words = if human {
        "Ready".to_string()
    } else {
        let rank = super::com_rank(&text.ranks, donor, costume, m.doubles);
        format!("COM · {}", RANK_NAMES.get(rank).copied().unwrap_or("COM").trim_start_matches("Tennis "))
    };
    let random = pl.random;
    let name = match who {
        _ if random => "Random".to_string(),
        Who::Mod(i) => m.mods[i].name.clone(),
        Who::Disc(c) => m.remote_mod_name(p).unwrap_or(CHAR_NAMES[c]).to_string(),
    };
    let owner = match remote.filter(|s| s.human()) {
        Some(s) => m.net.room.as_ref().and_then(|r| r.names.get(&s.peer)).map_or("Online".to_string(), |n| n.clone()),
        None if human => super::dev_name(m.seats[p], pads),
        None => "Computer".into(),
    };
    let sub = if random { "Picked at the start".to_string() } else { format!("{} · {}", frame::style_short(text, who), if m.hand(text, p) < 0.0 { "Left" } else { "Right" }) };
    let mod_note = m.remote_mod_note(p);
    let plate = Plate {
        p,
        owner: &owner,
        slow: false,
        name: &name,
        sub: &sub,
        chip: if human { Chip::Ready } else { Chip::Com },
        chip_words: &chip_words,
        dashed: !human,
        remove: false,
        focus,
        remove_focus: false,
        random,
        hide: false,
        empty: false,
        mod_note,
        device: remote.filter(|s| s.human()).and_then(|s| m.net.room.as_ref()?.devices.get(&s.peer).copied()),
    };
    frame::plate(d, &plate, rect, s);
}

/// B616: whether the local select's (`stepper`: the confirm screen's) cards keep their least height over the foot
/// on the view `aspect` wide at Menu Size `k`, one to four players (`fit_scale` lowers the size until they do).
pub(super) fn cards_fit(aspect: f32, k: f32, stepper: bool) -> bool {
    let [l, t, r, b] = Grid::fit_at(aspect, k).view;
    let [_, y, _, h] = frame::foot([l + super::roster::MARGIN * super::roster::PX, t, r - super::roster::MARGIN * super::roster::PX, b], true).stage;
    (1..=4).all(|n| super::roster::columns_stepped([l, t, r, y + h], n, [false; 4], stepper).whole)
}

#[cfg(test)]
mod tests {
    use super::super::{Grid, Screen};
    use super::*;

    /// Every row reachable with the arrows from Start, at any window.
    #[test]
    fn arrows_reach_every_row() {
        for aspect in [0.5, 0.75, 1.0, 4.0 / 3.0, 1.6, 16.0 / 9.0, 21.0 / 9.0] {
            for doubles in [false, true] {
                let m = Menu { screen: Screen::Confirm, doubles, grid: Grid::fit(aspect), ..default() };
                let n = rects(&m).len();
                let mut seen = vec![false; n];
                let mut todo = vec![0];
                while let Some(s) = todo.pop() {
                    if std::mem::replace(&mut seen[s], true) {
                        continue;
                    }
                    for f in [|k: &mut Press| k.up = true, |k: &mut Press| k.down = true, |k: &mut Press| k.left = true, |k: &mut Press| k.right = true] {
                        let mut k = Press::default();
                        f(&mut k);
                        todo.push(Menu { sel: s, ..m.clone() }.confirm_step(k));
                    }
                }
                assert!(seen.iter().all(|&s| s), "{aspect} {doubles}: {seen:?}");
            }
        }
    }

    /// B616: a short window's big Menu Size comes down until the cards keep a whole row of tiles over the foot (no
    /// further than the size that fills the screen); a window with room keeps the size chosen.
    #[test]
    fn a_short_window_lowers_the_confirm_screens_menu_size_until_the_cards_fit() {
        for a in [4.0 / 3.0, 16.0 / 9.0, 844.0 / 390.0] {
            let k = super::super::fit_scale(Screen::Confirm, a, 1.5);
            assert!(k < 1.5, "{a}: {k}");
            let (s, v) = crate::widescreen::menu_shares(a, 1.0);
            assert!(cards_fit(a, k, true) || (k - 1.0 / s.max(v)).abs() < 1e-4, "{a}: {k}");
        }
        assert_eq!(super::super::fit_scale(Screen::Confirm, 16.0 / 9.0, 1.0), 1.0);
        assert!(cards_fit(9.0 / 16.0, 1.5, true));
    }
}
