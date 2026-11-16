//! M166: the Hot Shots releases this player has: the main disc's, and the other releases' discs kept beside it
//! (`hst_data::discs`: any file name, disc ID checked). At start the main disc's folder and `voices/` are looked
//! through on a worker thread; each other release found
//! - has its voices read into `voices/` once, as an ISO put in `voices/` would ([`crate::voices::import_disc`]), so
//!   Voice Language offers its languages without the player adding it by hand;
//! - is registered here with its path and its gameplay digest (`hst_data::digest`: shot tables, character data and
//!   CSVs, the program's tuning tables), for the Rules setting (which release's game a match plays, M175) and online
//!   (what a lobby's host can stream to a peer whose discs lack it, M171). The European release has no digest yet: the
//!   remaster reads none of its program's tables.
//!
//! Phones: the main disc's folder is the app's own (Android's external files folder, iOS's Documents: the Files app),
//! so a disc put there or in `voices/` counts; their pickers take one file at a time, which Add Voices reads in for its
//! voices only. The browser has no folder to look through: only its main disc counts there.

use hst_data::discs::{self, Edition};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// One release this player has.
#[derive(Clone, Debug, PartialEq)]
pub struct Have {
    pub edition: Edition,
    /// Its disc image or pack.
    pub path: PathBuf,
    /// The main disc (the one the game runs from), else a disc beside it.
    pub main: bool,
    /// Its gameplay digest (`None`: not worked out yet, or a release whose gameplay data the remaster doesn't read).
    pub digest: Option<u64>,
}

/// The releases registered so far: the main disc's first, then the others in the order found.
static HAVE: Mutex<Vec<Have>> = Mutex::new(Vec::new());
/// Bumped whenever [`HAVE`] changes (a menu showing the list redraws it).
static CHANGES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Every release this player has, the main disc's first.
pub fn have() -> Vec<Have> {
    HAVE.lock().unwrap().clone()
}

/// How many times [`have`] has changed (a new release found, a digest worked out).
pub fn changes() -> u32 {
    CHANGES.load(std::sync::atomic::Ordering::Relaxed)
}

/// Release `edition`'s entry, if this player has it (the main disc's when it is that release).
pub fn get(edition: Edition) -> Option<Have> {
    HAVE.lock().unwrap().iter().find(|h| h.edition == edition).cloned()
}

/// Whether this player has release `edition` (main disc or one beside it).
pub fn has(edition: Edition) -> bool {
    get(edition).is_some()
}

/// The disc to read release `edition`'s gameplay data from.
pub fn disc_of(edition: Edition) -> Option<PathBuf> {
    get(edition).map(|h| h.path)
}

/// `h` into the list: in place of the same release's entry, else after the rest (the main disc's always first).
fn register(h: Have) {
    let mut list = HAVE.lock().unwrap();
    match list.iter().position(|o| o.edition == h.edition) {
        Some(i) => list[i] = h,
        None if h.main => list.insert(0, h),
        None => list.push(h),
    }
    CHANGES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// The game's start, once the main disc `main` is known: it and the releases beside it (its folder, `voices`)
/// registered, the others' voices read into `voices`. On a worker thread: digests take a few hundred milliseconds a
/// disc, a first voice import some seconds. The browser looks through no folder.
pub fn start(main: &str, voices: PathBuf) {
    let main = PathBuf::from(main);
    *HAVE.lock().unwrap() = Vec::new();
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || find(&main, &voices));
    #[cfg(target_arch = "wasm32")]
    {
        let _ = voices;
        let edition = hst_data::iso::Iso::open(&main).ok().and_then(|mut iso| Edition::of(&mut iso));
        if let Some(edition) = edition {
            register(Have { edition, path: main, main: true, digest: None });
        }
    }
}

/// [`start`]'s work: registers `main` and the discs beside it, then their digests and voices.
pub fn find(main: &Path, voices: &Path) {
    let edition = hst_data::iso::Iso::open(main).ok().and_then(|mut iso| Edition::of(&mut iso));
    if let Some(edition) = edition {
        register(Have { edition, path: main.to_path_buf(), main: true, digest: None });
    }
    let dirs: Vec<PathBuf> = main.parent().map(Path::to_path_buf).into_iter().chain([voices.to_path_buf()]).collect();
    // the main disc's release counts once: a second copy of it beside it is passed over
    let others: Vec<discs::Disc> = discs::beside(main, &dirs).into_iter().filter(|d| Some(d.edition) != edition).collect();
    for d in &others {
        eprintln!("hst: {} beside the main disc: {}", d.edition.name(), d.path.display());
        register(Have { edition: d.edition, path: d.path.clone(), main: false, digest: None });
    }
    // digests next (quick), the main disc's the same as the lobby's (`handshake::sim`, already under way)
    if let Some(edition) = edition {
        register(Have { edition, path: main.to_path_buf(), main: true, digest: crate::handshake::sim() });
    }
    for d in &others {
        if d.edition.release().is_none() {
            continue;
        }
        let digest = hst_data::iso::Iso::open(&d.path).map_err(|e| e.to_string()).and_then(|mut iso| hst_data::digest::of(&mut iso));
        match digest {
            Ok(digest) => register(Have { edition: d.edition, path: d.path.clone(), main: false, digest: Some(digest) }),
            Err(e) => eprintln!("hst: no gameplay digest for {}: {e}", d.path.display()),
        }
    }
    // then the voices (a first import of a disc takes some seconds)
    for d in &others {
        crate::voices::import_disc(&d.path, voices);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hst_data::fixture;

    /// The Japanese and European discs in `fixtures/discs/` (skipped without them) beside a US disc: both registered
    /// after the main one, the Japanese with its digest (unlike the US one's: their tables differ) and the European
    /// without; their voices read into packs in `voices`, and a second
    /// start reads nothing again. Some seconds (two voice imports).
    #[test]
    fn registers_the_discs_beside_and_their_voices() {
        if !fixture::has_iso() {
            return fixture::skip("the US disc");
        }
        let dir = fixture::path("fixtures/discs");
        if !dir.join("SCPS-15113.iso").exists() || !dir.join("SCES-54535.iso").exists() {
            return fixture::skip("the Japanese and European discs (fixtures/discs/)");
        }
        let tmp = std::env::temp_dir().join(format!("hst-releases-{}", std::process::id()));
        struct Gone(PathBuf);
        impl Drop for Gone {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _gone = Gone(tmp.clone());
        // links, so nothing is copied: the main disc and the others under names of their own beside it
        let (home, voices) = (tmp.join("home"), tmp.join("voices"));
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&voices).unwrap();
        let main = home.join("disc.iso");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(fixture::path(fixture::ISO), &main).unwrap();
            for n in ["SCPS-15113.iso", "SCES-54535.iso"] {
                std::os::unix::fs::symlink(dir.join(n), home.join(format!("other {n}"))).unwrap();
            }
        }
        #[cfg(not(unix))]
        return;
        find(&main, &voices);
        let list = have();
        assert_eq!(list.iter().map(|h| (h.edition, h.main)).collect::<Vec<_>>(), [(Edition::Us, true), (Edition::Europe, false), (Edition::Japan, false)]);
        assert_eq!(disc_of(Edition::Japan), Some(home.join("other SCPS-15113.iso")));
        let jp = get(Edition::Japan).unwrap().digest.expect("the Japanese disc's digest");
        assert!(get(Edition::Europe).unwrap().digest.is_none(), "no European digest yet");
        assert!(voices.join("SCPS15113.pack").exists() && voices.join("SCES54535.pack").exists());
        let us = hst_data::digest::of(&mut fixture::iso().unwrap()).unwrap();
        assert_ne!(jp, us, "the Japanese release plays its own tables");
        let written = std::fs::metadata(voices.join("SCPS15113.pack")).unwrap().modified().unwrap();
        assert!(!crate::voices::import_disc(&home.join("other SCPS-15113.iso"), &voices), "read once");
        assert_eq!(std::fs::metadata(voices.join("SCPS15113.pack")).unwrap().modified().unwrap(), written);
    }
}
