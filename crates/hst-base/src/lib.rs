//! The game's lower layer, below the match (`hst-play`) and the app (`hst`): disc assets and how they're drawn
//! (characters, the GS material, shadows, effects, weather), sound, mods and the mod store, the online lobby
//! transport, the platform glue (browser, iOS, Android) and the few app-wide types the rest share (P60).

#[cfg(any(target_os = "android", test))]
pub mod android_pad;
pub mod asset_use;
pub mod audio;
pub mod character;
pub mod court_anim;
pub mod cpu_morph;
pub mod cull;
#[cfg(not(target_arch = "wasm32"))]
pub mod disc;
#[cfg(target_os = "ios")]
pub mod ios;
/// M159: Settings' Install info line (iOS); the wording and profile reading, tested anywhere.
pub mod install_info;
#[cfg(target_os = "android")]
pub mod android;
#[cfg(not(target_arch = "wasm32"))]
pub mod start_page;
pub mod effects;
// M155: muOS/Knulli handhelds with no display server: the framebuffer instead of winit
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub mod fbdev;
pub mod graphics;
pub mod device;
pub mod handshake;
pub mod grades;
pub mod gs;
pub mod modfs;
pub mod modstore;
pub mod mod_debug;
pub mod mods;
pub mod mods_online;
pub mod mods_umpire;
pub mod mods_court;
pub mod mods_textures;
pub mod court_mod;
pub mod net;
pub mod chat;
pub mod hud_gamma;
pub mod inspect;
pub mod lod;
pub mod mips;
pub mod noise;
pub mod occlusion;
pub mod pipelines;
pub mod jobs;
pub mod seats;
pub mod shade;
pub mod shadow;
pub mod spring;
pub mod stall;
pub mod textures;
pub mod twin_view;
pub mod ui_census;
pub mod scene_census;
pub mod ui_pool;
pub mod vertex_patch;
pub mod weather;
#[cfg(target_arch = "wasm32")]
pub mod web;
pub mod widescreen;

use bevy::prelude::*;

/// Everything the game saves (settings, controls, online, hints, names, favorites, picks, pipelines.json, mods-store.json,
/// mods-wishlist.txt, hf-token.txt, mods/, mods-online/, cache/): on desktop one folder ([`store::dir`]: beside the game
/// on Windows and Linux, B668, else per-user), in the
/// browser its localStorage, on the phones the app's own folder beside the disc.
pub mod store {
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    #[cfg(not(target_arch = "wasm32"))]
    pub use std::fs::{read_to_string, write};
    #[cfg(target_arch = "wasm32")]
    pub use crate::web::{read_to_string, write};

    /// The per-user folder, once [`init`] has run (the desktop game's start-up). Without it (tests, tools, the phones,
    /// the browser) files stay where they always were: beside the disc.
    static DIR: OnceLock<PathBuf> = OnceLock::new();

    /// Desktop start-up: from now on the game's files live in [`crate::disc::data_dir`] (beside the game where it can
    /// write there, else `$XDG_DATA_HOME/hst`, `%APPDATA%\hst`, `~/Library/Application Support/hst`), so a launch from the desktop doesn't write into `$HOME`
    /// or `/`, a disc on read-only media keeps its settings and moving the disc keeps them. The first run brings the
    /// old ones over ([`migrate`]). `HST_DATA=<dir>` uses that folder instead, as it is (`tools/shot.sh` keeps a
    /// shot's files beside its disc).
    #[cfg(not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")))]
    pub fn init(iso: &str) {
        if let Some(d) = std::env::var_os("HST_DATA").filter(|d| !d.is_empty()) {
            let _ = DIR.set(d.into());
            return;
        }
        let dir = crate::disc::data_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("can't make {}: {e}", dir.display());
        }
        // B668: the files the per-user folder held before they went beside the game come over (but its mods: they
        // play from there until updated, `modfs::moved`; and mods-online/, the online peers' mods, fetched again).
        // What was beside the disc or in the working folder went into it already.
        let app = crate::disc::app_data_dir();
        if app.is_dir() && !same(&app, &dir) {
            migrate_only(&app, &dir, |n| !matches!(n, "mods" | "mods-online"));
            if app.join("mods").is_dir() {
                crate::modfs::moved(&dir.join("mods"), &app.join("mods"));
            }
            let _ = OLD.set(app);
        } else {
            migrate(beside(iso), &dir);
            migrate(Path::new("."), &dir);
        }
        let _ = DIR.set(dir);
    }

    /// B668: the per-user folder the game's files were in before they went beside the game, once [`init`] has found
    /// one.
    static OLD: OnceLock<PathBuf> = OnceLock::new();

    /// The folder [`OLD`] names: its `mods/texture-replacements` is still read.
    pub fn old_dir() -> Option<&'static Path> {
        OLD.get().map(PathBuf::as_path)
    }

    fn same(a: &Path, b: &Path) -> bool {
        a.canonicalize().ok().zip(b.canonicalize().ok()).is_some_and(|(a, b)| a == b)
    }

    /// The files and folders a saved game is made of, by name.
    const SAVED: [&str; 16] = [
        "settings.txt",
        "controls.txt",
        "online.txt",
        "hints.txt",
        "names.txt",
        "favorites.txt",
        "picks.txt",
        "rejoin.txt",
        "camera.txt",
        "pipelines.json",
        "mods-store.json",
        "mods-wishlist.txt",
        "hf-token.txt",
        "mods",
        "mods-online",
        "cache",
    ];

    /// Brings what an older build saved in `from` (beside the disc, or the working folder for the store's files) over
    /// to `to`: each one `to` doesn't have yet, copied (the old copies stay for an older build of the game).
    pub fn migrate(from: &Path, to: &Path) {
        migrate_only(from, to, |_| true);
    }

    /// [`migrate`], of the files and folders `take` names.
    pub fn migrate_only(from: &Path, to: &Path, take: impl Fn(&str) -> bool) {
        if same(from, to) {
            return;
        }
        for name in SAVED.into_iter().filter(|n| take(n)) {
            let (old, new) = (from.join(name), to.join(name));
            if !old.exists() || new.exists() {
                continue;
            }
            let link = std::fs::symlink_metadata(&old).is_ok_and(|m| m.file_type().is_symlink());
            let copied = match () {
                // a linked folder (mods/ on another drive) is linked again, not copied
                #[cfg(unix)]
                _ if link => old.canonicalize().and_then(|t| std::os::unix::fs::symlink(t, &new)),
                _ if old.is_dir() && !link => copy_dir(&old, &new),
                _ => std::fs::copy(&old, &new).map(drop),
            };
            match copied {
                Ok(()) => eprintln!("brought {} over to {}", old.display(), new.display()),
                Err(e) => eprintln!("can't bring {} over to {}: {e}", old.display(), new.display()),
            }
        }
    }

    fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(to)?;
        for e in std::fs::read_dir(from)? {
            let e = e?;
            let (a, b) = (e.path(), to.join(e.file_name()));
            if e.file_type()?.is_dir() { copy_dir(&a, &b)? } else { std::fs::copy(&a, &b).map(drop)? }
        }
        Ok(())
    }

    /// The disc's folder (`.` for a bare file name).
    fn beside(iso: &str) -> &Path {
        Path::new(iso).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(".".as_ref())
    }

    /// The folder the game's files for disc `iso` go in: the per-user one, else beside the disc.
    pub fn dir(iso: &str) -> PathBuf {
        DIR.get().cloned().unwrap_or_else(|| beside(iso).into())
    }

    /// `name` in [`dir`].
    pub fn path(iso: &str, name: &str) -> PathBuf {
        dir(iso).join(name)
    }

    /// A file the game keeps without a disc at hand (the mod store's): in the per-user folder; else the working
    /// folder as before (the browser's localStorage key, the phones' app folder), a scratch folder in tests.
    pub fn file(name: &str) -> PathBuf {
        if let Some(d) = DIR.get() {
            return d.join(name);
        }
        #[cfg(all(any(test, feature = "testing"), not(target_arch = "wasm32")))]
        {
            let d = std::env::temp_dir().join(format!("hst-store-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&d);
            return d.join(name);
        }
        #[allow(unreachable_code)]
        PathBuf::from(name)
    }

    /// settings.txt (`main_menu::Settings::path`).
    pub fn settings(iso: &str) -> PathBuf {
        path(iso, "settings.txt")
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn migrate_copies_what_is_missing() {
            let t = std::env::temp_dir().join(format!("hst-migrate-{}", std::process::id()));
            let (old, new) = (t.join("old"), t.join("new"));
            let _ = std::fs::remove_dir_all(&t);
            std::fs::create_dir_all(old.join("mods/a")).unwrap();
            std::fs::create_dir_all(&new).unwrap();
            std::fs::write(old.join("settings.txt"), "music = on").unwrap();
            std::fs::write(old.join("mods/a/mod.json"), "{}").unwrap();
            std::fs::write(old.join("controls.txt"), "old").unwrap();
            std::fs::write(new.join("controls.txt"), "new").unwrap();
            std::fs::write(old.join("other.txt"), "").unwrap();
            super::migrate(&old, &new);
            assert_eq!(std::fs::read_to_string(new.join("settings.txt")).unwrap(), "music = on");
            assert!(old.join("settings.txt").exists() && old.join("mods/a/mod.json").exists(), "the old copies stay");
            assert!(new.join("mods/a/mod.json").exists());
            assert_eq!(std::fs::read_to_string(new.join("controls.txt")).unwrap(), "new", "newer files stay");
            assert!(!new.join("other.txt").exists());
            super::migrate(&new, &new);
            let _ = std::fs::remove_dir_all(&t);
        }
    }
}

#[derive(Resource, Clone)]
pub struct Args {
    pub iso: String,
    pub archives: Vec<String>,
    pub shot: Option<String>,
    /// Seconds of game time before the shot (`--shot-at`, default 0.5).
    pub shot_at: f32,
    pub radius: Option<f32>,
    pub ball: bool,
    pub court: usize,
    pub stage: Option<u32>,
    pub play: bool,
    pub singles: bool,
    /// Characters on court (player order), `--chars 0,2,1,5`.
    pub chars: Vec<usize>,
    /// Their outfits (costumes 0..9), `--outfits 9,9,4,9`: the model, the panel face and the computer players' AI row.
    pub outfits: Vec<usize>,
    /// Character viewer: `--character N [--motion M]`.
    pub viewer: Option<(usize, usize)>,
    /// Sound audition: archive, bank header, program, key.
    pub sound: Option<(String, String, usize, usize)>,
    /// `--music`: the court's BGM in a match (off by default).
    pub music: bool,
    /// The umpire (`--umpire N`, the menu's order: 0 Chika, 1 Suzuki, 2 Lily, 3 Robot Tennis, 4 Anna; default 4).
    pub umpire: u8,
    /// Sets to win (`--sets`, default 1) and games to win a set (`--games`, default 4).
    pub sets: i32,
    pub games: i32,
    /// `--pads a,b,c,d`: what drives seats 1P..4P (`k` the keyboard, a gamepad's index into the sorted pad list, `-`
    /// the computer), as the main menu assigned them (`controls::PadSlots`).
    pub pads: Option<[Option<seats::Seat>; 4]>,
}

#[derive(Component)]
pub struct Orbit {
    pub focus: Vec3,
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Orbit {
    /// The camera's transform on this rig.
    pub fn transform(&self) -> Transform {
        let rot = Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0);
        Transform::from_translation(self.focus + rot * Vec3::Z * self.radius).looking_at(self.focus, Vec3::Y)
    }
}

/// Parent for everything in game space: game space is Y-down, a half-turn about X maps it to Bevy's
/// Y-up without mirroring.
#[derive(Component)]
pub struct GameSpace;

/// A sky, `_clo`, `_acc` or `_bg` model: the court draw stands it at the camera's eye + (0, lift, 0), turned by yaw
/// about game y (`gs::backdrop`; slot 5's RAM has these matrices).
#[derive(Component)]
pub struct Backdrop {
    pub yaw: f32,
    pub lift: f32,
}

/// The backdrops' blended-draw sort offset (`GsMaterial::order`): before the hole's `-1e6`, so before everything.
pub const BACKDROP: f32 = -2.0e6;

/// The phone builds turn the frame-rate log on here: setting `HST_FPS` while the OS's own threads read the
/// environment is undefined behaviour.
pub static FPS_LOG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `HST_FPS=1` or `FPS_LOG`.
pub fn fps_log() -> bool {
    FPS_LOG.load(std::sync::atomic::Ordering::Relaxed) || std::env::var_os("HST_FPS").is_some()
}

/// Whole seconds since the Unix epoch (the page's clock in a browser): lobby listings, the store's cache, a rejoin.
pub fn unix_secs() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// The one tokio runtime natively (online play, the mod store, the self-updater), built on first use. A job that
/// blocks a thread (an install unzipping, the updater's loop) runs on its own thread through `block_on`, so it never
/// holds up the others.
#[cfg(not(target_arch = "wasm32"))]
pub fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Builder::new_multi_thread().enable_all().thread_name("tokio").build().unwrap())
}

/// A `play/` module's tick state for a resync (`play::snapshot::encode`): as bytes, then this machine's own entities
/// and handles put back (`local`; the handles go as their defaults). Here so `effects` and `weather` can be one.
pub trait Wire: Resource + Clone + serde::Serialize + serde::de::DeserializeOwned {
    fn local(&mut self, _local: &Self) {}
}
