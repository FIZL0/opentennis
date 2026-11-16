//! The character data files' column layouts per release. Everything that reads `TParam.csv` or `AIParam.csv` reads
//! the US layout; an earlier release's file is widened to it when the archive entry is read ([`us_layout`]), so no
//! reader has to know which disc it is on.
//!
//! - `TParam.csv`: the Japanese release's data rows have 63 cells, the US's 67. It has no serve timing counts of its
//!   own (the US's strong-toss pair and weak-toss pair, cells 30–33): it grades every serve on the rally timing
//!   counts (cells 28, 29). Widening copies those two into the four missing cells, so the serve tables built from
//!   them are the rally table, as the Japanese game uses. Every later cell moves up by four.
//! - `AIParam.csv`: the Japanese data rows have 43 cells, the US's 45 (plus trailing empty ones). The two it lacks
//!   are the lob and special-serve reaction times (cells 23, 24); widening puts 0 there. The Japanese game makes no
//!   reaction draw for either (`hst_sim::build::Build`), so the value is never read there.

/// Data rows of the Japanese `TParam.csv`, and where its missing serve timing cells go.
const TPARAM_JP: usize = 63;
const TPARAM_RALLY: usize = 28;
/// Data rows of the Japanese `AIParam.csv` (non-empty cells), and where its missing reaction cells go.
const AIPARAM_JP: usize = 43;
const AIPARAM_REACT: usize = 23;

/// An archive entry's bytes in the US layout: `TParam.csv` and `AIParam.csv` rows of an earlier release widened (see
/// the module), anything else as it is. A file already in the US layout comes back unchanged.
pub fn us_layout(name: &str, data: Vec<u8>) -> Vec<u8> {
    let name = name.to_ascii_lowercase();
    if name.ends_with("tparam.csv") {
        widen(data, TPARAM_JP, |cells| {
            let rally = [cells[TPARAM_RALLY].clone(), cells[TPARAM_RALLY + 1].clone()];
            cells.splice(TPARAM_RALLY + 2..TPARAM_RALLY + 2, [rally[0].clone(), rally[1].clone(), rally[0].clone(), rally[1].clone()]);
        })
    } else if name.ends_with("aiparam.csv") {
        widen(data, AIPARAM_JP, |cells| {
            cells.splice(AIPARAM_REACT..AIPARAM_REACT, [b"0".to_vec(), b"0".to_vec()]);
        })
    } else {
        data
    }
}

/// `data` with every data row (first cell a number) of exactly `cells` non-empty-trailing cells changed by `fix`.
/// Rows are compared without their trailing empty cells and line end, which are kept.
fn widen(data: Vec<u8>, cells: usize, fix: impl Fn(&mut Vec<Vec<u8>>)) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 16);
    let mut changed = false;
    for (i, line) in data.split(|&b| b == b'\n').enumerate() {
        if i > 0 {
            out.push(b'\n');
        }
        let body_end = line.iter().rposition(|&b| b != b'\r').map_or(0, |p| p + 1);
        let (body, cr) = line.split_at(body_end);
        let mut row: Vec<Vec<u8>> = body.split(|&b| b == b',').map(<[u8]>::to_vec).collect();
        let trailing = row.iter().rev().take_while(|c| c.is_empty()).count();
        let filled = row.len() - trailing;
        if row[0].first().is_some_and(u8::is_ascii_digit) && filled == cells {
            let tail = row.split_off(filled);
            fix(&mut row);
            row.extend(tail);
            out.extend_from_slice(&row.join(&b','));
            changed = true;
        } else {
            out.extend_from_slice(body);
        }
        out.extend_from_slice(cr);
    }
    if changed { out } else { data }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tparam_rows_take_the_rally_counts_as_serve_counts() {
        let cells: Vec<String> = (0..63).map(|i| if i == 28 { "9/0/10".into() } else if i == 29 { "4/0/8".into() } else { i.to_string() }).collect();
        let csv = format!("#,head\r\n{}\r\n", cells.join(","));
        let out = String::from_utf8(us_layout("data\\TParam.csv", csv.into_bytes())).unwrap();
        let row: Vec<&str> = out.lines().nth(1).unwrap().split(',').collect();
        assert_eq!(row.len(), 67);
        assert_eq!(&row[28..34], ["9/0/10", "4/0/8", "9/0/10", "4/0/8", "9/0/10", "4/0/8"]);
        // the Japanese column c − 4 is the US column c from 34 on
        assert_eq!(row[34], "30");
        assert_eq!(row[66], "62");
        assert!(out.starts_with("#,head\r\n") && out.ends_with("\r\n"));
    }

    #[test]
    fn aiparam_rows_get_zero_lob_and_special_serve_reactions() {
        let cells: Vec<String> = (0..43).map(|i| i.to_string()).collect();
        let csv = format!("{}\n", cells.join(","));
        let out = String::from_utf8(us_layout("AIParam.csv", csv.into_bytes())).unwrap();
        let row: Vec<&str> = out.trim_end().split(',').collect();
        assert_eq!(row.len(), 45);
        assert_eq!(&row[21..26], ["21", "22", "0", "0", "23"]);
    }

    #[test]
    fn us_rows_are_left_alone() {
        let us: Vec<String> = (0..67).map(|i| i.to_string()).collect();
        let csv = format!("{}\r\n", us.join(",")).into_bytes();
        assert_eq!(us_layout("TParam.csv", csv.clone()), csv);
        // a US AIParam row: 45 cells and trailing empties
        let ai = format!("{},,,,\n", (0..45).map(|i| i.to_string()).collect::<Vec<_>>().join(",")).into_bytes();
        assert_eq!(us_layout("AIParam.csv", ai.clone()), ai);
    }
}
