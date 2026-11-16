# M168 — Mod voices in other languages

## What landed
- **Mod standard** (../opentennis-mods, pushed as 70e967a): character standard §5 *Other languages* — `mod.json`
  `voice_language` (what `voice/` speaks, default `en`) and `voice_languages` (`{"ja": "voice-ja/", …}`, keys the
  Voice Language setting's `en fr it de es ja`); each folder holds files named as `voice/`'s. The umpire standard §3
  takes the same fields for calls.
- **Game** (`crates/hst-base/src/mod_voices.rs`, new): checks the fields (both mod readers refuse a bad one), maps the
  setting to a tag (both Englishes → `en`), and layers the chosen folder over `voice/` take by take: `mods::voice`
  (every character-mod voice path: match, ceremony, emotes, menu previews) and `mods_umpire::bank` (calls over the
  donor's bank). The takes and their count are always `voice/`'s, so key draws, `voice_takes` on the wire and the
  match's timing don't depend on the language; online each player hears their own choice. Menu preview caches
  key on the language too (`previews.rs`, `plate_emote.rs`), so a changed setting is heard without a restart.
- **Extractors**: `tools/rerig/voice_langs.py` (new; also called by package.py after `voice/`) writes `voice-ja/` for
  Out of Bounds mods by rule: a take that is an English ga_sg cue slot's main wave, or a whole result cue mix, is the
  Japanese `ga_pc_NN.xb` bank's same slot (mixed the same way into `out/voices/oob_cues_ja/`). `tools/voices.py`
  now puts other-language archive copies under `out/voices/<game>-ja/`: before, `out/voices/opentee2/` linked the
  Japanese `.xb` copy by sort order (checked: no shipped mod had a take from it). `tools/audio.py` needed no change
  (it already converts every copy). No clip lists or line text committed.
- **Mods rebuilt** (live `out/mods`, versions bumped by the tool): oob pc01–06, 08–14 got `voice-ja/` (39–41 of
  ~45–50 takes each). pc00/pc07 (their `.xb` is English) and pc15–19 (no second copy) unchanged.

## Verified
- Unit tests `mod_voices::tests` (manifest checks, tags, per-take fallback, extra language-only takes ignored, umpire
  calls). Existing `mods::tests`, `mods_umpire` tests pass.
- Live: a throwaway test read `oob_pc04_sasha` through `mods::read`/`mods::voice` with Voice Language Japanese: 41
  takes differ, every program's count unchanged (removed after).
- faster-whisper language ID on the three longest takes of pc01/pc04: `voice/` en 0.65–0.99, `voice-ja/` ja
  0.86–0.99, and they say the English take's line (e.g. "Call me Ms. Perfect!" ↔ 「完璧でしょ!」).

## Not verified / not 1:1
- Not heard in a running match by ear (unattended run; the loading path is the same as `voice/`'s, checked by the
  live read above). Presentation only, nothing to prove against the original.
- Takes the English picks found by transcript outside the sg slots (hit cry, more_voices additions, a few result
  reactions) have no Japanese counterpart by rule and stay English in Japanese.
- Open Tee 2 carries a Japanese copy but its picks are by wave number from English transcripts: no `voice-ja/` yet
  (follow-up M169). Get a Grip, Fore and the HST umpire mods have no other-language copy on their discs.
- A peer's mod copy decides only what they hear; a looks pack sent online carries the language folders too (bigger).
