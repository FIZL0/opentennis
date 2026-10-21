//! The MENU overlay's character grades, as the original's character select shows them (Ashley, Jun and Kaito
//! checked on screen; the table read live over PINE at the select equals the disc's, P21c). Skips without the ISO.

use hst_data::fixture;
use hst_data::exe::Menu;

#[test]
fn character_grades() {
    let Ok(mut iso) = fixture::iso() else {
        return fixture::skip(format_args!("no ISO, skipped"));
    };
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/MENU.BIN").unwrap());
    let g = Menu::new(&cnf, &bin).unwrap().grades();
    let letters = |c: usize| g[c].map(|v| b"ABCDEF"[5 - v as usize] as char).iter().collect::<String>();
    let want = [
        "EFEAD", "EEFAE", "EFCBC", "DDCCC", "DDCCD", "FEFBA", "CCEDB", // Ashley Cody Jun Kaito Momoko JJ Carol
        "ADBDF", "BBBEB", "CADED", "BCAEC", "ABAFC", "CCCDD", "DDDCC", // Big Chief Miranda Kent Lola Will Gloria Suzuki
    ];
    for (c, w) in want.iter().enumerate() {
        assert_eq!(letters(c), *w, "character {c}");
    }
}
