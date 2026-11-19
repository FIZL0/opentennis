//! M196 (ours, concept tab "Match end"): what next, beside the stats. Once the stats page has been seen it moves to
//! the right of the screen (shrunk to fit), and the left gets a box of pills in the pause dialog's style (our box and
//! pills, the chosen one bright): Rematch, New Match, Return to the Menu; online Rematch and Back to Lobby. A line
//! under them says what the chosen one does. ↑↓ choose, ✕ picks, a click or tap on a pill picks it.
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

/// The ticks the stats page takes to move aside (and the pills to come in).
pub(super) const ASIDE: i32 = 20;

/// The stats page's own extent (its column frames and bar, 640×448), and where it goes: scaled by `SCALE` with its
/// top-left at `TO`, its right edge 20 px in from the screen's.
const FROM: [f32; 2] = [32.0, 72.0];
const SCALE: f32 = 0.66;
const TO: [f32; 2] = [240.0, 224.0 - 320.0 * SCALE / 2.0];

/// The pause dialog's colours (ours): rims, its fill, a pill's fill chosen and not.
const INK: [f32; 3] = [15.0; 3];
const PANEL: [f32; 3] = [52.0, 44.0, 36.0];
const CHOSEN: [f32; 3] = [123.0, 105.0, 79.0];
const UNCHOSEN: [f32; 3] = [94.0, 83.0, 70.0];
/// The words' size on a pill (the pause dialog's: 20 of its 32 px high), and the note's.
const FIT: f32 = 0.625;
const NOTE_H: f32 = 15.0;

const BOX_X: f32 = 16.0;
const BOX_W: f32 = 216.0;
const PILL_W: f32 = 192.0;
const PILL_H: f32 = 32.0;
const PITCH: f32 = 40.0;

/// How far along the move is, eased (0 the page centred, 1 aside).
pub(super) fn eased(aside: i32) -> f32 {
    let t = (aside as f32 / ASIDE as f32).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The stats page's quads moved `t` of the way aside.
pub(super) fn shrink(page: &mut [Quad], t: f32) {
    if t <= 0.0 {
        return;
    }
    let s = 1.0 + (SCALE - 1.0) * t;
    for q in page {
        let [x, y, w, h] = q.dst;
        let to = [TO[0] + (x - FROM[0]) * SCALE, TO[1] + (y - FROM[1]) * SCALE];
        q.dst = [x + (to[0] - x) * t, y + (to[1] - y) * t, w * s, h * s];
    }
}

/// The box's top, its pills' tops and the note's, for `n` pills (centred on the screen's middle).
fn rows(n: usize) -> (f32, Vec<f32>, f32) {
    let top = 224.0 - box_h(n) / 2.0;
    let ys = (0..n).map(|k| top + 16.0 + PITCH * k as f32).collect();
    (top, ys, top + 16.0 + PITCH * n as f32 - 8.0 + 10.0)
}

fn box_h(n: usize) -> f32 {
    16.0 + PITCH * n as f32 - 8.0 + 10.0 + NOTE_H + 14.0
}

/// The pill under screen point `p` (640×448), with the box fully in.
pub(super) fn pill_at(online: bool, p: Vec2) -> Option<usize> {
    let (_, ys, _) = rows(pills(online).len());
    let x = BOX_X + (BOX_W - PILL_W) / 2.0;
    ys.iter().position(|&y| (x..x + PILL_W).contains(&p.x) && (y..y + PILL_H).contains(&p.y))
}

/// The box, its pills (`sel` chosen), their words and the chosen one's note, `t` of the way in (sliding in from the
/// left and fading up), `dx` across with the page.
pub(super) fn panel(online: bool, sel: usize, t: f32, dx: f32) -> Vec<Quad> {
    let mut out = Vec::new();
    if t <= 0.0 {
        return out;
    }
    let opts = pills(online);
    let (top, ys, note_y) = rows(opts.len());
    let x0 = BOX_X + dx - (1.0 - t) * (BOX_X + BOX_W);
    let a = 128.0 * t;
    let layer = |out: &mut Vec<Quad>, quads: &[([f32; 4], [f32; 4])], rgb: [f32; 3]| {
        out.extend(quads.iter().map(|&(src, dst)| Quad { tex: Tex::Pill, src, dst, rgb, alpha: a }));
    };
    for (i, rgb) in [INK, WHITE, PANEL].into_iter().enumerate() {
        layer(&mut out, &super::super::menu_art::box_quads(i, [x0, top, BOX_W, box_h(opts.len())], 16.0), rgb);
    }
    let px = x0 + (BOX_W - PILL_W) / 2.0;
    for (k, &y) in ys.iter().enumerate() {
        let fill = if k == sel { CHOSEN } else { UNCHOSEN };
        for (i, rgb) in [INK, WHITE, fill].into_iter().enumerate() {
            layer(&mut out, &super::super::menu_art::pill_quads(i, [px, y, PILL_W, PILL_H]), rgb);
        }
    }
    for (k, (&n, &y)) in opts.iter().zip(&ys).enumerate() {
        let (rgb, alpha) = if k == sel { (WHITE, a) } else { (UNCHOSEN, a / 2.0) };
        let (w, h) = (256.0 * FIT, 32.0 * FIT);
        let src = [0.0, 0.0, 256.0, 32.0];
        out.push(Quad { tex: Tex::Own(words(n, online).0 as u8), src, dst: [px + (PILL_W - w) / 2.0, y + (PILL_H - h) / 2.0, w, h], rgb, alpha });
    }
    if let Some(&n) = opts.get(sel) {
        let w = NOTE_W as f32 * NOTE_H / 32.0;
        let src = [0.0, 0.0, NOTE_W as f32, 32.0];
        let tex = Tex::Own((LABELS.len() + words(n, online).1) as u8);
        out.push(Quad { tex, src, dst: [x0 + (BOX_W - w) / 2.0, note_y, w, NOTE_H], rgb: [110.0, 104.0, 92.0], alpha: a });
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

    /// Every pill sits inside the box and the box inside the 4:3 screen's left, clear of the stats page once aside.
    #[test]
    fn panel_fits_left_of_the_page() {
        for online in [false, true] {
            let n = pills(online).len();
            let quads = panel(online, 0, 1.0, 0.0);
            let right = quads.iter().map(|q| q.dst[0] + q.dst[2]).fold(0.0, f32::max);
            let (lo, hi) = (quads.iter().map(|q| q.dst[1]).fold(f32::MAX, f32::min), quads.iter().map(|q| q.dst[1] + q.dst[3]).fold(0.0, f32::max));
            assert!(right <= TO[0] - 4.0 && quads.iter().all(|q| q.dst[0] >= BOX_X - 0.01), "{online}: {right}");
            assert!(lo >= 0.0 && hi <= 448.0);
            // a tap on each pill's middle finds it
            let (_, ys, _) = rows(n);
            for (k, y) in ys.iter().enumerate() {
                assert_eq!(pill_at(online, Vec2::new(124.0, y + 16.0)), Some(k));
            }
            assert_eq!(pill_at(online, Vec2::new(400.0, ys[0] + 16.0)), None);
        }
    }

    /// Aside, the stats page (32..608 × 72..392) ends inside the screen, right of the box; at 0 it's untouched.
    #[test]
    fn page_moves_aside() {
        let q = |x, y, w, h| Quad { tex: Tex::Frame, src: [0.0; 4], dst: [x, y, w, h], rgb: WHITE, alpha: 128.0 };
        let mut page = [q(32.0, 72.0, 16.0, 16.0), q(592.0, 376.0, 16.0, 16.0)];
        shrink(&mut page, 0.0);
        assert_eq!(page[0].dst, [32.0, 72.0, 16.0, 16.0]);
        shrink(&mut page, 1.0);
        assert!((page[0].dst[0] - TO[0]).abs() < 0.01 && page[0].dst[1] >= 0.0);
        let [x, y, w, h] = page[1].dst;
        assert!(x + w <= 630.0 && y + h <= 448.0, "{:?}", page[1].dst);
        assert_eq!((eased(0), eased(ASIDE), eased(ASIDE * 2)), (0.0, 1.0, 1.0));
    }
}
