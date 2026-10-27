//! The character select's five grades (Serve, Stroke, Volley, Impact, Footwork; F through A) as TParam.csv cells:
//! a row at a grade takes, in each category, the mean of the cells of the stock characters at that grade. M147's
//! Everyone's stats grades every player; a mod's §6 `stats` grades its own row (`mods::tparam_row`). The other way,
//! [`grades_of`] reads the grades a row plays at, for a mod's select card.

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

/// `own` with each category's cells at its `grade` (0 F … 5 A): the mean of the cells of the characters in `rows` whose
/// grade in that category (`grades`) is `grade`, or the nearest grade any of them has. At [`S`] each cell part is the
/// best any character in `rows` has, best being the way the A characters' mean lies from the F ones' (their mean
/// where the two agree).
pub fn graded_row(own: &[String], rows: &[Vec<String>], grades: &[[u8; 5]], grade: [u8; 5]) -> Vec<String> {
    let donors = |k: usize, grade: u8| -> Vec<&Vec<String>> {
        let near = grades.iter().map(|g| g[k]).min_by_key(|&g| g.abs_diff(grade)).unwrap_or(grade);
        rows.iter().zip(grades).filter(|(_, g)| g[k] == near).map(|(r, _)| r).collect()
    };
    let cell = |rows: &[&Vec<String>], c: usize| mean(rows.iter().filter_map(|r| r.get(c).map(String::as_str)));
    let mut out = own.to_vec();
    for (k, cells) in CATEGORIES.iter().enumerate() {
        let (a, f) = (donors(k, grade[k].min(5)), donors(k, 0));
        let all: Vec<&Vec<String>> = rows.iter().collect();
        for &c in cells.iter().filter(|&&c| c < own.len()) {
            out[c] = if grade[k] < S { cell(&a, c) } else { best(&cell(&a, c), &cell(&f, c), &all, c) };
        }
    }
    out
}

/// Serv POW, Strk POW, Voley POW: the columns a character's shots fly by (its trajectory tables and shot records are
/// authored from them), so a mod's come from whoever's serve, strokes and volleys it hits.
pub const POWER: [usize; 3] = [12, 13, 14];

/// How good `row` is in each category, as one number the disc's hand-set grades all rise with (fit to the 14 stock
/// rows, research/journal/2026-10-27-mod-grades): power with a little control or spin, the volley against ground
/// power (net players grade up), Impact the early rally-hit window less a little serve power (the harder a
/// character hits, the harder to time), Footwork speed less a little reach. `power` rows give the POWER columns.
pub fn scores(row: &[String], power: [&[String]; 3]) -> [f64; 5] {
    let v = |r: &[String], c: usize| r.get(c).and_then(|s| s.split('/').next()?.trim().parse::<f64>().ok()).unwrap_or(0.0);
    let [serve, stroke, volley] = [0, 1, 2].map(|k| v(power[k], POWER[k]));
    [
        serve + 0.1 * v(row, 22),                       // Serv CON
        stroke + 0.1 * v(row, 17),                      // Top SPIN
        volley - 0.25 * stroke + 0.1 * v(row, 21),      // Voley CON
        v(row, 28) - 0.1 * serve,                       // rally hits' early good frames
        v(row, 40) - 0.01 * v(row, 57),                 // SPE, reach
    ]
}

/// The grades `row` plays at ([`scores`], POWER from `power`): per category, each cut halfway between the stock
/// characters' (`rows`, `grades`) neighbouring grades; [`S`] as far past the best of them as that one is past A's cut.
/// A stock row gets its own grades.
pub fn grades_of(row: &[String], power: [&[String]; 3], rows: &[Vec<String>], grades: &[[u8; 5]]) -> [u8; 5] {
    let stock: Vec<[f64; 5]> = rows.iter().map(|r| scores(r, [r, r, r])).collect();
    let own = scores(row, power);
    std::array::from_fn(|k| {
        let at = |pick: &dyn Fn(u8) -> bool| -> Vec<f64> { stock.iter().zip(grades).filter(|(_, g)| pick(g[k])).map(|(s, _)| s[k]).collect() };
        let cut = |g: u8| {
            let below = at(&|x| x < g).into_iter().fold(f64::NEG_INFINITY, f64::max);
            let above = at(&|x| x >= g).into_iter().fold(f64::INFINITY, f64::min);
            if below.is_finite() && above.is_finite() { (below + above) / 2.0 } else { f64::INFINITY }
        };
        let top = at(&|_| true).into_iter().fold(f64::NEG_INFINITY, f64::max);
        let grade = (1..=5).filter(|&g| own[k] > cut(g)).count() as u8;
        if grade == 5 && own[k] > 2.0 * top - cut(5) { S } else { grade }
    })
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
        let out = graded_row(&own, &rows, &grades, [5; 5]);
        assert_eq!(out[12], "212", "Serve A: characters 0 and 1");
        assert_eq!(out[13], "913", "Stroke A: character 2");
        assert_eq!(out[14], "914", "Volley: nearest to A is character 2's 2");
        assert_eq!(out[28], "928");
        assert_eq!(out[40], "940");
        let s = graded_row(&own, &rows, &grades, [S; 5]);
        let mixed = graded_row(&own, &rows, &grades, [0, 5, 5, 5, 5]);
        assert_eq!((mixed[12].as_str(), mixed[13].as_str()), ("912", "913"), "Serve F, Stroke A");
        assert_eq!(s[12], "112", "Serve S: A (1, 3) below F (9), so the roster's smallest");
        assert_eq!(s[13], "913", "Stroke S: A (9) above F (1, 3)");
        assert_eq!(s[23], own[23]);
        for c in [0, 11, 23, 55, 64] {
            assert_eq!(out[c], own[c], "cell {c} is the character's own");
        }
    }

    /// Every stock row reads back as its own grades, and a row past the roster's best is an S.
    #[test]
    fn rows_read_back_as_their_grades() {
        let Ok(mut iso) = hst_data::fixture::iso() else { return hst_data::fixture::skip("no ISO") };
        let grades = disc_grades(&mut iso).unwrap();
        let csv = crate::mods::tparam_csv(&mut iso).unwrap();
        let rows: Vec<Vec<String>> = (0..CHARACTERS).map(|c| crate::mods::row_of(&csv, c).unwrap()).collect();
        for c in 0..CHARACTERS {
            let r = &rows[c][..];
            assert_eq!(grades_of(r, [r, r, r], &rows, &grades), grades[c], "character {c}");
        }
        let mut fast = rows[5].clone(); // the best Footwork
        fast[40] = "14".into();
        assert_eq!(grades_of(&fast, [&rows[5]; 3], &rows, &grades)[4], S);
    }
}
