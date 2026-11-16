//! The disc's gameplay digest (M163b): one 64-bit hash of everything on a disc that feeds the match simulation, so two
//! players can be told apart by what their discs *play*, not by their bytes. Two releases, or two imports of one
//! release, with the same digest play the same match; a digest that differs means the matches would drift apart.
//!
//! What goes in:
//! - the game program's tuning tables (`exe::Game`), read in the US layout whatever the release (so a release that
//!   only moves them around digests the same), except what only looks or sounds different (HUD colours, ball and
//!   crowd sounds, cheer sprites, rain and weather looks, bounce looks, player lighting) and the umpire's word timing
//!   for the languages the match doesn't use;
//! - the archives the match reads its rules and world from: the shot tables (`TRAJ`), character data and CSVs
//!   (`PCDATA`), motions (`PCANI`), character models (their skeletons pose the players), the courts' layout,
//!   collision models and camera data, the ball archive, the effects archive and the match intro's banner motion.
//!   Of those only the data entries count (layout, tables, models, materials, motions, cameras); textures, texture
//!   pixels, sound banks, face morphs and texture animations are left out, as are a release's own file names and
//!   archive layout.
//!
//! Names are the US disc's; another release's files answer to them through `Iso::read`'s aliases.

use crate::exe::Game;
use crate::iso::Iso;
use crate::xb::Archive;
use std::hash::Hasher;
use twox_hash::XxHash3_64;

/// Characters on every disc (their `PCnn` files).
const CHARACTERS: usize = 14;
/// Costumes per character (`PC/PCnnCcc.XB`).
const COSTUMES: usize = 10;
/// Court numbers the match can load (`COURT/nn`); a missing one counts as missing.
const COURTS: u32 = 14;

/// The disc's gameplay digest, read through `iso` (without noting the reads in the asset-use log).
pub fn of(iso: &mut Iso) -> Result<u64, String> {
    of_files(|path| iso.read_unlogged(path).ok())
}

/// The gameplay digest of the disc whose files `read` returns (by US name; `None`: not on it). An error says why the
/// disc has none: its program's tables can't be read (a release the remaster doesn't read).
pub fn of_files(read: impl FnMut(&str) -> Option<Vec<u8>>) -> Result<u64, String> {
    of_entries(read, |_, _, b| b)
}

/// `of_files` with each counted entry's bytes passed through `map(path, name, bytes)` (tests change one).
fn of_entries(mut read: impl FnMut(&str) -> Option<Vec<u8>>, mut map: impl FnMut(&str, &str, Vec<u8>) -> Vec<u8>) -> Result<u64, String> {
    let mut h = XxHash3_64::new();
    let cnf = read("SYSTEM.CNF").ok_or("no SYSTEM.CNF")?;
    let bin = read("ZZBIN/GAME.BIN").ok_or("no game program")?;
    let game = Game::new(&cnf, &bin).map_err(|e| e.to_string())?;
    put(&mut h, tables(&game).as_bytes());
    for (path, only) in sources() {
        put(&mut h, path.as_bytes());
        match read(&path) {
            None => h.write_u8(0xff),
            Some(data) => match Archive::parse(&data) {
                Err(_) => put(&mut h, &data),
                Ok(arc) => {
                    let mut entries: Vec<(String, Vec<u8>)> = arc
                        .entries
                        .iter()
                        .filter_map(|e| {
                            let name = e.name.rsplit(['/', '\\']).next().unwrap_or(&e.name).to_ascii_lowercase();
                            let ext = name.rsplit_once('.').map_or("", |(_, x)| x);
                            only(ext).then(|| arc.read(e).ok().map(|b| (name.clone(), map(&path, &name, b)))).flatten()
                        })
                        .collect();
                    entries.sort();
                    h.write_u64(entries.len() as u64);
                    for (name, bytes) in &entries {
                        put(&mut h, name.as_bytes());
                        put(&mut h, bytes);
                    }
                }
            },
        }
    }
    Ok(h.finish())
}

/// `bytes`, length first, so two fields can't run into each other.
fn put(h: &mut XxHash3_64, bytes: &[u8]) {
    h.write_u64(bytes.len() as u64);
    h.write(bytes);
}

/// The data an archive's entry holds, by its extension: layout and tables (`dat`, `txt`, `csv`), models and their
/// materials (`mdl`, `mtl`), motions (`ani`, `ani2`), cameras (`cam`) and height maps (`hmp`).
fn data(ext: &str) -> bool {
    matches!(ext, "dat" | "txt" | "csv" | "mdl" | "mtl" | "ani" | "ani2" | "cam" | "hmp")
}

/// The archives that feed the match, each with which of its entries count (by extension), in a fixed order.
fn sources() -> Vec<(String, fn(&str) -> bool)> {
    let mut out: Vec<(String, fn(&str) -> bool)> = Vec::new();
    for c in 0..CHARACTERS {
        for ab in ["A", "B"] {
            out.push((format!("TRAJ/TRAJ{c:02}{ab}.XB"), data));
        }
    }
    for p in ["PCDATA/PCDATA.XB", "PCDATA/PCCG0.XB", "CMN/GAME.XB", "AZUMA/C_EFF/EFFCT.XB0"] {
        out.push((p.into(), data));
    }
    // the match intro's length follows its banner motion; the rest of the panel archive is pictures
    out.push(("AZUMA/INPANE/INPANE.XB0".into(), |ext| ext == "ani"));
    for c in 0..CHARACTERS {
        out.push((format!("PCANI/PC{c:02}ANI.XB"), data));
        for k in 0..COSTUMES {
            out.push((format!("PC/PC{c:02}C{k:02}.XB"), data));
        }
    }
    for n in 0..COURTS {
        for arc in ["CMN.XB", "GRD01.XB", "HOL01.XB", "SSN1/HOL01.XB"] {
            out.push((format!("COURT/{n:02}/{arc}"), data));
        }
    }
    out
}

/// The program tables the match reads, as text (their `Debug` forms), in a fixed order.
fn tables(g: &Game) -> String {
    use std::fmt::Write;
    let mut t = String::new();
    let p = g.shot_params();
    let _ = write!(t, "{:?}", (p.base, p.kinds, p.middle_mix, &p.weights));
    for c in 0..CHARACTERS {
        let _ = write!(t, "{:?}", (g.shot_variants(c), g.shot_effects(c), g.umpire_side(c)));
    }
    // material ids past 121 are no material: program bytes past the table, which differ per release
    let _ = write!(t, "{:?}", (g.line_margin(), &g.surfaces()[..122], g.foot(), g.bounce_debris(), g.tornado_fade(), g.tornado_floor()));
    let _ = write!(t, "{:?}", (g.court_margins(), g.second_toss(), g.serve_miss()));
    // a camera shot's bytes 64..68 are a pointer into the program, which sits elsewhere in each release
    let shots: Vec<_> = g.camera_shots().into_iter().map(|mut s| (s.record.splice(64..68, [0; 4]).count(), s)).collect();
    let _ = write!(t, "{:?}", (shots, g.cutaway_lists(), g.replay_cameras()));
    // the match's umpire reads language 0's word timing
    for u in 0..4 {
        for s in 0..2 {
            let _ = write!(t, "{:?}", g.umpire_words(0, u, s));
        }
    }
    for c in 0..13u32 {
        let _ = write!(t, "{:?}", (g.npc_roster(c), g.walkers(c)));
    }
    for c in 1..13u32 {
        let _ = write!(t, "{:?}", (g.wind(c), g.gusts(c), g.weather_odds(c)));
    }
    let _ = write!(t, "{:?}", (g.surf_bearings(), g.ball_routes()));
    for ty in 0..53u8 {
        let _ = write!(t, "{:?}", (g.emitter(ty), g.trigger_model(ty), g.trigger(ty)));
    }
    for v in 0..2u8 {
        for u in 0..2u8 {
            let _ = write!(t, "{:?}", g.scoreboard_timing(v, u));
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;

    /// The digest of the user's disc with file `path`'s entries named `…entry` changed by `f`; `None` when none was.
    fn tampered(iso: &mut Iso, path: &str, entry: &str, f: impl Fn(&mut Vec<u8>)) -> Option<u64> {
        let mut hit = false;
        let d = of_entries(|p| iso.read_unlogged(p).ok(), |p, n, mut b| {
            if p == path && n.ends_with(entry) {
                f(&mut b);
                hit = true;
            }
            b
        })
        .unwrap();
        hit.then_some(d)
    }

    #[test]
    fn digest_follows_gameplay_data_only() {
        let Ok(mut iso) = fixture::iso() else { return };
        let us = of(&mut iso).unwrap();
        assert_eq!(us, of(&mut iso).unwrap(), "same disc, same digest");
        // the character face pictures a pack blanks (old and new imports) aren't in it
        assert_eq!(us, of_files(|p| iso.read_unlogged(p).ok().map(|d| crate::iso::blank(p, d))).unwrap());
        // a shot table changes it
        assert_ne!(Some(us), tampered(&mut iso, "TRAJ/TRAJ03A.XB", ".dat", |b| b[100] ^= 1));
        // a court's plant records change it
        let plants = tampered(&mut iso, "COURT/01/HOL01.XB", "plant_c01_h01_0.dat", |b| b[40] ^= 1);
        assert!(plants.is_some_and(|d| d != us));
        // texture pixels (never counted, so never passed through) don't
        assert_eq!(tampered(&mut iso, "COURT/01/HOL01.XB", ".mti", |b| b[200] ^= 0xff), None);
        // a missing file changes it
        assert_ne!(us, of_files(|p| (p != "PCDATA/PCDATA.XB").then(|| iso.read_unlogged(p).ok()).flatten()).unwrap());
    }

    #[test]
    fn every_source_is_on_the_us_disc_but_courts_it_lacks() {
        let Ok(mut iso) = fixture::iso() else { return };
        let missing: Vec<String> = sources().into_iter().map(|(p, _)| p).filter(|p| iso.read_unlogged(p).is_err()).collect();
        assert!(missing.iter().all(|p| p.starts_with("COURT/")), "{missing:?}");
        // every court the match plays is there
        for n in 1..=11 {
            assert!(!missing.iter().any(|p| p == &format!("COURT/{n:02}/HOL01.XB")), "court {n}");
        }
    }
}
