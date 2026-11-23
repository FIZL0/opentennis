//! F1: the stats board as one widget; B764 builds it from the game's own parts: the "Game, Set, Match!" banner (the
//! `i_gameset_00` model's own art, flat), the original's final board under it (`popups::final_board`: frame, each
//! side's stripes, each set's games in its outlined digits, the sets won) with the pause board's plates (face in the
//! white ring; F3: the character's name where the slot label was, [`super::plate`]), then the stats board
//! ([`super::layout`]: the plates in its header read the player, every value in the outlined digits) and
//! its Longest Rally / Match Time bar. Laid out from y 0 down, centred on x 320 as the 640×448 screen; the end screen
//! puts the menu box beside it and fits the two on the screen ([`super::next::fit`]); F5: the pause screen draws it too
//! ([`Kind::Live`], the match so far, no banner) through [`super::pause_board`], the pause menu's box beside it.

use super::super::popups;
use super::{Quad, Tex, View, WHITE};

/// The dark sprite behind the cells: the stats board draws it this strong too, over the end screen's dim.
pub(super) const BACK_ALPHA: f32 = 112.0;
/// The banner's art (256×64, its words' ink 52 high from the top) and its index among the screen's own sheets.
pub(super) const BANNER: u8 = (super::next::LABELS.len() + super::next::NOTES.len()) as u8;
const BANNER_SIZE: [f32; 2] = [256.0, 64.0];
/// The gaps under the banner and between the two boards.
const GAP: f32 = 4.0;
const BOARD_GAP: f32 = 8.0;

/// Which screen the widget is for: the end screen's (the banner over the final board) or the pause screen's (F5: the
/// match so far, the set in play on the bright sheet even at 0-0, no banner).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    End,
    Live,
}

/// The whole widget for `v`, top to bottom from y 0.
pub(super) fn widget(v: &View, kind: Kind) -> Vec<Quad> {
    let (mut out, mut y) = match kind {
        Kind::End => (
            vec![Quad {
                tex: Tex::Own(BANNER),
                src: [0.0, 0.0, BANNER_SIZE[0], BANNER_SIZE[1]],
                dst: [320.0 - BANNER_SIZE[0] / 2.0, 0.0, BANNER_SIZE[0], BANNER_SIZE[1]],
                rgb: WHITE,
                alpha: 128.0,
            }],
            BANNER_SIZE[1] + GAP,
        ),
        Kind::Live => (Vec::new(), 0.0),
    };
    let score = match kind {
        Kind::End => score_board(v),
        Kind::Live => live_board(v),
    };
    y = stack(&mut out, score, y) + BOARD_GAP;
    stack(&mut out, super::layout(v), y);
    out
}

/// Puts `quads` on `out` with their top at `y`; their bottom.
fn stack(out: &mut Vec<Quad>, mut quads: Vec<Quad>, y: f32) -> f32 {
    let top = quads.iter().map(|q| q.dst[1]).fold(f32::MAX, f32::min);
    shift(&mut quads, y - top);
    let bottom = quads.iter().map(|q| q.dst[1] + q.dst[3]).fold(y, f32::max);
    out.extend(quads);
    bottom
}

/// F2: the pause screen's score board: every set played and the set in play, that one on the bright sheet with both
/// counts at full strength (the final board shows every set settled).
fn live_board(v: &View) -> Vec<Quad> {
    let cur = v.table.score.set.clamp(0, 4);
    let mut out = score_board_of(v, cur as usize + 1);
    let games_y = match v.sets_to_win {
        1 => 360.0,
        2 => 328.0,
        _ => 264.0,
    } + 32.0 * cur as f32
        - 40.0;
    for q in out.iter_mut().filter(|q| q.tex == Tex::Board(1) && q.dst[1] == games_y && [272.0, 308.0, 336.0].contains(&q.dst[0])) {
        (q.tex, q.alpha) = (Tex::Board(2), 128.0);
    }
    out
}

fn shift(quads: &mut [Quad], dy: f32) {
    for q in quads {
        q.dst[1] += dy;
    }
}

/// The original's final board for the match (every set settled, the winner's stripes pulsing with the header), its
/// slot labels set as the characters' names (F3). A match ended mid-set (as `HST_STATS` does) shows that set too.
pub(super) fn score_board(v: &View) -> Vec<Quad> {
    let cur = v.table.score.set.clamp(0, 4) as usize;
    let games = v.table.score.set_games[0][cur] + v.table.score.set_games[1][cur];
    score_board_of(v, cur + usize::from(games > 0))
}

/// F2: [`score_board`] with its first `shown` sets (the pause board's: the set in play too, even at 0-0).
pub(super) fn score_board_of(v: &View, shown: usize) -> Vec<Quad> {
    let mut score = v.table.score.clone();
    let cols = (v.sets_to_win.clamp(1, 3) * 2 - 1) as usize;
    score.set = shown.clamp(1, cols) as i32;
    let fin = popups::Final { highlight: v.pulse, scale: 1.0, dx: 0.0 };
    let board = popups::final_board(v.players, &score, v.sets_to_win, v.slots, v.pill, v.table.winner.min(1), fin);
    let mut out = Vec::new();
    // the board lists each player's ring, face and label in turn, then sorts by sheet: the k-th label is player k's
    let mut labels = 0;
    for q in board {
        let tex = match q.tex {
            popups::Tex::Board(k) => Tex::Board(k),
            popups::Tex::Face(p) => Tex::Face(p),
            popups::Tex::Slot => Tex::Slot,
            _ => continue,
        };
        let (src, dst, rgb, alpha) = (q.src, q.dst, q.rgb, q.alpha);
        if tex == Tex::Slot {
            let p = labels;
            labels += 1;
            // the pause board's plate: the ring 44 left of the label on the left team, its top 6 above the label's;
            // F3: the character's name beside the face, across the ring's middle
            let left = p & 1 == 0;
            let ring = [if left { 100.0 } else { 496.0 }, dst[1] - 6.0];
            let room = if left { [146.0, 183.0] } else { [457.0, 494.0] };
            if let Some([_, w]) = v.words.get(p).copied().flatten() {
                out.push(Quad { tex: Tex::Chara(p), src: [0.0; 4], dst: super::plate::place(left, ring[1] + 24.0, room, w), rgb: WHITE, alpha });
                continue;
            }
        }
        out.push(Quad { tex, src, dst, rgb, alpha });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::tests::table;
    use super::*;

    fn view(t: &hst_sim::stats::Table, players: usize, sets_to_win: i32) -> View<'_> {
        View { table: t, players, slots: [0, 4, 4, 4], pill: [[128; 3]; 4], ranks: [None; 4], rank_rgb: [[128; 3]; 4], pulse: 0, sets_to_win, words: [Some([40.0, 50.0]); 4] }
    }

    /// 6-4 3-6 7-5 of a best of five: each set's games on the settled (green) digits in its row, the sets won on the
    /// big ones; the character's name per player in place of the slot labels; the widget banner, board, stats from
    /// the top down.
    #[test]
    fn score_board_sets() {
        let mut t = table();
        t.score.set_games[0][..3].copy_from_slice(&[6, 3, 7]);
        t.score.set_games[1][..3].copy_from_slice(&[4, 6, 5]);
        t.score.sets = [2, 1];
        t.score.set = 3;
        for players in [2, 4] {
            let q = score_board(&view(&t, players, 3));
            let mut games: Vec<_> = q.iter().filter(|q| q.tex == Tex::Board(1) && q.src[0] < 320.0).map(|q| (q.dst[1] as i32, q.dst[0] as i32, (q.src[0] / 32.0) as i32)).collect();
            games.sort();
            let digits: Vec<_> = games.iter().map(|g| g.2).collect();
            assert_eq!(digits, [6, 4, 3, 6, 7, 5]);
            let sets: Vec<_> = q.iter().filter(|q| q.tex == Tex::Board(3) && q.src[1] == 0.0 && q.src[2] == 48.0).map(|q| q.src[0] / 48.0).collect();
            assert_eq!(sets, [2.0, 1.0]);
            assert!(!q.iter().any(|q| matches!(q.tex, Tex::Name(_))));
            assert_eq!(q.iter().filter(|q| matches!(q.tex, Tex::Chara(_))).count(), players);
            // beside the face, across the ring's middle
            for c in q.iter().filter(|q| matches!(q.tex, Tex::Chara(_))) {
                let ring = q.iter().find(|r| r.tex == Tex::Board(0) && (r.dst[1] + 24.0 - c.dst[1] - c.dst[3] / 2.0).abs() < 1e-3);
                assert!(ring.is_some_and(|r| c.dst[0] >= r.dst[0] + 46.0 || c.dst[0] + c.dst[2] <= r.dst[0] - 1.0), "{c:?}");
            }
            assert!(!q.iter().any(|q| q.tex == Tex::Slot));
            // nothing from the stats sheet: every digit is the board's
            assert!(!q.iter().any(|q| q.tex == Tex::Text));
        }
        // ended in the fourth set at 2-1: its row too
        t.score.set_games[0][3] = 2;
        t.score.set_games[1][3] = 1;
        let games = score_board(&view(&t, 2, 3)).iter().filter(|q| q.tex == Tex::Board(1) && q.src[0] < 320.0).count();
        assert_eq!(games, 8);
        let w = widget(&view(&t, 4, 3), Kind::End);
        assert_eq!(w[0].tex, Tex::Own(BANNER));
        let banner_bottom = w[0].dst[1] + w[0].dst[3];
        assert!(w[1..].iter().filter(|q| matches!(q.tex, Tex::Board(_))).all(|q| q.dst[1] >= banner_bottom));
    }
}
