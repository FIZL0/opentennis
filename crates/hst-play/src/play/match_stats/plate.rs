//! B764: the end screen's player plates, in the pause screen's score board's lettering (its `menu/plate.rs`, in the
//! front-end crate, which this one cannot name): white words in the menus' font, outlined in the plate's team colour.
//! F3 (the concept's 'End screen' tab): the stats header's plates read the player, where the original drew its
//! 1P..4P / COM tag (a human's own name, else the tag's words, a CPU's always "COM", its rank under it); the score
//! board's read the character (a mod by its own name). Long words narrow, then shrink, to fit. Drawing only: nothing
//! here touches the match.

use bevy::prelude::*;

use super::super::name_tags::{self, Names};
use super::super::{Game, Pads};
use crate::Args;

/// The plates' dark edges by player (the board's plate art: pink, blue, orange, green), the pause plates'.
const EDGE: [[u8; 3]; 4] = [[0x8a, 0x1f, 0x52], [0x14, 0x50, 0x7e], [0x7a, 0x3f, 0x0a], [0x3b, 0x62, 0x13]];
/// The words' height (disc px, outline included) and their outline, the pause plates' name's.
pub(super) const NAME_H: f32 = 22.0;
const NAME_EW: f32 = 2.0;
/// How narrow long words may be set before they shrink instead.
const NARROW: f32 = 0.7;

/// The disc's characters by the select's order, as the character select names them.
const CHARACTERS: [&str; 14] =
    ["Ashley", "Cody", "Jun", "Kaito", "Momoko", "JJ", "Carol", "Big Chief", "Miranda", "Kent", "Lola", "Will", "Gloria", "Suzuki"];

/// Each player's words as set: who plays (the stats header's) and the character (the score board's), their images
/// and widths at `NAME_H`.
#[derive(Resource, Default)]
pub(super) struct Words {
    made: [Option<[String; 2]>; 4],
    pub(super) name: [Option<(Handle<Image>, f32)>; 4],
    pub(super) chara: [Option<(Handle<Image>, f32)>; 4],
}

impl Words {
    /// Player `p`'s widths (name, character) once both are set.
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
        let (img, w) = name_tags::panel_words(&want[1], edge, NAME_H, NAME_EW);
        words.chara[p] = Some((images.add(img), w));
        words.made[p] = Some(want);
    }
}

/// Where a plate's words `w` wide go: in `room` (x from, to) beside the face, against its inner end on the left team
/// (`left`), from its outer end on the right, their middle at `y`. Too wide, they narrow to [`NARROW`], then shrink
/// about their middle at that aspect.
pub(super) fn place(left: bool, y: f32, room: [f32; 2], w: f32) -> [f32; 4] {
    let fit = room[1] - room[0];
    let narrow = (fit / w).clamp(NARROW, 1.0);
    let k = (fit / (w * narrow)).min(1.0);
    let (w, h) = (w * narrow * k, NAME_H * k);
    [if left { room[1] - w } else { room[0] }, y - h / 2.0, w, h]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A short name against the centre on the left, from it on the right; a longer one narrowed into the room; a
    /// long one narrowed as far as it goes, then shrunk.
    #[test]
    fn places() {
        assert_eq!(place(true, 318.0, [146.0, 183.0], 30.0), [153.0, 307.0, 30.0, 22.0]);
        let r = place(false, 318.0, [457.0, 494.0], 40.0);
        assert!(r[0] == 457.0 && (r[2] - 37.0).abs() < 1e-4 && r[3] == 22.0);
        let r = place(false, 318.0, [457.0, 494.0], 74.0);
        assert!((r[2] - 37.0).abs() < 1e-4 && (r[3] - 22.0 * 37.0 / 51.8).abs() < 1e-3);
        assert!((r[1] + r[3] / 2.0 - 318.0).abs() < 1e-4);
    }
}
