//! B750 (ours, the user's concept round): the pause screen's score board, frame, plates, faces' frames, each set's
//! games and the points, laid out for a saved match (`save_game::Summary`) so the Continue screen's cards
//! (`main_menu/continue_screen.rs`) show the score as the pause menu does. The main menu draws the pieces in its own
//! quads from the same disc sheets ([`SHEETS`]); faces, the seat tags and the character pills are its words and
//! icons in the places the pause board gives them.

use super::{BOARD, GAMES_NOW, GAMES_PAST, POINTS, TIEBREAK, Tex, View, board_rows, frame_top, layout};
use crate::play::save_game::Summary;

/// The board's sheets in `INPANE.XB0`, as [`Piece::Sheet`] numbers them.
pub(in crate::play) const SHEETS: [&str; 5] = ["inpane_pause00", "inpane_pause01", "result_gameset00", "result_gameset01", "result_gameset02"];
const OWN: [Tex; 5] = [POINTS, TIEBREAK, BOARD, GAMES_PAST, GAMES_NOW];
/// The board's left edge on the pause screen (its frame from 88 to 552).
const LEFT: f32 = 88.0;
/// The board's width (disc px).
pub(in crate::play) const WIDTH: f32 = 464.0;
/// A character pill's height on a card (disc px): the pause's 11 px pill made taller to read at a card's size.
const PILL_H: f32 = 17.0;

/// One of [`SHEETS`] drawn from `src` (its pixels) to `dst` (board px from its top-left), at `alpha` (128 opaque).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::play) struct Piece {
    pub(in crate::play) sheet: usize,
    pub(in crate::play) src: [f32; 4],
    pub(in crate::play) dst: [f32; 4],
    pub(in crate::play) alpha: f32,
}

/// The board for a save: its pieces, each player's face frame and seat tag's place, and its size (disc px).
#[derive(Clone, Debug, PartialEq)]
pub(in crate::play) struct Board {
    pub(in crate::play) pieces: Vec<Piece>,
    /// Player `p`'s face frame (48 px square).
    pub(in crate::play) faces: Vec<[f32; 4]>,
    /// Player `p`'s seat tag's place (the original's 1P..4P / COM cell) beside the face.
    pub(in crate::play) tags: Vec<[f32; 4]>,
    pub(in crate::play) size: [f32; 2],
}

impl Board {
    /// Player `p`'s character pill `w` px wide (its words and margin) under the face, kept on its plate, as the
    /// pause's (`plate::place`) but [`PILL_H`] high.
    pub(in crate::play) fn pill(&self, p: usize, w: f32) -> [f32; 4] {
        let [x, y, ..] = self.faces[p];
        let plate_l = [96.0, 456.0][p & 1] - LEFT;
        let w = w.min(84.0);
        [(x + 24.0 - w / 2.0).clamp(plate_l + 2.0, plate_l + 86.0 - w), y + 40.0, w, PILL_H]
    }
}

/// The board for a match of `players` playing to `sets_to_win` sets of `games`, its score at `s`.
pub(in crate::play) fn board(s: &Summary, sets_to_win: i32, games: i32, players: usize) -> Board {
    let sets_to_win = sets_to_win.clamp(1, 3);
    let set = s.set_games.len().min(4);
    let mut set_games = [[0; 5]; 2];
    for (c, g) in s.set_games.iter().take(4).chain([&s.games]).enumerate() {
        set_games[0][c] = g[0];
        set_games[1][c] = g[1];
    }
    let v = View {
        players,
        slots: [4; 4],
        pill: [[0; 3]; 4],
        sets_to_win,
        games: games.clamp(1, 6),
        points: s.points,
        deuce: s.deuce,
        advantage: s.advantage,
        tiebreak: s.tiebreak,
        set: set as i32,
        set_games,
        sel: 0,
        hand_x: 0.0,
        fade: None,
        ask: None,
        local: false,
        save: false,
        // the screen as it ends with the board: no dialog over it
        saved: true,
        words: [None; 4],
        tab: None,
    };
    let rows = board_rows(sets_to_win, players);
    let top = frame_top(rows);
    let at = |r: [f32; 4]| [r[0] - LEFT, r[1] - top, r[2], r[3]];
    let mut b = Board { pieces: Vec::new(), faces: vec![[0.0; 4]; players], tags: vec![[0.0; 4]; players], size: [WIDTH, (16 * rows + 16) as f32] };
    let mut tag = 0;
    for q in layout(&v) {
        match q.tex {
            // the face over its frame (`r(x + 2, y + 2, ..)`)
            Tex::Face(p) => b.faces[p] = at([q.dst[0] - 2.0, q.dst[1] - 2.0, 48.0, 48.0]),
            // the seat tags come in player order
            Tex::Slot => {
                b.tags[tag.min(players - 1)] = at(q.dst);
                tag += 1;
            }
            t => {
                if let Some(sheet) = OWN.iter().position(|&o| o == t) {
                    b.pieces.push(Piece { sheet, src: q.src, dst: at(q.dst), alpha: q.alpha });
                }
            }
        }
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The board sits at its own origin, every piece inside it; singles one face a side, doubles two; the set in play
    /// on the bright sheet, the points on theirs.
    #[test]
    fn saved_boards() {
        let s = Summary { games: [4, 2], points: [2, 1], set_games: vec![[6, 4]], ..Default::default() };
        for (players, sets) in [(2, 2), (4, 2), (2, 3), (4, 1)] {
            let b = board(&s, sets, 6, players);
            assert_eq!(b.faces.len(), players);
            for p in &b.pieces {
                let [x, y, w, h] = p.dst;
                assert!(x >= -0.01 && y >= -0.01 && x + w <= b.size[0] + 0.01 && y + h <= b.size[1] + 0.01, "{players} {sets} {p:?}");
            }
            for (p, f) in b.faces.iter().enumerate() {
                let pill = b.pill(p, 60.0);
                assert!(pill[1] + pill[3] <= b.size[1] - 8.0 && pill[0] >= 0.0, "{players} {sets} {p}");
                // the pill clears the next face down
                if let Some(g) = b.faces.get(p + 2) {
                    assert!(pill[1] + pill[3] <= g[1] + 0.01, "{players} {sets} {p}: {f:?} {g:?}");
                }
            }
        }
        let b = board(&s, 2, 6, 2);
        let sheet = |k: usize| b.pieces.iter().filter(|p| p.sheet == k).count();
        // this set's two counts and its column's dash on the bright sheet; the first set's on the dim one
        assert_eq!((sheet(4), sheet(3) >= 2, sheet(0), sheet(1)), (3, true, 2, 0));
        assert_eq!(b.size, [WIDTH, 112.0]);
    }
}
