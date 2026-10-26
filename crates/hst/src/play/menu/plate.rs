//! B671 (ours, the user's board B): the pause screen's score panel says who is playing. On each plate, where the
//! original drew its 1P..4P / COM tag, the player's own name (picked in the main menu; else the tag's words, a CPU's
//! always "COM") in the menus' font, white in the plate's own team colour; under the face the character's name (a mod
//! by its own name) on a navy pill. Long names shrink to fit. The plates, frame and centre stay the original's art.
//! Drawing only: nothing here touches the match.

use bevy::prelude::*;

use super::super::name_tags::{self, Names};
use super::{Game, Pads};
use crate::Args;

/// The plates' dark edges by player (the board's plate art: pink, blue, orange, green).
const EDGE: [[u8; 3]; 4] = [[0x8a, 0x1f, 0x52], [0x14, 0x50, 0x7e], [0x7a, 0x3f, 0x0a], [0x3b, 0x62, 0x13]];
/// The character pill's fill.
const NAVY: [u8; 3] = [0x16, 0x21, 0x3d];
/// The name's height (disc px, outline included), the tag cell's less a pixel either side, and its outline.
pub(super) const NAME_H: f32 = 22.0;
const NAME_EW: f32 = 2.0;
/// The character pill's height (disc px).
pub(super) const PILL_H: f32 = 11.0;

/// Each player's words as set: the name and the character pill, their images and their widths at `NAME_H` and
/// `PILL_H`.
#[derive(Resource, Default)]
pub(super) struct Plates {
    made: [Option<[String; 2]>; 4],
    pub(super) name: [Option<(Handle<Image>, f32)>; 4],
    pub(super) chara: [Option<(Handle<Image>, f32)>; 4],
}

impl Plates {
    /// Player `p`'s widths (name, pill) once both are set.
    pub(super) fn widths(&self, p: usize) -> Option<[f32; 2]> {
        Some([self.name.get(p)?.as_ref()?.1, self.chara.get(p)?.as_ref()?.1])
    }
}

/// Player `i`'s character in words: a mod by its name, a disc one by the select's.
fn character(args: &Args, mods: Option<&crate::mods::MatchMod>, i: usize) -> String {
    match mods.and_then(|m| m.get(i)) {
        Some(m) => m.name.clone(),
        None => {
            let c = args.chars.get(i).copied().unwrap_or([6, 1, 2, 3][i.min(3)]);
            super::super::main_menu::CHAR_NAMES.get(c).copied().unwrap_or("?").to_string()
        }
    }
}

/// The plate's words: a CPU is "COM", a human their name, or their seat's tag without one.
pub(super) fn who(slot: usize, name: Option<&str>) -> String {
    match name {
        Some(n) if slot < 4 => n.to_string(),
        _ => name_tags::slot_label(slot).to_string(),
    }
}

/// Sets the words again when they change (a controller handed over, a mod loaded), only while paused.
#[allow(clippy::too_many_arguments)]
pub(super) fn refresh(
    menu: Res<super::Menu>,
    g: Res<Game>,
    pads: Res<Pads>,
    args: Res<Args>,
    names: Option<Res<Names>>,
    mods: Option<Res<crate::mods::MatchMod>>,
    mut plates: ResMut<Plates>,
    mut images: ResMut<Assets<Image>>,
) {
    if !menu.open {
        return;
    }
    let n = g.players.len().min(4);
    for (p, edge) in EDGE.into_iter().enumerate().take(n) {
        let slot = pads.slot_of(p, n).unwrap_or(4);
        let want = [who(slot, names.as_deref().and_then(|s| s.get(p))), character(&args, mods.as_deref(), p)];
        if plates.made[p].as_ref() == Some(&want) {
            continue;
        }
        let (img, w) = name_tags::panel_words(&want[0], edge, NAME_H, NAME_EW);
        plates.name[p] = Some((images.add(img), w));
        let (img, w) = name_tags::panel_pill(&want[1], NAVY, PILL_H);
        plates.chara[p] = Some((images.add(img), w));
        plates.made[p] = Some(want);
    }
}

/// Player `p`'s name and pill on the board, face at `x, y` (48 px), `w` their widths: the name in the tag's room
/// between the face and the centre (the left team's against the centre, the right team's from it), the pill centred
/// under the face, kept on its plate. Each shrinks (about its own middle) to fit.
pub(super) fn place(p: usize, x: f32, y: f32, [nw, pw]: [f32; 2]) -> [[f32; 4]; 2] {
    let left = p & 1 == 0;
    let (room_l, room_r) = if left { (146.0, 183.0) } else { (457.0, 494.0) };
    let k = ((room_r - room_l) / nw).min(1.0);
    let (w, h) = (nw * k, NAME_H * k);
    let name = [if left { room_r - w } else { room_l }, y + 18.0 - h / 2.0, w, h];
    let (plate_l, plate_r) = if left { (96.0, 184.0) } else { (456.0, 544.0) };
    let k = ((plate_r - plate_l - 4.0) / pw).min(1.0);
    let (w, h) = (pw * k, PILL_H * k);
    let px = (x + 24.0 - w / 2.0).clamp(plate_l + 2.0, plate_r - 2.0 - w);
    let pill = [px, y + 42.0 + (PILL_H - h) / 2.0, w, h];
    [name, pill]
}
