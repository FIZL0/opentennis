//! The slide between menu screens (B40e2): the last screen's panels slide off and fade, then the new screen's slide
//! in from the other side. Ours, not the original's (menus need not be 1:1, user 2026-10-12): the wall and title bar
//! (`Anchor::Span`) switch at the midpoint without moving; a forward step (a later `Screen`) moves left, back moves right.
//! Between two Settings tabs only the page's cards move, and only a little: the tab bar and legend stay put, and the
//! new cards slide in at full strength under the old ones as those slide out and fade, so the wall never shows through.

use bevy::prelude::*;

use super::super::widescreen::Anchor;
use super::{Quad, Screen};

/// Frames (60 Hz) for each half: out, then in.
const HALF: f32 = 8.0;
/// How far the panels travel (640×448 units) while fading.
const TRAVEL: f32 = 160.0;
/// How far a Settings tab's cards travel.
const TAB_TRAVEL: f32 = 24.0;

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
    /// Between Settings tabs: the cards' y band (only quads starting in it move).
    band: Option<[f32; 2]>,
}

impl Default for Slide {
    fn default() -> Self {
        Slide { on: None, last: Vec::new(), old: Vec::new(), t: 2.0 * HALF, dir: 1.0, band: None }
    }
}

impl Slide {
    /// Feeds this frame's screen and quads (`dt` in 60 Hz frames) and the Settings cards' y band; returns what to draw.
    pub(super) fn step(&mut self, screen: Screen, quads: Vec<Quad>, dt: f32, cards: [f32; 2]) -> Vec<Quad> {
        if let Some(from) = self.on.filter(|&s| s != screen) {
            // no slide into a match (the window hides) or out of one (back to the menus)
            // M121: nor between the online select and the lobby, one screen with the cursor moving
            let still = matches!(screen, Screen::Playing) || matches!(from, Screen::Playing) || matches!((from, screen), (Screen::Chars, Screen::Lobby) | (Screen::Lobby, Screen::Chars));
            self.t = if still { 2.0 * HALF } else { 0.0 };
            self.dir = if screen as u8 > from as u8 { 1.0 } else { -1.0 };
            self.old = std::mem::take(&mut self.last);
            self.band = (settings(from) && settings(screen)).then_some(cards);
        } else {
            self.t += dt;
        }
        self.on = Some(screen);
        self.last = quads.clone();
        if let Some(b) = self.band.filter(|_| self.t < 2.0 * HALF) {
            // the old cards drawn last, over the new
            let e = ease(self.t / (2.0 * HALF));
            let mut out = shift(quads, self.dir * TAB_TRAVEL * (1.0 - e), 1.0, Some(b));
            out.extend(shift(self.old.clone(), -self.dir * TAB_TRAVEL * e, 1.0 - e, Some(b)).into_iter().filter(|q| in_band(q, b)));
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

    const NONE: [f32; 2] = [0.0; 2];

    /// Between Settings tabs only the cards (in the band) move, and less; the bar stays the new page's; the new cards
    /// are whole from the first frame, the old ones over them fading.
    #[test]
    fn tabs_slide_only_the_cards() {
        let mut s = Slide::default();
        let mut bar = quad(Anchor::Centre);
        bar.dst[1] = 10.0;
        let mut card = quad(Anchor::Centre);
        card.dst[1] = 100.0;
        s.step(Screen::Display, vec![bar, card], 1.0, [50.0, 400.0]);
        let mut new_bar = bar;
        new_bar.rgb = [0.0; 3];
        let out = (0..8).map(|_| s.step(Screen::Sound, vec![new_bar, card], 1.0, [50.0, 400.0])).last().unwrap();
        assert!(out.contains(&new_bar) && !out.contains(&bar));
        let [new, old] = [1, 2].map(|k| out[k]);
        assert!(new.alpha == 128.0 && new.dst[0] > 100.0 && new.dst[0] < 100.0 + TAB_TRAVEL, "new under, whole: {new:?}");
        assert!(old.alpha < 128.0 && old.alpha > 0.0 && old.dst[0] < 100.0 && old.dst[0] > 100.0 - TAB_TRAVEL, "old over, fading: {old:?}");
        let done = (0..9).map(|_| s.step(Screen::Sound, vec![new_bar, card], 1.0, [50.0, 400.0])).last().unwrap();
        assert_eq!(done, vec![new_bar, card]);
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
