//! B616: the local confirm screen's Match cell. When the foot is two cells a row (`frame::foot`), Sets, Games and
//! Markers share one cell; ✕ (or a tap) on it opens a small panel over the screen with a ‹ › stepper each: ↑↓ choose,
//! ←→ (or a tap on a stepper's ends) change, ✕ or ○ close it.

use super::frame::{self, CAPTION, PANEL, RULE};
use super::pointer::Act;
use super::{Draw, Effect, GAMES, MARKS_ROW, Menu, Press, SETS, WHITE};

/// The panel's rows, as the confirm screen's rows (`Menu::sel`).
const ROWS: [usize; 3] = [2, 3, MARKS_ROW];

impl Menu {
    /// The confirm screen's press with the Match cell in mind: ✕ on it opens the panel; while it's up it takes every
    /// press. False: not the panel's.
    pub(super) fn match_step(&mut self, k: Press, out: &mut Vec<Effect>) -> bool {
        if !self.match_panel {
            if k.ok && !k.name && ROWS.contains(&self.sel) && self.confirm_geo().merged {
                (self.match_panel, self.sel) = (true, 2);
                out.push(Effect::Sound(2));
                return true;
            }
            return false;
        }
        if k.ok || k.back {
            self.match_panel = false;
            out.push(Effect::Sound(if k.ok { 2 } else { 1 }));
        } else if k.up || k.down {
            let at = ROWS.iter().position(|&r| r == self.sel).unwrap_or(0);
            self.sel = ROWS[if k.up { (at + ROWS.len() - 1) % ROWS.len() } else { (at + 1) % ROWS.len() }];
            out.push(Effect::Sound(0));
        } else if k.left || k.right {
            let was = (self.sets, self.games, self.no_marks);
            let d = if k.right { 1 } else { -1 };
            let step = |v: usize, n: usize| (v as i32 + d).clamp(0, n as i32 - 1) as usize;
            match self.sel {
                2 => self.sets = step(self.sets, SETS.len()),
                3 => self.games = GAMES[step(GAMES.iter().position(|&n| n >= self.games).unwrap_or(1), GAMES.len())],
                _ => self.no_marks = !self.no_marks,
            }
            if was != (self.sets, self.games, self.no_marks) {
                out.push(Effect::Sound(0));
            }
        }
        true
    }
}

/// The Match cell's words: the sets and games.
pub(super) fn summary(m: &Menu) -> String {
    let sets = SETS[m.sets];
    format!("{sets} set{}, {} games", if sets == 1 { "" } else { "s" }, m.games)
}

/// The panel, while it's up: centred over the screen (its own shape at any window size), a stepper a row.
pub(super) fn draw(d: &mut Draw, m: &Menu) {
    if !m.match_panel {
        return;
    }
    d.anchor = super::Anchor::Span;
    d.q(super::Tex::Solid, [0.0; 4], [0.0, 0.0, 640.0, 448.0], [0.0; 3], 64.0);
    d.anchor = super::Anchor::Centre;
    d.popup(Some(Act::Press(Press { back: true, ..Default::default() })));
    let (rh, gap) = (34.0, 8.0);
    let (w, h) = (260.0, 46.0 + ROWS.len() as f32 * (rh + gap) + 6.0);
    let (x, y) = (320.0 - w / 2.0, 224.0 - h / 2.0);
    frame::ringed(d, [x, y, w, h], 12.0, 2.0, RULE, Some(PANEL));
    d.label("Match", x + w / 2.0, y + 12.0, 16.0, w - 20.0, WHITE);
    let at = |i: usize| match ROWS[i] {
        2 => (SETS[m.sets].to_string(), m.sets > 0, m.sets + 1 < SETS.len()),
        3 => {
            let g = GAMES.iter().position(|&n| n >= m.games).unwrap_or(1);
            (m.games.to_string(), g > 0, g + 1 < GAMES.len())
        }
        _ => (if m.no_marks { "Off" } else { "On" }.to_string(), true, true),
    };
    for (i, (row, name)) in ROWS.iter().zip(["SETS", "GAMES", "MARKERS"]).enumerate() {
        let rect = [x + 12.0, y + 42.0 + i as f32 * (rh + gap), w - 24.0, rh];
        let [rx, ry, rw, _] = rect;
        let on = m.sel == *row;
        frame::rounded(d, rect, 8.0, if on { RULE } else { super::frame::NAVY }, 128.0);
        if on {
            frame::ringed(d, rect, 8.0, 2.0, WHITE, None);
        }
        let (ls, vs) = (10.0, 16.0);
        d.text(name, rx + 10.0, ry + (rh - ls) / 2.0, ls, CAPTION);
        let (value, back, on_) = at(i);
        let (l, r) = (rx + rw * 0.45, rx + rw - 10.0);
        let vy = ry + (rh - vs) / 2.0;
        if back {
            d.text("<", l, vy, vs, WHITE);
        }
        if on_ {
            d.text(">", r - d.width(">", vs), vy, vs, WHITE);
        }
        d.label(&value, (l + r) / 2.0, vy, vs, r - l - 30.0, WHITE);
        // a tap on the row chooses it, on its ‹ / › halves steps it
        d.hit(rect, Act::Row(*row, Press::default()));
        d.hit([l - 6.0, ry, (r - l) / 2.0 + 6.0, rh], Act::Row(*row, Press { left: true, ..Default::default() }));
        d.hit([(l + r) / 2.0, ry, (r - l) / 2.0 + 10.0, rh], Act::Row(*row, Press { right: true, ..Default::default() }));
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Dev, Grid, Screen};
    use super::*;

    /// On a narrow window Sets, Games and Markers share the Match cell: ✕ opens its panel, ↑↓ choose, ←→ change,
    /// ○ closes it; on a wide one ✕ on Sets is the strip's own stepper, no panel.
    #[test]
    fn the_match_panel_steps_sets_games_and_markers() {
        let press = |f: fn(&mut Press)| {
            let mut k = Press::default();
            f(&mut k);
            k
        };
        let mut m = Menu { screen: Screen::Confirm, seats: [Some(Dev::Keys), None, None, None], sel: 2, grid: Grid::fit_at(0.75, 2.0), ..Default::default() };
        m.players.iter_mut().take(2).for_each(|p| p.ready = true);
        assert!(m.confirm_geo().merged, "{:?}", m.confirm_geo().cells);
        let mut out = vec![];
        assert!(m.match_step(press(|k| k.ok = true), &mut out) && m.match_panel);
        let (sets, games) = (m.sets, m.games);
        m.match_step(press(|k| k.right = true), &mut out);
        m.match_step(press(|k| k.down = true), &mut out);
        m.match_step(press(|k| k.left = true), &mut out);
        m.match_step(press(|k| k.down = true), &mut out);
        m.match_step(press(|k| k.right = true), &mut out);
        m.match_step(press(|k| k.back = true), &mut out);
        assert!(!m.match_panel);
        assert_eq!((m.sets, m.no_marks), (sets + 1, true));
        assert!(m.games < games);
        let mut wide = Menu { grid: Grid::fit(16.0 / 9.0), match_panel: false, sel: 2, ..m };
        assert!(!wide.confirm_geo().merged);
        assert!(!wide.match_step(press(|k| k.ok = true), &mut out));
    }
}
