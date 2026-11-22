//! F1: the stats board as one widget (ours, in the original's look, user concept 2026-11-21 "End screen"): the final
//! set-score board on top, in the stats board's own frame (the olive rim, dark cells, each side's players on their
//! stripes with their faces and slot labels, each set's games with the set's winner in gold, bigger), over the stats
//! board ([`super::layout`]) and its Longest Rally / Match Time bar. Laid out in the stats board's 640×448 coordinates,
//! the score board above them; the end screen fits it on the screen ([`super::next::fit`]) and the pause menu can too.

use super::{Quad, Tex, View, WHITE};

/// The widget's top (the score board's) and bottom (the bar's), in the stats board's coordinates.
pub(super) const TOP: f32 = SCORE_BOTTOM - SCORE_H;
pub(super) const BOTTOM: f32 = 392.0;

/// The score board ends this far above the stats board's frame (at 72).
const SCORE_BOTTOM: f32 = 66.0;
/// Its rim (the frame sheet's, outline in), a row (a team's games), a player's cell and a set's column.
const RIM: f32 = 8.0;
const ROW: f32 = 32.0;
const SCORE_H: f32 = 2.0 * ROW + 2.0 * RIM;
const CELL_W: f32 = 96.0;
const SET_W: f32 = 40.0;
/// The digits of a set's winner and loser: tint and height (the sheet's digits are 16×24).
const GOLD: [f32; 3] = [121.0, 91.0, 25.0];
const GREY: [f32; 3] = [78.0, 82.0, 88.0];
const WIN_H: f32 = 30.0;
const LOSE_H: f32 = 24.0;
/// The dark sprite behind the cells: the stats board draws it this strong too, over the end screen's dim.
pub(super) const BACK_ALPHA: f32 = 112.0;

/// The whole widget for `v`: the stats board and its bar, then the score board above them (after, so the stats
/// board's slot labels are the ones a named player's plate takes first).
pub(super) fn widget(v: &View) -> Vec<Quad> {
    let mut out = super::layout(v);
    out.extend(score_board(v));
    out
}

fn quad(tex: Tex, src: [f32; 4], dst: [f32; 4], rgb: [f32; 3], alpha: f32) -> Quad {
    Quad { tex, src, dst, rgb, alpha }
}

/// The final set-score board: team 0 (players 0 and 2) on the left, team 1 on the right, a column per set played (a
/// match ended mid-set, as `HST_STATS` does, shows the set it was in too), the first team's games on the top row.
pub(super) fn score_board(v: &View) -> Vec<Quad> {
    let mut out = Vec::new();
    let score = &v.table.score;
    let cur = score.set.clamp(0, 4) as usize;
    let sets = (cur + usize::from(score.set_games[0][cur] + score.set_games[1][cur] > 0)).clamp(1, 5);
    let inner_w = 2.0 * CELL_W + SET_W * sets as f32;
    let (x0, y0) = (320.0 - inner_w / 2.0 - RIM, TOP);
    let (ix, iy) = (x0 + RIM, y0 + RIM);
    out.push(quad(Tex::Back, [0.0, 0.0, 8.0, 8.0], [ix, iy, inner_w, 2.0 * ROW], WHITE, BACK_ALPHA));
    // each side's players on their stripes: singles one to the cell's height, doubles one a row
    let n = v.players.min(4);
    for p in 0..n {
        let side = p & 1;
        let (y, h) = if n < 3 { (iy, 2.0 * ROW) } else { (iy + ROW * (p / 2) as f32, ROW) };
        let x = if side == 0 { ix } else { ix + CELL_W + SET_W * sets as f32 };
        let vv = 0x110 as f32 + 0x28 as f32 * p as f32;
        let (l, m, r) = if side == 0 { (24.0, 16.0, 56.0) } else { (56.0, 16.0, 24.0) };
        out.push(quad(Tex::Frame, [0.0, vv, 8.0, 40.0], [x, y, l, h], WHITE, 128.0));
        out.push(quad(Tex::Frame, [8.0, vv, 16.0, 40.0], [x + l, y, m, h], WHITE, 128.0));
        out.push(quad(Tex::Frame, [24.0, vv, 7.0, 40.0], [x + l + m, y, r, h], WHITE, 128.0));
        // the face (its image has a margin round the disc) at the cell's outer end, the label beside it
        let f = h.min(64.0);
        let fy = y + (h - f) / 2.0;
        let fx = if side == 0 { x - f * 0.12 } else { x + CELL_W - f * 0.88 };
        out.push(quad(Tex::Face(p), [0.0, 0.0, 64.0, 64.0], [fx, fy, f, f], WHITE, 128.0));
        let lx = if side == 0 { x + CELL_W - 52.0 } else { x + 4.0 };
        let src = [0.0, v.slots[p] as f32 * 24.0 + 392.0, 48.0, 24.0];
        out.push(quad(Tex::Text, src, [lx, y + (h - 24.0) / 2.0, 48.0, 24.0], v.pill[p].map(f32::from), 128.0));
    }
    // the frame in nine pieces (rows clear of the stats board's row lines)
    let mid_h = 2.0 * ROW;
    for (u, x, w) in [(0.0, x0, 16.0), (16.0, x0 + 16.0, inner_w + 2.0 * RIM - 32.0), (40.0, x0 + inner_w + 2.0 * RIM - 16.0, 16.0)] {
        out.push(quad(Tex::Frame, [u, 0.0, if u == 16.0 { 8.0 } else { 16.0 }, RIM], [x, y0, w, RIM], WHITE, 128.0));
        out.push(quad(Tex::Frame, [u, 12.0, if u == 16.0 { 8.0 } else { 16.0 }, 32.0], [x, iy, w, mid_h], WHITE, 128.0));
        out.push(quad(Tex::Frame, [u, 272.0 - RIM, if u == 16.0 { 8.0 } else { 16.0 }, RIM], [x, iy + mid_h, w, RIM], WHITE, 128.0));
    }
    // the set columns' dividers, then each set's games
    for k in 0..=sets {
        let x = ix + CELL_W + SET_W * k as f32 - 1.0;
        out.push(quad(Tex::Frame, [40.0, 273.0, 2.0, 14.0], [x, iy, 2.0, mid_h], WHITE, 128.0));
    }
    for c in 0..sets {
        let cx = ix + CELL_W + SET_W * (c as f32 + 0.5);
        for team in 0..2 {
            let (g, other) = (score.set_games[team][c], score.set_games[team ^ 1][c]);
            let won = g > other;
            let (h, rgb) = if won { (WIN_H, GOLD) } else { (LOSE_H, GREY) };
            let w = h * 16.0 / 24.0;
            let digits: Vec<u32> = g.max(0).to_string().chars().filter_map(|d| d.to_digit(10)).collect();
            let x = cx - w * digits.len() as f32 / 2.0;
            let y = iy + ROW * team as f32 + (ROW - h) / 2.0;
            for (k, d) in digits.into_iter().enumerate() {
                out.push(quad(Tex::Text, [d as f32 * 16.0, 0.0, 16.0, 24.0], [x + w * k as f32, y, w, h], rgb, 128.0));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::tests::table;
    use super::*;

    fn view(t: &hst_sim::stats::Table, players: usize) -> View<'_> {
        View { table: t, players, slots: [0, 4, 4, 4], pill: [[128; 3]; 4], ranks: [None; 4], rank_rgb: [[128; 3]; 4], pulse: 0 }
    }

    /// 6-4 3-6 7-5: a column a set, each set's winner gold and bigger; the board centred above the stats board,
    /// inside the widget's extent.
    #[test]
    fn score_board_sets() {
        let mut t = table();
        t.score.set_games[0][..3].copy_from_slice(&[6, 3, 7]);
        t.score.set_games[1][..3].copy_from_slice(&[4, 6, 5]);
        t.score.set = 3;
        for players in [2, 4] {
            let q = score_board(&view(&t, players));
            let digits: Vec<_> = q.iter().filter(|q| q.tex == Tex::Text && q.src[1] == 0.0).map(|q| ((q.src[0] / 16.0) as i32, q.rgb == GOLD, q.dst[3])).collect();
            assert_eq!(digits, [(6, true, WIN_H), (4, false, LOSE_H), (3, false, LOSE_H), (6, true, WIN_H), (7, true, WIN_H), (5, false, LOSE_H)]);
            let (l, r) = (q.iter().map(|q| q.dst[0]).fold(f32::MAX, f32::min), q.iter().map(|q| q.dst[0] + q.dst[2]).fold(0.0, f32::max));
            assert!(((l + r) / 2.0 - 320.0).abs() < 0.01, "{l} {r}");
            assert!(q.iter().all(|q| q.dst[1] >= TOP - 0.01 && q.dst[1] + q.dst[3] <= SCORE_BOTTOM + 0.01));
            // a label per player, for the name plates
            assert_eq!(q.iter().filter(|q| q.tex == Tex::Text && q.src[1] >= 392.0).count(), players);
        }
        // ended in the fourth set at 2-1: its column too
        t.score.set_games[0][3] = 2;
        t.score.set_games[1][3] = 1;
        let digits = score_board(&view(&t, 2)).iter().filter(|q| q.tex == Tex::Text && q.src[1] == 0.0).count();
        assert_eq!(digits, 8);
        let w = widget(&view(&t, 4));
        assert!(w.iter().all(|q| q.dst[1] >= TOP - 0.01 && q.dst[1] + q.dst[3] <= BOTTOM + 0.01));
    }
}
