//! Crossplay (M171): a lobby plays its host's disc. A peer on another release (or an import that plays differently)
//! gets from the host what its own disc plays differently, and plays the match on the host's data, in memory only.
//!
//! - The peer says what its disc plays, piece by piece ([`digest::Manifest`]: the program tables' hash, each source
//!   archive's).
//! - The host answers with a [`Patch`]: for each source archive whose hash differs, the host's counted entries of it
//!   (the gameplay ones: tables, layout, models, motions); and when the program tables differ, the host's
//!   `SYSTEM.CNF` and programs (`GAME.BIN`, whose tables the match reads, and `MENU.BIN`, which is read with the same
//!   `SYSTEM.CNF`).
//! - The peer installs it as an [`Overlay`] on its disc's path ([`install`]): from then on `Iso::open` of that path
//!   reads a patched archive as the peer's own with its counted entries swapped for the host's, so its textures,
//!   sounds and voices stay its own. The digest of its disc read through the overlay must be the host's
//!   ([`verify`]), or the overlay is taken off again. [`remove`] ends it (leaving the lobby). Nothing is written.

use crate::digest::{self, Manifest};
use crate::iso::Iso;
use crate::xb::Archive;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The program files a patch sends whole when the program tables differ.
const PROGRAM: [&str; 3] = ["SYSTEM.CNF", "ZZBIN/GAME.BIN", "ZZBIN/MENU.BIN"];

/// What a host sends a peer whose disc plays differently: the files its disc must read as the host's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Patch {
    /// Files read whole as the host's (US names); `None`: the host has none.
    pub files: Vec<(String, Option<Vec<u8>>)>,
    /// Source archives (US names) whose counted entries are the host's: (name in the host's archive, bytes as read);
    /// `None`: the host has no such file.
    pub archives: Vec<(String, Option<Entries>)>,
}

/// An archive's entries: (name in it, bytes).
pub type Entries = Vec<(String, Vec<u8>)>;

impl Patch {
    /// Nothing to change: the two discs play alike.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.archives.is_empty()
    }

    /// Bytes of data it carries (names and framing left out).
    pub fn size(&self) -> usize {
        let f: usize = self.files.iter().filter_map(|(_, b)| b.as_ref()).map(Vec::len).sum();
        let a: usize = self.archives.iter().filter_map(|(_, e)| e.as_ref()).flatten().map(|(_, b)| b.len()).sum();
        f + a
    }

    /// As bytes for the wire: magic, then files and archives, each length-prefixed (u32 LE).
    pub fn encode(&self) -> Vec<u8> {
        let mut o = MAGIC.to_vec();
        let put = |o: &mut Vec<u8>, b: &[u8]| {
            o.extend((b.len() as u32).to_le_bytes());
            o.extend_from_slice(b);
        };
        let opt = |o: &mut Vec<u8>, some: bool| o.push(some as u8);
        o.extend((self.files.len() as u32).to_le_bytes());
        for (p, b) in &self.files {
            put(&mut o, p.as_bytes());
            opt(&mut o, b.is_some());
            if let Some(b) = b {
                put(&mut o, b);
            }
        }
        o.extend((self.archives.len() as u32).to_le_bytes());
        for (p, e) in &self.archives {
            put(&mut o, p.as_bytes());
            opt(&mut o, e.is_some());
            if let Some(e) = e {
                o.extend((e.len() as u32).to_le_bytes());
                for (n, b) in e {
                    put(&mut o, n.as_bytes());
                    put(&mut o, b);
                }
            }
        }
        o
    }

    /// [`Patch::encode`]'s bytes back; `None` if they aren't one.
    pub fn decode(b: &[u8]) -> Option<Patch> {
        let mut r = Reader { b: b.strip_prefix(MAGIC)? };
        let mut p = Patch::default();
        for _ in 0..r.u32()? {
            let path = r.text()?;
            let b = if r.flag()? { Some(r.bytes()?.to_vec()) } else { None };
            p.files.push((path, b));
        }
        for _ in 0..r.u32()? {
            let path = r.text()?;
            let e = if r.flag()? {
                let n = r.u32()?;
                let mut v = Vec::new();
                for _ in 0..n {
                    v.push((r.text()?, r.bytes()?.to_vec()));
                }
                Some(v)
            } else {
                None
            };
            p.archives.push((path, e));
        }
        r.b.is_empty().then_some(p)
    }
}

/// What an encoded [`Patch`] starts with (a mod's pack, a zip, never does).
pub const MAGIC: &[u8; 8] = b"HSTXPLY1";

struct Reader<'a> {
    b: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        (self.b.len() >= n).then(|| {
            let (a, rest) = self.b.split_at(n);
            self.b = rest;
            a
        })
    }
    fn u32(&mut self) -> Option<u32> {
        self.take(4).map(|b| u32::from_le_bytes(b.try_into().unwrap()))
    }
    fn flag(&mut self) -> Option<bool> {
        self.take(1).map(|b| b[0] != 0)
    }
    fn bytes(&mut self) -> Option<&'a [u8]> {
        let n = self.u32()? as usize;
        self.take(n)
    }
    fn text(&mut self) -> Option<String> {
        String::from_utf8(self.bytes()?.to_vec()).ok()
    }
}

/// The host's side: what a peer whose disc's manifest is `peer` lacks of the host's disc (read through `read`, by US
/// name). Empty when the two play alike.
pub fn patch_for(mut read: impl FnMut(&str) -> Option<Vec<u8>>, peer: &Manifest) -> Result<Patch, String> {
    let (_, here) = digest::survey(&mut read)?;
    let mut p = Patch::default();
    if here.tables != peer.tables {
        p.files = PROGRAM.iter().map(|f| (f.to_string(), read(f))).collect();
    }
    for (i, path) in digest::source_paths().into_iter().enumerate() {
        if peer.sources.get(i) == here.sources.get(i) {
            continue;
        }
        let entries = match read(&path) {
            None => None,
            // not an archive: counted whole, so sent whole
            Some(data) => match digest::host_entries(&path, data.clone()) {
                Some(e) => Some(e),
                None => {
                    p.files.push((path, Some(data)));
                    continue;
                }
            },
        };
        p.archives.push((path, entries));
    }
    Ok(p)
}

/// A patch installed on a disc: what its reads change to.
pub struct Overlay {
    files: BTreeMap<String, Option<Arc<[u8]>>>,
    archives: BTreeMap<String, Option<Entries>>,
}

impl Overlay {
    pub fn new(p: Patch) -> Overlay {
        Overlay {
            files: p.files.into_iter().map(|(k, b)| (k, b.map(Arc::from))).collect(),
            archives: p.archives.into_iter().collect(),
        }
    }

    /// Whether it changes file `key` (upper case, `/`, US name).
    pub(crate) fn covers(&self, key: &str) -> bool {
        self.files.contains_key(key) || self.archives.contains_key(key)
    }

    /// File `key` as the host's, from the disc's own bytes `own` (`None`: not on this disc). `None`: the host has none.
    pub(crate) fn read(&self, key: &str, own: Option<Vec<u8>>) -> Option<Vec<u8>> {
        if let Some(f) = self.files.get(key) {
            return f.as_ref().map(|b| b.to_vec());
        }
        let host = self.archives.get(key)?.as_ref()?;
        let only = digest::source(key)?;
        // the disc's own entries that don't count (textures, sounds, voices), then the host's counted ones
        let mut out: Vec<(String, Vec<u8>)> = Vec::new();
        if let Some(own) = &own
            && let Ok(arc) = Archive::parse(own)
        {
            for e in &arc.entries {
                if !only(digest::ext(&digest::counted_name(&e.name))) {
                    out.push((e.name.clone(), arc.decode(e).ok()?));
                }
            }
        }
        out.extend(host.iter().cloned());
        Some(crate::xb::build(&out))
    }
}

static OVERLAYS: Mutex<BTreeMap<PathBuf, Arc<Overlay>>> = Mutex::new(BTreeMap::new());

/// From now on `Iso::open(disc)` reads `patch` over the disc (an `Iso` already open keeps reading the disc alone).
pub fn install(disc: impl AsRef<Path>, patch: Patch) {
    OVERLAYS.lock().unwrap().insert(disc.as_ref().to_path_buf(), Arc::new(Overlay::new(patch)));
}

/// Ends [`install`]: `disc` reads as itself again. Whether there was one.
pub fn remove(disc: impl AsRef<Path>) -> bool {
    OVERLAYS.lock().unwrap().remove(disc.as_ref()).is_some()
}

/// Whether `disc` has a patch installed.
pub fn installed(disc: impl AsRef<Path>) -> bool {
    OVERLAYS.lock().unwrap().contains_key(disc.as_ref())
}

/// The overlay `Iso::open(disc)` reads through.
pub(crate) fn overlay(disc: &Path) -> Option<Arc<Overlay>> {
    OVERLAYS.lock().unwrap().get(disc).cloned()
}

/// Installs `patch` on `disc` and checks that the disc now digests as `host` (the host's gameplay digest): `Ok` and
/// installed, or the reason, not installed.
pub fn verify(disc: impl AsRef<Path>, patch: Patch, host: u64) -> Result<(), String> {
    let disc = disc.as_ref();
    install(disc, patch);
    let got = Iso::open(disc).map_err(|e| e.to_string()).and_then(|mut iso| digest::of(&mut iso));
    match got {
        Ok(d) if d == host => Ok(()),
        other => {
            remove(disc);
            Err(match other {
                Ok(d) => format!("the host's data didn't make your disc play as theirs (digest {d:016x}, host {host:016x})"),
                Err(e) => e,
            })
        }
    }
}

/// M174: another release's data a player's own discs lend each other: the European disc reads the US disc's
/// gameplay data when the player has both, and plays the US game exactly (its menus, text and voices its own).
static LENT: Mutex<BTreeMap<PathBuf, Arc<Overlay>>> = Mutex::new(BTreeMap::new());

/// Gameplay data a match doesn't read today, lent whole all the same so nothing reads it timed for the other release:
/// the second copy of the shot tables (`SAITOU/TRAJ`, re-solved for 50 Hz on the European disc).
fn also_lent(path: &str) -> bool {
    path.starts_with("SAITOU/TRAJ/")
}

/// From now on `Iso::open(disc)` (and `Iso::open_own`) reads the gameplay data of the disc at `from` under its own
/// files: the program and its tables, and every source archive's counted entries that differ (as a lobby host's data,
/// [`patch_for`]), plus [`also_lent`]'s files. Checked: the disc must then digest as `from` does, or nothing is lent.
/// The digest both now play, or why not.
pub fn lend(disc: impl AsRef<Path>, from: impl AsRef<Path>) -> Result<u64, String> {
    let disc = disc.as_ref();
    let err = |e: std::io::Error| e.to_string();
    let mut own = Iso::open_bare(disc).map_err(err)?;
    let mut lender = Iso::open_bare(from.as_ref()).map_err(err)?;
    let (_, wants) = digest::survey(|f| own.read_unlogged(f).ok())?;
    let (target, _) = digest::survey(|f| lender.read_unlogged(f).ok())?;
    let mut patch = patch_for(|f| lender.read_unlogged(f).ok(), &wants)?;
    // the lender's program always: its `SYSTEM.CNF` says which release's rules (and timing) the match plays
    for f in PROGRAM {
        if !patch.files.iter().any(|(k, _)| k == f) {
            patch.files.push((f.to_string(), lender.read_unlogged(f).ok()));
        }
    }
    let extra: Vec<String> = lender.files.keys().filter(|k| also_lent(k)).cloned().collect();
    for k in extra {
        let data = lender.read_unlogged(&k).ok();
        patch.files.push((k, data));
    }
    LENT.lock().unwrap().insert(disc.to_path_buf(), Arc::new(Overlay::new(patch)));
    let got = Iso::open_own(disc).map_err(err).and_then(|mut iso| digest::of(&mut iso));
    match got {
        Ok(d) if d == target => Ok(d),
        other => {
            unlend(disc);
            Err(match other {
                Ok(d) => format!("the lent data didn't make the disc play as the lender's (digest {d:016x}, lender {target:016x})"),
                Err(e) => e,
            })
        }
    }
}

/// Ends [`lend`]: `disc` reads as itself again. Whether anything was lent to it.
pub fn unlend(disc: impl AsRef<Path>) -> bool {
    LENT.lock().unwrap().remove(disc.as_ref()).is_some()
}

/// Whether another release's data is lent to `disc` ([`lend`]).
pub fn lent(disc: impl AsRef<Path>) -> bool {
    LENT.lock().unwrap().contains_key(disc.as_ref())
}

/// Whether any disc has another release's data lent to it (the main menu's header says so).
pub fn any_lent() -> bool {
    !LENT.lock().unwrap().is_empty()
}

/// The lent data `Iso::open(disc)` reads under the disc's own.
pub(crate) fn lent_overlay(disc: &Path) -> Option<Arc<Overlay>> {
    LENT.lock().unwrap().get(disc).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;

    #[test]
    fn a_patch_round_trips_and_rebuilt_archives_read_back() {
        let p = Patch {
            files: vec![("SYSTEM.CNF".into(), Some(b"BOOT2".to_vec())), ("X".into(), None)],
            archives: vec![("TRAJ/TRAJ00A.XB".into(), Some(vec![("a\\b.dat".into(), vec![1, 2, 3])])), ("Y".into(), None)],
        };
        assert_eq!(Patch::decode(&p.encode()), Some(p.clone()));
        assert_eq!(Patch::decode(&p.encode()[..20]), None);
        let built = crate::xb::build(&[("a\\b.dat".into(), vec![1, 2, 3]), ("c.tm2".into(), vec![9; 5])]);
        let arc = Archive::parse(&built).unwrap();
        let got: Vec<_> = arc.entries.iter().map(|e| (e.name.clone(), arc.decode(e).unwrap())).collect();
        assert_eq!(got, [("a\\b.dat".to_string(), vec![1, 2, 3]), ("c.tm2".to_string(), vec![9; 5])]);
    }

    /// Two releases' discs: the peer's, patched with what the host sends, digests as the host's, both ways round,
    /// and the patch is small. Skips without `fixtures/discs/SCPS-15113.iso` (not the user's disc).
    #[test]
    fn a_japanese_disc_patched_plays_the_us_disc_and_back() {
        let jp = fixture::path("fixtures/discs/SCPS-15113.iso");
        if fixture::iso().is_err() {
            return;
        }
        if !jp.exists() {
            return eprintln!("skipped: no {}", jp.display());
        }
        let us = std::path::PathBuf::from(fixture::ISO_PATH);
        for (host, peer) in [(&us, &jp), (&jp, &us)] {
            let mut h = Iso::open(host).unwrap();
            let mut p = Iso::open(peer).unwrap();
            let (hd, _) = digest::survey(|f| h.read_unlogged(f).ok()).unwrap();
            let (pd, pm) = digest::survey(|f| p.read_unlogged(f).ok()).unwrap();
            assert_ne!(hd, pd);
            let patch = patch_for(|f| h.read_unlogged(f).ok(), &pm).unwrap();
            let patch = Patch::decode(&patch.encode()).unwrap();
            eprintln!(
                "{} -> {}: {} files, {} archives, {} KB",
                host.display(),
                peer.display(),
                patch.files.len(),
                patch.archives.len(),
                patch.size() / 1024
            );
            for (a, _) in &patch.archives {
                eprintln!("  {a}");
            }
            // the peer plays from a link of its own, so the other tests reading the disc read it as itself
            let dir = std::env::temp_dir().join(format!("hst-crossplay-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let link = dir.join(peer.file_name().unwrap());
            let _ = std::fs::remove_file(&link);
            std::os::unix::fs::symlink(std::fs::canonicalize(peer).unwrap(), &link).unwrap();
            verify(&link, patch, hd).unwrap();
            assert!(installed(&link));
            // its own sounds still read as its own: a voice set is the peer's, not the host's
            let mut patched = Iso::open(&link).unwrap();
            let voice = "SND/VOICE/PC/PC03VCE0.XB0";
            assert_eq!(patched.read_unlogged(voice).ok(), p.read_unlogged(voice).ok());
            assert!(remove(&link));
            let _ = std::fs::remove_file(&link);
        }
    }
}
