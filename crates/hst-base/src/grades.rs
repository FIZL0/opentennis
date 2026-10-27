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
/// character hits, the harder to time), Footwork speed less a little reach. `power` rows give the POWER columns;
/// `gain` ([`power_gain`]) is what the serve's, strokes' and volleys' tables fly faster than those rows' own.
pub fn scores(row: &[String], power: [&[String]; 3], gain: [f64; 3]) -> [f64; 5] {
    let v = |r: &[String], c: usize| r.get(c).and_then(|s| s.split('/').next()?.trim().parse::<f64>().ok()).unwrap_or(0.0);
    let [serve, stroke, volley] = [0, 1, 2].map(|k| v(power[k], POWER[k]));
    [
        serve + gain[0] + 0.1 * v(row, 22),             // Serv CON
        stroke + gain[1] + 0.1 * v(row, 17),            // Top SPIN
        volley + gain[2] - 0.25 * stroke + 0.1 * v(row, 21), // Voley CON
        v(row, 28) - 0.1 * serve,                       // rally hits' early good frames
        v(row, 40) - 0.01 * v(row, 57),                 // SPE, reach
    ]
}

/// The grades `row` plays at ([`scores`], POWER from `power`, faster tables' `gain`): per category, each cut halfway
/// between the stock characters' (`rows`, `grades`) neighbouring grades; [`S`] as far past the best of them as that
/// one is past A's cut. A stock row gets its own grades.
pub fn grades_of(row: &[String], power: [&[String]; 3], gain: [f64; 3], rows: &[Vec<String>], grades: &[[u8; 5]]) -> [u8; 5] {
    let stock: Vec<[f64; 5]> = rows.iter().map(|r| scores(r, [r, r, r], [0.0; 3])).collect();
    let own = scores(row, power, gain);
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

/// The POW (TParam.csv's Serv / Strk / Voley POW) one mph of a class's mean table speed is worth: the stock
/// characters' tables were authored from their POW, and across the 14 their serve, ground stroke and volley tables'
/// mean speeds rise with it in a near straight line (r 0.97, 0.99, 1.00; research/journal/2026-10-27-m151c-grades).
const POW_PER_MPH: [f64; 3] = [0.305, 0.937, 0.651];
/// Metres a frame to mph (60 frames a second).
const MPH: f64 = 60.0 * 3600.0 / 1609.344;
/// Every this many cells along each axis a table's speed is sampled at (4 × 4 × 4 of its 16³).
const SAMPLE: usize = 5;

/// A mod's serve, strokes and volleys at a §6 `shots` `power`, as the POW they'd take to fly that fast: the
/// generated tables' mean speed (`Table::mean_speed`, the generator's own cells on a sample) less the stock ones' it
/// was built from, by [`POW_PER_MPH`]. Topspin, slice and flat of each (the serve's base tables), the way the match
/// regenerates them. 0 at power 1; remembered for the run, by character, class and power.
pub fn power_gain(iso: &mut Iso, m: &crate::mods::Mod) -> [f64; 3] {
    let from = [0, 1, 2].map(|k| m.build.shots[k].map_or(m.donor, usize::from));
    gains(iso, from, [0, 1, 2].map(|k| m.build.power[k]))
}

/// [`power_gain`] of serves, strokes and volleys from characters `from` at `power`.
fn gains(iso: &mut Iso, from: [usize; 3], power: [f32; 3]) -> [f64; 3] {
    static SEEN: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<(usize, usize, u32), f64>>> = std::sync::LazyLock::new(Default::default);
    let mut out = [0.0; 3];
    let Some(params) = shot_params(iso) else { return out };
    for (k, gain) in out.iter_mut().enumerate().filter(|&(k, _)| power[k] != 1.0) {
        let (c, p) = (from[k], power[k]);
        if let Some(&g) = SEEN.lock().unwrap().get(&(c, k, p.to_bits())) {
            *gain = g;
            continue;
        }
        let Some(tables) = class_tables(iso, c, k) else { continue };
        let mean = |p: f32| {
            let one = |(kind, t): (usize, &hst_sim::shot::Table)| class_speed(&params, c, k, kind, t, p);
            #[cfg(not(target_arch = "wasm32"))]
            let speeds: Vec<f64> = std::thread::scope(|s| tables.iter().enumerate().map(|t| s.spawn(move || one(t))).collect::<Vec<_>>().into_iter().map(|h| h.join().unwrap()).collect());
            #[cfg(target_arch = "wasm32")]
            let speeds: Vec<f64> = tables.iter().enumerate().map(one).collect();
            speeds.iter().sum::<f64>() / speeds.len() as f64
        };
        *gain = (mean(p) - mean(1.0)) * MPH * POW_PER_MPH[k];
        SEEN.lock().unwrap().insert((c, k, p.to_bits()), *gain);
    }
    out
}

/// The disc's shot records.
fn shot_params(iso: &mut Iso) -> Option<hst_sim::params::ShotParams> {
    let src = hst_data::exe::Game::new(&iso.read("SYSTEM.CNF").ok()?, &iso.read("ZZBIN/GAME.BIN").ok()?).ok()?.shot_params();
    Some(hst_sim::params::ShotParams::build(&src.base, &src.kinds, &src.weights, src.middle_mix))
}

/// Character `c`'s topspin, slice and flat tables of class `k` (0 serve, 1 stroke, 2 volley).
fn class_tables(iso: &mut Iso, c: usize, k: usize) -> Option<Vec<hst_sim::shot::Table>> {
    let name = ["serv", "strk", "voly"][k];
    let archives: Vec<Vec<u8>> = ["A", "B"].iter().map(|ab| iso.read(&format!("TRAJ/TRAJ{c:02}{ab}.XB"))).collect::<Result<_, _>>().ok()?;
    let archives: Vec<hst_data::xb::Archive> = archives.iter().map(|d| hst_data::xb::Archive::parse(d).ok()).collect::<Option<_>>()?;
    (0..3)
        .map(|kind| {
            let file = format!("tr_pc{c:02}_{name}{kind}.dat");
            let (arc, e) = archives.iter().find_map(|a| Some((a, a.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(&file))?)))?;
            hst_sim::shot::Table::parse(&arc.read(e).ok()?)
        })
        .collect()
}

/// Table `t`'s ([`class_tables`]) mean speed at power `p` (m a frame), each cell launched with the spin its shot
/// record gives it.
fn class_speed(params: &hst_sim::params::ShotParams, c: usize, k: usize, kind: usize, t: &hst_sim::shot::Table, p: f32) -> f64 {
    use hst_sim::{params, shot::Bounds};
    let rec = params.record(k, kind, params::record_of(c));
    let speed = match k {
        0 => {
            let spin = params::spin(rec);
            t.mean_speed(&Bounds::serve(false, hst_sim::ball::Params::default().radius), p, |_, _| spin, SAMPLE)
        }
        _ => {
            let b = if k == 1 { Bounds::stroke(kind as i32, 0.0) } else { Bounds::volley(kind as i32, 0.0) };
            t.mean_speed(&b, p, |hit, target| params::rally_spin(rec, k, kind, hit, target).0, SAMPLE)
        }
    };
    speed as f64
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
            assert_eq!(grades_of(r, [r, r, r], [0.0; 3], &rows, &grades), grades[c], "character {c}");
        }
        let mut fast = rows[5].clone(); // the best Footwork
        fast[40] = "14".into();
        assert_eq!(grades_of(&fast, [&rows[5]; 3], [0.0; 3], &rows, &grades)[4], S);
    }

    /// A serve at power 1.4 from the best server's tables flies far past the roster's best: it reads S, where the POW
    /// it was taken from alone reads A. Strokes a little faster grade up from their stock grade, slower ones down; power
    /// 1 adds nothing, and the gain is the same twice (one sample, cached).
    #[test]
    fn faster_tables_grade_up() {
        let Ok(mut iso) = hst_data::fixture::iso() else { return hst_data::fixture::skip("no ISO") };
        let grades = disc_grades(&mut iso).unwrap();
        let csv = crate::mods::tparam_csv(&mut iso).unwrap();
        let rows: Vec<Vec<String>> = (0..CHARACTERS).map(|c| crate::mods::row_of(&csv, c).unwrap()).collect();
        let start = std::time::Instant::now();
        let gain = gains(&mut iso, [7, 4, 4], [1.4, 1.15, 0.8]);
        eprintln!("gains {gain:?} in {:?}", start.elapsed());
        assert!(start.elapsed().as_secs_f32() < 3.0);
        assert_eq!(gains(&mut iso, [7, 4, 4], [1.4, 1.15, 0.8]), gain);
        assert_eq!(gains(&mut iso, [7, 4, 4], [1.0; 3]), [0.0; 3]);
        assert!(gain[0] > 5.0 && gain[1] > 0.0 && gain[2] < 0.0, "{gain:?}");
        let (chief, b) = (&rows[7][..], &rows[4][..]);
        assert_eq!(grades_of(chief, [chief, b, b], [0.0; 3], &rows, &grades)[0], 5, "his own serve: A");
        let g = grades_of(chief, [chief, b, b], gain, &rows, &grades);
        assert_eq!(g[0], S, "the cannon");
        assert!(g[1] > grades[4][1] && g[2] < grades[4][2], "{g:?} vs {:?}", grades[4]);
    }
}
