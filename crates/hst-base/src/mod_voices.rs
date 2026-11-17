//! M168: a character or umpire mod's voice lines in other languages (the character standard §5, the umpire standard
//! §3a). `mod.json` may name a folder per language beside its default `voice`:
//!
//! ```json
//! "voice": "voice/", "voice_language": "en", "voice_languages": { "fr": "voice-fr/", "ja": "voice-ja/" }
//! ```
//!
//! Each folder holds `<program>_<key>.wav` named as the default folder's. Settings → Sound → Voice Language
//! ([`crate::voices::chosen`]) picks the folder: every take the default folder has is heard from the chosen
//! language's folder where it has the same file, else from the default (the mod's own language, `voice_language`,
//! English when unset). The takes and their count stay the default folder's, so which line a cue draws, the emote
//! cycles online (`mods::voice_takes`) and the match's timing never depend on the language: purely what this machine
//! hears, and online each player hears their own choice.

use std::path::{Path, PathBuf};

use crate::audio::SoundBank;

/// The languages a mod can carry, by their `voice_languages` key: the [`crate::voices::LANGS`] the Voice Language
/// setting offers, both Englishes as one.
pub const TAGS: [&str; 6] = ["en", "fr", "it", "de", "es", "ja"];

/// The mod language tag Voice Language `lang` (a `voices::LANGS` index) asks for.
pub fn tag(lang: u8) -> &'static str {
    match crate::voices::LANGS.get(lang as usize).map(|l| l.code) {
        Some(c) if TAGS.contains(&c) => TAGS[TAGS.iter().position(|t| *t == c).unwrap()],
        _ => "en",
    }
}

/// Checks a manifest's `voice_language` and `voice_languages` (the reason when they're wrong).
pub fn check(j: &serde_json::Value) -> Result<(), String> {
    match &j["voice_language"] {
        serde_json::Value::Null => {}
        serde_json::Value::String(t) if TAGS.contains(&t.as_str()) => {}
        v => return Err(format!("`voice_language` {v} must be one of {}", TAGS.join(", "))),
    }
    match &j["voice_languages"] {
        serde_json::Value::Null => Ok(()),
        serde_json::Value::Object(m) => {
            for (k, v) in m {
                if !TAGS.contains(&k.as_str()) {
                    return Err(format!("`voice_languages` key `{k}` must be one of {}", TAGS.join(", ")));
                }
                if !v.as_str().is_some_and(|f| crate::modfs::plain(f.strip_suffix('/').unwrap_or(f))) {
                    return Err(format!("`voice_languages.{k}` {v} must be a path inside the mod folder"));
                }
            }
            Ok(())
        }
        v => Err(format!("`voice_languages` {v} must map a language to a folder")),
    }
}

/// The folder (relative to the mod) manifest `j` gives language tag `t`, unless it is the default folder's language
/// or the manifest is wrong.
pub fn folder(j: &serde_json::Value, t: &str) -> Option<String> {
    check(j).ok()?;
    if j["voice_language"].as_str().unwrap_or("en") == t {
        return None;
    }
    j["voice_languages"][t].as_str().map(str::to_string)
}

/// The chosen language's folder of the mod in `dir` (from its `mod.json`), if it carries one.
pub fn chosen_folder(dir: &Path) -> Option<PathBuf> {
    folder_in(dir, crate::voices::chosen())
}

/// [`chosen_folder`] for Voice Language `lang`.
pub fn folder_in(dir: &Path, lang: u8) -> Option<PathBuf> {
    let text = crate::modfs::read_to_string(&dir.join("mod.json")).ok()?;
    let j: serde_json::Value = serde_json::from_str(&text).ok()?;
    folder(&j, tag(lang)).map(|f| dir.join(f))
}

/// Every take in `default` (`<program>_<key>.wav`): its program, key and the file it's heard from, `lang`'s copy of
/// it where `lang` has one.
pub fn takes(default: &Path, lang: Option<&Path>) -> Vec<((usize, usize), PathBuf)> {
    let mut out = Vec::new();
    for e in crate::modfs::read_dir(default) {
        let file = e.file_name().unwrap_or_default().to_owned();
        let name = file.to_string_lossy().to_ascii_lowercase();
        let Some((p, k)) = name.strip_suffix(".wav").and_then(|s| s.split_once('_')) else { continue };
        let (Ok(p), Ok(k)) = (p.parse(), k.parse()) else { continue };
        let heard = lang.map(|l| l.join(&file)).filter(|f| crate::modfs::is_file(f)).unwrap_or(e);
        out.push(((p, k), heard));
    }
    out
}

/// A character mod's voice (`mods::voice`): the default folder's takes, keys in order, each heard in language
/// folder `lang` where it has it; `None` if the default has none.
pub fn bank(default: &Path, lang: Option<&Path>) -> Option<SoundBank> {
    let mut wav: std::collections::HashMap<usize, Vec<(usize, (std::sync::Arc<[i16]>, u32))>> = Default::default();
    for ((p, k), f) in takes(default, lang) {
        match crate::modfs::read(&f).ok().and_then(|d| crate::audio::pcm(&d)) {
            Some((pcm, rate)) => wav.entry(p).or_default().push((k, (pcm.into(), rate))),
            None => bevy::log::warn!("{}: not a PCM16 wav, skipped", f.display()),
        }
    }
    let wav: std::collections::HashMap<_, _> = wav
        .into_iter()
        .map(|(p, mut ks)| {
            ks.sort_by_key(|k| k.0);
            (p, ks.into_iter().map(|k| k.1).collect())
        })
        .collect();
    (!wav.is_empty()).then(|| SoundBank { hd: Vec::new(), bd: Vec::new(), wav, takes: Default::default(), stream: None, language: Default::default() })
}

/// An umpire mod's calls over its donor's bank: `bank`'s [`SoundBank::takes`] with every call the default folder
/// ships replaced by language folder `lang`'s copy where it has one; how many it replaced.
pub fn over_takes(bank: &mut SoundBank, default: &Path, lang: &Path) -> usize {
    let mut n = 0;
    for (pk, f) in takes(default, Some(lang)) {
        if !f.starts_with(lang) || !bank.takes.contains_key(&pk) {
            continue;
        }
        if let Some((pcm, rate)) = crate::modfs::read(&f).ok().and_then(|d| crate::audio::pcm(&d)) {
            bank.takes.insert(pk, (pcm.into(), rate));
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(rate: u32, samples: &[i16]) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut w = b"RIFF".to_vec();
        w.extend((36 + data.len() as u32).to_le_bytes());
        w.extend(b"WAVEfmt ");
        w.extend(16u32.to_le_bytes());
        w.extend(1u16.to_le_bytes());
        w.extend(1u16.to_le_bytes());
        w.extend(rate.to_le_bytes());
        w.extend((rate * 2).to_le_bytes());
        w.extend(2u16.to_le_bytes());
        w.extend(16u16.to_le_bytes());
        w.extend(b"data");
        w.extend((data.len() as u32).to_le_bytes());
        w.extend(data);
        w
    }

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hst-mod-voices-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// The manifest fields are checked; the setting's languages map to mod tags (both Englishes to `en`); the folder
    /// for the default language is the default one.
    #[test]
    fn manifest_and_tags() {
        let ok = serde_json::json!({"voice_language": "ja", "voice_languages": {"en": "voice-en/", "fr": "v_fr"}});
        assert_eq!(check(&ok), Ok(()));
        assert_eq!(folder(&ok, "en").as_deref(), Some("voice-en/"));
        assert_eq!(folder(&ok, "ja"), None);
        assert_eq!(folder(&ok, "de"), None);
        for bad in [
            serde_json::json!({"voice_language": "xx"}),
            serde_json::json!({"voice_languages": {"klingon": "v/"}}),
            serde_json::json!({"voice_languages": {"fr": "../v/"}}),
            serde_json::json!({"voice_languages": ["fr"]}),
        ] {
            assert!(check(&bad).is_err(), "{bad}");
            assert_eq!(folder(&bad, "fr"), None);
        }
        let names: Vec<&str> = (0..crate::voices::LANGS.len() as u8).map(tag).collect();
        assert_eq!(names, ["en", "en", "fr", "it", "de", "es", "ja"]);
    }

    /// A language folder replaces the takes it has, the rest stay the default's, and its extra takes are left out:
    /// the takes and their order are the default folder's whatever language is heard. An umpire's calls alike.
    #[test]
    fn language_takes_over_the_default() {
        let d = dir("layer");
        std::fs::create_dir_all(d.join("voice")).unwrap();
        std::fs::create_dir_all(d.join("voice-fr")).unwrap();
        for (f, s) in [("voice/7_0.wav", 1), ("voice/7_1.wav", 2), ("voice/9_0.wav", 3), ("voice-fr/7_1.wav", 20), ("voice-fr/7_2.wav", 30), ("voice-fr/9_0.wav", 40)] {
            std::fs::write(d.join(f), wav(22050, &[s])).unwrap();
        }
        let first = |b: &SoundBank, p: usize| b.wav[&p].iter().map(|t| t.0[0]).collect::<Vec<_>>();
        let own = bank(&d.join("voice"), None).unwrap();
        assert_eq!((first(&own, 7), first(&own, 9)), (vec![1, 2], vec![3]));
        let fr = bank(&d.join("voice"), Some(&d.join("voice-fr"))).unwrap();
        assert_eq!((first(&fr, 7), first(&fr, 9)), (vec![1, 20], vec![40]));
        // a language the mod doesn't carry (no folder) is the default
        let none = bank(&d.join("voice"), Some(&d.join("voice-de"))).unwrap();
        assert_eq!(first(&none, 7), vec![1, 2]);

        let mut ump = SoundBank { hd: Vec::new(), bd: Vec::new(), wav: Default::default(), takes: Default::default(), stream: None, language: Default::default() }.with_takes(&d.join("voice")).0;
        assert_eq!(over_takes(&mut ump, &d.join("voice"), &d.join("voice-fr")), 2);
        let mut calls: Vec<_> = ump.takes.iter().map(|(k, v)| (*k, v.0[0])).collect();
        calls.sort();
        assert_eq!(calls, [((7, 0), 1), ((7, 1), 20), ((9, 0), 40)]);

        // the chosen language's folder comes from mod.json
        std::fs::write(d.join("mod.json"), r#"{"voice_languages":{"fr":"voice-fr/"}}"#).unwrap();
        assert_eq!(folder_in(&d, 2), Some(d.join("voice-fr/")));
        assert_eq!((folder_in(&d, 0), folder_in(&d, 4)), (None, None));
        let _ = std::fs::remove_dir_all(&d);
    }
}
