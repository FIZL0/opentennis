//! M196 (ours): what next, at the match's end. F1 (concept tab "End screen", 2026-11-21): the end screen is one
//! screen, the stats board widget at full size in the middle and the pills in a row under it in the board's colours
//! (olive, the chosen one yellow in a white ring): Rematch, New Match, Return to the Menu; online Rematch and Back to
//! Lobby. A line under them says what the chosen one does. ←→ (↑↓ too) choose, ✕ picks, a click or tap on a pill
//! picks it.
//!
//! The pick goes to the front-end the match goes back to ([`take`]): the match itself only fades out as Continue
//! did. Presentation only: nothing here reads or moves the sim, its RNG or the match's clock.

use super::{Quad, Tex, WHITE};
use bevy::prelude::*;
use std::sync::Mutex;

/// What the result screen chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// The same match again: players, characters, court, umpire, length (online: once everyone is ready).
    Rematch,
    /// The character select, everyone's picks kept; court and rules on the screen after it.
    NewMatch,
    Menu,
    /// Online: the lobby, as it was.
    Lobby,
}

static NEXT: Mutex<Option<Next>> = Mutex::new(None);

/// The result screen's choice, once (the front-end takes it when the match has come back).
pub fn take() -> Option<Next> {
    NEXT.lock().unwrap().take()
}

pub fn choose(n: Next) {
    *NEXT.lock().unwrap() = Some(n);
}

/// The pills, top to bottom.
pub(super) fn pills(online: bool) -> &'static [Next] {
    if online { &[Next::Rematch, Next::Lobby] } else { &[Next::Rematch, Next::NewMatch, Next::Menu] }
}

/// The words on the pills and the notes, in [`label`]'s order: a pill's own (local, online Rematch differs only in its
/// note).
pub(super) const LABELS: [&str; 4] = ["Rematch", "New Match", "Return to the Menu", "Back to Lobby"];
pub(super) const NOTES: [&str; 5] =
    ["Same players, court and rules", "Change picks, court or rules", "Back to the main menu", "Starts when everyone is ready", "Back to the lobby"];
/// The notes' sheets are this wide (the pills' are 256).
pub(super) const NOTE_W: usize = 448;

/// Pill `n`'s word and its note, as indices into [`LABELS`] and [`NOTES`].
fn words(n: Next, online: bool) -> (usize, usize) {
    match n {
        Next::Rematch => (0, if online { 3 } else { 0 }),
        Next::NewMatch => (1, 1),
        Next::Menu => (2, 2),
        Next::Lobby => (3, 4),
    }
}

/// F1: the end screen in the stats board's coordinates (the widget's, [`super::board`]): "GAME, SET, MATCH!" over the
/// widget, the pills in a row under its bar and the chosen one's note under them. All of it is fitted to the screen
/// at one scale ([`fit`]), centred across, so it keeps its shape at any aspect ratio (the 4:3 view scales uniformly).
pub(super) const GSM_H: f32 = 28.0;
const GSM_Y: f32 = super::board::TOP - 4.0 - GSM_H;
/// "GAME, SET, MATCH!"'s sheet width (its words are set on it at 32 px high).
pub(super) const GSM_W: usize = 384;
const PILLS_Y: f32 = super::board::BOTTOM + 8.0;
const NOTE_Y: f32 = PILLS_Y + PILL_H + 5.0;
const SPAN: [f32; 2] = [GSM_Y, NOTE_Y + NOTE_H];
/// The screen's margins above and below (the key legend's strip at the bottom).
const MARGIN: [f32; 2] = [6.0, 16.0];

/// The fit's scale.
pub(super) fn scale() -> f32 {
    (448.0 - MARGIN[0] - MARGIN[1]) / (SPAN[1] - SPAN[0])
}

/// The end screen's quads from the board's coordinates to the screen's.
pub(super) fn fit(quads: &mut [Quad]) {
    let k = scale();
    for q in quads {
        let [x, y, w, h] = q.dst;
        q.dst = [320.0 + (x - 320.0) * k, MARGIN[0] + (y - SPAN[0]) * k, w * k, h * k];
    }
}

/// A screen point back in the board's coordinates.
fn unfit(p: Vec2) -> Vec2 {
    let k = scale();
    Vec2::new(320.0 + (p.x - 320.0) / k, SPAN[0] + (p.y - MARGIN[0]) / k)
}

/// "GAME, SET, MATCH!" in gold, centred over the widget (`Tex::Own(k)`, its sheet's index).
pub(super) fn banner(k: u8) -> Quad {
    let w = GSM_W as f32 * GSM_H / 32.0;
    Quad { tex: Tex::Own(k), src: [0.0, 0.0, GSM_W as f32, 32.0], dst: [320.0 - w / 2.0, GSM_Y, w, GSM_H], rgb: GOLD, alpha: 128.0 }
}

/// The board's colours (the concept's): its dark rim, a pill's olive fill, the chosen one's yellow, the words on an
/// unchosen pill, the note's, and the title's gold.
const INK: [f32; 3] = [29.0, 31.0, 22.0];
const OLIVE: [f32; 3] = [72.0, 74.0, 54.0];
const CHOSEN: [f32; 3] = [111.0, 114.0, 58.0];
const DIM_WORDS: [f32; 3] = [106.0, 108.0, 92.0];
const NOTE_RGB: [f32; 3] = [116.0, 118.0, 104.0];
const GOLD: [f32; 3] = [121.0, 91.0, 25.0];
/// The words' size on a pill (the pause dialog's: 20 of its 32 px high), and the note's.
const FIT: f32 = 0.625;
const NOTE_H: f32 = 15.0;
const PILL_H: f32 = 32.0;
const GAP: f32 = 16.0;
/// Each pill's width, by its word ([`LABELS`]).
const PILL_W: [f32; 4] = [128.0, 144.0, 208.0, 176.0];

/// The pills' rects in a row, centred, in the board's coordinates.
fn rects(online: bool) -> Vec<[f32; 4]> {
    let ws: Vec<f32> = pills(online).iter().map(|&n| PILL_W[words(n, online).0]).collect();
    let total = ws.iter().sum::<f32>() + GAP * (ws.len() as f32 - 1.0);
    let mut x = 320.0 - total / 2.0;
    ws.iter()
        .map(|&w| {
            let r = [x, PILLS_Y, w, PILL_H];
            x += w + GAP;
            r
        })
        .collect()
}

/// The pill under screen point `p` (640×448).
pub(super) fn pill_at(online: bool, p: Vec2) -> Option<usize> {
    let p = unfit(p);
    rects(online).iter().position(|&[x, y, w, h]| (x..x + w).contains(&p.x) && (y..y + h).contains(&p.y))
}

/// The pills (`sel` chosen: yellow in a white ring, its words white; the rest olive, their words pale), and the chosen
/// one's note, in the board's coordinates.
pub(super) fn row(online: bool, sel: usize) -> Vec<Quad> {
    let mut out = Vec::new();
    let opts = pills(online);
    for (k, (&n, r)) in opts.iter().zip(rects(online)).enumerate() {
        let chosen = k == sel;
        let ring = if chosen { WHITE } else { OLIVE };
        let fill = if chosen { CHOSEN } else { OLIVE };
        for (i, rgb) in [INK, ring, fill].into_iter().enumerate() {
            out.extend(super::super::menu_art::pill_quads(i, r).iter().map(|&(src, dst)| Quad { tex: Tex::Pill, src, dst, rgb, alpha: 128.0 }));
        }
        let (w, h) = (256.0 * FIT, 32.0 * FIT);
        let rgb = if chosen { WHITE } else { DIM_WORDS };
        let src = [0.0, 0.0, 256.0, 32.0];
        out.push(Quad { tex: Tex::Own(words(n, online).0 as u8), src, dst: [r[0] + (r[2] - w) / 2.0, r[1] + (r[3] - h) / 2.0, w, h], rgb, alpha: 128.0 });
    }
    if let Some(&n) = opts.get(sel) {
        let w = NOTE_W as f32 * NOTE_H / 32.0;
        let src = [0.0, 0.0, NOTE_W as f32, 32.0];
        let tex = Tex::Own((LABELS.len() + words(n, online).1) as u8);
        out.push(Quad { tex, src, dst: [320.0 - w / 2.0, NOTE_Y, w, NOTE_H], rgb: NOTE_RGB, alpha: 128.0 });
    }
    out
}

/// ↑ (−1) / ↓ (1) this frame: keys, the d-pad, the left stick past half way (`last`: its last y).
pub(super) fn updown(keys: &ButtonInput<KeyCode>, gamepads: &Query<&Gamepad>, last: &mut f32) -> i32 {
    let any = |b: GamepadButton| gamepads.iter().filter(|g| super::super::controls::readable(g)).any(|p| p.just_pressed(b));
    let y = gamepads.iter().filter(|g| super::super::controls::readable(g)).filter_map(|p| p.get(GamepadAxis::LeftStickY)).find(|y| y.abs() >= 0.5).unwrap_or(0.0);
    let pushed = |s: f32| y * s >= 0.5 && *last * s < 0.5;
    let up = keys.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) || any(GamepadButton::DPadUp) || pushed(1.0);
    let down = keys.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) || any(GamepadButton::DPadDown) || pushed(-1.0);
    *last = y;
    down as i32 - up as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pills sit in a row under the bar, fitted inside the 4:3 screen with the banner; a tap on each pill's
    /// middle finds it, off them nothing.
    #[test]
    fn row_fits_under_the_board() {
        for online in [false, true] {
            let mut quads = row(online, 0);
            quads.push(banner(9));
            assert!(quads.iter().all(|q| q.dst[1] >= SPAN[0] - 0.01 && q.dst[1] + q.dst[3] <= SPAN[1] + 0.01));
            fit(&mut quads);
            for q in &quads {
                let [x, y, w, h] = q.dst;
                assert!(x >= 0.0 && x + w <= 640.0 && y >= MARGIN[0] - 0.01 && y + h <= 448.0 - MARGIN[1] + 0.01, "{:?}", q.dst);
            }
            for (k, r) in rects(online).into_iter().enumerate() {
                let mut q = [Quad { tex: Tex::Pill, src: [0.0; 4], dst: r, rgb: WHITE, alpha: 128.0 }];
                fit(&mut q);
                let [x, y, w, h] = q[0].dst;
                assert_eq!(pill_at(online, Vec2::new(x + w / 2.0, y + h / 2.0)), Some(k));
            }
            assert_eq!(pill_at(online, Vec2::new(320.0, 200.0)), None);
        }
        // the board at most a fifth smaller than its own size
        assert!(scale() > 0.8, "{}", scale());
    }
}
