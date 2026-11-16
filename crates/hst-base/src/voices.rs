//! Settings → Sound → Voice Language: the players', the umpire's and the gallery's voices from another release of the
//! game (the European disc's English, French, Italian, German, Spanish and Japanese sets; the Japanese disc's
//! Japanese), heard in place of the main disc's. Purely what this machine hears: the match still times every take by
//! the main disc's bank (`Sound::centre_frames`, `Sound::take_ticks` play the main bank, never the dub), the sim and
//! its random numbers never see the choice, and online each player hears their own.
//!
//! A player adds a release by putting its ISO in `voices/` beside the settings (desktop, phones' Files folder): the
//! next start copies its voice banks into `voices/<disc id>.pack` (about 25 MB a language) and the ISO can go. Each
//! release's archives hold the same banks under the same names as the US disc's (the European one once per language,
//! told apart by the archive's last letter; the Japanese one keeps its player voices inside the motion archives), so a
//! bank is looked up by its name. A bank whose sequences don't line up with the main disc's (a different take count
//! or order) stays the main disc's, so a cue never plays the wrong line.

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

/// The language banks load in from now on (the next match; the menus' previews too).
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
pub fn dub_in(mut bank: SoundBank, hd: &str, lang: u8) -> SoundBank {
    let k = key(hd);
    if lang == 0 || !voiced(&k) {
        return bank;
    }
    if let Some((lhd, lbd)) = find(lang, &k).filter(|(lhd, _)| same_cues(&bank.hd, lhd)) {
        bank.language = Some(Arc::new(SoundBank { hd: lhd, bd: lbd, wav: Default::default(), takes: Default::default(), stream: None, language: None }));
        bevy::log::info!("voices: {k} in {}", LANGS[lang as usize].name);
    }
    bank
}

/// Reads the voice banks off release `iso` into `out` (a [`MAGIC`] pack); the languages it brought. A disc this
/// doesn't know (the US one included: its voices are the main disc's) is refused.
pub fn import(iso: &mut Iso, out: &Path) -> io::Result<Vec<u8>> {
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
    for (path, lang) in archives {
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
        let Ok(mut iso) = Iso::open(&p) else { continue };
        let Some(id) = disc_id(&mut iso) else { continue };
        let out = dir.join(format!("{}.pack", id.replace(['_', '.'], "")));
        if out.exists() {
            continue;
        }
        match import(&mut iso, &out) {
            Ok(langs) => eprintln!("voices: {} → {} ({})", p.display(), out.display(), langs.iter().map(|&l| LANGS[l as usize].name).collect::<Vec<_>>().join(", ")),
            Err(e) => eprintln!("voices: {}: {e}", p.display()),
        }
    }
}

#[cfg(test)]
mod tests {
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
        let tmp = std::env::temp_dir().join(format!("hst-voices-{}", std::process::id()));
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
                    if dubbed.language.is_none() {
                        misses.push(format!("{} voice {c} {players}p court {court} {v}", LANGS[lang as usize].code));
                    }
                }
            }
            for n in 0..5u8 {
                let bank = umpire_bank(&mut us, n, 0).unwrap();
                if dub_in(bank, &format!("data/sound/UMPIRE/gag_vc{n:02}a.hd"), lang).language.is_none() {
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
        let german = de.language.clone().unwrap();
        let call = |key| sound::Play { slot: 5, program: 0, key, volume: 0x80, speed: 1.0 };
        let mut differ = 0;
        for key in 0..14 {
            assert_eq!(sound.centre_frames(&de, call(key)), sound.centre_frames(&main, call(key)), "call {key} timed by the US take");
            differ += (sound.centre_frames(&german, call(key)) != sound.centre_frames(&main, call(key))) as usize;
        }
        assert!(differ > 0, "the German takes are other lengths, so the timing test means something");
        let id = sound.play_centre(&de, call(0));
        assert!(sound.bank_of(id).is_some_and(|b| Arc::ptr_eq(&b, &german)), "the call is heard in German");
        let id = sound.play_centre(&main, call(0));
        assert!(sound.bank_of(id).is_some_and(|b| Arc::ptr_eq(&b, &main)), "undubbed: the disc's");
    }
}
