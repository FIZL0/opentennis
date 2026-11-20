//! B317: a newer build is out. The main menu shows a notice (`home.rs`) whose button refreshes the page (browser),
//! installs the new build (B374, desktop) or opens the release page to download it.
//!
//! Browser: `web/index.html` reads the site's `version.txt` (`tools/web.sh` writes the commit there) when the page
//! loads and again every few minutes; once the site's differs from the one this page loaded with, it sets
//! `hstLatest`. Desktop: builds made by GitHub Actions (`pages.yml`, the beta) ask the beta site which commit
//! its downloads are at ([`BETA_COMMIT`], B745: the beta is no GitHub release), at start and every half hour; a local
//! build never asks (its commit is never the beta's). `HST_LATEST=<commit>` in the environment fakes a newer one (the browser: set `hstLatest` in the console).
//!
//! B374: a release build (`tools/release.sh` builds it with `HST_VERSION=v1.2.3`) asks GitHub for the latest `v*`
//! release instead, at start and every half hour, unless Settings' Check for Updates is off or `--no-update-check`
//! was given; no network, a rate limit or bad JSON say nothing. A newer one (semver order, `-beta.N` before the
//! release) is offered; Update downloads the platform's bare binary beside the game as `opentennis.new`, checks it against
//! the release's `SHA256SUMS` (signed with minisign, the key in [`PUBLIC_KEY`]), swaps it in (Windows can't replace
//! a running .exe: it becomes `opentennis.exe.old`, removed at the next start) and starts it with the same arguments. A game
//! whose folder can't be written (installed system-wide, Flatpak, Steam) only gets the release page.
//! `HST_UPDATE_API=<url>` checks another releases/latest JSON (a local test server); `HST_UPDATE_NOW=1` installs what
//! it finds without waiting for the button (an unattended end-to-end run).
//!
//! M140: every platform updates in place ([`Way`], the write-up in `research/journal/2026-10-20-update-everywhere/`).
//! A game whose folder it can't write (a system-wide install, a Flatpak, Steam's folder, Program Files) installs the
//! new binary into the user data folder instead (`update/<tag>/`), and [`relaunch`], the first thing the game does at
//! start, hands over to the newest such copy that's newer than itself (Unix: `exec`, the same process for Steam or
//! Flatpak to follow). Android downloads the release's .apk, checks it the same way and gives it to the system's
//! installer (the player confirms; same signing key, so it installs over the app and keeps its files). iOS can't run
//! new code it downloads, and an installer's URL scheme can't be trusted to be that installer (B652: stock iOS opens
//! `apple-magnifier` itself): Download opens the release page in Safari, unless the installer left its mark in our own
//! bundle (M159a, [`Installer`]: AltStore, SideStore, TrollStore), and then Update opens that installer. Settings → Updates (Ask,
//! Always, Never; `updates` in settings.txt, Never being the older Check for Updates off) and the main menu's Update?
//! popup the first time a release is found ([`offer`]).

/// The release page the desktop notice's Download opens.
#[cfg(not(target_arch = "wasm32"))]
pub const RELEASES: &str = "https://beta.opentennis.dev/download/";

/// The full commit the beta's downloads were built from (`tools/beta-bundle.sh` writes it).
#[cfg(not(target_arch = "wasm32"))]
const BETA_COMMIT: &str = "https://beta.opentennis.dev/download/commit.txt";

/// This build's release tag (`v0.1.0-beta.9`, `v1.0.0`): only builds made by `tools/release.sh` have one, and only
/// they check for and install newer releases.
pub const VERSION: Option<&str> = option_env!("HST_VERSION");

/// The minisign key `tools/release.sh` signs each release's `SHA256SUMS` with (its secret half stays with the
/// maintainer, `~/.config/hst/release.key`).
#[cfg(not(target_arch = "wasm32"))]
pub const PUBLIC_KEY: &str = "RWTW5uzveKqmVluF6+6vYS5Uq/H7St1d9WMjyxVxXGpJAbr8+sT6HimM";

/// B374: the Check for Updates setting (settings.txt `update_check`) and `--no-update-check`.
pub static CHECK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
pub static NO_CHECK_FLAG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// This platform's bare binary on a release.
#[cfg(not(target_arch = "wasm32"))]
const ASSET: Option<&str> = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
    Some("opentennis-linux-x86_64")
} else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
    Some("opentennis-linux-aarch64")
} else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
    Some("opentennis-windows-x86_64.exe")
} else if cfg!(target_os = "android") {
    Some("opentennis-android.apk")
} else if cfg!(target_os = "ios") {
    Some("opentennis-ios.ipa")
} else {
    None
};

/// The newer build's commit (short), if one is out. Not this build's own: the page's load-time `version.txt` can
/// be a stale copy from around a deploy, so a later read "differs" while this wasm is already that commit.
#[cfg(target_arch = "wasm32")]
pub fn latest() -> Option<String> {
    let ours = env!("HST_COMMIT").trim_end_matches('+');
    js_sys::Reflect::get(&web_sys::window()?.into(), &"hstLatest".into())
        .ok()?
        .as_string()
        .filter(|s| !s.is_empty() && !s.starts_with(ours) && !ours.starts_with(s.as_str()))
}

/// B625: the main menu's "Updated to …" toast, once, when this is the first run of a new build: the browser keeps
/// the last build it ran (`hst-build` in local storage, written here), so a page that picked up a newer one (at
/// launch, `web/index.html`, or a Refresh) says so. A first visit says nothing. `HST_UPDATED=1` fakes one (`--shot`s).
pub fn updated() -> Option<String> {
    let ours = env!("HST_COMMIT").trim_end_matches('+');
    // as the main menu's version corner says it (`home.rs`)
    let says = || format!("Updated to {} - {}", VERSION.unwrap_or(concat!("v", env!("CARGO_PKG_VERSION"))), ours.get(..7).unwrap_or(ours));
    if std::env::var("HST_UPDATED").is_ok_and(|s| !s.is_empty()) {
        return Some(says());
    }
    #[cfg(target_arch = "wasm32")]
    {
        let store = web_sys::window()?.local_storage().ok()??;
        let before = store.get_item("hst-build").ok()?;
        let _ = store.set_item("hst-build", ours);
        before.filter(|b| !b.is_empty() && b != ours).map(|_| says())
    }
    #[cfg(not(target_arch = "wasm32"))]
    None
}

/// What the desktop check found and what an install is doing.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct State {
    /// The rolling beta's newer commit (short).
    commit: Option<String>,
    /// A newer release.
    found: Option<Found>,
    /// ○ Later: no offer for the rest of this run (a lobby refused for the version brings it back).
    later: bool,
    /// The install: its words and how far it is (0..1), or why it failed.
    job: Option<(String, Option<f32>)>,
    busy: bool,
}

#[cfg(not(target_arch = "wasm32"))]
static STATE: std::sync::Mutex<State> = std::sync::Mutex::new(State { commit: None, found: None, later: false, job: None, busy: false });

/// A newer release: its tag, page and the files an install needs.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Default, PartialEq)]
struct Found {
    tag: String,
    page: String,
    /// This platform's binary, `SHA256SUMS` and `SHA256SUMS.sig` (all three, or no install).
    files: Option<[String; 3]>,
    /// B710: Linux's software renderer (`vulkan/`, B708) as the release's `<binary>-vulkan.tar.gz`, if it has one.
    vulkan: Option<String>,
}

/// The newer build's release tag or commit (short), if one is out (and not put off till later).
#[cfg(not(target_arch = "wasm32"))]
pub fn latest() -> Option<String> {
    use std::sync::atomic::Ordering::Relaxed;
    static START: std::sync::Once = std::sync::Once::new();
    if !CHECK.load(Relaxed) || NO_CHECK_FLAG.load(Relaxed) {
        return None;
    }
    START.call_once(|| {
        if cfg!(test) {
            return;
        }
        if let Some(fake) = std::env::var("HST_LATEST").ok().filter(|s| !s.is_empty()) {
            STATE.lock().unwrap().commit = Some(fake);
            return;
        }
        // a release build asks for releases; else this build's full commit, only in an Actions build
        let ours = match (VERSION, option_env!("GITHUB_SHA")) {
            (Some(v), _) => Ok(v),
            (None, Some(sha)) => Err(sha),
            (None, None) => return,
        };
        let t = std::thread::spawn(move || {
            let rt = crate::runtime();
            let api = std::env::var("HST_UPDATE_API").unwrap_or_else(|_| "https://api.github.com/repos/FIZL0/opentennis/releases/latest".into());
            // B710: a game updated by an updater from before B710 came without vulkan/: its own release's, once
            if let (Ok(v), Some(exe)) = (ours, exe()) {
                let own = format!("https://api.github.com/repos/FIZL0/opentennis/releases/tags/{v}");
                if let Err(e) = rt.block_on(mend(&own, ASSET, &exe, PUBLIC_KEY)) {
                    bevy::log::info!("update: no software renderer beside the game: {e}");
                }
            }
            loop {
                let asked = std::time::SystemTime::now();
                match ours {
                    Ok(v) => match rt.block_on(check(&api, v, ASSET)) {
                        Ok(f) => {
                            // in the log (a phone's hst-log.txt): whether this check saw the new release
                            bevy::log::info!("update check: {}", f.as_ref().map_or(format!("{v} is the latest"), |f| format!("{} is out", f.tag)));
                            let now = f.is_some() && std::env::var_os("HST_UPDATE_NOW").is_some();
                            STATE.lock().unwrap().found = f;
                            if now {
                                act();
                            }
                        }
                        Err(e) => bevy::log::info!("update check: {e}"),
                    },
                    Err(ours) => match rt.block_on(beta()) {
                        Ok(sha) if sha != ours => STATE.lock().unwrap().commit = Some(sha[..7.min(sha.len())].to_string()),
                        Ok(_) => {}
                        Err(e) => bevy::log::info!("update check: {e}"),
                    },
                }
                // B567: half an hour by the wall clock, not one 30-minute sleep: a phone keeps the app suspended for
                // hours or days and a sleep's clock stops meanwhile, so a resumed app asked again only after half an
                // hour more of use (a 1.0 IPA opened on a phone after 1.1.0 came out didn't offer it); [`recheck`]
                // wakes it at once when the app comes back to the foreground
                while !due(asked, std::time::SystemTime::now()) && !WOKE.swap(false, std::sync::atomic::Ordering::Relaxed) {
                    std::thread::park_timeout(std::time::Duration::from_secs(5));
                }
            }
        });
        let _ = CHECKER.set(t.thread().clone());
    });
    let s = STATE.lock().unwrap();
    // an install under way stays on show
    if s.later && s.job.is_none() {
        return None;
    }
    s.found.as_ref().map(|f| f.tag.clone()).or_else(|| s.commit.clone())
}

#[cfg(not(target_arch = "wasm32"))]
static CHECKER: std::sync::OnceLock<std::thread::Thread> = std::sync::OnceLock::new();
#[cfg(not(target_arch = "wasm32"))]
static WOKE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// B567: the app came back to the foreground: check again now rather than at the half hour.
pub fn recheck(mut life: bevy::prelude::MessageReader<bevy::window::AppLifecycle>) {
    if life.read().any(|l| *l == bevy::window::AppLifecycle::WillResume) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(t) = CHECKER.get() {
            WOKE.store(true, std::sync::atomic::Ordering::Relaxed);
            t.unpark();
        }
    }
}

/// Whether the next check is due: half an hour after the last one `asked` by the wall clock (which keeps counting
/// while a phone has the app suspended, and a clock set back to before it counts as due).
#[cfg(not(target_arch = "wasm32"))]
fn due(asked: std::time::SystemTime, now: std::time::SystemTime) -> bool {
    now.duration_since(asked).map_or(true, |d| d >= std::time::Duration::from_secs(30 * 60))
}

/// The commit the beta's downloads were built from.
#[cfg(not(target_arch = "wasm32"))]
async fn beta() -> Result<String, String> {
    beta_commit(&fetch(BETA_COMMIT, 256).await?)
}

/// `commit.txt`'s full commit (40 hex digits and a newline).
#[cfg(not(target_arch = "wasm32"))]
fn beta_commit(body: &[u8]) -> Result<String, String> {
    std::str::from_utf8(body).ok().map(str::trim).filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit())).map(str::to_string).ok_or_else(|| "no commit".into())
}

/// `url`'s JSON.
#[cfg(not(target_arch = "wasm32"))]
async fn json(url: &str) -> Result<serde_json::Value, String> {
    serde_json::from_slice(&fetch(url, 1 << 20).await?).map_err(|e| e.to_string())
}

/// `url`'s body, at most `cap` bytes.
#[cfg(not(target_arch = "wasm32"))]
async fn fetch(url: &str, cap: u64) -> Result<Vec<u8>, String> {
    let b = get(url).await?.bytes().await.map_err(|e| e.to_string())?;
    if b.len() as u64 > cap {
        return Err(format!("{url}: too big"));
    }
    Ok(b.to_vec())
}

#[cfg(not(target_arch = "wasm32"))]
async fn get(url: &str) -> Result<reqwest::Response, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let r = crate::modstore::client()
        .get(url)
        .header("User-Agent", "hot-shots-tennis")
        .header("Accept", "application/vnd.github+json, application/octet-stream")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !r.status().is_success() {
        return Err(format!("{url}: {}", r.status()));
    }
    Ok(r)
}

/// The latest release at `api` (GitHub's releases/latest JSON), if it's a `v*` newer than `ours`; `asset` is this
/// platform's binary.
#[cfg(not(target_arch = "wasm32"))]
async fn check(api: &str, ours: &str, asset: Option<&str>) -> Result<Option<Found>, String> {
    Ok(found(&json(api).await?, ours, asset))
}

/// A releases/latest answer `j` as an offer: a `v*` tag newer than `ours`, with its files when it has them all.
#[cfg(not(target_arch = "wasm32"))]
fn found(j: &serde_json::Value, ours: &str, asset: Option<&str>) -> Option<Found> {
    let tag = j["tag_name"].as_str()?;
    if !tag.starts_with('v') || j["draft"].as_bool() == Some(true) || !newer(tag, ours) {
        return None;
    }
    let url = |name: &str| {
        j["assets"].as_array()?.iter().find(|a| a["name"].as_str() == Some(name))?["browser_download_url"].as_str().map(str::to_string)
    };
    let files = asset.and_then(|a| Some([url(a)?, url("SHA256SUMS")?, url("SHA256SUMS.sig")?]));
    let vulkan = asset.filter(|a| a.starts_with("opentennis-linux-")).and_then(|a| url(&format!("{a}-vulkan.tar.gz")));
    Some(Found { tag: tag.into(), page: j["html_url"].as_str().unwrap_or(RELEASES).into(), files, vulkan })
}

/// Whether version `a` comes after `b` (`v1.2.3`, `v0.1.0-beta.10`; semver order: a pre-release before its
/// release, numeric parts by number). Unreadable ones are never newer.
#[cfg(not(target_arch = "wasm32"))]
pub fn newer(a: &str, b: &str) -> bool {
    fn parse(s: &str) -> Option<([u64; 3], Option<&str>)> {
        let s = s.trim().trim_start_matches('v').split('+').next()?;
        let (core, pre) = s.split_once('-').map_or((s, None), |(c, p)| (c, Some(p)));
        let mut n = core.split('.').map(|x| x.parse().ok());
        let v = [n.next()??, n.next()??, n.next()??];
        n.next().is_none().then_some((v, pre))
    }
    let (Some((va, pa)), Some((vb, pb))) = (parse(a), parse(b)) else { return false };
    if va != vb {
        return va > vb;
    }
    match (pa, pb) {
        (None, Some(_)) => true,
        (_, None) => false,
        (Some(pa), Some(pb)) => {
            use std::cmp::Ordering::*;
            for (x, y) in pa.split('.').zip(pb.split('.')) {
                let o = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(x), Ok(y)) => x.cmp(&y),
                    (Ok(_), Err(_)) => Less,
                    (Err(_), Ok(_)) => Greater,
                    _ => x.cmp(y),
                };
                if o != Equal {
                    return o == Greater;
                }
            }
            pa.split('.').count() > pb.split('.').count()
        }
    }
}

/// The game's own file, as it was at start (Linux's `/proc/self/exe` says "(deleted)" once it's been swapped).
#[cfg(not(target_arch = "wasm32"))]
fn exe() -> Option<std::path::PathBuf> {
    static EXE: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    EXE.get_or_init(|| std::env::current_exe().ok().and_then(|p| p.canonicalize().ok())).clone()
}

/// At start: notes the game's path and removes the `.old` an update on Windows leaves behind.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(target_os = "android", allow(dead_code))] // only `main` calls it, which the Android .so doesn't run
pub fn cleanup() {
    if let Some(e) = exe() {
        let _ = std::fs::remove_file(old(&e));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn old(exe: &std::path::Path) -> std::path::PathBuf {
    let mut s = exe.as_os_str().to_owned();
    s.push(".old");
    s.into()
}

/// How Update installs a release here (M140).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, PartialEq)]
enum Way {
    /// The new binary takes the game file's place: its folder is the player's (a build unpacked somewhere, a Steam
    /// Deck's non-Steam game).
    #[cfg_attr(any(target_os = "android", target_os = "ios"), allow(dead_code))]
    Swap(std::path::PathBuf),
    /// The new binary goes to this path in the user data folder's `update/<tag>/`, which [`relaunch`] starts from
    /// then on: the game's folder isn't the player's to write (a system-wide install, Program Files, a Flatpak, Steam's
    /// own folder, which only Steam updates), or the game already is such a copy.
    #[cfg_attr(any(target_os = "android", target_os = "ios"), allow(dead_code))]
    Copy(std::path::PathBuf),
    /// Android: the .apk, checked, to the system's installer.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    Apk,
    /// iOS: the sideload installer that put this app here, opened at this link (M159a, [`Installer`]).
    #[cfg_attr(not(target_os = "ios"), allow(dead_code))]
    Installer(String),
    /// The release page.
    Page,
}

/// How `f` installs on this machine.
#[cfg(not(target_arch = "wasm32"))]
fn way(f: &Found) -> Way {
    let Some(files) = &f.files else { return Way::Page };
    #[cfg(target_os = "android")]
    {
        let _ = files;
        return if from_store() { Way::Page } else { Way::Apk };
    }
    // B652: an app can't replace itself on iOS, and the sideload installers' schemes aren't safe to guess (stock iOS
    // answers TrollStore's `apple-magnifier` itself and offers to restore Magnifier): the release page, in Safari
    // M159a: unless what installed it left its mark in our own bundle ([`installer`]): then that installer
    #[cfg(target_os = "ios")]
    {
        return match ios_installer() {
            Some(i) => Way::Installer(i.link(&files[0])),
            None => Way::Page,
        };
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = files;
        let Some(exe) = exe() else { return Way::Page };
        let flatpak = std::env::var_os("FLATPAK_ID").is_some();
        if !flatpak && !steams(&exe) && !copied(&exe) && writable() {
            return Way::Swap(exe);
        }
        Way::Copy(copies().join(&f.tag).join(format!("opentennis{}", std::env::consts::EXE_SUFFIX)))
    }
}

/// M159a: a sideload installer for iOS, told only by what it leaves in our own bundle ([`installer`]). Feather adds
/// nothing of its own to the apps it signs, so a Feather install (or Sideloadly, a developer build) gets the release
/// page; its source updates the app anyway.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)] // the apps' own names
pub enum Installer {
    AltStore,
    SideStore,
    TrollStore,
}

/// The source `web/ios.html` adds to AltStore and SideStore, which lists this app.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
const SOURCE: &str = "https://fizl0.github.io/apps/source.json";

#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
impl Installer {
    pub fn name(self) -> &'static str {
        match self {
            Installer::AltStore => "AltStore",
            Installer::SideStore => "SideStore",
            Installer::TrollStore => "TrollStore",
        }
    }

    /// The link that opens it on the new release: AltStore and SideStore at our source (it shows the update),
    /// TrollStore installing the release's .ipa (`ipa`) over this app.
    fn link(self, ipa: &str) -> String {
        match self {
            Installer::AltStore => format!("altstore://source?url={}", escape(SOURCE)),
            Installer::SideStore => format!("sidestore://source?url={}", escape(SOURCE)),
            // TrollStore's own scheme; only asked for once its marker is in our bundle (B652: stock iOS answers it
            // with Magnifier on a phone without TrollStore)
            Installer::TrollStore => format!("apple-magnifier://install?url={}", escape(ipa)),
        }
    }
}

/// M159a: which installer put this app here, from our bundle's URL schemes and whether TrollStore's marker file sits
/// beside it (`hst_base::ios::install_marks`): TrollStore marks the app's container; AltStore and SideStore each add a
/// scheme of their own name and the app's bundle id (`altstore-…`, `sidestore-…`) so they can open it. SideStore
/// first: it drops an earlier `sidestore-` scheme when it re-signs but leaves AltStore's.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
fn installer(schemes: &[String], troll: bool) -> Option<Installer> {
    let has = |prefix: &str| schemes.iter().any(|s| s.len() > prefix.len() && s.to_ascii_lowercase().starts_with(prefix));
    if troll {
        Some(Installer::TrollStore)
    } else if has("sidestore-") {
        Some(Installer::SideStore)
    } else if has("altstore-") {
        Some(Installer::AltStore)
    } else {
        None
    }
}

/// [`installer`] on this phone (looked at once a run: [`button`] asks every frame).
#[cfg(target_os = "ios")]
fn ios_installer() -> Option<Installer> {
    static ONE: std::sync::OnceLock<Option<Installer>> = std::sync::OnceLock::new();
    *ONE.get_or_init(|| {
        let (schemes, troll) = crate::ios::install_marks();
        installer(&schemes, troll)
    })
}

/// M159: Settings' Install info line on iOS (`hst_base::ios::install_info`), ending with which installer Update
/// takes it for ([`installer`]), so a screenshot checks both; None elsewhere.
pub fn install_info() -> Option<String> {
    #[cfg(target_os = "ios")]
    return Some(format!("{} Update opens: {}.", crate::ios::install_info(), ios_installer().map_or("the release page", Installer::name)));
    #[cfg(not(target_os = "ios"))]
    None
}

/// M159a: the installer the notice's Update opens with the new release, if any (iOS).
pub fn through() -> Option<&'static str> {
    #[cfg(target_os = "ios")]
    return STATE.lock().unwrap().found.as_ref().and_then(|f| f.files.as_ref()).and(ios_installer()).map(Installer::name);
    #[cfg(not(target_os = "ios"))]
    None
}

/// `s` as one query value (`encodeURIComponent`).
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
fn escape(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Whether the game's folder takes new files (looked at once a run: [`button`] asks every frame).
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
fn writable() -> bool {
    static OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OK.get_or_init(|| {
        let Some(dir) = exe().and_then(|e| e.parent().map(|d| d.to_path_buf())) else { return false };
        let probe = dir.join(".hst-update-probe");
        let ok = std::fs::write(&probe, b"").is_ok();
        let _ = std::fs::remove_file(&probe);
        ok
    })
}

/// Whether `exe` is in a Steam library (`steamapps/`), whose files Steam owns and checks. A Steam Deck's non-Steam game
/// (Steam's environment variables, but the player's own folder) isn't.
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
fn steams(exe: &std::path::Path) -> bool {
    exe.components().any(|c| c.as_os_str().eq_ignore_ascii_case("steamapps"))
}

/// Where [`Way::Copy`] puts releases: `update/` in the per-user data folder (a Flatpak's own,
/// `~/.var/app/<id>/data/hst`), never beside the game (B668: a copy's own folder isn't where its data goes, and a game
/// that copies at all can't write its own folder).
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
fn copies() -> std::path::PathBuf {
    let d = crate::disc::app_data_dir().join("update");
    d.canonicalize().unwrap_or(d)
}

/// Whether `exe` is one of [`copies`].
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
fn copied(exe: &std::path::Path) -> bool {
    exe.starts_with(copies())
}

/// The newest release in `root` (`root/<tag>/<bin>`) newer than `ours`: its tag and binary.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(any(target_os = "android", target_os = "ios"), allow(dead_code))]
fn newest(root: &std::path::Path, ours: &str, bin: &str) -> Option<(String, std::path::PathBuf)> {
    let mut best: Option<(String, std::path::PathBuf)> = None;
    for e in std::fs::read_dir(root).ok()?.flatten() {
        let tag = e.file_name().to_string_lossy().into_owned();
        let path = e.path().join(bin);
        if tag.starts_with('v') && newer(&tag, ours) && path.is_file() && best.as_ref().is_none_or(|b| newer(&tag, &b.0)) {
            best = Some((tag, path));
        }
    }
    best
}

/// The releases in `root` that `keep` (the one started, or none) doesn't need: every `v*` folder but `keep`'s and
/// the running game's own.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(any(target_os = "android", target_os = "ios"), allow(dead_code))]
fn stale(root: &std::path::Path, keep: Option<&str>, running: Option<&std::path::Path>) -> Vec<std::path::PathBuf> {
    let Ok(dir) = std::fs::read_dir(root) else { return Vec::new() };
    dir.flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with('v') && keep != Some(&*e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .filter(|p| running.is_none_or(|r| !r.starts_with(p)))
        .collect()
}

/// M140, the thin launcher: the first thing a release build does at start. A newer release that [`Way::Copy`] put in
/// the user data folder runs instead of this game, with the same arguments (Unix: `exec`, so Steam or Flatpak keep
/// following the same process; Windows: it waits for it and exits with its code); copies no newer than this game (a
/// Steam or Flatpak update overtook them) are removed. `OPENTENNIS_RELAUNCHED` keeps the copy from handing over again (not an
/// `HST_*` name: a player build drops those at start).
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(any(target_os = "android", target_os = "ios"), allow(dead_code))]
pub fn relaunch() {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let Some(ours) = VERSION else { return };
        if std::env::var_os("OPENTENNIS_RELAUNCHED").is_some() {
            return;
        }
        let root = copies();
        let next = newest(&root, ours, &format!("opentennis{}", std::env::consts::EXE_SUFFIX));
        for p in stale(&root, next.as_ref().map(|n| n.0.as_str()), exe().as_deref()) {
            let _ = std::fs::remove_dir_all(p);
        }
        let Some((tag, bin)) = next else { return };
        let mut c = std::process::Command::new(&bin);
        c.args(std::env::args_os().skip(1)).env("OPENTENNIS_RELAUNCHED", &tag);
        #[cfg(unix)]
        let e = std::os::unix::process::CommandExt::exec(&mut c);
        #[cfg(windows)]
        let e = match c.status() {
            Ok(s) => std::process::exit(s.code().unwrap_or(1)),
            Err(e) => e,
        };
        // it won't start: this game plays on
        eprintln!("update: couldn't start {}: {e}", bin.display());
    }
}

/// Android: whether Google Play installed the app (its rules: updates only through Play, so the release page).
#[cfg(target_os = "android")]
fn from_store() -> bool {
    static PLAY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *PLAY.get_or_init(crate::android::from_store)
}

/// The notice's button: the page reloads (once its mod saves are done, `web.rs`) into the new build, which the
/// service worker has already downloaded in the background (`web/sw.js`, B470); a newer release installs (B374, M140's
/// [`Way`]); else the release page opens.
pub fn act() {
    #[cfg(target_arch = "wasm32")]
    crate::web::reload();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut s = STATE.lock().unwrap();
        if s.busy {
            return;
        }
        let Some(f) = s.found.clone() else {
            drop(s);
            return crate::modstore::open(RELEASES);
        };
        let w = way(&f);
        let files = match (&w, f.files.clone()) {
            (Way::Page, _) | (_, None) => {
                drop(s);
                return crate::modstore::open(&f.page);
            }
            (Way::Installer(link), _) => {
                drop(s);
                return crate::modstore::open(link);
            }
            (_, Some(files)) => files,
        };
        (s.busy, s.job) = (true, Some((format!("Downloading {}...", f.tag), Some(0.0))));
        drop(s);
        std::thread::spawn(move || {
            let rt = crate::runtime();
            let shown = |got: u64, total: u64| {
                let k = got as f32 / total.max(1) as f32;
                STATE.lock().unwrap().job = Some((format!("Downloading {}... {} / {} MB", f.tag, got >> 20, total >> 20), Some(k.min(1.0))));
            };
            let job = |says: String| STATE.lock().unwrap().job = Some((says, None));
            let done = match &w {
                Way::Swap(exe) | Way::Copy(exe) => {
                    let dir = exe.parent().map(|d| std::fs::create_dir_all(d).map_err(|e| format!("can't make {}: {e}", d.display())));
                    match dir.unwrap_or(Ok(())).and_then(|()| rt.block_on(install(&files, exe, PUBLIC_KEY, &shown))) {
                        Ok(()) => {
                            // B710: the new game's folder gets Rendering → Software's driver too, or the row says where it went
                            if let Err(e) = rt.block_on(vulkan(exe, self::exe().as_deref(), f.vulkan.as_deref(), &files, PUBLIC_KEY, &shown)) {
                                bevy::log::warn!("update: no software renderer beside {}: {e}", exe.display());
                            }
                            job(format!("Starting {}...", f.tag));
                            // the same arguments (the disc), from the new file (at the old one's path, or the copy)
                            match std::process::Command::new(exe).args(std::env::args_os().skip(1)).spawn() {
                                Ok(_) => std::process::exit(0),
                                Err(e) => Ok(format!("Updated to {}: start the game again ({e}).", f.tag)),
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
                Way::Apk => apk(rt, &files, &f.tag, &shown, &job),
                Way::Page | Way::Installer(_) => unreachable!(),
            };
            match done {
                Ok(says) => job(says),
                Err(e) => {
                    bevy::log::warn!("update: {e}");
                    job(format!("Update failed: {e}"));
                }
            }
            STATE.lock().unwrap().busy = false;
        });
    }
}

/// Android: the release's .apk downloaded beside the app's files (`update.apk`), checked as [`install`] checks a
/// binary, and handed to the system's installer; then its answer, while the player confirms (an install that goes
/// through ends the app: the system replaces it). The last words to show.
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_os = "android"), allow(unused_variables))]
fn apk(rt: &tokio::runtime::Runtime, files: &[String; 3], tag: &str, shown: &dyn Fn(u64, u64), job: &dyn Fn(String)) -> Result<String, String> {
    #[cfg(target_os = "android")]
    {
        let to = std::env::current_dir().map_err(|e| e.to_string())?.join("update.apk");
        rt.block_on(verified(files, &to, PUBLIC_KEY, shown))?;
        crate::android::install_apk(&to)?;
        job(format!("Install {tag}: confirm on the screen..."));
        loop {
            std::thread::sleep(std::time::Duration::from_millis(250));
            match crate::android::install_status() {
                None => continue,
                Some(0) => return Ok(format!("Installed {tag}: open the game again.")),
                // `PackageInstaller.STATUS_FAILURE_ABORTED`: the player said no
                Some(3) => {
                    let _ = std::fs::remove_file(&to);
                    return Ok(format!("{tag} wasn't installed."));
                }
                Some(n) => {
                    let _ = std::fs::remove_file(&to);
                    return Err(format!("the installer turned it down (status {n})"));
                }
            }
        }
    }
    #[allow(unreachable_code)]
    Err("no installer on this platform".into())
}

/// ○ on the notice: no offer again this run.
pub fn later() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut s = STATE.lock().unwrap();
        if !s.busy {
            (s.later, s.job) = (true, None);
        }
    }
}

/// The install's words and how far its download is (0..1), while one runs or after it failed.
pub fn progress() -> Option<(String, Option<f32>)> {
    #[cfg(not(target_arch = "wasm32"))]
    return STATE.lock().unwrap().job.clone();
    #[cfg(target_arch = "wasm32")]
    None
}

/// The button's word.
pub fn button() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    return "Refresh";
    #[cfg(not(target_arch = "wasm32"))]
    if installs() || through().is_some() { "Update" } else { "Download" }
}

/// Whether the button installs in place (M140; never on iOS, B652), else it opens the release page.
pub fn installs() -> bool {
    #[cfg(target_arch = "wasm32")]
    return false;
    #[cfg(not(target_arch = "wasm32"))]
    STATE.lock().unwrap().found.as_ref().is_some_and(|f| !matches!(way(f), Way::Page | Way::Installer(_)))
}

/// M140: whether an install runs to its end here with no one at the controls but the system's own confirm (Always
/// starts it by itself).
#[cfg(not(target_arch = "wasm32"))]
fn installs_alone() -> bool {
    STATE.lock().unwrap().found.as_ref().is_some_and(|f| matches!(way(f), Way::Swap(_) | Way::Copy(_) | Way::Apk))
}

/// M140: Settings → Updates on Always: a release found installs by itself from the main menu (else Ask: the Update?
/// popup; Never is the older Check for Updates off, [`CHECK`]).
pub static ALWAYS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// M140: settings.txt's `updates` line for Check for Updates `check` and Always `always` ([`ALWAYS`]).
pub fn settings_line(check: bool, always: bool) -> String {
    let word = if !check { "never" } else if always { "always" } else { "ask" };
    format!("updates = {word}\n")
}

/// What the main menu does about a newer version, once per one found: [`Due::Ask`] the Update? popup, [`Due::Install`]
/// it (Always, where it installs by itself).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))] // the browser's `offer` is always None
pub enum Due {
    Ask,
    Install,
}

/// The release the main menu last asked about (or installed by itself).
#[cfg(not(target_arch = "wasm32"))]
static ASKED: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// M140: called by the main menu while nothing else is up on it: what to do about a newer version, the first time
/// it's seen (a later one asks again). None in the browser, whose notice refreshes the page.
pub fn offer() -> Option<Due> {
    #[cfg(target_arch = "wasm32")]
    return None;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let tag = latest()?;
        let mut asked = ASKED.lock().unwrap();
        if asked.as_deref() == Some(tag.as_str()) || STATE.lock().unwrap().busy {
            return None;
        }
        *asked = Some(tag);
        drop(asked);
        if !ALWAYS.load(std::sync::atomic::Ordering::Relaxed) {
            Some(Due::Ask)
        } else {
            installs_alone().then_some(Due::Install)
        }
    }
}

/// This build's version as the notice shows it: its release tag, else its commit.
pub fn ours() -> String {
    VERSION.map_or_else(|| env!("HST_COMMIT").to_string(), str::to_string)
}

/// A lobby's refusal (`why`, `net.rs`): one for a different version whose host runs a newer published release
/// brings the main menu's offer back and says so.
pub fn refused(why: String) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let host = why.split_once("different version (host ").and_then(|(_, r)| r.split([',', ')']).next()).map(|h| h.trim().trim_start_matches("hst ").to_string());
        let mut s = STATE.lock().unwrap();
        if let (Some(host), Some(f), Some(ours)) = (host, s.found.clone(), VERSION)
            && newer(&host, ours)
            && !newer(&host, &f.tag)
        {
            s.later = false;
            return format!("{why} {} is out: update from the main menu.", f.tag);
        }
    }
    why
}

/// `files` (this platform's binary, `SHA256SUMS`, `SHA256SUMS.sig`) installed over `exe` (or as `exe`, a new
/// [`Way::Copy`]): the binary downloads to `opentennis.new` beside it ([`verified`]), then takes `exe`'s place. Any
/// failure leaves `exe` as it was and no `opentennis.new`.
#[cfg(not(target_arch = "wasm32"))]
async fn install(files: &[String; 3], exe: &std::path::Path, key: &str, shown: &dyn Fn(u64, u64)) -> Result<(), String> {
    let new = exe.with_file_name("opentennis.new");
    verified(files, &new, key, shown).await?;
    let ok = swap(&new, exe, cfg!(windows));
    if ok.is_err() {
        let _ = std::fs::remove_file(&new);
    }
    ok
}

/// `files[0]` downloaded to `to`, once the sums' signature is checked against `key` and the download's sha256
/// against them; any failure leaves no `to`.
#[cfg(not(target_arch = "wasm32"))]
async fn verified(files: &[String; 3], to: &std::path::Path, key: &str, shown: &dyn Fn(u64, u64)) -> Result<(), String> {
    let name = files[0].rsplit('/').next().unwrap_or_default().to_string();
    let sums = fetch(&files[1], 1 << 16).await?;
    let sig = fetch(&files[2], 1 << 12).await?;
    let want = signed_sum(&sums, &sig, key, &name)?;
    let got = download(&files[0], to, shown).await;
    let ok = got.and_then(|h| if h == want { Ok(()) } else { Err(format!("{name}'s checksum doesn't match the signed one")) });
    if ok.is_err() {
        let _ = std::fs::remove_file(to);
    }
    ok
}

/// `name`'s sha256 (hex) from `sums` (`sha256sum`'s lines), once `sig` proves `key` signed them.
#[cfg(not(target_arch = "wasm32"))]
fn signed_sum(sums: &[u8], sig: &[u8], key: &str, name: &str) -> Result<String, String> {
    let key = minisign_verify::PublicKey::from_base64(key).map_err(|e| e.to_string())?;
    let sig = minisign_verify::Signature::decode(std::str::from_utf8(sig).map_err(|e| e.to_string())?).map_err(|e| format!("bad signature file: {e}"))?;
    key.verify(sums, &sig, false).map_err(|e| format!("the checksums' signature doesn't match ({e})"))?;
    String::from_utf8_lossy(sums)
        .lines()
        .find_map(|l| {
            let (h, n) = l.split_once(char::is_whitespace)?;
            (n.trim().trim_start_matches('*') == name).then(|| h.to_ascii_lowercase())
        })
        .ok_or_else(|| format!("{name} isn't in the checksums"))
}

/// `url` streamed into `to` (at most 1 GB): its sha256 (hex).
#[cfg(not(target_arch = "wasm32"))]
async fn download(url: &str, to: &std::path::Path, shown: &dyn Fn(u64, u64)) -> Result<String, String> {
    use n0_future::StreamExt;
    use sha2::Digest;
    use std::io::Write;
    let r = get(url).await?;
    let total = r.content_length().unwrap_or(0);
    let mut f = std::fs::File::create(to).map_err(|e| format!("can't write {}: {e}", to.display()))?;
    let (mut got, mut hash, mut chunks) = (0u64, sha2::Sha256::new(), r.bytes_stream());
    while let Some(c) = chunks.next().await {
        let c = c.map_err(|e| format!("download cut short: {e}"))?;
        got += c.len() as u64;
        if got > 1 << 30 {
            return Err("download too big".into());
        }
        hash.update(&c);
        f.write_all(&c).map_err(|e| format!("can't write {}: {e}", to.display()))?;
        shown(got, total.max(got));
    }
    f.sync_all().map_err(|e| e.to_string())?;
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// B710: `vulkan/` (B708's software renderer, which the Linux tarballs carry beside the game) beside `exe`, a game
/// just installed from the bare binary: left as it is when it's there; else copied from the folder of the game that's
/// running (`running`: a tarball install whose update went to [`Way::Copy`]'s folder); else the release's
/// `-vulkan.tar.gz` (`url`) downloaded, checked against the signed sums in `files` like the binary, and unpacked. A
/// release without one, or a platform with none, is fine. A failure leaves no half-made `vulkan/`.
#[cfg(not(target_arch = "wasm32"))]
async fn vulkan(exe: &std::path::Path, running: Option<&std::path::Path>, url: Option<&str>, files: &[String; 3], key: &str, shown: &dyn Fn(u64, u64)) -> Result<(), String> {
    const ICD: &str = "vulkan/lvp_icd.json";
    let Some(dir) = exe.parent() else { return Ok(()) };
    if dir.join(ICD).is_file() {
        return Ok(());
    }
    let here = running.and_then(|r| r.parent()).filter(|r| r.join(ICD).is_file() && *r != dir);
    if here.is_none() && url.is_none() {
        return Ok(());
    }
    let new = dir.join(".vulkan.new");
    let _ = std::fs::remove_dir_all(&new);
    let made = async {
        let e = |what: String, err: std::io::Error| format!("can't {what}: {err}");
        std::fs::create_dir_all(new.join("vulkan")).map_err(|x| e(format!("make {}", new.display()), x))?;
        if let Some(from) = here {
            for f in std::fs::read_dir(from.join("vulkan")).map_err(|x| e(format!("read {}", from.display()), x))?.flatten() {
                if f.path().is_file() {
                    std::fs::copy(f.path(), new.join("vulkan").join(f.file_name())).map_err(|x| e(format!("copy {}", f.path().display()), x))?;
                }
            }
        } else if let Some(url) = url {
            let tgz = new.join("vulkan.tar.gz");
            verified(&[url.to_string(), files[1].clone(), files[2].clone()], &tgz, key, shown).await?;
            let out = std::process::Command::new("tar").arg("-xzf").arg(&tgz).arg("-C").arg(&new).output().map_err(|x| e("run tar".into(), x))?;
            if !out.status.success() {
                return Err(format!("tar couldn't unpack it: {}", String::from_utf8_lossy(&out.stderr).trim()));
            }
        }
        if !new.join(ICD).is_file() {
            return Err("it has no lvp_icd.json".into());
        }
        std::fs::rename(new.join("vulkan"), dir.join("vulkan")).map_err(|x| e("put vulkan/ in place".into(), x))
    }
    .await;
    let _ = std::fs::remove_dir_all(&new);
    made
}

/// B710: `vulkan/` for the game at `exe` from its own release (`api`, GitHub's JSON for this build's tag) when it has
/// none, as [`vulkan`] puts it beside a game it updates: a game updated by an updater from before B710 got only the
/// bare binary. Used from the next start (the Vulkan loader reads its drivers when the renderer starts). Nothing to do
/// on a platform without the tarballs, with one there, or for a release from before B710 (no `-vulkan.tar.gz`).
#[cfg(not(target_arch = "wasm32"))]
async fn mend(api: &str, asset: Option<&str>, exe: &std::path::Path, key: &str) -> Result<(), String> {
    let Some(a) = asset.filter(|a| a.starts_with("opentennis-linux-")) else { return Ok(()) };
    if exe.parent().is_none_or(|d| d.join("vulkan/lvp_icd.json").is_file()) {
        return Ok(());
    }
    let j = json(api).await?;
    let url = |name: &str| {
        j["assets"].as_array()?.iter().find(|x| x["name"].as_str() == Some(name))?["browser_download_url"].as_str().map(str::to_string)
    };
    let (Some(tgz), Some(sums), Some(sig)) = (url(&format!("{a}-vulkan.tar.gz")), url("SHA256SUMS"), url("SHA256SUMS.sig")) else { return Ok(()) };
    vulkan(exe, None, Some(&tgz), &[String::new(), sums, sig], key, &|_, _| {}).await?;
    bevy::log::info!("update: software renderer added beside {}", exe.display());
    Ok(())
}

/// `new` in `exe`'s place. Linux: made executable and renamed over it (the running game keeps its open copy).
/// Windows (`windows`), where a running .exe can't be replaced but can be renamed: `exe` → `exe.old`, `new` → `exe`
/// (put back if that fails). No `exe` yet (a new [`Way::Copy`]): `new` is renamed.
#[cfg(not(target_arch = "wasm32"))]
fn swap(new: &std::path::Path, exe: &std::path::Path, windows: bool) -> Result<(), String> {
    let e = |what: &str, err: std::io::Error| format!("can't {what}: {err}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(new, std::fs::Permissions::from_mode(0o755)).map_err(|x| e("make it executable", x))?;
    }
    if windows && exe.exists() {
        let old = old(exe);
        let _ = std::fs::remove_file(&old);
        std::fs::rename(exe, &old).map_err(|x| e("move the old game aside", x))?;
        if let Err(x) = std::fs::rename(new, exe) {
            let _ = std::fs::rename(&old, exe);
            return Err(e("put the new game in place", x));
        }
        Ok(())
    } else {
        std::fs::rename(new, exe).map_err(|x| e("put the new game in place", x))
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A throwaway minisign key (made for this test, `minisign -G -W`; its secret is gone) and what it signed: the
    /// sums of a "new build" (`new build\n`), and sums with a wrong hash.
    const KEY: &str = "RWQ0kP1JHGoSYVPgnwfOLD6H9XcVIKbCA5aEvP+d/2LBcSCrIyQt8NAf";
    const BIN: &[u8] = b"new build\n";
    const SUMS: &str = "288765569ab03c0fbf107b0f870fb74c42fff3d4b295613911a955cf8e967492  opentennis-linux-x86_64\n";
    const SIG: &str = "untrusted comment: signature from minisign secret key
RUQ0kP1JHGoSYTAkgxWHTHHvP2pCY4h1ctSsWHKIjTxiY/z+oi8ZdJw/+Ic+bgMEt9JkFLTpaCISrwZqFiR08q2S1qJ9TjsC5wk=
trusted comment: test
TSJCWvsq7fomptfEpCRTH7MHbYfPh5RL3EmWX1QTPjCy7RS865E20JP3m5b2ihq7G1xZIzLnRRxEcuEEPA4vCA==
";
    const BAD_SUMS: &str = "0000000000000000000000000000000000000000000000000000000000000000  opentennis-linux-x86_64\n";
    const BAD_SUMS_SIG: &str = "untrusted comment: signature from minisign secret key
RUQ0kP1JHGoSYUkaD1BQLExdG3UnXb09a7FadICs8hLXYf+CcHtNbhcCJjPMy5O25Txf3pnOSfMxNHwZrKRw3/Ke1KrOiMME6gU=
trusted comment: test
PGdwmy/sa9Kqh1f1dYqI+JP0Iweg2dLM/mEgnRNTVfsQ9pI1u5aeusKILuRdn61QcyAsnzeOSfS5RRAENygxBg==
";

    /// An HTTP server on localhost answering GETs from `files` (path → body), 404 otherwise. Its base URL.
    fn serve(files: HashMap<String, Vec<u8>>) -> String {
        use std::io::{BufRead, BufReader, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        std::thread::spawn(move || {
            for mut c in l.incoming().flatten() {
                let mut line = String::new();
                let mut r = BufReader::new(c.try_clone().unwrap());
                let _ = r.read_line(&mut line);
                loop {
                    let mut h = String::new();
                    if r.read_line(&mut h).map_or(true, |n| n == 0) || h.trim().is_empty() {
                        break;
                    }
                }
                let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
                let (status, body) = files.get(&path).map_or(("404 Not Found", &b""[..]), |b| ("200 OK", b));
                let _ = write!(c, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = c.write_all(body);
            }
        });
        base
    }

    /// A release `tag` at a local server whose sums and signature are `sums`/`sig`: the API URL.
    fn release(tag: &str, sums: &str, sig: &str) -> String {
        // the JSON names the files' server, so it comes from a second one
        let files: HashMap<String, Vec<u8>> =
            [("/opentennis-linux-x86_64", BIN.to_vec()), ("/SHA256SUMS", sums.as_bytes().to_vec()), ("/SHA256SUMS.sig", sig.as_bytes().to_vec())].into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        let base = serve(files);
        let asset = |n: &str| serde_json::json!({ "name": n, "browser_download_url": format!("{base}/{n}") });
        let j = serde_json::json!({
            "tag_name": tag, "draft": false, "html_url": format!("{base}/page"),
            "assets": [asset("opentennis-linux-x86_64"), asset("SHA256SUMS"), asset("SHA256SUMS.sig"), asset("opentennis-linux-x86_64.tar.gz")],
        });
        let api = serve([("/latest".to_string(), j.to_string().into_bytes())].into());
        format!("{api}/latest")
    }

    /// The check → verify → swap path against a local server: a newer release replaces the game file (and on the
    /// Windows path leaves the old one as `.old`); a bad signature or checksum keeps the old file and no `opentennis.new`;
    /// the same or an older version isn't offered.
    #[test]
    fn newer_release_swaps_and_bad_ones_keep_the_old_game() {
        let rt = crate::runtime();
        let dir = std::env::temp_dir().join(format!("hst-update-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let game = |name: &str| {
            let p = dir.join(name);
            std::fs::write(&p, b"old build\n").unwrap();
            p
        };
        let quiet = |_: u64, _: u64| {};
        let asset = Some("opentennis-linux-x86_64");

        // same version: nothing
        let api = release("v0.1.0-beta.9", SUMS, SIG);
        assert_eq!(rt.block_on(check(&api, "v0.1.0-beta.9", asset)).unwrap(), None);
        assert_eq!(rt.block_on(check(&api, "v0.1.0-beta.10", asset)).unwrap(), None);

        // newer: offered with its files, installs, swaps
        let f = rt.block_on(check(&api, "v0.1.0-beta.8", asset)).unwrap().expect("offered");
        assert_eq!(f.tag, "v0.1.0-beta.9");
        let files = f.files.expect("files");
        let exe = game("opentennis");
        rt.block_on(install(&files, &exe, KEY, &quiet)).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), BIN);
        assert!(!dir.join("opentennis.new").exists());
        #[cfg(unix)]
        assert_eq!(std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(&exe).unwrap().permissions()) & 0o111, 0o111);

        // Windows' renames: opentennis.exe → opentennis.exe.old, opentennis.new → opentennis.exe
        let win = game("opentennis.exe");
        std::fs::write(dir.join("opentennis.new"), BIN).unwrap();
        swap(&dir.join("opentennis.new"), &win, true).unwrap();
        assert_eq!(std::fs::read(&win).unwrap(), BIN);
        assert_eq!(std::fs::read(old(&win)).unwrap(), b"old build\n");

        // a signature by another key, sums that don't match their signature, a wrong checksum: old game kept
        for (api, key) in [
            (release("v0.1.0-beta.9", SUMS, SIG), PUBLIC_KEY),
            (release("v0.1.0-beta.9", BAD_SUMS, SIG), KEY),
            (release("v0.1.0-beta.9", BAD_SUMS, BAD_SUMS_SIG), KEY),
        ] {
            let f = rt.block_on(check(&api, "v0.1.0-beta.8", asset)).unwrap().unwrap();
            let exe = game("opentennis");
            let e = rt.block_on(install(&f.files.unwrap(), &exe, key, &quiet)).unwrap_err();
            assert!(e.contains("signature") || e.contains("checksum"), "{e}");
            assert_eq!(std::fs::read(&exe).unwrap(), b"old build\n");
            assert!(!dir.join("opentennis.new").exists());
        }

        // no binary for this platform: offered (the page), nothing to install
        let f = rt.block_on(check(&api, "v0.1.0-beta.8", Some("hst-plan9"))).unwrap().unwrap();
        assert!(f.files.is_none() && f.page.ends_with("/page"));
        // no server: an error, which the menu never shows
        assert!(rt.block_on(check("http://127.0.0.1:9/latest", "v0.0.1", asset)).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn versions_in_semver_order() {
        let order = ["v0.1.0-alpha.1", "v0.1.0-alpha.2", "v0.1.0-alpha.10", "v0.1.0-beta", "v0.1.0", "v0.1.1", "v0.2.0", "v1.0.0-alpha.1", "v1.0.0", "v1.10.0"];
        for (i, a) in order.iter().enumerate() {
            for (j, b) in order.iter().enumerate() {
                assert_eq!(newer(a, b), i > j, "{a} vs {b}");
            }
        }
        assert!(!newer("beta", "v0.1.0") && !newer("v0.1.0", "beta") && !newer("v1.2", "v1.0.0"));
        assert!(newer("1.0.0", "v0.9.9"));
    }

    /// B567: a check is due half an hour after the last by the wall clock, however long a phone kept the app
    /// suspended in between; a clock set back counts as due.
    #[test]
    fn checks_again_half_an_hour_later_by_the_wall_clock() {
        let t = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_800_000_000);
        let min = |m: u64| std::time::Duration::from_secs(60 * m);
        assert!(!due(t, t) && !due(t, t + min(29)));
        assert!(due(t, t + min(30)) && due(t, t + min(7 * 60)));
        assert!(due(t, t - min(1)));
    }

    /// M140: the launcher hands over to the newest copy newer than the game (one with its binary), and clears out the
    /// others but the running one; a new copy installs into its empty folder on both platforms' renames.
    #[test]
    fn copies_launch_the_newest_and_install_into_a_fresh_folder() {
        let root = std::env::temp_dir().join(format!("hst-update-copies-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (tag, bin) in [("v1.0.0", true), ("v1.2.0", true), ("v1.10.0-beta.1", true), ("v1.11.0", false), ("v0.9.0", true), ("notes", true)] {
            std::fs::create_dir_all(root.join(tag)).unwrap();
            if bin {
                std::fs::write(root.join(tag).join("opentennis"), b"x").unwrap();
            }
        }
        let (tag, bin) = newest(&root, "v1.1.0", "opentennis").unwrap();
        assert_eq!((tag.as_str(), bin), ("v1.10.0-beta.1", root.join("v1.10.0-beta.1/opentennis")));
        assert_eq!(newest(&root, "v1.10.0", "opentennis"), None);
        assert_eq!(newest(&root.join("none"), "v1.0.0", "opentennis"), None);
        let mut gone = stale(&root, Some("v1.10.0-beta.1"), Some(&root.join("v1.0.0/opentennis")));
        gone.sort();
        assert_eq!(gone, ["v0.9.0", "v1.11.0", "v1.2.0"].map(|t| root.join(t)));
        assert_eq!(stale(&root, None, None).len(), 5);

        // the download into a folder with no game yet (`Way::Copy`): the Windows renames too
        let rt = crate::runtime();
        let api = release("v1.12.0", SUMS, SIG);
        let files = rt.block_on(check(&api, "v1.0.0", Some("opentennis-linux-x86_64"))).unwrap().unwrap().files.unwrap();
        for (k, name) in ["opentennis", "opentennis.exe"].into_iter().enumerate() {
            let dir = root.join(format!("fresh{k}"));
            std::fs::create_dir_all(&dir).unwrap();
            let new = dir.join("opentennis.new");
            rt.block_on(verified(&files, &new, KEY, &|_, _| {})).unwrap();
            swap(&new, &dir.join(name), k == 1).unwrap();
            assert_eq!(std::fs::read(dir.join(name)).unwrap(), BIN);
            assert!(!new.exists() && !old(&dir.join(name)).exists());
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// M140: a Steam library's folder is Steam's; a Deck's non-Steam game elsewhere isn't.
    #[test]
    fn steam_owns_only_its_library() {
        assert!(steams(std::path::Path::new("/home/deck/.local/share/Steam/steamapps/common/OpenTennis/opentennis")));
        assert!(steams(std::path::Path::new("D:/SteamLibrary/SteamApps/common/OpenTennis/opentennis.exe")));
        assert!(!steams(std::path::Path::new("/home/deck/Games/opentennis/opentennis")));
    }

    /// M140: the settings word.
    #[test]
    fn the_updates_word() {
        assert_eq!([settings_line(true, false), settings_line(true, true), settings_line(false, true)], ["updates = ask\n", "updates = always\n", "updates = never\n"]);
    }

    /// M159a: the installer is told by its own marks only, and each link opens it on the new release.
    #[test]
    fn the_installer_from_its_marks() {
        let v = |s: &[&str]| s.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(installer(&v(&[]), false), None);
        // our own schemes, or a bare prefix, say nothing
        assert_eq!(installer(&v(&["opentennis", "altstore-", "altstore"]), false), None);
        assert_eq!(installer(&v(&["altstore-dev.opentennis.app"]), false), Some(Installer::AltStore));
        assert_eq!(installer(&v(&["altstore-dev.opentennis.app", "sidestore-dev.opentennis.app"]), false), Some(Installer::SideStore));
        assert_eq!(installer(&v(&["sidestore-dev.opentennis.app"]), true), Some(Installer::TrollStore));
        let ipa = "https://github.com/FIZL0/opentennis/releases/download/v1.0.0/opentennis-ios.ipa";
        assert_eq!(Installer::AltStore.link(ipa), "altstore://source?url=https%3A%2F%2Ffizl0.github.io%2Fapps%2Fsource.json");
        assert_eq!(Installer::SideStore.link(ipa), "sidestore://source?url=https%3A%2F%2Ffizl0.github.io%2Fapps%2Fsource.json");
        assert_eq!(
            Installer::TrollStore.link(ipa),
            "apple-magnifier://install?url=https%3A%2F%2Fgithub.com%2FFIZL0%2Fopentennis%2Freleases%2Fdownload%2Fv1.0.0%2Fopentennis-ios.ipa"
        );
    }

    #[test]
    fn the_beta_commit_is_a_full_hash() {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(beta_commit(format!("{sha}\n").as_bytes()).as_deref(), Ok(sha));
        assert!(beta_commit(b"0123456\n").is_err());
        assert!(beta_commit(b"<html>404</html>").is_err());
    }

    #[test]
    fn a_found_release_needs_a_v_tag_and_all_its_files() {
        let j = serde_json::json!({ "tag_name": "beta", "assets": [] });
        assert_eq!(found(&j, "v0.0.1", ASSET), None);
        let j = serde_json::json!({ "tag_name": "v2.0.0", "html_url": "p", "assets": [{ "name": "SHA256SUMS", "browser_download_url": "s" }] });
        assert_eq!(found(&j, "v1.0.0", Some("opentennis-linux-x86_64")), Some(Found { tag: "v2.0.0".into(), page: "p".into(), files: None, vulkan: None }));
    }

    /// B710: a release's software renderer (`<binary>-vulkan.tar.gz`, a stand-in `vulkan/` here) next to its binary,
    /// signed by a second throwaway key (`minisign -G -W`, secret gone) with both files' sums.
    const VK_KEY: &str = "RWSm/sV746l5mHOL2Kwpbsey/Y1Ep7Q50ToEmdTSG3eSUmDbqDSAwfuO";
    const VK_TGZ: &[u8] = &[31, 139, 8, 0, 0, 0, 0, 0, 2, 3, 237, 213, 207, 10, 130, 64, 16, 199, 241, 61, 247, 20, 178, 15, 96, 254, 219, 2, 175, 117, 233, 41, 196, 50, 200, 18, 149, 213, 22, 34, 124, 247, 36, 233, 162, 96, 151, 148, 162, 239, 231, 50, 3, 187, 183, 31, 51, 99, 174, 217, 37, 206, 151, 98, 74, 78, 107, 173, 212, 179, 182, 250, 117, 216, 187, 174, 227, 42, 97, 41, 49, 131, 107, 85, 199, 218, 178, 132, 46, 138, 122, 236, 223, 187, 247, 31, 101, 186, 252, 179, 116, 223, 117, 81, 102, 74, 187, 42, 62, 158, 255, 42, 8, 70, 242, 95, 247, 242, 15, 60, 175, 205, 223, 33, 255, 201, 37, 58, 53, 71, 189, 16, 248, 79, 175, 249, 55, 101, 148, 30, 18, 251, 92, 21, 249, 36, 251, 127, 116, 254, 213, 170, 55, 255, 190, 239, 187, 204, 255, 28, 238, 114, 183, 217, 202, 240, 46, 219, 11, 160, 99, 125, 139, 202, 184, 62, 201, 80, 218, 131, 147, 32, 155, 134, 53, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 192, 87, 122, 0, 223, 225, 118, 248, 0, 40, 0, 0];
    const VK_SUMS: &str = "288765569ab03c0fbf107b0f870fb74c42fff3d4b295613911a955cf8e967492  opentennis-linux-x86_64\n8eb186fe2c9c2d53cf640624029a3acd4fed4570e7a5151803b521d5deddbe40  opentennis-linux-x86_64-vulkan.tar.gz\n";
    const VK_SIG: &str = "untrusted comment: signature from minisign secret key
RUSm/sV746l5mLwKVP/7yzt7JZfAJh4xujz0dI7WNj1dMZdDV3Rze/tTlKb48/Xxv/VtciCw1vWIoUrPm1VRWSdS/JhM2FwPawE=
trusted comment: test
YzKgWNACEJeYrYmarV4RlDlUpsWcWUOvhsRtxHZGvmrUjFv/AcnSOg/hmIv4u4B0RnYH9Mk92mW/ioSbwIS9DA==
";

    /// B710: an update that brings only the bare binary also puts `vulkan/` beside it: from the release's tarball
    /// (checked against the signed sums, a bad sum leaves none), from the running game's folder when that has one
    /// (a [`Way::Copy`] of a tarball install), and leaves one that's there alone; a release without it is fine.
    #[test]
    fn updates_bring_the_software_renderer() {
        let rt = crate::runtime();
        let quiet = |_: u64, _: u64| {};
        let root = std::env::temp_dir().join(format!("hst-update-vulkan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let files: HashMap<String, Vec<u8>> = [
            ("/opentennis-linux-x86_64", BIN.to_vec()),
            ("/opentennis-linux-x86_64-vulkan.tar.gz", VK_TGZ.to_vec()),
            ("/SHA256SUMS", VK_SUMS.as_bytes().to_vec()),
            ("/SHA256SUMS.sig", VK_SIG.as_bytes().to_vec()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        let base = serve(files);
        let asset = |n: &str| serde_json::json!({ "name": n, "browser_download_url": format!("{base}/{n}") });
        let names = ["opentennis-linux-x86_64", "opentennis-linux-x86_64-vulkan.tar.gz", "SHA256SUMS", "SHA256SUMS.sig"];
        let j = serde_json::json!({ "tag_name": "v2.0.0", "assets": names.map(asset) });
        let f = found(&j, "v1.0.0", Some("opentennis-linux-x86_64")).unwrap();
        assert_eq!(f.vulkan.as_deref(), Some(&*format!("{base}/opentennis-linux-x86_64-vulkan.tar.gz")));
        assert_eq!(found(&j, "v1.0.0", Some("opentennis-windows-x86_64.exe")).unwrap().vulkan, None);
        let fs = f.files.clone().unwrap();
        let icd = |d: &std::path::Path| d.join("vulkan/lvp_icd.json");

        // a bare-binary install: the binary, then vulkan/ from the tarball
        let a = root.join("a");
        std::fs::create_dir_all(&a).unwrap();
        let exe = a.join("opentennis");
        rt.block_on(install(&fs, &exe, VK_KEY, &quiet)).unwrap();
        rt.block_on(vulkan(&exe, Some(&exe), f.vulkan.as_deref(), &fs, VK_KEY, &quiet)).unwrap();
        assert_eq!(std::fs::read(a.join("vulkan/libvulkan_lvp.so")).unwrap(), b"driver\n");
        assert!(icd(&a).is_file() && !a.join(".vulkan.new").exists());

        // a wrong key: no vulkan/, nothing half-made
        let b = root.join("b");
        std::fs::create_dir_all(&b).unwrap();
        assert!(rt.block_on(vulkan(&b.join("opentennis"), None, f.vulkan.as_deref(), &fs, KEY, &quiet)).is_err());
        assert!(!b.join("vulkan").exists() && !b.join(".vulkan.new").exists());

        // a copy into the update folder takes the running game's vulkan/ without a download (no URL to get it from)
        let c = root.join("update/v2.0.0");
        std::fs::create_dir_all(&c).unwrap();
        rt.block_on(vulkan(&c.join("opentennis"), Some(&exe), None, &fs, VK_KEY, &quiet)).unwrap();
        assert_eq!(std::fs::read(icd(&c)).unwrap(), std::fs::read(icd(&a)).unwrap());

        // one that's there stays; a release without the tarball and no copy to take: nothing, no error
        std::fs::write(icd(&a), b"mine").unwrap();
        rt.block_on(vulkan(&exe, Some(&exe), f.vulkan.as_deref(), &fs, VK_KEY, &quiet)).unwrap();
        assert_eq!(std::fs::read(icd(&a)).unwrap(), b"mine");
        rt.block_on(vulkan(&b.join("opentennis"), None, None, &fs, VK_KEY, &quiet)).unwrap();
        assert!(!b.join("vulkan").exists());

        // a game an older updater left bare mends itself from its own release's JSON; not on Windows
        let api = format!("{}/tag", serve([("/tag".to_string(), j.to_string().into_bytes())].into()));
        rt.block_on(mend(&api, Some("opentennis-windows-x86_64.exe"), &b.join("opentennis"), VK_KEY)).unwrap();
        assert!(!b.join("vulkan").exists());
        rt.block_on(mend(&api, Some("opentennis-linux-x86_64"), &b.join("opentennis"), VK_KEY)).unwrap();
        assert!(icd(&b).is_file());
        let _ = std::fs::remove_dir_all(&root);
    }
}
