//! Disc import: the first start copies what the remaster reads out of the user's ISO into a pack in the user data dir
//! (`hst_data::iso::Iso::write_pack`); from then on the game runs from the pack and the ISO can go. settings.txt,
//! mods/ and cache/ live beside the pack too (`crate::store`, M143), whatever disc a run starts from.
//!
//! `hst [flags]` runs from the pack (importing first if there is none: a file dialog asks for the ISO);
//! `hst --import <iso> [flags]` (re)imports; `hst <iso or pack> [flags]` runs straight from that file, as before.

use hst_data::iso::Iso;
use std::path::{Path, PathBuf};

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

/// iOS: the first ISO in the app's folder (Files → On My iPhone → OpenTennis).
pub fn first_iso() -> Option<PathBuf> {
    std::fs::read_dir(data_dir()).into_iter().flatten().flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("iso")))
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
        if Iso::open(&argv[0]).is_ok() {
            return;
        }
        eprintln!("Couldn't open the disc image {}: using the imported disc", argv.remove(0));
    }
    let mut pack = pack_path();
    if import || !pack.exists() {
        let iso = iso.or_else(pick).unwrap_or_else(|| fail("No disc image picked. Run: hst --import <your Hot Shots Tennis (USA) ISO>"));
        let was = std::mem::replace(&mut pack, data_dir().join("disc.pack"));
        if let Err(e) = import_iso(Path::new(&iso), &pack) {
            fail(&format!("Couldn't import {iso}: {e}"));
        }
        // B668: a re-import into the new folder takes the old folder's pack's place
        if was != pack {
            let _ = std::fs::remove_file(was);
        }
    }
    argv.insert(0, pack.to_string_lossy().into_owned());
}

fn import_iso(iso: &Path, pack: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(pack.parent().unwrap())?;
    let mut disc = Iso::open(iso)?;
    let mut last = 0;
    let hash = disc.write_pack(pack, |i, n| {
        if i * 10 / n != last {
            last = i * 10 / n;
            eprintln!("importing the disc: {}%", last * 10);
        }
    })?;
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

/// The ISO from the system's file dialog (the xdg portal on Linux, as the mod picker), if the user picked a file.
#[cfg(not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")))]
fn pick() -> Option<String> {
    let f = rfd::FileDialog::new().set_title("Hot Shots Tennis: pick your disc image (SCUS-97610)").add_filter("Disc image", &["iso", "ISO"]).pick_file()?;
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
