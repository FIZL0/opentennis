# added3 — Voice Language option

Sound → **Voice Language** picks which release's voices play: the players' grunts and calls, the chair umpire
and the crowd (galleries). English (the disc's) by default; more languages appear once another release's disc
image has been imported.

## Findings (other releases' discs)

- European disc (SCES-54535, *Everybody's Tennis*, PAL) carries a copy of every sound archive per language, told apart by
  the archive extension. Identified by speech recognition over the umpire's score calls (faster-whisper, scratch venv):
  `.XB0` English (European cast), `.XB1` French, `.XB2` Italian, `.XB3` German, `.XB4` Spanish, `.XBB` Japanese.
  Text archives use `XB0`–`XBA` for eleven text languages (not used here).
- The European `.XBB` banks match the Japanese disc's byte for byte.
- Japanese disc (SCPS-15113, *Minna no Tennis*) uses plain `.XB`. Its player voice banks sit in the per-character
  animation archives and its crowd banks in other folders, but the banks' own names (the path stored inside the
  archive) are the same on every release. So a bank is found by that inner name, wherever its archive is.
- European English is a different recording from the US one for 20 player voice banks, umpire 2 and the menu/prize
  banks. It's listed as its own language ("English (Europe)").
- All 176 voice/umpire/gallery banks a US match loads exist in every language. Their program/key layouts match the
  US ones except: the Japanese crowd banks for courts 1 and 2, and German doubles voices for character 9 (`a`/`c`
  sets), whose program 6 take is on key 0 where the US one is on key 2. A bank whose set of cues differs from the
  US one isn't dubbed and plays the US bank. The test pins the German one as the only player/umpire exception.
- Clip lengths differ between languages (German umpire calls checked). The match times the umpire's call and the
  crowd's reactions by the clip's length (it feeds the netplay timeline). So **timing always comes from the main
  disc's bank and the dub is only what's heard**, the same way the umpire mods' takes work. The sim, its RNG and
  the online timeline don't see the setting. Online each player hears their own choice.

## How it works

- `hst-base/src/voices.rs`: an ISO dropped into `voices/` beside settings.txt is read once at start (on a thread)
  into `<disc id>.pack` (only the voice/umpire/gallery banks, ~180 MB EU / ~30 MB JP); the ISO can then be
  deleted. A US disc or an unknown one is refused. `voice_bank`/`umpire_bank`/`gallery_bank` in `audio.rs` attach the
  chosen language's copy (`SoundBank::language`), which `play*` use; `centre(…)` for timing keeps the disc's.
- `voice_language = <code>` in settings.txt (disc, en-eu, fr, it, de, es, ja); unknown → disc.
- The name plate's emote voice preview caches per language.

## How to try it

    mkdir -p <data dir>/voices && ln -s <EU or JP iso> <data dir>/voices/
    target/release/hst <US iso> --play --pads -,-,-,-     # imports; then Options → Sound → Voice Language

The log shows `voices: sound/… in German` for each dubbed bank. Checked with both discs: German singles
(players + umpire) and Japanese doubles on court 3 (players, umpire, crowd) loaded dubbed.

## Not verified / not 1:1

- Language labels per archive come from speech recognition plus the disc's language order, not from listening
  through every bank.
- Heard only by log, not by ear in this unattended run: the dubbed banks load and play through `play*` (the unit
  test checks which bank plays), but nobody listened to a match.
- A dubbed clip can be longer or shorter than the US clip it stands in for, while timing stays on the US clip's
  length: a long German call can overlap the next sound or get cut where the US one wouldn't. Presentation only.
  Never in the original (no release mixes languages).
- Banks whose cues don't line up (Japanese crowd on courts 1–2, German doubles character 9 `a`/`c`) stay US.
- Adding a language is by dropping an ISO into the folder. There's no in-game "add a disc" picker yet, and there's
  no import on the web build (no thread/file access; its Help text says so). Not tried on phones.
- If the main disc itself is European/Japanese (another task's work), "English" still names the main disc's voices.
- Menu/prize/announcer banks outside the match aren't switched.
