//! The slide between menu screens (B40e2): the last screen's panels slide off and fade, then the new screen's slide
//! in from the other side. Ours, not the original's (menus need not be 1:1, user 2026-10-12): the wall and title bar
//! (`Anchor::Span`) switch at the midpoint without moving; a forward step (a later `Screen`) moves left, back moves right.
//! Between two Settings tabs nothing moves: the tab bar and legend stay put; the old cards fade onto blank paper cards,
//! then the new ones fade in over them, so only one page shows at a time and the wall never shows through (user
//! 2026-10-21: both pages at once was too much; a dimmed blank with the cards sliding off it read as a gray ghost).

use bevy::prelude::*;

use super::super::widescreen::Anchor;
use super::roster::PX;
use super::{Quad, Screen, Tex, look};

/// Frames (60 Hz) for each half: out, then in.
const HALF: f32 = 8.0;
/// How far the panels travel (640×448 units) while fading.
const TRAVEL: f32 = 160.0;
/// B626: the web build switches Settings tabs at once, no slide (user 2026-10-21: it animated badly there).
const INSTANT_TABS: bool = cfg!(target_arch = "wasm32");

#[derive(Resource)]
pub(super) struct Slide {
    on: Option<Screen>,
    /// The last frame's quads, and those of the screen being left.
    last: Vec<Quad>,
    old: Vec<Quad>,
    /// Frames since the screen changed.
    t: f32,
    /// +1 forward, −1 back.
    dir: f32,
    /// Between Settings tabs: the old page's cards' boxes (list, side); only quads starting in their y band move.
    band: Option<[[f32; 4]; 2]>,
    /// The last frame's cards' boxes.
    cards: [[f32; 4]; 2],
    /// Switch between Settings tabs without a slide ([`INSTANT_TABS`]).
    instant_tabs: bool,
}

impl Default for Slide {
    fn default() -> Self {
        Slide { on: None, last: Vec::new(), old: Vec::new(), t: 2.0 * HALF, dir: 1.0, band: None, cards: [[0.0; 4]; 2], instant_tabs: INSTANT_TABS }
    }
}

impl Slide {
    /// Feeds this frame's screen and quads (`dt` in 60 Hz frames) and the Settings cards' boxes (a page of one card,
    /// Controls: the second empty); returns what to draw.
    pub(super) fn step(&mut self, screen: Screen, quads: Vec<Quad>, dt: f32, cards: [[f32; 4]; 2]) -> Vec<Quad> {
        if let Some(from) = self.on.filter(|&s| s != screen) {
            // no slide into a match (the window hides) or out of one (back to the menus)
            // M121: nor between the online select and the lobby, one screen with the cursor moving
            let still = matches!(screen, Screen::Playing) || matches!(from, Screen::Playing) || matches!((from, screen), (Screen::Chars, Screen::Lobby) | (Screen::Lobby, Screen::Chars));
            let tabs = settings(from) && settings(screen);
            self.t = if still || (tabs && self.instant_tabs) { 2.0 * HALF } else { 0.0 };
            self.dir = if screen as u8 > from as u8 { 1.0 } else { -1.0 };
            self.old = std::mem::take(&mut self.last);
            self.band = (settings(from) && settings(screen)).then_some(self.cards);
        } else {
            self.t += dt;
        }
        self.on = Some(screen);
        self.last = quads.clone();
        self.cards = cards;
        if let Some(old) = self.band.filter(|_| self.t < 2.0 * HALF) {
            let b = [old[0][1].min(cards[0][1]), (old[0][1] + old[0][3]).max(old[1][1] + old[1][3]).max(cards[0][1] + cards[0][3]).max(cards[1][1] + cards[1][3])];
            let (mut out, new): (Vec<_>, Vec<_>) = quads.into_iter().partition(|q| q.anchor == Anchor::Span || !in_band(q, b));
            // the blank under the page on view: the old one's cards going, the new one's coming
            let paper = look::c(look::PAPER);
            let blank = if self.incoming().is_some() { cards } else { old };
            out.extend(blank.into_iter().filter(|r| r[2] > 0.0).map(|dst| Quad { tex: Tex::Solid, src: [0.0; 4], dst, rgb: paper, alpha: 128.0, tile: false, anchor: Anchor::Centre, angle: 0.0, round: [18.0 * PX; 4], rim: 0.0 }));
            out.extend(match self.incoming() {
                Some((_, e)) => shift(new, 0.0, e, None),
                None => {
                    let e = ease(self.t / HALF);
                    let old = self.old.iter().copied().filter(|q| q.anchor != Anchor::Span && in_band(q, b)).collect();
                    shift(old, 0.0, 1.0 - e, None)
                }
            });
            return out;
        }
        match self.incoming() {
            Some((dx, a)) => shift(quads, dx, a, None),
            None => {
                let e = ease(self.t / HALF);
                shift(self.old.clone(), -self.dir * TRAVEL * e, 1.0 - e, None)
            }
        }
    }

    /// The new screen's x shift and alpha factor, or `None` while the old one is still leaving.
    pub(super) fn incoming(&self) -> Option<(f32, f32)> {
        (self.t >= HALF).then(|| {
            let e = ease((self.t - HALF) / HALF);
            (self.dir * TRAVEL * (1.0 - e), e)
        })
    }
}

/// The Settings tabs' pages (`Display..=Mobile`).
fn settings(s: Screen) -> bool {
    (Screen::Display as u8..=Screen::Mobile as u8).contains(&(s as u8))
}

fn in_band(q: &Quad, [top, bottom]: [f32; 2]) -> bool {
    q.dst[1] >= top && q.dst[1] < bottom
}

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn shift(mut quads: Vec<Quad>, dx: f32, a: f32, band: Option<[f32; 2]>) -> Vec<Quad> {
    for q in quads.iter_mut().filter(|q| q.anchor != Anchor::Span && band.is_none_or(|b| in_band(q, b))) {
        q.dst[0] += dx;
        q.alpha *= a;
    }
    quads
}

#[cfg(test)]
mod tests {
    use super::super::Tex;
    use super::*;

    fn quad(anchor: Anchor) -> Quad {
        Quad { tex: Tex::Solid, src: [0.0; 4], dst: [100.0, 0.0, 10.0, 10.0], rgb: [128.0; 3], alpha: 128.0, tile: false, anchor, angle: 0.0, round: [0.0; 4], rim: 0.0 }
    }

    const NONE: [[f32; 4]; 2] = [[0.0; 4]; 2];
    const CARDS: [[f32; 4]; 2] = [[0.0, 50.0, 300.0, 350.0], [320.0, 50.0, 100.0, 350.0]];

    /// Between Settings tabs only the cards (in the band) fade, in place; the bar stays the new page's; the old cards
    /// fade onto the blank panels, then the new ones come in over them, never both at once.
    #[test]
    fn tabs_fade_through_a_dim_panel() {
        let mut s = Slide::default();
        let mut bar = quad(Anchor::Centre);
        bar.dst[1] = 10.0;
        let mut card = quad(Anchor::Centre);
        card.dst[1] = 100.0;
        s.step(Screen::Display, vec![bar, card], 1.0, CARDS);
        let mut new_bar = bar;
        new_bar.rgb = [0.0; 3];
        let mut new_card = card;
        new_card.rgb = [0.0; 3];
        let mut go = |n| (0..n).map(|_| s.step(Screen::Sound, vec![new_bar, new_card], 1.0, CARDS)).last().unwrap();
        let out = go(4);
        assert_eq!(out.len(), 4);
        assert!(out[0] == new_bar && out[1].dst == CARDS[0] && out[2].dst == CARDS[1]);
        let old = out[3];
        assert!(old.rgb == card.rgb && old.alpha < 128.0 && old.alpha > 0.0 && old.dst[0] == 100.0, "old leaving: {old:?}");
        let out = go(8);
        let new = out[3];
        assert!(new.rgb == new_card.rgb && new.alpha < 128.0 && new.alpha > 0.0 && new.dst[0] == 100.0, "new coming: {new:?}");
        assert_eq!(go(5), vec![new_bar, new_card]);
    }

    /// Into a one-card page (Controls) the old page's two blanks go with it: the new card fades in over one blank.
    #[test]
    fn one_card_page_comes_in_over_its_own_blank() {
        let mut s = Slide::default();
        let mut card = quad(Anchor::Centre);
        card.dst[1] = 100.0;
        s.step(Screen::Display, vec![card], 1.0, CARDS);
        let one = [[0.0, 50.0, 420.0, 350.0], [0.0; 4]];
        let blanks = |out: &[Quad]| out.iter().filter(|q| q.rgb == look::c(look::PAPER)).map(|q| q.dst).collect::<Vec<_>>();
        let out = (0..4).map(|_| s.step(Screen::Controls, vec![card], 1.0, one)).last().unwrap();
        assert_eq!(blanks(&out), CARDS);
        let out = (0..8).map(|_| s.step(Screen::Controls, vec![card], 1.0, one)).last().unwrap();
        assert_eq!(blanks(&out), [one[0]]);
    }

    /// B626: the web build's tabs switch at once, the bar and cards the new page's from the first frame.
    #[test]
    fn web_tabs_switch_at_once() {
        let mut s = Slide { instant_tabs: true, ..Slide::default() };
        let mut card = quad(Anchor::Centre);
        card.dst[1] = 100.0;
        s.step(Screen::Display, vec![card], 1.0, CARDS);
        let mut new_card = card;
        new_card.rgb = [0.0; 3];
        assert_eq!(s.step(Screen::Sound, vec![new_card], 1.0, CARDS), vec![new_card]);
        // other screens still slide
        assert_ne!(s.step(Screen::Main, vec![card], 1.0, NONE), vec![card]);
    }

    #[test]
    fn slides_out_then_in() {
        let mut s = Slide::default();
        let main = vec![quad(Anchor::Centre), quad(Anchor::Span)];
        let mut mode = vec![quad(Anchor::Centre)];
        mode[0].dst[1] = 50.0;
        // first frame: nothing to slide from
        assert_eq!(s.step(Screen::Main, main.clone(), 1.0, NONE), main);
        // forward: Main's panel leaves to the left, fading; its wall stays
        let out = s.step(Screen::Mode, mode.clone(), 1.0, NONE);
        assert_eq!((out[0].dst[1], out[1].dst[0], out[1].alpha), (0.0, 100.0, 128.0));
        let mid = (1..=8).map(|_| s.step(Screen::Mode, mode.clone(), 1.0, NONE)).last().unwrap();
        assert!(mid[0].dst[1] == 50.0 && mid[0].dst[0] == 100.0 + TRAVEL && mid[0].alpha == 0.0);
        let half = (0..4).map(|_| s.step(Screen::Mode, mode.clone(), 1.0, NONE)).last().unwrap();
        assert!(half[0].dst[0] > 100.0 && half[0].dst[0] < 100.0 + TRAVEL);
        let done = (0..4).map(|_| s.step(Screen::Mode, mode.clone(), 1.0, NONE)).last().unwrap();
        assert_eq!(done, mode);
        // back: Mode's panel leaves to the right
        s.step(Screen::Main, main.clone(), 1.0, NONE);
        assert!(s.step(Screen::Main, main.clone(), 4.0, NONE)[0].dst[0] > 100.0);
        // into a match: no slide
        assert_eq!(s.step(Screen::Playing, main.clone(), 1.0, NONE), main);
    }
}
