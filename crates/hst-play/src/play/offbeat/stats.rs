//! M147: Everyone's stats, the offbeat rule that gives every player one grade, F through A, in each of the character
//! select's five categories (Serve, Stroke, Volley, Impact, Footwork). A grade stands for the TParam.csv cells behind
//! its category: player `i` keeps their own row (build, reach, heights, collision, hand) with each category's cells
//! replaced by the mean of the stock characters' cells at that grade in it (the select screen's grades, read from the
//! disc). The match then feeds that row through the same paths a character's own row takes: run, stamina, aim,
//! power, shot timing and the serve's. Trajectory tables, shot records and the computer's AI row stay the
//! character's.

use crate::grades::{CHARACTERS, disc_grades, graded_row};
use hst_data::iso::Iso;
use hst_sim::swing;

use super::super::{Game, aim_of, mod_match, serve_character, serve_data, stats_of, tparam};

/// Every player takes `grade` (0 F … 5 A) in every category: their row (a mod's own in its slot) graded, fed to the
/// run, stamina, aim, power, shot timing and serve.
pub(in crate::play) fn apply(g: &mut Game, iso: &mut Iso, mm: Option<&crate::mods::MatchMod>, grade: u8) {
    let Some(grades) = disc_grades(iso) else { return };
    let rows: Vec<Vec<String>> = (0..CHARACTERS).map(|c| tparam(iso, c)).collect();
    for i in 0..g.players.len() {
        let c = g.chars[i] as usize;
        let row = graded_row(&mod_match::row(iso, mm, i, c), &rows, &grades, grade);
        let p = &mut g.players[i];
        p.stats = stats_of(&row);
        p.body.stamina = p.stats.stamina;
        g.aim_stats[i] = aim_of(&row);
        g.finish.power[i] = [13, 14].map(|k| row[k].parse().expect("TParam power"));
        let s = hst_sim::player::ReachStats::from_tparam(&row.join(","));
        g.reaches[i].grades = swing::timing(s.after, s.before).0;
        g.timing[i].restat(&row);
        let mut serve = serve_data(&row, &g.data[i]);
        serve_character(iso, c, g.aim_stats[i], &mut serve);
        g.serve_data[i] = serve;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On the disc every grade of every category has a character, and the grades run the right way: A serves, hits
    /// and volleys harder than F, runs faster and has longer timing windows. (Not step by step: the disc's D volleyers
    /// hit harder on average than its C ones.)
    #[test]
    fn disc_grades_run_from_f_to_a() {
        let Ok(mut iso) = hst_data::fixture::iso() else { return };
        let grades = disc_grades(&mut iso).expect("grades");
        for k in 0..5 {
            for g in 0..6 {
                assert!(grades.iter().any(|c| c[k] == g), "category {k} has grade {g}");
            }
        }
        let rows: Vec<Vec<String>> = (0..CHARACTERS).map(|c| tparam(&mut iso, c)).collect();
        let at = |g: u8, c: usize| -> f64 { graded_row(&rows[0], &rows, &grades, g)[c].split('/').next().unwrap().parse().unwrap() };
        for (c, what) in [(12, "Serv POW"), (13, "Strk POW"), (14, "Voley POW"), (40, "SPE"), (28, "timing after")] {
            let v: Vec<f64> = (0..6).map(|g| at(g, c)).collect();
            assert!(v[0] < v[5], "{what} by grade: {v:?}");
        }
        // the graded row still reads as a TParam row
        for g in 0..6 {
            let row = graded_row(&rows[3], &rows, &grades, g);
            let _ = (stats_of(&row), aim_of(&row), hst_sim::player::ReachStats::from_tparam(&row.join(",")));
        }
    }
}
