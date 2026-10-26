//! The character select's five grades (Serve, Stroke, Volley, Impact, Footwork; F through A) as TParam.csv cells:
//! a row at a grade takes, in each category, the mean of the cells of the stock characters at that grade. M147's
//! Everyone's stats grades every player; a mod's §6 `stats` grades its own row (`mods::tparam_row`).

use hst_data::iso::Iso;

/// Every category's TParam.csv columns, in the grades' order.
const CATEGORIES: [&[usize]; 5] = [SERVE, STROKE, VOLLEY, IMPACT, FOOTWORK];
/// Serv POW, Serv CON, the serve's near-miss scatter.
const SERVE: &[usize] = &[12, 22, 54];
/// Strk, Lob and Lob2 POW, the three spins, Strk CON, Rizing ADJ, the ground stroke's height errors (Strk GH / NH),
/// Back DWN, LOW POW, Strk HI POW.
const STROKE: &[usize] = &[13, 15, 16, 17, 18, 19, 20, 26, 34, 35, 46, 50, 53];
/// Voley POW, Voley CON, the volley's height errors (Voley GH / NH), V DWN, V LOW POW.
const VOLLEY: &[usize] = &[14, 21, 37, 38, 48, 51];
/// The timing frames after / before the sweet one: rally hits, strong toss, weak toss.
const IMPACT: &[usize] = &[28, 29, 30, 31, 32, 33];
/// SPE, STA, the dive / backhand / smash stamina costs, Agili.
const FOOTWORK: &[usize] = &[40, 41, 42, 43];

/// The stock characters on the disc.
pub const CHARACTERS: usize = 14;

/// The grade above A ([`graded_row`]): every cell the roster's best.
pub const S: u8 = 6;

/// `own` with every category's cells at `grade` (0 F … 5 A): the mean of the cells of the characters in `rows` whose
/// grade in that category (`grades`) is `grade`, or the nearest grade any of them has. At [`S`] each cell part is the
/// best any character in `rows` has, best being the way the A characters' mean lies from the F ones' (their mean
/// where the two agree).
pub fn graded_row(own: &[String], rows: &[Vec<String>], grades: &[[u8; 5]], grade: u8) -> Vec<String> {
    let donors = |k: usize, grade: u8| -> Vec<&Vec<String>> {
        let near = grades.iter().map(|g| g[k]).min_by_key(|&g| g.abs_diff(grade)).unwrap_or(grade);
        rows.iter().zip(grades).filter(|(_, g)| g[k] == near).map(|(r, _)| r).collect()
    };
    let cell = |rows: &[&Vec<String>], c: usize| mean(rows.iter().filter_map(|r| r.get(c).map(String::as_str)));
    let mut out = own.to_vec();
    for (k, cells) in CATEGORIES.iter().enumerate() {
        let (a, f) = (donors(k, grade.min(5)), donors(k, 0));
        let all: Vec<&Vec<String>> = rows.iter().collect();
        for &c in cells.iter().filter(|&&c| c < own.len()) {
            out[c] = if grade < S { cell(&a, c) } else { best(&cell(&a, c), &cell(&f, c), &all, c) };
        }
    }
    out
}

/// Part by part ('/'): `a` where it equals `f`, else the largest (`a` above `f`) or smallest of cell `c` in `rows`.
fn best(a: &str, f: &str, rows: &[&Vec<String>], c: usize) -> String {
    let num = |s: &str| s.parse::<f64>().unwrap_or(0.0);
    let parts = |s: &str| s.split('/').map(str::trim).map(String::from).collect::<Vec<_>>();
    let (a, f) = (parts(a), parts(f));
    let cells: Vec<Vec<String>> = rows.iter().filter_map(|r| r.get(c)).map(|s| parts(s)).collect();
    (0..a.len().min(f.len()))
        .map(|i| {
            let vals = cells.iter().filter_map(|p| p.get(i));
            let pick = match num(&a[i]).partial_cmp(&num(&f[i])) {
                Some(std::cmp::Ordering::Greater) => vals.max_by(|x, y| num(x).total_cmp(&num(y))),
                Some(std::cmp::Ordering::Less) => vals.min_by(|x, y| num(x).total_cmp(&num(y))),
                _ => None,
            };
            pick.cloned().unwrap_or_else(|| a[i].clone())
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// The mean of cells like "4", "2.7" or "9/0/10", part by part ('/'): whole numbers stay whole (rounded half away
/// from zero), anything else keeps up to four decimals.
fn mean<'a>(cells: impl Iterator<Item = &'a str>) -> String {
    let parts: Vec<Vec<&str>> = cells.map(|c| c.split('/').map(str::trim).collect()).collect();
    let width = parts.iter().map(Vec::len).min().unwrap_or(0);
    (0..width)
        .map(|i| {
            let vals: Vec<&str> = parts.iter().map(|p| p[i]).collect();
            let n = vals.len() as f64;
            if let Ok(ints) = vals.iter().map(|v| v.parse::<i64>()).collect::<Result<Vec<_>, _>>() {
                ((ints.iter().sum::<i64>() as f64) / n).round().to_string()
            } else {
                let m = vals.iter().map(|v| v.parse::<f64>().unwrap_or(0.0)).sum::<f64>() / n;
                let s = format!("{m:.4}");
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// The select screen's grades from the disc, or None off the supported disc.
pub fn disc_grades(iso: &mut Iso) -> Option<[[u8; 5]; 14]> {
    let (cnf, bin) = (iso.read("SYSTEM.CNF").ok()?, iso.read("ZZBIN/MENU.BIN").ok()?);
    hst_data::exe::Menu::new(&cnf, &bin).ok().map(|m| m.grades())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn means_keep_whole_numbers_whole() {
        assert_eq!(mean(["4", "4", "5"].into_iter()), "4");
        assert_eq!(mean(["13", "14"].into_iter()), "14", "half away from zero");
        assert_eq!(mean(["3", "2.7"].into_iter()), "2.85");
        assert_eq!(mean(["9/0/10", "4/0/8"].into_iter()), "7/0/9");
        assert_eq!(mean(["2.7", "2.7"].into_iter()), "2.7");
    }

    /// A graded row changes only its categories' cells, each the mean of that grade's characters (nearest grade
    /// when none has it).
    #[test]
    fn a_grade_takes_its_characters_cells() {
        let row = |v: &str| -> Vec<String> { (0..67).map(|c| if c == 11 { String::new() } else { format!("{v}{c}") }).collect() };
        let num = |n: i32| -> Vec<String> { (0..67).map(|c| if c == 11 { String::new() } else { (n * 100 + c).to_string() }).collect() };
        let rows = vec![num(1), num(3), num(9)];
        let grades = [[5, 0, 0, 0, 0], [5, 0, 1, 0, 0], [0, 5, 2, 3, 4]];
        let own = row("x");
        let out = graded_row(&own, &rows, &grades, 5);
        assert_eq!(out[12], "212", "Serve A: characters 0 and 1");
        assert_eq!(out[13], "913", "Stroke A: character 2");
        assert_eq!(out[14], "914", "Volley: nearest to A is character 2's 2");
        assert_eq!(out[28], "928");
        assert_eq!(out[40], "940");
        let s = graded_row(&own, &rows, &grades, S);
        assert_eq!(s[12], "112", "Serve S: A (1, 3) below F (9), so the roster's smallest");
        assert_eq!(s[13], "913", "Stroke S: A (9) above F (1, 3)");
        assert_eq!(s[23], own[23]);
        for c in [0, 11, 23, 55, 64] {
            assert_eq!(out[c], own[c], "cell {c} is the character's own");
        }
    }
}
