//! Disc import: the first start copies what the remaster reads out of the user's ISO into a pack in the user data dir
//! (`hst_data::iso::Iso::write_pack`); from then on the game runs from the pack and the ISO can go. settings.txt,
//! mods/ and cache/ live beside the pack too (`crate::store`, M143), whatever disc a run starts from.
//!
//! `hst [flags]` runs from the pack (importing first if there is none: a file dialog asks for the ISO);
//! `hst --import <iso> [flags]` (re)imports; `hst <iso or pack> [flags]` runs straight from that file, as before.

use hst_data::discs::Edition;
use hst_data::iso::Iso;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The game's files (`crate::store`, M143): on Windows and Linux beside the game ([`beside_game`], B668), else (and
/// where that folder won't take files) [`app_data_dir`].
pub fn data_dir() -> PathBuf {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| beside_game().unwrap_or_else(app_data_dir)).clone()
}

/// B668: the game's own folder (an AppImage's: the folder the `.AppImage` is in, its mount is read-only), on Windows and
/// Linux, if the game can write there. Not: Program Files or `/usr` (not writable), a Flatpak, a Steam library (Steam
/// owns those files and an uninstall takes the folder), a release Update put in [`app_data_dir`]'s `update/` (its data
/// stays where the game that fetched it keeps it), or a build in a cargo checkout (`target/`: a developer's runs share
/// one folder, `HST_DATA` picks another).
pub fn beside_game() -> Option<PathBuf> {
    if !cfg!(any(windows, target_os = "linux")) || std::env::var_os("FLATPAK_ID").is_some() {
        return None;
    }
    let exe = std::env::var_os("APPIMAGE").filter(|v| !v.is_empty()).map(PathBuf::from).or_else(|| std::env::current_exe().ok())?;
    let dir = exe.canonicalize().ok()?.parent()?.to_path_buf();
    let app = app_data_dir();
    let ours = |d: &Path| !d.components().any(|c| c.as_os_str().eq_ignore_ascii_case("steamapps")) && !d.ancestors().any(|a| a.join("Cargo.lock").is_file()) && !d.starts_with(app.canonicalize().unwrap_or(app.clone()));
    (ours(&dir) && writable(&dir)).then_some(dir)
}

/// Whether `dir` takes a new file.
fn writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".hst-probe-{}", std::process::id()));
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// `$XDG_DATA_HOME/hst` (else `~/.local/share/hst`); `%APPDATA%\hst` on Windows; `~/Library/Application Support/hst`
/// on macOS; the app's Documents on iOS (the folder the Files app shows, so the ISO, settings.txt and mods/ go in and
/// out there). The game's saved files lived here on every platform before B668, and still do where [`beside_game`]
/// has no folder.
pub fn app_data_dir() -> PathBuf {
    let env = |k| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if cfg!(target_os = "ios") {
        return env("HOME").unwrap_or_default().join("Documents");
    }
    if cfg!(windows) {
        return env("APPDATA").unwrap_or_default().join("hst");
    }
    if cfg!(target_os = "macos") {
        let home = env("HOME").unwrap_or_default();
        let dir = home.join("Library/Application Support/hst");
        // M143: earlier builds used the Linux folder on macOS too; its pack and files move over once
        let old = env("XDG_DATA_HOME").unwrap_or_else(|| home.join(".local/share")).join("hst");
        if !dir.exists() && old.is_dir() {
            let _ = std::fs::create_dir_all(dir.parent().unwrap());
            let _ = std::fs::rename(&old, &dir);
        }
        return dir;
    }
    env("XDG_DATA_HOME").unwrap_or_else(|| env("HOME").unwrap_or_default().join(".local/share")).join("hst")
}

/// The imported disc: in [`data_dir`], else one imported into [`app_data_dir`] before B668 (an import then moves it).
pub fn pack_path() -> PathBuf {
    let new = data_dir().join("disc.pack");
    let old = app_data_dir().join("disc.pack");
    if !new.exists() && old.exists() { old } else { new }
}

/// iOS: no disc in the app's folder at launch; the app shows where it goes until one is there.
pub static WAITING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// iOS: the ISO in the app's folder (Files → On My iPhone → OpenTennis); with other releases' discs there too (M166),
/// the US one.
pub fn first_iso() -> Option<PathBuf> {
    hst_data::discs::main_first(std::fs::read_dir(data_dir()).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("iso"))).collect())
}

/// Puts the disc to run from in front of `argv` (see the module doc).
pub fn resolve(argv: &mut Vec<String>) {
    // iOS (M91): no dialogs, and a launch that takes too long gets killed, so no import before the window: the pack if
    // there is one, else straight from the first ISO in the app's folder
    #[cfg(target_os = "ios")]
    if argv.is_empty() {
        // none: the pack's path, which isn't there yet; the app waits for one (`WAITING`)
        let disc = Some(pack_path()).filter(|p| p.exists()).or_else(first_iso).unwrap_or_else(|| {
            WAITING.store(true, std::sync::atomic::Ordering::Relaxed);
            pack_path()
        });
        argv.push(disc.to_string_lossy().into_owned());
        return;
    }
    let import = argv.first().is_some_and(|a| a == "--import");
    if import {
        argv.remove(0);
    }
    let iso = if import && argv.first().is_some_and(|a| !a.starts_with("--")) { Some(argv.remove(0)) } else { None };
    if !import && argv.first().is_some_and(|a| !a.starts_with("--")) {
        // B94: a path that won't open (a launcher's placeholder) used to panic in the startup systems with the window
        // already up; fall back to the pack / picker instead
        if let Ok(mut iso) = Iso::open(&argv[0]) {
            // M163a: a disc the game can read but not play (the Japanese release) says so before the window opens
            if let Err(e) = iso.playable() {
                fail(&format!("Can't play {}: {e}", argv[0]));
            }
            return;
        }
        eprintln!("Couldn't open the disc image {}: using the imported disc", argv.remove(0));
    }
    let mut pack = pack_path();
    if import || !pack.exists() {
        let picked = iso.is_none();
        let iso = iso.or_else(pick).unwrap_or_else(|| fail("No disc image picked. Run: hst --import <your Hot Shots Tennis (USA) ISO>"));
        let was = std::mem::replace(&mut pack, data_dir().join("disc.pack"));
        if let Err(e) = import_iso(Path::new(&iso), &pack) {
            fail(&format!("Couldn't import {iso}: {e}"));
        }
        // B668: a re-import into the new folder takes the old folder's pack's place
        if was != pack {
            let _ = std::fs::remove_file(was);
        }
        // the picker's window came and went before ours: on the Steam Deck's Gaming Mode (gamescope) the game's
        // window then stays black, so start over as a fresh process, which finds the pack
        if picked {
            restart();
        }
    }
    argv.insert(0, pack.to_string_lossy().into_owned());
}

fn import_iso(iso: &Path, pack: &Path) -> std::io::Result<()> {
    let hash = write_pack(iso, pack)?;
    // the settings beside the old ISO come along (mods/ stay where they are: they can be large)
    let (old, new) = (iso.with_file_name("settings.txt"), pack.with_file_name("settings.txt"));
    if old.exists() && !new.exists() {
        std::fs::copy(old, new)?;
    }
    let mb = |p: &Path| std::fs::metadata(p).map_or(0, |m| m.len() >> 20);
    tell(&format!(
        "Imported the disc into {} ({} MB of the {} MB image, hash {hash:016x}).\nThe game no longer needs the ISO: you can move or delete it.",
        pack.display(),
        mb(pack),
        mb(iso)
    ));
    Ok(())
}

/// `iso` into a new pack at `pack` (written beside it as a `.part` until complete), checked playable: the pack's hash.
fn write_pack(iso: &Path, pack: &Path) -> std::io::Result<u64> {
    std::fs::create_dir_all(pack.parent().unwrap())?;
    // the disc alone: no other release's data laid over it (Rules, M175; a European disc's borrowed US data, M174)
    let mut disc = Iso::open_own(iso)?;
    disc.playable()?;
    let mut last = 0;
    disc.write_pack(pack, |i, n| {
        if i * 10 / n != last {
            last = i * 10 / n;
            eprintln!("importing the disc: {}%", last * 10);
        }
    })
}

/// M180: whether Base Disc can switch on this build: the desktop ones, which restart themselves (the phones' and the
/// browser's disc stays the one they start with).
pub const CAN_SWITCH: bool = cfg!(not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")));

/// M180: the releases Settings → Gameplay → Base Disc offers (`releases::have`: the main disc's and the discs beside
/// it), US, Japanese, European; none with fewer than two, or where the game can't switch ([`CAN_SWITCH`]).
pub fn bases() -> Vec<Edition> {
    if !CAN_SWITCH {
        return Vec::new();
    }
    let have = crate::releases::have();
    let list: Vec<Edition> = Edition::ALL.into_iter().filter(|e| have.iter().any(|h| h.edition == *e)).collect();
    if list.len() < 2 { Vec::new() } else { list }
}

/// M180: the release the game runs from (the main disc's), once known.
pub fn base() -> Option<Edition> {
    crate::releases::have().into_iter().find(|h| h.main).map(|h| h.edition)
}

/// M180: where a Base Disc switch is.
#[derive(Clone, Debug, PartialEq)]
pub enum Switch {
    Idle,
    /// Copying the disc in, or restarting.
    Busy,
    /// It couldn't (why): the game carries on on the disc it has.
    Failed(String),
}

static SWITCH: Mutex<Switch> = Mutex::new(Switch::Idle);

/// M180: where the last [`switch_base`] is.
pub fn switching() -> Switch {
    SWITCH.lock().unwrap().clone()
}

/// M180: Base Disc picks release `to`: the game restarts on its disc, off the frame (an image is copied into a pack
/// first, some seconds). The imported pack (`disc.pack`) the game runs from takes the new disc's place and the one it
/// had stays beside it as `disc-<serial>.pack`, so it is still found (`releases`, Rules, Voice Language) and switching
/// back is a rename; a disc named on the command line is replaced on the restarted one's. Nothing for the main disc's
/// own release, a release not here, or while a switch is under way.
pub fn switch_base(to: Edition) {
    let (Some(main), Some(target)) = (crate::releases::have().into_iter().find(|h| h.main), crate::releases::get(to)) else { return };
    if main.edition == to || !CAN_SWITCH {
        return;
    }
    {
        let mut s = SWITCH.lock().unwrap();
        if *s == Switch::Busy {
            return;
        }
        *s = Switch::Busy;
    }
    std::thread::spawn(move || {
        let pack = pack_path();
        let started = match swap(&pack, &main.path, main.edition, &target.path) {
            Ok(disc) => {
                eprintln!("hst: base disc now {}: {}", to.name(), disc.display());
                let args = restart_args(std::env::args_os().skip(1).collect(), &main.path, &disc, &pack);
                restart_with(args)
            }
            Err(e) => e.to_string(),
        };
        eprintln!("hst: couldn't switch the base disc to {}: {started}", to.name());
        *SWITCH.lock().unwrap() = Switch::Failed(started);
    });
}

/// [`switch_base`]'s files: the game runs from `main` (release `edition`) and is to run from the disc at `to`; `pack`
/// is the imported pack's place. Running from the pack, `to` takes its place (a pack moved in, an image imported) and
/// the old one stays beside it under its serial; else nothing moves. The disc to restart on.
fn swap(pack: &Path, main: &Path, edition: Edition, to: &Path) -> std::io::Result<PathBuf> {
    let same = |a: &Path, b: &Path| a.canonicalize().ok().is_some_and(|a| b.canonicalize().is_ok_and(|b| a == b));
    if !same(main, pack) {
        return Ok(to.to_path_buf());
    }
    let dir = pack.parent().unwrap_or(Path::new("."));
    let serial = edition.name().split_whitespace().next().unwrap_or("disc");
    let park = (1..).map(|n| dir.join(if n == 1 { format!("disc-{serial}.pack") } else { format!("disc-{serial}-{n}.pack") })).find(|p| !p.exists()).unwrap();
    // an image is imported first, so a failed import leaves everything as it was
    let next = if to.extension().is_some_and(|x| x.eq_ignore_ascii_case("pack")) {
        to.to_path_buf()
    } else {
        let next = dir.join("disc-next.pack");
        write_pack(to, &next)?;
        next
    };
    std::fs::rename(pack, &park)?;
    if let Err(e) = std::fs::rename(&next, pack) {
        let _ = std::fs::rename(&park, pack);
        return Err(e);
    }
    Ok(pack.to_path_buf())
}

/// The arguments to restart with on `disc` after running from `main` with `args` (this run's, the program left out):
/// an import's `--import [iso]` dropped (it would import again); a disc named in them (`main`) becomes `disc`; with
/// none named, `disc` goes first unless it is the pack (which a run with no disc starts from).
fn restart_args(mut args: Vec<std::ffi::OsString>, main: &Path, disc: &Path, pack: &Path) -> Vec<std::ffi::OsString> {
    if args.first().is_some_and(|a| a == "--import") {
        args.remove(0);
        if args.first().is_some_and(|a| !a.to_string_lossy().starts_with("--")) {
            args.remove(0);
        }
    }
    let named = args.first().filter(|a| !a.to_string_lossy().starts_with("--")).is_some_and(|a| Path::new(a) == main || Path::new(a) == pack);
    if named {
        args[0] = disc.into();
    } else if disc != pack {
        args.insert(0, disc.into());
    }
    args
}

/// Runs this game again in this process's place with `args`; returns only when it couldn't (why).
fn restart_with(args: Vec<std::ffi::OsString>) -> String {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => return e.to_string(),
    };
    #[cfg(all(unix, not(target_os = "ios"), not(target_os = "android")))]
    {
        use std::os::unix::process::CommandExt;
        std::process::Command::new(exe).args(args).exec().to_string()
    }
    #[cfg(not(all(unix, not(target_os = "ios"), not(target_os = "android"))))]
    match std::process::Command::new(exe).args(args).spawn() {
        Ok(_) => std::process::exit(0),
        Err(e) => e.to_string(),
    }
}

/// Runs this game again in this process's place, with the same arguments but `--import` (it would ask again) (Unix);
/// elsewhere carries on.
fn restart() {
    #[cfg(all(unix, not(target_os = "ios"), not(target_os = "android")))]
    if let Ok(exe) = std::env::current_exe() {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(exe).args(std::env::args_os().skip(1).filter(|a| a != "--import")).exec();
        eprintln!("Couldn't restart after the import: {err}");
    }
}

/// The ISO from a file dialog, if the user picked a file: zenity or kdialog first on Linux (Gaming Mode on the Steam
/// Deck has no portal file chooser; openmx's picker does the same), else the system's (the xdg portal, as the mod
/// picker).
#[cfg(not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")))]
fn pick() -> Option<String> {
    const TITLE: &str = "Hot Shots Tennis: pick your disc image (SCUS-97610)";
    #[cfg(target_os = "linux")]
    for (cmd, args) in [
        ("zenity", &["--file-selection", "--title", TITLE, "--file-filter", "Disc image | *.iso *.ISO", "--file-filter", "All files | *"][..]),
        ("kdialog", &["--title", TITLE, "--getopenfilename", ".", "*.iso *.ISO"][..]),
    ] {
        let Ok(out) = std::process::Command::new(cmd).args(args).output() else { continue };
        // shown: the player's answer, a cancel included
        let picked = String::from_utf8_lossy(&out.stdout).trim().to_string();
        return (out.status.success() && !picked.is_empty()).then_some(picked);
    }
    let f = rfd::FileDialog::new().set_title(TITLE).add_filter("Disc image", &["iso", "ISO"]).pick_file()?;
    Some(f.to_string_lossy().into_owned())
}

#[cfg(any(target_arch = "wasm32", target_os = "ios", target_os = "android"))]
fn pick() -> Option<String> {
    None
}

/// A message on stderr, and in a dialog when there is one (a launcher start has no terminal).
fn tell(msg: &str) {
    eprintln!("{msg}");
    #[cfg(not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")))]
    if std::env::var_os("HST_BG").is_none() {
        rfd::MessageDialog::new().set_title("Hot Shots Tennis").set_description(msg).show();
    }
}

fn fail(msg: &str) -> ! {
    tell(msg);
    std::process::exit(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    /// A scratch folder of its own, gone after the test.
    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Tmp {
            let d = std::env::temp_dir().join(format!("hst-base-disc-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            Tmp(d)
        }
        /// For a full-size import (a whole disc pack): under the build's `target/`, on disk, not the system temp dir,
        /// which is often a small RAM disk with a per-user quota (an import there fails with "Disk quota exceeded").
        fn on_disk(tag: &str) -> Tmp {
            let exe = std::env::current_exe().unwrap();
            let target = exe.ancestors().find(|a| a.file_name().is_some_and(|n| n == "target"));
            let base = target.map_or_else(std::env::temp_dir, |t| t.join("tmp"));
            let d = base.join(format!("hst-base-disc-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            Tmp(d)
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// M180: running from the pack, a pack beside it takes its place and the old one stays beside under its serial;
    /// switching back swaps them again; a disc named on the command line moves nothing.
    #[test]
    fn base_disc_swaps_the_packs() {
        let t = Tmp::new("swap");
        let (pack, jp) = (t.0.join("disc.pack"), t.0.join("minna.pack"));
        std::fs::write(&pack, b"us").unwrap();
        std::fs::write(&jp, b"jp").unwrap();
        assert_eq!(swap(&pack, &pack, Edition::Us, &jp).unwrap(), pack);
        let us = t.0.join("disc-SCUS-97610.pack");
        assert_eq!((std::fs::read(&pack).unwrap(), std::fs::read(&us).unwrap()), (b"jp".to_vec(), b"us".to_vec()));
        assert!(!jp.exists());
        assert_eq!(swap(&pack, &pack, Edition::Japan, &us).unwrap(), pack);
        assert_eq!(std::fs::read(&pack).unwrap(), b"us");
        assert_eq!(std::fs::read(t.0.join("disc-SCPS-15113.pack")).unwrap(), b"jp");
        assert!(!us.exists());
        // run from an image named on the command line: nothing moves, the restart names the other disc
        let iso = t.0.join("us.iso");
        std::fs::write(&iso, b"us").unwrap();
        let other = t.0.join("disc-SCPS-15113.pack");
        assert_eq!(swap(&pack, &iso, Edition::Us, &other).unwrap(), other);
        assert!(other.exists() && pack.exists());
        // a failed import (not a disc) leaves the pack as it was
        let bad = t.0.join("bad.iso");
        std::fs::write(&bad, b"nope").unwrap();
        assert!(swap(&pack, &pack, Edition::Us, &bad).is_err());
        assert_eq!(std::fs::read(&pack).unwrap(), b"us");
        assert!(!t.0.join("disc-SCUS-97610.pack").exists());
    }

    /// M180: the restart's arguments: the disc named in place of the old one, an import's dropped, the flags kept.
    #[test]
    fn base_disc_restart_args() {
        let a = |v: &[&str]| v.iter().map(OsString::from).collect::<Vec<_>>();
        let (pack, iso, jp) = (Path::new("/d/disc.pack"), Path::new("/x/us.iso"), Path::new("/x/jp.iso"));
        assert_eq!(restart_args(a(&["--windowed"]), pack, pack, pack), a(&["--windowed"]));
        assert_eq!(restart_args(a(&["--import", "/x/us.iso", "--windowed"]), pack, pack, pack), a(&["--windowed"]));
        assert_eq!(restart_args(a(&["--import"]), pack, pack, pack), a(&[]));
        assert_eq!(restart_args(a(&["/x/us.iso", "--windowed"]), iso, jp, pack), a(&["/x/jp.iso", "--windowed"]));
        assert_eq!(restart_args(a(&["/d/disc.pack"]), pack, pack, pack), a(&["/d/disc.pack"]));
        assert_eq!(restart_args(a(&[]), pack, jp, pack), a(&["/x/jp.iso"]));
    }

    /// M180: the Japanese disc image (`fixtures/discs/`, skipped without it) imported in the US pack's place: the pack
    /// is then the Japanese release, the US one beside it. A few seconds (one import).
    #[test]
    fn base_disc_imports_an_image() {
        use hst_data::fixture;
        let jp = fixture::path("fixtures/discs/SCPS-15113.iso");
        if !jp.exists() {
            return fixture::skip("the Japanese disc (fixtures/discs/SCPS-15113.iso)");
        }
        let t = Tmp::on_disk("import");
        let pack = t.0.join("disc.pack");
        std::fs::write(&pack, b"us").unwrap();
        assert_eq!(swap(&pack, &pack, Edition::Us, &jp).unwrap(), pack);
        assert_eq!(Iso::open(&pack).ok().and_then(|mut i| Edition::of(&mut i)), Some(Edition::Japan));
        assert_eq!(std::fs::read(t.0.join("disc-SCUS-97610.pack")).unwrap(), b"us");
        assert!(!t.0.join("disc-next.pack").exists() && !t.0.join("disc-next.part").exists());
        assert!(jp.exists(), "the image stays");
    }
}
