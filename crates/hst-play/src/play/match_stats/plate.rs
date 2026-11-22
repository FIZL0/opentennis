//! B764: the end screen's player plates, as the pause screen's score board sets them (its `menu/plate.rs`, in the
//! front-end crate, which this one cannot name): on each plate, where the original drew its 1P..4P / COM tag, the
//! player's own name (else the tag's words, a CPU's always "COM") in the menus' font, white in the plate's team
//! colour; under the face the character's name (a mod by its own name) on a navy pill. Long names shrink to fit.
//! Drawing only: nothing here touches the match.

use bevy::prelude::*;

use super::super::name_tags::{self, Names};
use super::super::{Game, Pads};
use crate::Args;

/// The plates' dark edges by player (the board's plate art: pink, blue, orange, green), the pause plates'.
const EDGE: [[u8; 3]; 4] = [[0x8a, 0x1f, 0x52], [0x14, 0x50, 0x7e], [0x7a, 0x3f, 0x0a], [0x3b, 0x62, 0x13]];
/// The character pill's fill, the pause plates'.
const NAVY: [u8; 3] = [0x16, 0x21, 0x3d];
/// The name's height (disc px, outline included) and its outline; the character pill's height. The pause plates'.
pub(super) const NAME_H: f32 = 22.0;
const NAME_EW: f32 = 2.0;
pub(super) const PILL_H: f32 = 11.0;

/// The disc's characters by the select's order, as the character select names them.
const CHARACTERS: [&str; 14] =
    ["Ashley", "Cody", "Jun", "Kaito", "Momoko", "JJ", "Carol", "Big Chief", "Miranda", "Kent", "Lola", "Will", "Gloria", "Suzuki"];

/// Each player's words as set: the name and the character pill, their images and widths at `NAME_H` and `PILL_H`.
#[derive(Resource, Default)]
pub(super) struct Words {
    made: [Option<[String; 2]>; 4],
    pub(super) name: [Option<(Handle<Image>, f32)>; 4],
    pub(super) chara: [Option<(Handle<Image>, f32)>; 4],
}

impl Words {
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
            CHARACTERS.get(c).copied().unwrap_or("?").to_string()
        }
    }
}

/// The plate's words: a CPU is "COM", a human their name, or their seat's tag without one.
fn who(slot: usize, name: Option<&str>) -> String {
    match name {
        Some(n) if slot < 4 => n.to_string(),
        _ => name_tags::slot_label(slot).to_string(),
    }
}

/// Sets the words again when they change, once the match has a result (F2: or while the pause board is up).
pub(super) fn refresh(
    g: Res<Game>,
    pads: Res<Pads>,
    args: Res<Args>,
    names: Option<Res<Names>>,
    mods: Option<Res<crate::mods::MatchMod>>,
    mut words: ResMut<Words>,
    mut images: ResMut<Assets<Image>>,
    pause: Option<Res<super::pause_board::PauseBoard>>,
) {
    // F2: the pause screen's board shows them too
    if g.match_stats.result.is_none() && !pause.is_some_and(|p| p.open) {
        return;
    }
    let n = g.players.len().min(4);
    for (p, edge) in EDGE.into_iter().enumerate().take(n) {
        let slot = pads.slot_of(p, n).unwrap_or(4);
        let want = [who(slot, names.as_deref().and_then(|s| s.get(p))), character(&args, mods.as_deref(), p)];
        if words.made[p].as_ref() == Some(&want) {
            continue;
        }
        let (img, w) = name_tags::panel_words(&want[0], edge, NAME_H, NAME_EW);
        words.name[p] = Some((images.add(img), w));
        let (img, w) = name_tags::panel_pill(&want[1], NAVY, PILL_H);
        words.chara[p] = Some((images.add(img), w));
        words.made[p] = Some(want);
    }
}

/// A plate's name and pill, as the pause board places them: the name in `room` (x from, to) beside the face, against
/// its inner end on the left team (`left`), from its outer end on the right; its middle `name_y`; the pill centred
/// under the 48-px ring at `ring` (x, y), kept on the `plate` (x from, to). Each shrinks about its middle to fit;
/// `[nw, pw]` their widths.
pub(super) fn place(left: bool, ring: [f32; 2], name_y: f32, room: [f32; 2], plate: [f32; 2], [nw, pw]: [f32; 2]) -> [[f32; 4]; 2] {
    let k = ((room[1] - room[0]) / nw).min(1.0);
    let (w, h) = (nw * k, NAME_H * k);
    let name = [if left { room[1] - w } else { room[0] }, name_y - h / 2.0, w, h];
    let k = ((plate[1] - plate[0] - 4.0) / pw).min(1.0);
    let (w, h) = (pw * k, PILL_H * k);
    let px = (ring[0] + 24.0 - w / 2.0).clamp(plate[0] + 2.0, plate[1] - 2.0 - w);
    let pill = [px, ring[1] + 42.0 + (PILL_H - h) / 2.0, w, h];
    [name, pill]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pause board's places: a short name against the centre on the left, from it on the right; a long one shrunk
    /// into the room; the pill kept on its plate.
    #[test]
    fn places() {
        let [name, pill] = place(true, [100.0, 300.0], 318.0, [146.0, 183.0], [96.0, 184.0], [30.0, 40.0]);
        assert_eq!(name, [153.0, 307.0, 30.0, 22.0]);
        assert_eq!(pill, [104.0, 342.0, 40.0, 11.0]);
        let [name, pill] = place(false, [496.0, 300.0], 318.0, [457.0, 494.0], [456.0, 544.0], [74.0, 200.0]);
        assert_eq!(name[0], 457.0);
        assert!((name[2] - 37.0).abs() < 1e-4 && (name[3] - 11.0).abs() < 1e-4);
        assert!(pill[0] >= 458.0 && pill[0] + pill[2] <= 542.0 + 1e-3);
    }
}
