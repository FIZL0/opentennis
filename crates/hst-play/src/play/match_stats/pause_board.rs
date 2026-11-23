//! F2 (ours, the user's concept round): the end screen's stats board on the pause screen, live (F5: drawn by the end
//! screen's own widget, [`super::board::widget`] as [`super::board::Kind::Live`]). The score so far on
//! the original's final board ([`super::board::score_board_of`]: every set played, the set in play on the bright
//! sheet, even at 0-0), and under it the stats board ([`super::layout`]) of the match so far, its Longest Rally /
//! Match Time bar included; no banner. Laid out from y 0 down, centred on x 320 as the end screen's widget; the pause
//! menu (front-end crate) puts its box beside it and fits the two on the screen. Drawing only: it reads the match's
//! counts and never touches them.

use bevy::prelude::*;

use super::super::panel::{self, Colours};
use super::super::{Game, Pads, popups};
use super::{Quad, Tex, View};

/// One of the board's pictures: `image` (its `rect`, or the whole image) drawn to `dst` (the 640×448 screen's px).
#[derive(Clone, Debug, PartialEq)]
pub struct BoardQuad {
    pub image: Handle<Image>,
    pub rect: Option<Rect>,
    pub dst: [f32; 4],
    pub color: Color,
}

/// The pause screen's board: `open` while the pause screen wants it (the pause menu sets it), and its pictures then.
#[derive(Resource, Default)]
pub struct PauseBoard {
    pub open: bool,
    pub quads: Vec<BoardQuad>,
}

/// The side ahead in the match: on sets, then on games in the set in play (the left side when level).
fn leader(score: &hst_sim::score::Score) -> usize {
    let s = score.sets;
    if s[0] != s[1] {
        return usize::from(s[1] > s[0]);
    }
    usize::from(score.games[1] > score.games[0])
}

/// The board for `v`, its table's score the match's so far: F1's widget, live.
fn quads(v: &View) -> Vec<Quad> {
    super::board::widget(v, super::board::Kind::Live)
}

/// The system that sets the board's pictures: the pause menu opens the board before it and draws them after.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Build;

/// Sets the board's pictures each frame while it is open.
#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    mut board: ResMut<PauseBoard>,
    g: Res<Game>,
    pads: Res<Pads>,
    art: Option<Res<super::Art>>,
    panel_art: Option<Res<panel::Art>>,
    colours: Option<Res<Colours>>,
    board_art: Option<Res<popups::Art>>,
    words: Res<super::plate::Words>,
) {
    board.quads.clear();
    if !board.open {
        return;
    }
    let (Some(art), Some(panel_art), Some(colours), Some(board_art)) = (art, panel_art, colours, board_art) else {
        return;
    };
    let n = g.players.len();
    let slot = |i: usize| if i < n { pads.slot_of(i, n).unwrap_or(4) } else { 4 };
    let mut table = g.match_stats.finish(leader(&g.score));
    table.score = g.score.clone();
    let v = View {
        table: &table,
        players: n,
        slots: std::array::from_fn(slot),
        pill: colours.0,
        ranks: std::array::from_fn(|i| (i < n && slot(i) == 4).then(|| g.players[i].ai.rank as usize)),
        rank_rgb: colours.1,
        // no winner yet: no header lit
        pulse: 0,
        sets_to_win: g.rules.sets,
        words: std::array::from_fn(|p| if p < n { words.widths(p) } else { None }),
    };
    for q in quads(&v) {
        let image = match q.tex {
            Tex::Frame => art.0[0].clone(),
            Tex::Text => art.0[1].clone(),
            Tex::Gameset => art.0[3].clone(),
            Tex::Back => art.0[5].clone(),
            Tex::Team => art.0[7].clone(),
            Tex::Pill => art.0[8].clone(),
            Tex::Own(k) => art.0[9 + k as usize].clone(),
            Tex::Face(p) => panel_art.0[panel::FACES + p].clone(),
            Tex::Rank => panel_art.0[6].clone(),
            Tex::Board(k) => board_art.0[5 + k as usize].clone(),
            Tex::Slot => panel_art.0[1].clone(),
            Tex::Name(p) => match &words.name[p] {
                Some(w) => w.0.clone(),
                None => continue,
            },
            Tex::Chara(p) => match &words.chara[p] {
                Some(w) => w.0.clone(),
                None => continue,
            },
            Tex::Hand => continue,
        };
        let [u, v, w, h] = q.src;
        let rect = (!matches!(q.tex, Tex::Face(_) | Tex::Name(_) | Tex::Chara(_))).then(|| Rect::new(u, v, u + w, v + h));
        let [r, g, b] = q.rgb.map(|c| c / 128.0);
        board.quads.push(BoardQuad { image, rect, dst: q.dst, color: Color::srgba(r, g, b, q.alpha / 128.0) });
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::table;
    use super::*;

    fn view(t: &hst_sim::stats::Table, players: usize, sets_to_win: i32) -> View<'_> {
        View { table: t, players, slots: [0, 4, 4, 4], pill: [[128; 3]; 4], ranks: [None; 4], rank_rgb: [[128; 3]; 4], pulse: 0, sets_to_win, words: [Some([40.0, 50.0]); 4] }
    }

    /// 6-4, 2-3 into the second set of a best of three: the first set settled, the set in play bright at full
    /// strength; at 0-0 in a new set it shows too; the board from y 0, the stats under the score board.
    #[test]
    fn live_set() {
        let mut t = table();
        t.score.set_games[0][..2].copy_from_slice(&[6, 2]);
        t.score.set_games[1][..2].copy_from_slice(&[4, 3]);
        t.score.set = 1;
        let q = quads(&view(&t, 2, 2));
        let digits = |k: u8| {
            let mut d: Vec<_> = q.iter().filter(|q| q.tex == Tex::Board(k) && q.src[0] < 320.0 && q.src[1] == 0.0 && [272.0, 336.0].contains(&q.dst[0]) && q.dst[1] < 120.0).map(|q| (q.dst[1] as i32, q.dst[0] as i32, (q.src[0] / 32.0) as i32, q.alpha)).collect();
            d.sort_by_key(|d| (d.0, d.1));
            d.into_iter().map(|d| (d.2, d.3)).collect::<Vec<_>>()
        };
        assert_eq!(digits(1), [(6, 128.0), (4, 64.0)]);
        assert_eq!(digits(2), [(2, 128.0), (3, 128.0)]);
        assert_eq!(q.iter().map(|q| q.dst[1]).fold(f32::MAX, f32::min), 0.0);
        assert!(q.iter().any(|q| q.tex == Tex::Frame) && !q.iter().any(|q| q.tex == Tex::Own(super::super::board::BANNER)));
        // a new set at 0-0: its zeros on the bright sheet
        t.score.set = 2;
        let q = quads(&view(&t, 4, 2));
        let bright: Vec<_> = q.iter().filter(|q| q.tex == Tex::Board(2) && q.src[0] == 0.0 && [272.0, 336.0].contains(&q.dst[0]) && q.dst[1] < 120.0).collect();
        assert_eq!(bright.len(), 2);
        assert_eq!(leader(&t.score), 0);
    }

    /// Ahead on sets wins over games; level on both, the left side.
    #[test]
    fn leaders() {
        let mut s = hst_sim::score::Score::new();
        assert_eq!(leader(&s), 0);
        s.games = [1, 3];
        assert_eq!(leader(&s), 1);
        s.sets = [1, 0];
        assert_eq!(leader(&s), 0);
    }
}
