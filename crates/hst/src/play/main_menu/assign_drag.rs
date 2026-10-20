//! B597: the controller assignment's cards dragged with the mouse. A press on a card that can move and a drag past a
//! few px lifts it: it follows the pointer (its seat left as a dashed outline), the seat under the pointer it can swap
//! with rings gold, and letting go there swaps the two seats' controllers, the same trade the pads' and keys' carry
//! makes (1P never left empty, over a match the computer's and other machines' seats fixed). A press that never
//! moves is the plain click on release (`pointer.rs`).

use super::assign::cards;
use super::roster::{BG, PX, PY, dashed, hex, rounded, seat};
use super::{Effect, GOLD, Menu, Quad, Screen, look};

/// The card being dragged: its seat, where the pointer grabbed it and where the pointer is now (centred 640×448 units).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Drag {
    pub from: usize,
    pub grab: [f32; 2],
    pub at: [f32; 2],
}

impl Drag {
    /// How far the card has moved from its seat.
    fn offset(&self) -> [f32; 2] {
        [self.at[0] - self.grab[0], self.at[1] - self.grab[1]]
    }
}

impl Menu {
    /// Whether seats `p` and `q` may trade their controllers by a drag: the swap ←→ makes with a held controller,
    /// between any two seats.
    pub(super) fn drag_swaps(&self, p: usize, q: usize) -> bool {
        let n = self.players();
        if p == q || p >= n || q >= n || self.keep.is_some() {
            return false;
        }
        // online only this machine's own seats; over a match the computer's stay put
        if self.net_pause.is_some_and(|r| r[p] || r[q]) || self.online() && !self.paused && p.max(q) >= self.local_max() {
            return false;
        }
        if self.paused && (self.bots[p] || self.bots[q]) {
            return false;
        }
        let f = self.first_seat();
        let empties_1p = q == f && self.seats[p].is_none() || p == f && self.seats[q].is_none();
        let differ = self.seats[p] != self.seats[q] || !self.paused && self.bots[p] != self.bots[q];
        !empties_1p && differ
    }

    /// Whether the mouse can pick up seat `p`'s card: something on it (a controller, or before a match a computer
    /// player) that can trade with another seat.
    pub(in crate::play) fn draggable(&self, p: usize) -> bool {
        let n = self.players();
        let some = p < n && (matches!(self.seats[p], Some(super::Dev::Keys | super::Dev::Pad(_))) || self.bots[p] && !self.paused);
        self.screen == Screen::Assign && some && (0..n).any(|q| self.drag_swaps(p, q))
    }

    /// The seat a card from `p` dropped at `at` swaps with, if any.
    pub(super) fn drop_target(&self, p: usize, at: [f32; 2]) -> Option<usize> {
        let inside = |[x, y, w, h]: [f32; 4]| (x..=x + w).contains(&at[0]) && (y..=y + h).contains(&at[1]);
        cards(self.grid.view, self.players()).into_iter().position(inside).filter(|&q| self.drag_swaps(p, q))
    }

    /// The pointer carries seat `p`'s card, grabbed at `grab`, to `at`.
    pub(super) fn carry(&mut self, p: usize, grab: [f32; 2], at: [f32; 2], out: &mut Vec<Effect>) {
        if !self.draggable(p) {
            self.drag = None;
            return;
        }
        if self.drag.is_none_or(|d| d.from != p) {
            // focused and lifted, as a pad's ✕ picks it up
            (self.sel, self.held) = (p, false);
            out.push(Effect::Sound(2));
        }
        self.drag = Some(Drag { from: p, grab, at });
    }

    /// The card from `p` let go at `at`: on a seat it can trade with, the two swap and the focus follows it there.
    pub(super) fn land(&mut self, p: usize, at: [f32; 2], out: &mut Vec<Effect>) {
        let was = self.drag.take();
        if self.screen != Screen::Assign || was.is_none() {
            return;
        }
        let Some(q) = self.drop_target(p, at) else {
            out.push(Effect::Sound(1));
            return;
        };
        // B437: shown at once, but the driver drives the screen until it leaves
        self.assign_lead = self.lead();
        self.seats.swap(p, q);
        if !self.paused {
            self.bots.swap(p, q);
        }
        (self.sel, self.held) = (q, false);
        out.push(Effect::Sound(0));
    }
}

/// `HST_MENU=assign-drag` (for `--shot`s), on each fit of the view: doubles, the keyboard on 1P and a pad on 2P,
/// 1P's card dragged most of the way onto 2P's.
pub(super) fn demo(m: &mut Menu) {
    if std::env::var("HST_MENU").as_deref() != Ok("assign-drag") {
        return;
    }
    m.doubles = true;
    m.bots = [false; 4];
    m.seats = [Some(super::Dev::Keys), Some(super::Dev::Pad(bevy::prelude::Entity::from_raw_u32(1).unwrap())), None, None];
    let c = cards(m.grid.view, 4);
    let mid = |[x, y, w, h]: [f32; 4]| [x + w / 2.0, y + h / 2.0];
    let (a, b) = (mid(c[0]), mid(c[1]));
    m.drag = Some(Drag { from: 0, grab: a, at: [a[0] + (b[0] - a[0]) * 0.8, a[1] + (b[1] - a[1]) * 0.8 + 12.0] });
}

/// While a card is dragged: `quads` (seat `p`'s card, drawn at `card`) lift off with the pointer, its seat left a
/// dashed outline; returned to draw over everything else. Any other seat's card just passes through.
pub(super) fn lift(d: &mut super::Draw, m: &Menu, p: usize, card: [f32; 4], quads: Vec<Quad>) -> Vec<Quad> {
    let Some(drag) = m.drag.filter(|g| g.from == p) else {
        d.out.extend(quads);
        return Vec::new();
    };
    dashed(d, card, 14.0, look::RIM, seat(p).ring, hex(BG));
    let [dx, dy] = drag.offset();
    let start = d.out.len();
    // a soft shadow under the lifted card
    rounded(d, [card[0] + 8.0 * PX, card[1] + 10.0 * PY, card[2], card[3]], 14.0, [0.0; 3], 60.0);
    d.out.extend(quads);
    d.out.drain(start..)
        .map(|mut q| {
            q.dst[0] += dx;
            q.dst[1] += dy;
            q
        })
        .collect()
}

/// The seat the dragged card would swap with rings gold.
pub(super) fn target(d: &mut super::Draw, m: &Menu) {
    let Some(drag) = m.drag else { return };
    let Some(q) = m.drop_target(drag.from, drag.at) else { return };
    let [x, y, w, h] = cards(m.grid.view, m.players())[q];
    super::frame::ringed(d, [x - 6.0 * PX, y - 6.0 * PY, w + 12.0 * PX, h + 12.0 * PY], 20.0 * PX, 4.0 * PX, GOLD, None);
}

#[cfg(test)]
mod tests {
    use super::super::{Dev, pointer};
    use super::*;

    fn centre(m: &Menu, p: usize) -> [f32; 2] {
        let [x, y, w, h] = cards(m.grid.view, m.players())[p];
        [x + w / 2.0, y + h / 2.0]
    }

    /// Dragging the keyboard's card onto a pad's seat swaps them, the focus following; let go off every seat, or on
    /// its own, nothing changes.
    #[test]
    fn a_drag_swaps_two_seats() {
        let mut w = bevy::prelude::World::new();
        let pad = Dev::Pad(w.spawn_empty().id());
        let mut m = Menu { screen: Screen::Assign, doubles: true, seats: [Some(Dev::Keys), Some(pad), None, None], ..Default::default() };
        let (a, b) = (centre(&m, 0), centre(&m, 1));
        let fx = m.point(Dev::Keys, pointer::Act::Carry(0, a, b), true);
        assert_eq!(fx, [Effect::Sound(2)]);
        assert_eq!(m.drop_target(0, b), Some(1));
        m.point(Dev::Keys, pointer::Act::Land(0, b), true);
        assert_eq!(m.seats, [Some(pad), Some(Dev::Keys), None, None]);
        assert_eq!((m.sel, m.drag), (1, None));
        // off every card, then back on its own seat: no change
        for at in [[-500.0, -500.0], centre(&m, 1)] {
            m.point(Dev::Keys, pointer::Act::Carry(1, centre(&m, 1), at), true);
            assert_eq!(m.point(Dev::Keys, pointer::Act::Land(1, at), true), [Effect::Sound(1)]);
            assert_eq!(m.seats, [Some(pad), Some(Dev::Keys), None, None]);
        }
    }

    /// The carry's rules: 1P's seat never left empty, an empty seat's card isn't lifted, over a match the computer's
    /// seats stay put, online only this machine's seats.
    #[test]
    fn drag_rules() {
        let mut m = Menu { screen: Screen::Assign, doubles: true, seats: [Some(Dev::Keys), None, None, None], ..Default::default() };
        assert!(!m.drag_swaps(0, 1), "1P's seat emptied");
        assert!(!m.draggable(1), "nothing on it");
        // before a match a computer player moves, not onto 1P's seat
        m.bots[2] = true;
        assert!(m.draggable(2) && m.drag_swaps(2, 1) && !m.drag_swaps(2, 0));
        m.paused = true;
        m.bots = [false, true, true, false];
        assert!(!m.draggable(2) && !m.drag_swaps(0, 1));
        assert!(!m.drag_swaps(0, 3), "1P's seat emptied");
        m.seats[3] = Some(Dev::Pad(bevy::prelude::World::new().spawn_empty().id()));
        assert!(m.drag_swaps(0, 3) && m.drag_swaps(3, 0));
        m.net_pause = Some([false, false, false, true]);
        assert!(!m.drag_swaps(0, 3), "another machine's seat");
        m.net_pause = None;
        m.keep = Some(0);
        assert!(!m.drag_swaps(0, 3), "under the Keep / Undo popup");
    }

    /// The lifted card draws offset by the drag, over a dashed outline left on its seat; the target rings gold. Every
    /// aspect still fits the pause overlay's pool.
    #[test]
    fn the_lifted_card_follows_the_pointer() {
        let t = pointer::tests::text_with_mods(0);
        let mut m = Menu { screen: Screen::Assign, doubles: true, paused: true, seats: [Some(Dev::Keys), Some(Dev::Pad(bevy::prelude::World::new().spawn_empty().id())), None, None], bots: [false, false, true, true], flash: [1.0; 4], ..Default::default() };
        for p in &mut m.players {
            p.ready = true;
        }
        let bind = super::super::controls::Bindings::default();
        for a in [4.0, 16.0 / 9.0, 4.0 / 3.0, 9.0 / 16.0, 0.4] {
            m.grid = super::super::Grid::fit(a);
            m.drag = None;
            let still = super::super::drawn(&m, &t, &bind, &[], 0, 0.0).out;
            let (c0, c1) = (centre(&m, 0), centre(&m, 1));
            m.drag = Some(Drag { from: 0, grab: c0, at: c1 });
            assert_eq!(m.drop_target(0, c1), Some(1), "{a}");
            let q = super::super::drawn(&m, &t, &bind, &[], 0, 0.0).out;
            assert!(q.len() > still.len() && q.len() < super::super::pause::POOL_HERE, "{a}: {} quads", q.len());
            // the card's ring moved by the drag
            let [x, y, ..] = cards(m.grid.view, 4)[0];
            let moved = q.iter().any(|q| (q.dst[0] - (x + c1[0] - c0[0])).abs() < 0.5 && (q.dst[1] - (y + c1[1] - c0[1])).abs() < 0.5);
            assert!(moved, "{a}");
        }
    }
}
