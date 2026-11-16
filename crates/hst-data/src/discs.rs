//! M166: the other Hot Shots Tennis releases a player keeps beside the main disc. A folder is looked through for disc
//! images and packs under any file name; each is told apart by the disc ID its `SYSTEM.CNF` boots and checked to
//! really be that release (its main program and game overlay on the disc, the program's tables where the remaster
//! reads them), so a renamed or damaged file, another game or a different version is passed over.
//!
//! Nothing here keeps a disc open or reads it whole: a found disc is its path and release, read again by whoever needs
//! its data (its voices, its gameplay digest, its rules).

use crate::exe::{Game, Release};
use crate::iso::Iso;
use std::path::{Path, PathBuf};

/// A Hot Shots Tennis release, the European one included (which the match can't play from alone, `Release::of`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Edition {
    /// Hot Shots Tennis, USA (SCUS-97610).
    Us,
    /// Minna no Tennis, Japan (SCPS-15113).
    Japan,
    /// Everybody's Tennis, Europe (SCES-54535).
    Europe,
}

impl Edition {
    pub const ALL: [Edition; 3] = [Edition::Us, Edition::Japan, Edition::Europe];

    /// Its main program's name, as `SYSTEM.CNF`'s boot line names it (`SCUS_976.10`).
    pub fn boot(self) -> &'static str {
        match self {
            Edition::Us => "SCUS_976.10",
            Edition::Japan => "SCPS_151.13",
            Edition::Europe => "SCES_545.35",
        }
    }

    /// Its serial and region, for the player (`"SCPS-15113 Japan"`).
    pub fn name(self) -> &'static str {
        match self {
            Edition::Us => "SCUS-97610 USA",
            Edition::Japan => "SCPS-15113 Japan",
            Edition::Europe => "SCES-54535 Europe",
        }
    }

    /// The release the match plays it as (`None`: the European one, which no match plays from yet).
    pub fn release(self) -> Option<Release> {
        match self {
            Edition::Us => Some(Release::Us),
            Edition::Japan => Some(Release::Japan),
            Edition::Europe => None,
        }
    }

    /// The release whose `SYSTEM.CNF` this is, by its boot line (`None`: another disc).
    pub fn of_cnf(cnf: &[u8]) -> Option<Edition> {
        let text = String::from_utf8_lossy(cnf);
        let line = text.lines().find(|l| l.trim_start().starts_with("BOOT2"))?;
        let id = line.rsplit(['\\', ':']).next()?.trim().split(';').next()?.to_ascii_uppercase();
        Edition::ALL.into_iter().find(|e| e.boot() == id)
    }

    /// The release disc (image or pack) `iso` is, checked: its boot line names one, its main program and game overlay
    /// are on it, and for the US and Japanese releases the overlay is that release's 1.00 one (the tables the
    /// remaster reads are where it reads them). The European overlay is only checked to be an overlay: the remaster
    /// reads none of its tables yet.
    pub fn of(iso: &mut Iso) -> Option<Edition> {
        // the disc's own files, not another release's read over them (`crossplay::lend`, a lobby host's)
        let cnf = iso.read_disc("SYSTEM.CNF").ok()?;
        let edition = Edition::of_cnf(&cnf)?;
        if !iso.files.contains_key(edition.boot()) {
            return None;
        }
        let bin = iso.read_disc("ZZBIN/GAME.BIN").ok()?;
        let ok = match edition.release() {
            Some(_) => Game::new(&cnf, &bin).is_ok(),
            None => bin.get(..4) == Some(b"MWo3"),
        };
        ok.then_some(edition)
    }
}

/// A release's disc found beside the main one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Disc {
    pub path: PathBuf,
    pub edition: Edition,
}

/// Smallest file looked into: a pack is ≈ 170 MB and an image over 1 GB, so anything under this is something else.
const SMALLEST: u64 = 32 << 20;

/// How sure a copy of a release is the retail disc, higher first: a pack whose hash is a known retail copy
/// (`iso::KNOWN_PACKS`), then an image by its volume's creation date (a prototype or preview build of a release is
/// mastered before the disc that shipped), then anything else.
fn rank(path: &Path) -> (bool, [u8; 16]) {
    let Ok(iso) = Iso::open_bare(path) else { return (false, [0; 16]) };
    if let Some(hash) = iso.pack_hash {
        return (crate::iso::release_name(hash).is_some(), [0; 16]);
    }
    (false, volume_date(path).unwrap_or([0; 16]))
}

/// An ISO 9660 image's volume creation date, as its primary volume descriptor writes it (`YYYYMMDDhhmmsscc`, digits
/// that sort in time order); `None` if it holds none.
fn volume_date(path: &Path) -> Option<[u8; 16]> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let mut pvd = [0; 830];
    file.seek(SeekFrom::Start(16 * 2048)).ok()?;
    file.read_exact(&mut pvd).ok()?;
    let date: [u8; 16] = pvd[813..829].try_into().ok()?;
    (&pvd[1..6] == b"CD001" && date.iter().all(u8::is_ascii_digit)).then_some(date)
}

/// The release discs in `dirs` (not their subfolders), any file name, the main disc `main` left out, in file-name
/// order per folder. Each release counts once: of two copies of one, the one more surely the retail disc ([`rank`]:
/// a known pack, else the later-mastered image, so a prototype gives way to the release), else the first found.
pub fn beside(main: &Path, dirs: &[PathBuf]) -> Vec<Disc> {
    let main = main.canonicalize().unwrap_or_else(|_| main.to_path_buf());
    let mut found: Vec<Disc> = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            // followed through a link: a disc linked in from elsewhere counts
            .filter(|p| std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.len() >= SMALLEST))
            .filter(|p| !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("part")))
            .collect();
        files.sort();
        for path in files {
            if path.canonicalize().is_ok_and(|p| p == main) {
                continue;
            }
            let Some(edition) = Iso::open(&path).ok().and_then(|mut iso| Edition::of(&mut iso)) else { continue };
            match found.iter_mut().find(|d| d.edition == edition) {
                None => found.push(Disc { path, edition }),
                Some(kept) if rank(&path) > rank(&kept.path) => kept.path = path,
                Some(_) => {}
            }
        }
    }
    found
}

/// Of the disc images `paths`, the one to play from when nothing says which: the US disc if one is there (the release
/// the remaster is proven against), else the first.
pub fn main_first(mut paths: Vec<PathBuf>) -> Option<PathBuf> {
    paths.sort();
    let us = paths.iter().position(|p| Iso::open(p).ok().and_then(|mut iso| Edition::of(&mut iso)) == Some(Edition::Us));
    us.map(|i| paths.swap_remove(i)).or_else(|| paths.into_iter().next())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;

    #[test]
    fn boot_lines() {
        assert_eq!(Edition::of_cnf(b"BOOT2 = cdrom0:\\SCUS_976.10;1\r\nVER = 1.00\r\n"), Some(Edition::Us));
        assert_eq!(Edition::of_cnf(b"BOOT2 = cdrom0:\\scps_151.13;1\n"), Some(Edition::Japan));
        assert_eq!(Edition::of_cnf(b"BOOT2 = cdrom0:\\SCES_545.35;1\n"), Some(Edition::Europe));
        assert_eq!(Edition::of_cnf(b"BOOT2 = cdrom0:\\SLUS_123.45;1\n"), None);
        assert_eq!(Edition::of_cnf(b"VMODE = NTSC\n"), None);
    }

    /// A link to `target` at `at` (a copy where links aren't made).
    fn link(target: &Path, at: &Path) {
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, at).unwrap();
        #[cfg(not(unix))]
        std::fs::copy(target, at).unwrap();
    }

    /// The Japanese and European discs in `fixtures/discs/` (skipped without them), linked into a folder of the
    /// test's own (whatever else lands in `fixtures/discs/`), are found beside the US one under their own names, the
    /// US disc itself and the folders there passed over; a renamed copy is found as well, a small file or a second
    /// copy of a release is not. A prototype of the European release beside it (`SCES-54535-proto-0119.iso`, if
    /// there) gives way to the retail disc though its name sorts first. The US disc is the one to play from.
    #[test]
    fn finds_the_other_releases() {
        let Ok(mut us) = fixture::iso() else { return };
        assert_eq!(Edition::of(&mut us), Some(Edition::Us));
        let discs = fixture::path("fixtures/discs");
        let (jp, eu) = (discs.join("SCPS-15113.iso"), discs.join("SCES-54535.iso"));
        if !jp.exists() || !eu.exists() {
            return fixture::skip("the Japanese and European discs (fixtures/discs/)");
        }
        let tmp = std::env::temp_dir().join(format!("hst-discs-{}", std::process::id()));
        struct Gone(PathBuf);
        impl Drop for Gone {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _gone = Gone(tmp.clone());
        let (dir, other) = (tmp.join("discs"), tmp.join("other"));
        std::fs::create_dir_all(dir.join("SCES-54535")).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        let (jp_here, eu_here) = (dir.join("SCPS-15113.iso"), dir.join("SCES-54535.iso"));
        link(&jp, &jp_here);
        link(&eu, &eu_here);

        let found = beside(&jp_here, std::slice::from_ref(&dir));
        assert_eq!(found, [Disc { path: eu_here.clone(), edition: Edition::Europe }], "the main disc itself left out");
        let found = beside(&fixture::path(fixture::ISO), std::slice::from_ref(&dir));
        assert_eq!(found.iter().map(|d| d.edition).collect::<Vec<_>>(), [Edition::Europe, Edition::Japan]);

        // any name: a renamed copy (a link, no space taken) is found; a second copy of a release counts once
        let renamed = other.join("minna");
        link(&jp, &renamed);
        std::fs::write(other.join("notes.iso"), b"not a disc").unwrap();
        let found = beside(&eu_here, &[other.clone(), dir.clone()]);
        assert_eq!(found, [Disc { path: renamed.clone(), edition: Edition::Japan }]);
        assert_eq!(main_first(vec![eu_here.clone(), jp_here.clone()]), Some(eu_here.clone()), "no US disc: the first by name");

        // the retail disc over a prototype of the same release, found first or not
        let proto = discs.join("SCES-54535-proto-0119.iso");
        if !proto.exists() {
            return;
        }
        link(&proto, &dir.join("SCES-54535-proto-0119.iso"));
        link(&proto, &other.join("0-proto"));
        let found = beside(&jp_here, &[other.clone(), dir.clone()]);
        assert_eq!(found, [Disc { path: eu_here.clone(), edition: Edition::Europe }]);
    }
}
