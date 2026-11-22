//! M196 (ours): what next, at the match's end: Rematch, New Match, Return to the Menu; online Rematch and Back to
//! Lobby. B764: in the pause menu's box on the left of the board widget, with its pills, words, pointing hand and
//! colours, a line under the pills in the box saying what the chosen one does. ↑↓ (←→ too) choose, ✕ picks, a click
//! or tap on a pill picks it.
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

/// B764: the menu box, the pause menu's (its box, pills, words and pointing hand, in its colours): 320 wide, a pill
/// 288 wide every 40 px from 20 below its top, the chosen one's note under them in the box. Laid out with the box's
/// left at x 0 and its top at y 0; [`compose`] puts it left of the board widget.
const BOX_W: f32 = 320.0;
const PILL_X: f32 = 16.0;
const PILL_W: f32 = 288.0;
const PILL_H: f32 = 32.0;
const STEP: f32 = 40.0;
const FIRST: f32 = 20.0;
/// The note: 6 under the last pill, 15 high (its sheet's 32 px row at that height), the box's bottom 20 under it.
const NOTE_GAP: f32 = 6.0;
const NOTE_H: f32 = 15.0;
/// The pause menu's colours: the rims, the box's fill, a chosen and an unchosen pill (the words on an unchosen one).
const INK: [f32; 3] = [15.0; 3];
const PANEL: [f32; 3] = [52.0, 44.0, 36.0];
const CHOSEN: [f32; 3] = [123.0, 105.0, 79.0];
const UNCHOSEN: [f32; 3] = [94.0, 83.0, 70.0];
/// The words' size on a pill (the pause dialog's: 20 of its 32 px high).
const FIT: f32 = 0.625;
/// The gap between the box and the board, and how far left of the box the hand reaches (the pause's: 48 left of the
/// pill, then the 3D hand's own 12).
const GAP: f32 = 24.0;
const HAND_ROOM: f32 = 44.0;
/// The screen's margins above and below (the key legend's strip at the bottom), and at the sides.
const MARGIN: [f32; 2] = [6.0, 16.0];
const SIDE: f32 = 8.0;

/// The pause menu's pointing hand (its 3D hand's picture), which the front-end shares: without one the screen has no
/// hand.
#[derive(Resource, Clone)]
pub struct Pointer(pub Handle<Image>);

/// The pills' rects on the screen (640×448) as last drawn, for clicks and taps.
#[derive(Resource, Default)]
pub(super) struct Hits(pub(super) Vec<[f32; 4]>);

fn box_h(n: usize) -> f32 {
    FIRST + STEP * n as f32 - (STEP - PILL_H) + NOTE_GAP + NOTE_H + 20.0
}

/// The pills' rects, in the box's coordinates.
fn rects(online: bool) -> Vec<[f32; 4]> {
    (0..pills(online).len()).map(|k| [PILL_X, FIRST + STEP * k as f32, PILL_W, PILL_H]).collect()
}

/// The menu box with `sel` chosen: the box, the pills (the chosen one bright), their words (the chosen white, the rest
/// dim), the hand at the chosen pill and its note, in the box's coordinates.
pub(super) fn menu(online: bool, sel: usize) -> Vec<Quad> {
    let mut out = Vec::new();
    let opts = pills(online);
    let quad = |tex, src, dst, rgb, alpha| Quad { tex, src, dst, rgb, alpha };
    for (i, rgb) in [INK, WHITE, PANEL].into_iter().enumerate() {
        out.extend(super::super::menu_art::box_quads(i, [0.0, 0.0, BOX_W, box_h(opts.len())], 16.0).into_iter().map(|(src, dst)| quad(Tex::Pill, src, dst, rgb, 128.0)));
    }
    for (k, r) in rects(online).into_iter().enumerate() {
        let fill = if k == sel { CHOSEN } else { UNCHOSEN };
        for (i, rgb) in [INK, WHITE, fill].into_iter().enumerate() {
            out.extend(super::super::menu_art::pill_quads(i, r).iter().map(|&(src, dst)| quad(Tex::Pill, src, dst, rgb, 128.0)));
        }
    }
    for (k, (&n, r)) in opts.iter().zip(rects(online)).enumerate() {
        let (rgb, alpha) = if k == sel { (WHITE, 128.0) } else { (UNCHOSEN, 64.0) };
        let (w, h) = (256.0 * FIT, 32.0 * FIT);
        let dst = [r[0] + (r[2] - w) / 2.0, r[1] + (r[3] - h) / 2.0, w, h];
        out.push(quad(Tex::Own(words(n, online).0 as u8), [0.0, 0.0, 256.0, 32.0], dst, rgb, alpha));
    }
    if let Some((&n, r)) = opts.get(sel).zip(rects(online).get(sel)) {
        // the hand as the pause menu draws its 3D one: its 64×32 quad 48 left of the pill and 8 down, moved 12 left
        // and 5 up, the picture three times the quad round its middle
        let (x, y) = (r[0] - 48.0 - 12.0, r[1] + 8.0 - 5.0);
        out.push(quad(Tex::Hand, [0.0; 4], [x - 64.0, y - 32.0, 192.0, 96.0], WHITE, 128.0));
        let w = NOTE_W as f32 * NOTE_H / 32.0;
        let y = FIRST + STEP * opts.len() as f32 - (STEP - PILL_H) + NOTE_GAP;
        let tex = Tex::Own((LABELS.len() + words(n, online).1) as u8);
        out.push(quad(tex, [0.0, 0.0, NOTE_W as f32, 32.0], [BOX_W / 2.0 - w / 2.0, y, w, NOTE_H], UNCHOSEN.map(|c| c + 24.0), 128.0));
    }
    out
}

/// The end screen: the menu box left of the board widget (`widget`), its middle level with the widget's; all of it
/// and the pills' rects in the widget's coordinates.
pub(super) fn compose(mut widget: Vec<Quad>, online: bool, sel: usize) -> (Vec<Quad>, Vec<[f32; 4]>) {
    let (l, t, _, b) = bounds(&widget);
    let (x, y) = (l - GAP - BOX_W, (t + b) / 2.0 - box_h(pills(online).len()) / 2.0);
    let moved = |[qx, qy, w, h]: [f32; 4]| [qx + x, qy + y, w, h];
    widget.extend(menu(online, sel).into_iter().map(|q| Quad { dst: moved(q.dst), ..q }));
    (widget, rects(online).into_iter().map(moved).collect())
}

/// The quads' extent (left, top, right, bottom), the hand's picture left out but room for the hand itself.
fn bounds(quads: &[Quad]) -> (f32, f32, f32, f32) {
    let mut e = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for q in quads.iter().filter(|q| q.tex != Tex::Hand) {
        let [x, y, w, h] = q.dst;
        e = (e.0.min(x), e.1.min(y), e.2.max(x + w), e.3.max(y + h));
    }
    if quads.iter().any(|q| q.tex == Tex::Hand) {
        e.0 -= HAND_ROOM;
    }
    e
}

/// The 640×448 screen's visible part in a window of `aspect` (the 4:3 screen's units, round its middle).
pub(super) fn visible(aspect: f32) -> Vec2 {
    Vec2::new(640.0 / hst_base::widescreen::share(aspect), 448.0 / hst_base::widescreen::vshare(aspect))
}

/// Fits the end screen (`quads`, `rects`) to the visible part `vis` of the screen at one scale, no larger than its own
/// size, centred: it keeps its shape at any aspect ratio, the room left over goes round it.
pub(super) fn fit(quads: &mut [Quad], rects: &mut [[f32; 4]], vis: Vec2) {
    let (l, t, r, b) = bounds(quads);
    let room = Vec2::new(vis.x - 2.0 * SIDE, vis.y - MARGIN[0] - MARGIN[1]);
    let k = (room.x / (r - l)).min(room.y / (b - t)).min(1.0);
    let x0 = 320.0 - (r - l) * k / 2.0;
    let y0 = 224.0 - vis.y / 2.0 + MARGIN[0] + (room.y - (b - t) * k) / 2.0;
    let map = |[x, y, w, h]: [f32; 4]| [x0 + (x - l) * k, y0 + (y - t) * k, w * k, h * k];
    for q in quads {
        q.dst = map(q.dst);
    }
    for r in rects {
        *r = map(*r);
    }
}

/// The pill under screen point `p` (640×448).
pub(super) fn pill_at(hits: &Hits, p: Vec2) -> Option<usize> {
    hits.0.iter().position(|&[x, y, w, h]| (x..x + w).contains(&p.x) && (y..y + h).contains(&p.y))
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

    /// The pills stacked in the box, the hand at the chosen one, the note under them inside the box; fitted with a
    /// board-sized block at 16:9, 4:3 and 21:9, all of it on the screen at one scale, the box left of the board, and a
    /// tap on each pill's middle finds it.
    #[test]
    fn box_fits_beside_the_board() {
        for online in [false, true] {
            let n = pills(online).len();
            let quads = menu(online, 1);
            let boxed = quads.iter().filter(|q| q.tex != Tex::Hand).all(|q| q.dst[0] >= 0.0 && q.dst[0] + q.dst[2] <= BOX_W && q.dst[1] >= 0.0 && q.dst[1] + q.dst[3] <= box_h(n) + 0.01);
            assert!(boxed);
            let hand = quads.iter().find(|q| q.tex == Tex::Hand).unwrap();
            assert_eq!(hand.dst[1] + hand.dst[3] / 2.0, FIRST + STEP + 3.0 + 16.0);
            for aspect in [16.0 / 9.0, 4.0 / 3.0, 21.0 / 9.0] {
                let block = Quad { tex: Tex::Back, src: [0.0; 4], dst: [40.0, 0.0, 560.0, 520.0], rgb: WHITE, alpha: 128.0 };
                let (mut quads, mut rects) = compose(vec![block], online, 0);
                let vis = visible(aspect);
                fit(&mut quads, &mut rects, vis);
                let k = quads[0].dst[2] / 560.0;
                assert!(k > 0.6 && k <= 1.0, "{aspect} {k}");
                for q in quads.iter().filter(|q| q.tex != Tex::Hand) {
                    let [x, y, w, h] = q.dst;
                    assert!(x >= 320.0 - vis.x / 2.0 && x + w <= 320.0 + vis.x / 2.0 + 0.01 && y >= MARGIN[0] - 0.01 && y + h <= 448.0 - MARGIN[1] + 0.01, "{aspect} {:?}", q.dst);
                }
                assert!(rects.iter().all(|r| r[0] + r[2] < quads[0].dst[0]));
                let hits = Hits(rects.clone());
                for (i, r) in rects.iter().enumerate() {
                    assert_eq!(pill_at(&hits, Vec2::new(r[0] + r[2] / 2.0, r[1] + r[3] / 2.0)), Some(i));
                }
                assert_eq!(pill_at(&hits, Vec2::new(quads[0].dst[0] + 10.0, 200.0)), None);
            }
        }
    }
}
