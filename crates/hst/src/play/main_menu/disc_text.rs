//! The menus' text table (`message0.dat` in the menu archive) and our own words for a disc without one: the
//! Japanese release (M163a1a) keeps its menu text inside its program, in Japanese, which the menus' letter sheet
//! can't set. The few entries the menus read get plain English there; court and umpire names become numbers.

use hst_data::iso::Iso;
use hst_data::xb::Archive;

/// The disc's serial for the main menu's header ("SCUS-97610", "SCPS-15113"), noted when the menus read their text.
static SERIAL: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

/// The serial of the disc the menus last read ([`messages`]); the US one before any.
pub(in crate::play) fn serial() -> String {
    let s = SERIAL.read().map(|s| s.clone()).unwrap_or_default();
    if s.is_empty() { "SCUS-97610".into() } else { s }
}

/// The menu archive's text table, or [`fallback`] when the disc has none.
pub(in crate::play) fn messages(iso: &mut Iso) -> Vec<String> {
    let release = iso.read("SYSTEM.CNF").ok().and_then(|cnf| hst_data::exe::Release::of(&cnf).ok());
    if let (Some(r), Ok(mut s)) = (release, SERIAL.write()) {
        *s = r.name().split(' ').next().unwrap_or_default().to_string();
    }
    let table = iso.read("MENU/MENU00B.XB0").ok().and_then(|data| {
        let arc = Archive::parse(&data).ok()?;
        let e = arc.entries.iter().find(|e| e.name.to_ascii_lowercase().replace('\\', "/").ends_with("/message0.dat"))?;
        arc.read(e).ok()
    });
    match table {
        Some(d) => super::messages(&d),
        None => fallback(),
    }
}

/// Where the menus read from the table: the Local and Settings rows' words, the mode screen's line, each court's
/// two-line name and each umpire's.
const LOCAL: usize = 188;
const SETTINGS: usize = 190;
const MODE: usize = 192;
const COURT: usize = 276;
const UMPIRE: usize = 669;

/// Our own words at the entries the menus read, everything else empty.
fn fallback() -> Vec<String> {
    let mut t = vec![String::new(); UMPIRE + 2 * super::UMPIRES];
    t[LOCAL] = "Play a match with one to four players, against each other or the computer.".into();
    t[SETTINGS] = "Display, graphics, sound and controller settings.".into();
    t[MODE] = "Singles is one player against one, doubles two against two.".into();
    for c in 0..super::COURTS {
        t[COURT + 2 * c] = format!("Court {}", c + 1);
    }
    for u in 0..super::UMPIRES {
        t[UMPIRE + 2 * u] = format!("Umpire {}", u + 1);
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every entry the menus read has words, and the court and umpire captions read as the disc's do.
    #[test]
    fn the_fallback_fills_what_the_menus_read() {
        let t = fallback();
        assert!([LOCAL, SETTINGS, MODE].iter().all(|&i| !t[i].is_empty()));
        assert_eq!((t[COURT].as_str(), t[COURT + 1].as_str()), ("Court 1", ""));
        assert_eq!(super::super::pause::umpire_caption(&t[UMPIRE + 2]), "Umpire 2");
    }

    /// The Japanese disc (`fixtures/discs/SCPS-15113.iso`; skipped without it, even under HST_REQUIRE_DISC: it isn't
    /// the user's disc) has no text table: the menus get the fallback instead of stopping.
    #[test]
    fn the_japanese_disc_reads_the_fallback() {
        let path = hst_data::fixture::path("fixtures/discs/SCPS-15113.iso");
        if !path.exists() {
            return eprintln!("skipped: no fixtures/discs/SCPS-15113.iso");
        }
        let mut iso = Iso::open(&path).unwrap();
        assert_eq!(messages(&mut iso), fallback());
    }
}
