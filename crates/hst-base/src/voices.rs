//! Settings → Sound → Voice Language: the players', the umpire's and the gallery's voices from another release of the
//! game (the European disc's English, French, Italian, German, Spanish and Japanese sets; the Japanese disc's
//! Japanese), heard in place of the main disc's. Purely what this machine hears: the match still times every take by
//! the main disc's bank (`Sound::centre_frames`, `Sound::take_ticks` play the main bank, never the dub), the sim and
//! its random numbers never see the choice, and online each player hears their own.
//!
//! A player adds a release by putting its ISO in `voices/` beside the settings (desktop, phones' Files folder): the
//! next start copies its voice banks into `voices/<disc id>.pack` (about 25 MB a language) and the ISO can go; or
//! Settings → Gameplay → Add Disc picks the ISO and reads it in at once ([`add_disc`], M165). Each
//! release's archives hold the same banks under the same names as the US disc's (the European one once per language,
//! told apart by the archive's last letter; the Japanese one keeps its player voices inside the motion archives), so a
//! bank is looked up by its name. A bank whose sequences don't line up with the main disc's (a different take count
//! or order) stays the main disc's, so a cue never plays the wrong line.
//!
//! Character and umpire mods follow the same choice with their own folders per language (`mod_voices`, M168).

use crate::audio::SoundBank;
use hst_data::iso::Iso;
use hst_data::snd::Bank;
use hst_data::xb::Archive;
use std::collections::HashMap;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

/// A voice language: its settings.txt word and its Settings row's name. The first is the main disc's own voices.
pub struct Lang {
    pub code: &'static str,
    pub name: &'static str,
}

/// Every language a release can bring, by index (settings, packs).
pub const LANGS: [Lang; 7] = [
    Lang { code: "disc", name: "English" },
    Lang { code: "en-eu", name: "English (Europe)" },
    Lang { code: "fr", name: "French" },
    Lang { code: "it", name: "Italian" },
    Lang { code: "de", name: "German" },
    Lang { code: "es", name: "Spanish" },
    Lang { code: "ja", name: "Japanese" },
];

/// The releases a voice pack can come from: the disc ID `SYSTEM.CNF` boots, and the language of each of its archives.
/// Europe (SCES-54535): one copy of each sound archive per language, its extension's last letter saying which
/// (`.XB0` English, `1` French, `2` Italian, `3` German, `4` Spanish, `B` Japanese; heard and checked one by one, the
/// Japanese set byte for byte the Japanese disc's). Japan (SCPS-15113): Japanese only.
fn release(boot: &str) -> Option<fn(&str) -> Option<u8>> {
    fn europe(archive: &str) -> Option<u8> {
        let (_, ext) = archive.rsplit_once('.')?;
        match ext {
            "XB0" => Some(1),
            "XB1" => Some(2),
            "XB2" => Some(3),
            "XB3" => Some(4),
            "XB4" => Some(5),
            "XBB" => Some(6),
            _ => None,
        }
    }
    fn japan(archive: &str) -> Option<u8> {
        archive.ends_with(".XB").then_some(6)
    }
    match boot {
        "SCES_545.35" => Some(europe),
        "SCPS_151.13" => Some(japan),
        _ => None,
    }
}

/// The disc ID a disc boots (`SCUS_976.10`), from its `SYSTEM.CNF`.
pub fn disc_id(iso: &mut Iso) -> Option<String> {
    let cnf = iso.read("SYSTEM.CNF").ok()?;
    let text = String::from_utf8_lossy(&cnf);
    let line = text.lines().find(|l| l.trim_start().starts_with("BOOT2"))?;
    let name = line.rsplit(['\\', ':']).next()?.trim();
    Some(name.split(';').next()?.to_string())
}

/// A bank name as the lookup knows it: lower case, `/`, from `sound/` on (`data/sound/VOICE/SGL/sgv_vc00a.hd` and an
/// archive's `..\data\sound\VOICE\SGL\sgv_vc00a.hd` are both `sound/voice/sgl/sgv_vc00a.hd`).
pub fn key(name: &str) -> String {
    let n = name.replace('\\', "/").to_ascii_lowercase();
    n.find("sound/").map_or(n.clone(), |i| n[i..].to_string())
}

/// The banks a dub covers: the players' (singles and doubles), the umpires' and the galleries'.
fn voiced(key: &str) -> bool {
    key.ends_with(".hd") && ["sound/voice/sgl/", "sound/voice/dbl/", "sound/umpire/", "sound/voice/gallery/"].iter().any(|d| key.starts_with(d))
}

/// The chosen language (a [`LANGS`] index; 0 the main disc's voices).
static CHOSEN: AtomicU8 = AtomicU8::new(0);

/// The language every voice bank is heard in from now on, mid-match too ([`Dub`]).
pub fn choose(lang: u8) {
    CHOSEN.store(lang.min(LANGS.len() as u8 - 1), Ordering::Relaxed);
}

/// The chosen language.
pub fn chosen() -> u8 {
    CHOSEN.load(Ordering::Relaxed)
}

/// The settings.txt line's word for language `lang`.
pub fn setting_line(lang: u8) -> String {
    format!("voice_language = {}\n", LANGS[lang as usize].code)
}

/// The language a settings.txt names (`voice_language = fr`), else the main disc's.
pub fn parse_setting(text: &str) -> u8 {
    text.lines()
        .rev()
        .filter_map(|l| l.split('#').next()?.split_once('='))
        .filter(|(k, _)| k.trim() == "voice_language")
        .find_map(|(_, v)| LANGS.iter().position(|l| l.code == v.trim()))
        .unwrap_or(0) as u8
}

/// Where the packs (and the ISOs waiting to be read) are: `voices/` beside the settings.
static DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// The packs live in `dir` from now on (the game's start: `voices/` in `store::dir`).
pub fn set_dir(dir: PathBuf) {
    *DIR.lock().unwrap() = Some(dir);
    *PACKS.lock().unwrap() = None;
}

fn dir() -> Option<PathBuf> {
    DIR.lock().unwrap().clone()
}

/// One bank in a pack: where its `.hd` and `.bd` are.
#[derive(Clone, Copy)]
struct At {
    hd: (u64, u32),
    bd: (u64, u32),
}

/// A pack's banks by (language, [`key`]).
struct Pack {
    path: PathBuf,
    banks: HashMap<(u8, String), At>,
}

/// The packs in [`dir`], read on first use (`None`: not read yet).
static PACKS: Mutex<Option<Arc<Vec<Pack>>>> = Mutex::new(None);

/// Pack layout: the banks' bytes, then the index (u32 count; per bank u8 language, u16 name length, name, u64 and u32
/// offset and size of the `.hd`, the same of the `.bd`), then the index's offset (u64) and [`MAGIC`].
const MAGIC: &[u8; 8] = b"HSTVOX01";

fn packs() -> Arc<Vec<Pack>> {
    let mut p = PACKS.lock().unwrap();
    p.get_or_insert_with(|| {
        let files = dir().and_then(|d| std::fs::read_dir(d).ok()).into_iter().flatten().flatten().map(|e| e.path());
        let mut files: Vec<_> = files.filter(|f| f.extension().is_some_and(|x| x == "pack")).collect();
        files.sort();
        Arc::new(files.into_iter().filter_map(|f| read_index(&f).map_err(|e| eprintln!("voices: {}: {e}", f.display())).ok()).collect())
    })
    .clone()
}

fn read_index(path: &Path) -> io::Result<Pack> {
    let bad = || io::Error::new(io::ErrorKind::InvalidData, "not a voice pack");
    let mut f = std::fs::File::open(path)?;
    let len = f.seek(SeekFrom::End(0))?;
    if len < 16 {
        return Err(bad());
    }
    f.seek(SeekFrom::End(-16))?;
    let mut tail = [0; 16];
    f.read_exact(&mut tail)?;
    if &tail[8..] != MAGIC {
        return Err(bad());
    }
    let at = u64::from_le_bytes(tail[..8].try_into().unwrap());
    if at > len - 16 {
        return Err(bad());
    }
    f.seek(SeekFrom::Start(at))?;
    let mut index = vec![0; (len - 16 - at) as usize];
    f.read_exact(&mut index)?;
    let mut r = &index[..];
    let mut take = |n: usize| -> io::Result<&[u8]> {
        let (a, b) = r.split_at_checked(n).ok_or_else(bad)?;
        r = b;
        Ok(a)
    };
    let count = u32::from_le_bytes(take(4)?.try_into().unwrap());
    let mut banks = HashMap::new();
    for _ in 0..count {
        let lang = take(1)?[0];
        let n = u16::from_le_bytes(take(2)?.try_into().unwrap()) as usize;
        let name = String::from_utf8_lossy(take(n)?).into_owned();
        let mut span = || -> io::Result<(u64, u32)> { Ok((u64::from_le_bytes(take(8)?.try_into().unwrap()), u32::from_le_bytes(take(4)?.try_into().unwrap()))) };
        let (hd, bd) = (span()?, span()?);
        if (lang as usize) < LANGS.len() && hd.0 + hd.1 as u64 <= at && bd.0 + bd.1 as u64 <= at {
            banks.insert((lang, name), At { hd, bd });
        }
    }
    Ok(Pack { path: path.to_path_buf(), banks })
}

/// The languages the packs bring, the main disc's first (the Settings row's choices).
pub fn available() -> Vec<u8> {
    let packs = packs();
    (0..LANGS.len() as u8).filter(|&l| l == 0 || packs.iter().any(|p| p.banks.keys().any(|k| k.0 == l))).collect()
}

/// Bank `key` of language `lang` from the packs: its `.hd` and `.bd`.
fn find(lang: u8, key: &str) -> Option<(Vec<u8>, Vec<u8>)> {
    let packs = packs();
    let (pack, at) = packs.iter().find_map(|p| p.banks.get(&(lang, key.to_string())).map(|at| (p, *at)))?;
    let mut f = std::fs::File::open(&pack.path).ok()?;
    let mut read = |(off, n): (u64, u32)| -> Option<Vec<u8>> {
        f.seek(SeekFrom::Start(off)).ok()?;
        let mut v = vec![0; n as usize];
        f.read_exact(&mut v).ok()?;
        Some(v)
    };
    Some((read(at.hd)?, read(at.bd)?))
}

/// Whether two banks play the same cues: the same (program, key) sequences there and none more.
fn same_cues(a: &[u8], b: &[u8]) -> bool {
    let (Ok(a), Ok(b)) = (Bank::parse(a), Bank::parse(b)) else { return false };
    (0..128).all(|p| (0..128).all(|k| a.sequence(p, k).is_some() == b.sequence(p, k).is_some()))
}

/// `bank` (main disc bank `hd`, a [`voiced`] one) with the chosen language's take of it as its `language`, where a
/// pack has it and it plays the same cues; else as it was.
pub fn dub(bank: SoundBank, hd: &str) -> SoundBank {
    dub_in(bank, hd, chosen())
}

/// [`dub`] in language `lang`.
/// [`dub`] in language `lang`: that one's take is read now, the others the first time they're heard.
pub fn dub_in(mut bank: SoundBank, hd: &str, lang: u8) -> SoundBank {
    let k = key(hd);
    if voiced(&k) {
        bank.language.key = k;
        bank.language.take(&bank.hd, lang);
    }
    bank
}

/// A main-disc voice bank's takes in the other languages (`SoundBank::language`), each read from the packs the first
/// time it's asked for, so a Voice Language change is heard at once, mid-match too.
#[derive(Default)]
pub struct Dub {
    /// The bank's [`key`]; empty on a bank with no other languages.
    key: String,
    takes: [std::sync::OnceLock<Option<Arc<SoundBank>>>; LANGS.len()],
}

impl Dub {
    /// Language `lang`'s take of the bank whose header is `hd`, where a pack has it and it plays the same cues.
    pub fn take(&self, hd: &[u8], lang: u8) -> Option<&Arc<SoundBank>> {
        if lang == 0 || self.key.is_empty() {
            return None;
        }
        self.takes[lang as usize]
            .get_or_init(|| {
                let (lhd, lbd) = find(lang, &self.key).filter(|(lhd, _)| same_cues(hd, lhd))?;
                bevy::log::info!("voices: {} in {}", self.key, LANGS[lang as usize].name);
                Some(Arc::new(SoundBank { hd: lhd, bd: lbd, wav: Default::default(), takes: Default::default(), stream: None, language: Default::default() }))
            })
            .as_ref()
    }
}

/// Reads the voice banks off release `iso` into `out` (a [`MAGIC`] pack); the languages it brought. A disc this
/// doesn't know (the US one included: its voices are the main disc's) is refused.
pub fn import(iso: &mut Iso, out: &Path) -> io::Result<Vec<u8>> {
    import_with(iso, out, &mut |_| {})
}

/// [`import`], telling `progress` how far it is (0..=1, by the archives' bytes read) as it goes.
pub fn import_with(iso: &mut Iso, out: &Path, progress: &mut dyn FnMut(f32)) -> io::Result<Vec<u8>> {
    let id = disc_id(iso).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no SYSTEM.CNF: not a PS2 disc"))?;
    let lang_of = release(&id).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("{id} brings no other voices (Everybody's Tennis Europe SCES-54535 or Minna no Tennis SCPS-15113 do)")))?;
    let archives: Vec<(String, u8)> = iso.files.keys().filter(|p| p.starts_with("SND/") || p.starts_with("PCANI/")).filter_map(|p| Some((p.clone(), lang_of(p)?))).collect();
    let tmp = out.with_extension("part");
    let mut w = BufWriter::new(std::fs::File::create(&tmp)?);
    let mut index: Vec<(u8, String, (u64, u32), (u64, u32))> = Vec::new();
    let mut at = 0u64;
    let mut put = |w: &mut BufWriter<std::fs::File>, d: &[u8]| -> io::Result<(u64, u32)> {
        w.write_all(d)?;
        let span = (at, d.len() as u32);
        at += d.len() as u64;
        Ok(span)
    };
    let total: u64 = archives.iter().map(|(p, _)| iso.files.get(p).map_or(0, |f| f.1 as u64)).sum();
    let mut read = 0u64;
    for (path, lang) in archives {
        progress(read as f32 / total.max(1) as f32);
        read += iso.files.get(&path).map_or(0, |f| f.1 as u64);
        let data = iso.read(&path)?;
        let Ok(arc) = Archive::parse(&data) else { continue };
        for e in &arc.entries {
            let k = key(&e.name);
            if !voiced(&k) || index.iter().any(|b| b.0 == lang && b.1 == k) {
                continue;
            }
            let bd_name = e.name.replace(".hd", ".bd");
            let (Some(bd), Ok(hd)) = (arc.entries.iter().find(|b| b.name == bd_name), arc.read(e)) else { continue };
            let Ok(bd) = arc.read(bd) else { continue };
            let (h, b) = (put(&mut w, &hd)?, put(&mut w, &bd)?);
            index.push((lang, k, h, b));
        }
    }
    if index.is_empty() {
        let _ = std::fs::remove_file(&tmp);
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("{id}: no voice banks found")));
    }
    let index_at = at;
    w.write_all(&(index.len() as u32).to_le_bytes())?;
    for (lang, k, h, b) in &index {
        w.write_all(&[*lang])?;
        w.write_all(&(k.len() as u16).to_le_bytes())?;
        w.write_all(k.as_bytes())?;
        for (off, n) in [h, b] {
            w.write_all(&off.to_le_bytes())?;
            w.write_all(&n.to_le_bytes())?;
        }
    }
    w.write_all(&index_at.to_le_bytes())?;
    w.write_all(MAGIC)?;
    w.into_inner().map_err(|e| e.into_error())?.sync_all()?;
    std::fs::rename(&tmp, out)?;
    *PACKS.lock().unwrap() = None;
    progress(1.0);
    let mut langs: Vec<u8> = index.iter().map(|b| b.0).collect();
    langs.sort_unstable();
    langs.dedup();
    Ok(langs)
}

/// The game's start: every ISO waiting in [`dir`] whose release has no pack there yet is read into one (the ISO stays;
/// the player can delete it once its pack is there). Run off the main thread: a disc takes some seconds.
pub fn import_waiting() {
    let Some(dir) = dir() else { return };
    let isos = std::fs::read_dir(&dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("iso")));
    for p in isos {
        import_disc(&p, &dir);
    }
}

/// One import at a time: the start's ([`import_waiting`]) and the other discs beside the main one's (M166) may name
/// the same release, whose pack they would both write.
static IMPORTING: Mutex<()> = Mutex::new(());

/// M166: disc image or pack `path` read into its release's pack in `dir`, unless one is there already or the disc
/// brings no voices (the US disc, another game); whether a pack was written. Run off the main thread.
pub fn import_disc(path: &Path, dir: &Path) -> bool {
    let _one = IMPORTING.lock().unwrap_or_else(|e| e.into_inner());
    let Ok(mut iso) = Iso::open(path) else { return false };
    let Some(id) = disc_id(&mut iso) else { return false };
    let out = pack_in(dir, &id);
    if out.exists() || release(&id).is_none() {
        return false;
    }
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("voices: {}: {e}", dir.display());
        return false;
    }
    match import(&mut iso, &out) {
        Ok(langs) => {
            eprintln!("voices: {} → {} ({})", path.display(), out.display(), langs.iter().map(|&l| LANGS[l as usize].name).collect::<Vec<_>>().join(", "));
            true
        }
        Err(e) => {
            eprintln!("voices: {}: {e}", path.display());
            false
        }
    }
}

/// M165: Settings → Gameplay → Add Disc: where reading a disc in from the menu is.
#[derive(Clone, Debug, PartialEq)]
pub enum Adding {
    Idle,
    /// The system's file picker is up.
    Picking,
    /// Reading the disc's voices: how far (0..=1).
    Reading(f32),
    /// Done: the languages the disc brought.
    Added(Vec<u8>),
    /// It didn't work: why, for the player.
    Failed(String),
}

static ADDING: Mutex<Adding> = Mutex::new(Adding::Idle);

fn set_adding(a: Adding) {
    *ADDING.lock().unwrap() = a;
}

/// Where Add Voices is (the Settings row's text).
pub fn adding() -> Adding {
    ADDING.lock().unwrap().clone()
}

/// Whether a disc is being picked or read (a second press waits for it).
pub fn adding_busy() -> bool {
    matches!(adding(), Adding::Picking | Adding::Reading(_))
}

/// The pack release `id` (`SCES_545.35`) is read into in `dir`.
fn pack_in(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{}.pack", id.replace(['_', '.'], "")))
}

/// M165: Add Voices pressed: the system's file picker for a European or Japanese disc image (desktop: the file dialog;
/// Android: the system's picker; iOS: the images already put in the voices folder through the Files app), then its
/// voices read into a pack in [`dir`] on a worker thread, [`adding`] saying how far. The new languages are on the
/// Voice Language row as soon as it's done; a release read before is read again (a better copy replaces it).
pub fn add_disc() {
    if adding_busy() {
        return;
    }
    let Some(dir) = dir() else {
        return set_adding(Adding::Failed("The voices folder isn't ready yet.".into()));
    };
    set_adding(if cfg!(target_os = "ios") { Adding::Reading(0.0) } else { Adding::Picking });
    std::thread::spawn(move || {
        set_adding(match add_from(&dir) {
            Ok(Some(langs)) => Adding::Added(langs),
            Ok(None) => Adding::Idle,
            Err(e) => {
                eprintln!("voices: {e}");
                Adding::Failed(e)
            }
        })
    });
}

/// The picked disc read into its pack in `dir`: its languages, or `None` if nothing was picked.
#[cfg(not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")))]
fn add_from(dir: &Path) -> Result<Option<Vec<u8>>, String> {
    let Some(path) = pick_disc() else { return Ok(None) };
    let iso = Iso::open(&path).map_err(|e| format!("That file isn't a disc image ({e})."))?;
    read_in(iso, dir).map(Some)
}

#[cfg(target_os = "android")]
fn add_from(dir: &Path) -> Result<Option<Vec<u8>>, String> {
    if !crate::android::pick() {
        return Err("The file picker didn't open.".into());
    }
    // the picker answers through the activity; asked a few times a second (it's the player's, so no time limit)
    let file = loop {
        match crate::android::taken() {
            None => std::thread::sleep(std::time::Duration::from_millis(250)),
            Some(None) => return Ok(None),
            Some(Some(f)) => break f,
        }
    };
    set_adding(Adding::Reading(0.0));
    let iso = Iso::from_source(Box::new(file)).map_err(|e| format!("That file isn't a disc image ({e})."))?;
    read_in(iso, dir).map(Some)
}

/// iOS has no picker yet: the images the player put in the voices folder (the Files app) are read in now, without
/// waiting for the next start.
#[cfg(target_os = "ios")]
fn add_from(dir: &Path) -> Result<Option<Vec<u8>>, String> {
    let _ = std::fs::create_dir_all(dir);
    let isos: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("iso"))).collect();
    if isos.is_empty() {
        return Err("No disc image in the voices folder: in the Files app, put the European or Japanese disc image in Hot Shots Tennis → voices, then press again.".into());
    }
    let mut langs = Vec::new();
    for p in isos {
        let iso = Iso::open(&p).map_err(|e| format!("{}: not a disc image ({e}).", p.file_name().unwrap_or_default().to_string_lossy()))?;
        langs.extend(read_in(iso, dir)?);
    }
    langs.sort_unstable();
    langs.dedup();
    Ok(Some(langs))
}

#[cfg(target_arch = "wasm32")]
fn add_from(_dir: &Path) -> Result<Option<Vec<u8>>, String> {
    Err("The browser build can't read a disc's voices: the desktop and phone apps can.".into())
}

/// Disc `iso`'s voices into its release's pack in `dir`, [`adding`] following along; its languages.
#[cfg(not(target_arch = "wasm32"))]
fn read_in(mut iso: Iso, dir: &Path) -> Result<Vec<u8>, String> {
    let id = disc_id(&mut iso).ok_or_else(|| "That file isn't a PS2 disc image.".to_string())?;
    if release(&id).is_none() {
        return Err(if id == "SCUS_976.10" {
            "That's the game's own disc: its voices are the English ones already. Choose the European (Everybody's Tennis) or Japanese (Minna no Tennis) disc.".into()
        } else {
            format!("That disc ({id}) has no other voices: choose the European (Everybody's Tennis) or Japanese (Minna no Tennis) disc.")
        });
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("Couldn't make the voices folder: {e}"))?;
    set_adding(Adding::Reading(0.0));
    let out = pack_in(dir, &id);
    let langs = import_with(&mut iso, &out, &mut |f| set_adding(Adding::Reading(f))).map_err(|e| format!("Couldn't read the disc's voices: {e}"))?;
    eprintln!("voices: {id} → {} ({})", out.display(), langs.iter().map(|&l| LANGS[l as usize].name).collect::<Vec<_>>().join(", "));
    Ok(langs)
}

/// The desktop's file dialog for a disc image (as the first start's, `disc.rs`: zenity or kdialog first on Linux, for
/// the Steam Deck's Gaming Mode, else the system's).
// ponytail: macOS wants dialogs on the main thread; this runs on a worker (fine on Linux/Windows), as Add mod's
#[cfg(not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")))]
fn pick_disc() -> Option<PathBuf> {
    // a test pressing ✕ on every Settings row never pops a dialog up on the desktop
    if cfg!(any(test, feature = "testing")) {
        return None;
    }
    const TITLE: &str = "Hot Shots Tennis: pick the European or Japanese disc image for its voices";
    #[cfg(target_os = "linux")]
    for (cmd, args) in [
        ("zenity", &["--file-selection", "--title", TITLE, "--file-filter", "Disc image | *.iso *.ISO", "--file-filter", "All files | *"][..]),
        ("kdialog", &["--title", TITLE, "--getopenfilename", ".", "*.iso *.ISO"][..]),
    ] {
        let Ok(out) = std::process::Command::new(cmd).args(args).output() else { continue };
        let picked = String::from_utf8_lossy(&out.stdout).trim().to_string();
        return (out.status.success() && !picked.is_empty()).then(|| PathBuf::from(picked));
    }
    rfd::FileDialog::new().set_title(TITLE).add_filter("Disc image", &["iso", "ISO"]).pick_file()
}

#[cfg(test)]
mod tests {
    /// Where the tests write voice packs: under the build's `target/tmp/`, on disk, not the system temp dir, which is
    /// often a small RAM disk with a per-user quota ("Disk quota exceeded" mid-import).
    fn scratch_on_disk() -> PathBuf {
        let exe = std::env::current_exe().unwrap();
        let target = exe.ancestors().find(|a| a.file_name().is_some_and(|n| n == "target"));
        target.map_or_else(std::env::temp_dir, |t| t.join("tmp"))
    }

    use super::*;
    use crate::audio::{Sound, umpire_bank, voice_bank};
    use hst_data::fixture;
    use hst_sim::sound;

    #[test]
    fn settings_word() {
        assert_eq!(parse_setting(""), 0);
        assert_eq!(parse_setting("voice_language = ja\n"), 6);
        assert_eq!(parse_setting("voice_language = klingon\n"), 0);
        for l in 0..LANGS.len() as u8 {
            assert_eq!(parse_setting(&setting_line(l)), l);
        }
        assert_eq!(key("..\\data\\sound\\VOICE\\SGL\\sgv_vc00a.hd"), key("data/sound/VOICE/SGL/sgv_vc00a.hd"));
    }

    /// Reads the European and Japanese discs (`fixtures/discs/SCES-54535.iso`, `SCPS-15113.iso`, the player's own
    /// copies: skipped without them) into packs, then: each brings its languages; every player voice and umpire bank
    /// the US match loads has its dub in every language but one German bank whose cues don't line up (it stays US);
    /// the dub plays in place of the disc's take while the match's timing (`centre_frames`) stays the US bank's.
    /// About 10 s (the two discs' sound archives).
    #[test]
    fn other_releases_dub_the_us_banks() {
        let (eu, jp) = ("fixtures/discs/SCES-54535.iso", "fixtures/discs/SCPS-15113.iso");
        if !fixture::exists(eu) || !fixture::exists(jp) {
            return fixture::skip("the European and Japanese discs (fixtures/discs/)");
        }
        let Ok(mut us) = fixture::iso() else { return };
        let tmp = scratch_on_disk().join(format!("hst-voices-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        // the packs (≈ 200 MB) go whether the test passes or not
        struct Gone(PathBuf);
        impl Drop for Gone {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
                set_dir(PathBuf::new());
            }
        }
        let _gone = Gone(tmp.clone());
        set_dir(tmp.clone());
        let langs = import(&mut Iso::open(fixture::path(eu)).unwrap(), &tmp.join("SCES54535.pack")).unwrap();
        assert_eq!(langs, [1, 2, 3, 4, 5, 6], "the European disc's six voice sets");
        let langs = import(&mut Iso::open(fixture::path(jp)).unwrap(), &tmp.join("SCPS15113.pack")).unwrap();
        assert_eq!(langs, [6], "the Japanese disc's");
        assert_eq!(available(), [0, 1, 2, 3, 4, 5, 6]);
        // the US disc is the main one: nothing to take
        assert!(import(&mut us, &tmp.join("us.pack")).is_err());

        let mut misses = Vec::new();
        for lang in 1..LANGS.len() as u8 {
            for c in 0..14 {
                for (players, court, b) in [(2, 1, false), (2, 1, true), (4, 1, false), (2, 7, false)] {
                    let bank = voice_bank(&mut us, c, players, court, b).unwrap();
                    let (main_hd, set) = (bank.hd.clone(), if players < 3 { "SGL/sgv" } else { "DBL/dvv" });
                    let v = (b'a' + b as u8 + if court == 7 { 2 } else { 0 }) as char;
                    let dubbed = dub_in(bank, &format!("data/sound/VOICE/{set}_vc{c:02}{v}.hd"), lang);
                    assert_eq!(dubbed.hd, main_hd, "the main bank stays");
                    if dubbed.language.take(&dubbed.hd, lang).is_none() {
                        misses.push(format!("{} voice {c} {players}p court {court} {v}", LANGS[lang as usize].code));
                    }
                }
            }
            for n in 0..5u8 {
                let bank = dub_in(umpire_bank(&mut us, n, 0).unwrap(), &format!("data/sound/UMPIRE/gag_vc{n:02}a.hd"), lang);
                if bank.language.take(&bank.hd, lang).is_none() {
                    misses.push(format!("{} umpire {n}", LANGS[lang as usize].code));
                }
            }
        }
        // the one bank that doesn't line up: the German doubles set's program 6 has its take on key 0, the US one on 2
        assert_eq!(misses, ["de voice 9 4p court 1 a"], "the banks left the US disc's");

        // umpire 0's calls in German: heard from the German bank, timed by the US one
        let sound = Sound::test(&mut us);
        let main = Arc::new(umpire_bank(&mut us, 0, 0).unwrap());
        let de = Arc::new(dub_in(umpire_bank(&mut us, 0, 0).unwrap(), "data/sound/UMPIRE/gag_vc00a.hd", 4));
        let german = de.language.take(&de.hd, 4).unwrap().clone();
        // (heard in the chosen language: German for these plays)
        choose(4);
        let call = |key| sound::Play { slot: 5, program: 0, key, volume: 0x80, speed: 1.0 };
        let mut differ = 0;
        for key in 0..14 {
            assert_eq!(sound.centre_frames(&de, call(key)), sound.centre_frames(&main, call(key)), "call {key} timed by the US take");
            differ += (sound.centre_frames(&german, call(key)) != sound.centre_frames(&main, call(key))) as usize;
        }
        assert!(differ > 0, "the German takes are other lengths, so the timing test means something");
        let id = sound.play_centre(&de, call(0));
        assert!(sound.bank_of(id).is_some_and(|b| Arc::ptr_eq(&b, &german)), "the call is heard in German");
        // back to the disc's English: heard at once, no bank loaded again
        choose(0);
        let id = sound.play_centre(&de, call(0));
        assert!(sound.bank_of(id).is_some_and(|b| Arc::ptr_eq(&b, &de)), "the disc's again");
        let id = sound.play_centre(&main, call(0));
        assert!(sound.bank_of(id).is_some_and(|b| Arc::ptr_eq(&b, &main)), "the disc's");
    }

    /// M165: Add Voices' reading: the game's own disc is turned away with words for the player; the Japanese disc
    /// (`fixtures/discs/SCPS-15113.iso`, skipped without it) reads into its pack with its progress rising to 1 and
    /// Adding following it. A few seconds.
    #[test]
    fn add_voices_reads_a_disc_with_progress() {
        let Ok(us) = fixture::iso() else { return };
        let tmp = scratch_on_disk().join(format!("hst-voices-add-{}", std::process::id()));
        struct Gone(PathBuf);
        impl Drop for Gone {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _gone = Gone(tmp.clone());
        let e = read_in(us, &tmp).unwrap_err();
        assert!(e.contains("game's own disc"), "{e}");
        assert!(!tmp.exists(), "nothing written for the US disc");
        let jp = "fixtures/discs/SCPS-15113.iso";
        if !fixture::exists(jp) {
            return fixture::skip("the Japanese disc (fixtures/discs/)");
        }
        assert_eq!(read_in(Iso::open(fixture::path(jp)).unwrap(), &tmp).unwrap(), [6]);
        assert_eq!(adding(), Adding::Reading(1.0), "the last progress said is the end");
        assert!(tmp.join("SCPS15113.pack").exists());
        let mut seen = Vec::new();
        let langs = import_with(&mut Iso::open(fixture::path(jp)).unwrap(), &pack_in(&tmp, "SCPS_151.13"), &mut |f| seen.push(f)).unwrap();
        assert_eq!(langs, [6]);
        assert!(seen.len() > 2 && seen.windows(2).all(|w| w[0] <= w[1]) && seen[0] == 0.0 && *seen.last().unwrap() == 1.0, "{seen:?}");
    }
}
