//! M175: Settings → Gameplay → Rules: which release's game a match plays: the US one (Hot Shots Tennis) or the
//! Japanese one (Minna no Tennis); the European disc plays the US game (on its own, on regenerated shot tables: M170).
//! A choice shows only where that release's disc is here: the main disc, or another release beside it (M166
//! registers those with [`add_sibling`]). The default is the main disc's own.
//!
//! Choosing another release lays that disc's gameplay data (what `hst_data::digest` counts: the shot tables, the
//! character tables, the program's tuning) over the main disc, the way an online guest takes its host's
//! (`hst_data::crossplay`), checked to play exactly as that disc: from then on every `Iso::open` of the main disc reads
//! the chosen release's game with the main disc's own textures, sounds and voices. So an European disc with a US disc
//! beside it plays the US game exactly.
//!
//! Online the lobby plays its host's rules (`net`, M171): a host's choice stays laid and is what its lobby plays and
//! streams to guests ([`lobby`], [`laid`]); a guest's is taken off while it is in a lobby ([`Online::join`]), so it
//! plays the host's data and its own choice counts for nothing. Neither changes until the lobby ends.

use hst_data::crossplay::{self, Patch};
use hst_data::iso::Iso;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering::Relaxed};

/// The releases, by the setting's word (`handshake::rules_of`); index 0 is "the main disc's own".
pub const RULES: [&str; 4] = ["", "us", "jp", "eu"];

/// The chosen [`RULES`] index (0: the main disc's own).
static CHOSEN: AtomicU8 = AtomicU8::new(0);
/// Other releases' discs beside the main one: (their [`RULES`] index, path).
static SIBLINGS: Mutex<Vec<(u8, String)>> = Mutex::new(Vec::new());
/// The overlay this module laid on the main disc: (disc, release, its patch, the gameplay digest it reads as), kept
/// to lay it again after a lobby.
static OURS: Mutex<Option<(String, u8, Patch, u64)>> = Mutex::new(None);
/// Lobbies this app is in ([`Online`]): the overlay stays off while any is.
static ONLINE: AtomicUsize = AtomicUsize::new(0);

/// `rules` (`"us"`, `"jp"`, `"eu"`) as a [`RULES`] index, 0 when unknown.
fn index(rules: &str) -> u8 {
    RULES.iter().position(|r| !r.is_empty() && *r == rules).unwrap_or(0) as u8
}

/// The main disc's release ([`RULES`] index); the US one without a disc or for one not known.
pub fn own() -> u8 {
    match index(crate::handshake::rules()) {
        0 => 1,
        r => r,
    }
}

/// The releases a match can play here, in [`RULES`] order: the main disc's and its siblings'.
pub fn available() -> Vec<u8> {
    let own = own();
    let sib = SIBLINGS.lock().unwrap();
    (1..RULES.len() as u8).filter(|&r| r == own || sib.iter().any(|s| s.0 == r)).collect()
}

/// The release a match plays offline: the chosen one where its disc is here, else the main disc's.
pub fn chosen() -> u8 {
    let c = CHOSEN.load(Relaxed);
    if c != 0 && available().contains(&c) { c } else { own() }
}

/// The setting as saved (0: the main disc's own); kept when its disc isn't here, so it comes back with it.
pub fn setting() -> u8 {
    CHOSEN.load(Relaxed)
}

/// Settings → Rules picks `r` (a [`RULES`] index; 0 the main disc's own) and the main disc reads as it from now on.
pub fn choose(r: u8) {
    CHOSEN.store(r.min(RULES.len() as u8 - 1), Relaxed);
    apply();
}

/// A release's name in the player's words.
pub fn name(r: u8) -> &'static str {
    match RULES[r as usize] {
        "us" => "US",
        "jp" => "Japanese (Minna no Tennis)",
        "eu" => "European",
        _ => "Disc's own",
    }
}

/// The settings.txt line (`rules = jp`; `rules = disc` for the main disc's own).
pub fn setting_line(r: u8) -> String {
    format!("rules = {}\n", if r == 0 { "disc" } else { RULES[r as usize] })
}

/// The release a settings.txt names, else 0 (the main disc's own).
pub fn parse_setting(text: &str) -> u8 {
    text.lines()
        .rev()
        .filter_map(|l| l.split('#').next()?.split_once('='))
        .find(|(k, _)| k.trim() == "rules")
        .map_or(0, |(_, v)| index(v.trim()))
}

/// M166: another release's disc at `path`, beside the main one: its release joins the Rules choices. The release it
/// is (`None`: not a Hot Shots disc, the main disc's own release, or already there).
pub fn add_sibling(path: &str) -> Option<u8> {
    let cnf = Iso::open_own(path).ok()?.read_unlogged("SYSTEM.CNF").ok()?;
    let r = index(crate::handshake::rules_of(&cnf));
    {
        let mut sib = SIBLINGS.lock().unwrap();
        if r == 0 || r == own() || sib.iter().any(|s| s.0 == r) {
            return None;
        }
        sib.push((r, path.to_string()));
    }
    apply();
    Some(r)
}

/// The sibling disc of release `r`.
fn sibling(r: u8) -> Option<String> {
    SIBLINGS.lock().unwrap().iter().find(|s| s.0 == r).map(|s| s.1.clone())
}

/// Lays the chosen release's data over the main disc, or takes this module's off when it is the disc's own; nothing
/// while in a lobby ([`Online`]) or before a disc. Where the other disc can't be read or doesn't check out, the match
/// plays the main disc's own (the reason on stderr).
pub fn apply() {
    if ONLINE.load(Relaxed) > 0 {
        return;
    }
    let Some(disc) = crate::handshake::disc() else { return };
    let want = chosen();
    let mut ours = OURS.lock().unwrap();
    if want == own() {
        if let Some((d, ..)) = ours.take() {
            crossplay::remove(d);
        }
        return;
    }
    match ours.as_ref() {
        // laid already (after a lobby took it off: again, from the patch kept)
        Some((d, r, p, _)) if *d == disc && *r == want => {
            if !crossplay::installed(d) {
                crossplay::install(d, p.clone());
            }
            return;
        }
        Some((d, ..)) => {
            crossplay::remove(d);
            *ours = None;
        }
        None => {}
    }
    match lay(&disc, want) {
        Ok((p, digest)) => *ours = Some((disc, want, p, digest)),
        Err(e) => eprintln!("hst: {} rules unavailable, playing the disc's own: {e}", name(want)),
    }
}

/// Lays sibling release `r`'s gameplay data over `disc`, checked to digest as that disc: the patch laid and that digest.
fn lay(disc: &str, r: u8) -> Result<(Patch, u64), String> {
    lay_from(disc, &sibling(r).ok_or("no disc of that release here")?)
}

/// Lays the gameplay data of the disc at `path` over `disc`, checked to digest as that disc.
fn lay_from(disc: &str, path: &str) -> Result<(Patch, u64), String> {
    let (want, _) = survey(path)?;
    let (_, here) = survey(disc)?;
    let mut sib = Iso::open_own(path).map_err(|e| e.to_string())?;
    let p = crossplay::patch_for(|f| sib.read_unlogged(f).ok(), &here)?;
    crossplay::verify(disc, p.clone(), want)?;
    Ok((p, want))
}

/// The digest the main disc reads as with this module's overlay laid on it, `None` when there is none.
fn laid_digest() -> Option<u64> {
    let disc = crate::handshake::disc()?;
    OURS.lock().unwrap().as_ref().filter(|o| o.0 == disc && crossplay::installed(&o.0)).map(|o| o.3)
}

/// Whether the main disc reads as another release (the Rules choice laid): a host then streams its data from the
/// disc as read, not the disc alone (`handshake::patch_for`).
pub fn laid() -> bool {
    laid_digest().is_some()
}

/// A lobby this app hosts: the rules it plays and their gameplay digest (`Room::rules`, `Room::sim`): the Rules
/// choice where it is laid, else the main disc's own.
pub fn lobby() -> (String, Option<u64>) {
    match laid_digest() {
        Some(d) => (RULES[chosen() as usize].into(), Some(d)),
        None => (crate::handshake::rules().into(), crate::handshake::sim()),
    }
}

/// A disc's own gameplay digest and manifest (no overlay).
fn survey(path: &str) -> Result<(u64, hst_data::digest::Manifest), String> {
    let mut iso = Iso::open_own(path).map_err(|e| e.to_string())?;
    hst_data::digest::survey(|f| iso.read_unlogged(f).ok())
}

/// Held while this app is in an online lobby (the match included): the Rules choice stays as it is meanwhile;
/// dropping the last lays it again where a guest's was taken off.
pub struct Online(());

impl Online {
    /// Hosting: the choice stays laid, the lobby plays it ([`lobby`]).
    pub fn host() -> Online {
        ONLINE.fetch_add(1, Relaxed);
        Online(())
    }

    /// Joining: the choice comes off, so the disc plays the host's rules (its data where they differ, M171).
    pub fn join() -> Online {
        ONLINE.fetch_add(1, Relaxed);
        if let Some((d, ..)) = OURS.lock().unwrap().as_ref() {
            crossplay::remove(d);
        }
        Online(())
    }
}

impl Drop for Online {
    fn drop(&mut self) {
        if ONLINE.fetch_sub(1, Relaxed) == 1 {
            apply();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_words() {
        assert_eq!(parse_setting(""), 0);
        assert_eq!(parse_setting("rules = jp\n"), 2);
        assert_eq!(parse_setting("rules = us\nrules = eu\n"), 3);
        assert_eq!(parse_setting("rules = xx\n"), 0);
        for r in 0..RULES.len() as u8 {
            assert_eq!(parse_setting(&setting_line(r)), r);
        }
        assert_eq!(index(""), 0);
        assert_eq!((1..4).map(name).collect::<Vec<_>>(), ["US", "Japanese (Minna no Tennis)", "European"]);
    }

    /// `disc` under a path of its own (the overlay is keyed by path, so no other test's reads of it change).
    fn own_path(disc: &std::path::Path, tag: &str) -> String {
        let dir = std::env::temp_dir().join(format!("hst-rules-{}-{tag}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let link = dir.join("disc.iso");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(std::fs::canonicalize(disc).unwrap(), &link).unwrap();
        link.to_string_lossy().into()
    }

    /// The main disc with another release's disc beside it, that release chosen: the main disc reads as the other
    /// one's game (its digest, its release's SYSTEM.CNF and so `Build`), its own textures and voices kept; taken off,
    /// it reads as itself. The US disc with the Japanese one beside it, and the European with the US one (it then
    /// plays the US game exactly, no regenerated tables). Skipped without those discs (not the user's).
    /// It lays the other disc by its path, not through the shared sibling list: other tests register siblings in temp
    /// dirs they delete afterwards (`releases`), and the list hands back the first of a release.
    #[test]
    fn another_release_beside_the_main_disc() {
        use hst_data::fixture;
        let (us, jp, eu) = (fixture::path(fixture::ISO), fixture::path("fixtures/discs/SCPS-15113.iso"), fixture::path("fixtures/discs/SCES-54535.iso"));
        if !fixture::has_iso() || !jp.exists() || !eu.exists() {
            return eprintln!("skipped: no ISO, fixtures/discs/SCPS-15113.iso or SCES-54535.iso");
        }
        for (main, beside, r, tag) in [(&us, &jp, 2, "us-jp"), (&eu, &us, 1, "eu-us")] {
            let disc = own_path(main, tag);
            let beside = beside.to_string_lossy();
            let (want, _) = survey(&beside).unwrap();
            let (alone, _) = survey(&disc).unwrap();
            assert_ne!(want, alone, "{tag}: the two discs play alike");
            assert_eq!(lay_from(&disc, &beside).unwrap().1, want);
            let mut iso = Iso::open(&disc).unwrap();
            assert_eq!(hst_data::digest::of(&mut iso).unwrap(), want, "{tag}");
            let cnf = iso.read("SYSTEM.CNF").unwrap();
            assert_eq!(index(crate::handshake::rules_of(&cnf)), r, "{tag}");
            // the shot tables play as read (the European disc's 60 Hz regeneration stays off under the US data)
            hst_sim::shot::regen::install();
            assert_eq!(iso.read("TRAJ/TRAJ00A.XB").unwrap(), iso.read_unlogged("TRAJ/TRAJ00A.XB").unwrap(), "{tag}");
            // the menus' text stays the main disc's
            let mut alone_iso = Iso::open_own(&disc).unwrap();
            assert_eq!(iso.read("MENU/MENU00B.XB0").ok(), alone_iso.read_unlogged("MENU/MENU00B.XB0").ok(), "{tag}");
            // hosting on it: a guest on the main disc alone gets the other release's data from it, as read
            let guest = own_path(main, &format!("{tag}-guest"));
            let (_, gm) = survey(&guest).unwrap();
            let patch = crossplay::patch_for(|f| iso.read_unlogged(f).ok(), &gm).unwrap();
            crossplay::verify(&guest, patch, want).unwrap();
            assert!(crossplay::remove(&guest));
            assert!(crossplay::remove(&disc));
            assert_eq!(hst_data::digest::of(&mut Iso::open(&disc).unwrap()).unwrap(), alone, "{tag}");
        }
    }
}
