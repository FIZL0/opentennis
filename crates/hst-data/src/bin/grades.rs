//! grades <disc.iso> — the select screen's grades (Serve, Stroke, Volley, Impact, Footwork; 0 F … 5 A) of the 14
//! disc characters, one line each, for the mods repo's tools.
fn main() {
    let path = std::env::args().nth(1).expect("usage: grades <disc.iso>");
    let mut iso = hst_data::iso::Iso::open(&path).unwrap();
    let (cnf, bin) = (iso.read("SYSTEM.CNF").unwrap(), iso.read("ZZBIN/MENU.BIN").unwrap());
    for g in hst_data::exe::Menu::new(&cnf, &bin).unwrap().grades() {
        println!("{}", g.map(|v| v.to_string()).join(" "));
    }
}
