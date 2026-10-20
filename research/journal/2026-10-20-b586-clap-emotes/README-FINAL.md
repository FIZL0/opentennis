# B586 — clap emotes: racket away, clap sound

Task (user 2026-10-20): hide the racket while a character claps, and play the clap SFX the character's own game uses.

## Who claps
- HST's own characters: none of the disc's reaction/emote clips (re_gu, re_di, re_gu_set, re_di_set) claps (a
  hand-meeting check over every clip finds at most one meeting). Nothing to do for them.
- Golf mods: the source games name their clap clips — Out of Bounds / Open Tee 2 putt-gallery `kuNN_cl`, Out of
  Bounds result `rslt_clap_pcNN`, Fore! `clap_NN`. Takes built from those (takes.py) are the claps: 61 of the packaged
  golf mods have one (OOB re_gu.N, Open Tee 2 re_gu.2, Fore re_gu_set.N). Get a Grip mods use no clap clip.
  Picked by name, not geometry: a hand-meeting threshold missed slow claps.

## Each game's clap sound
Details in the mods repo, `notes/audio.md` *Claps*:
- Out of Bounds: the lobby avatars' SE bank, the `abata_se_clap` cue (cue → sequence → note → region → wave decoded).
- Open Tee 2: the common SE bank's `hakusyu` ("handclap") wave; its cue's sequence bytes read as (length, BCD
  digits) pairs giving program 1 note 39 → that wave.
- Fore!: its SE bank holds the same recording as Open Tee 2's hakusyu, sample for sample.

## What changed
- Mods standard: `extras: {"racket": false, "clap": true}` on a take marks a clap (§6 *Claps*); voice program 11
  (`voice/11_0.wav`, §5) is its clap sound. Mods repo `tools/rerig/claps.py` marks the takes by source clip name and
  copies the game's clap SE; `package.py` runs it; all 61 affected packaged mods patched and their `mod.json`
  `version` bumped (commit 2378e2a there; not pushed or uploaded to the store from this unattended run).
- Game: `mods.rs` reads `extras.clap` into `CharacterData::claps` (a clap is racketless too, so the existing racket
  hiding covers it); `play/emote.rs` `clap()` queues a `Line::Clap` on the player's voice slot when the clip a motion
  starts drawing is a clap (emote or post-point reaction alike), hooked from `motions` in play.rs as the motion
  starts; `play_lines` plays it without cutting or being cut by other lines. Once per player per tick however many
  figures draw them.
- Tests: `mods::tests::takes_load_and_take_turns` (clap flag → claps + racketless), emote
  `a_clap_take_claps` (a clap take plays the emote's line and the clap), `a_mod_emote_speaks` counts the clap when the
  drawn take is one.

## Not verified / not 1:1
- Fore!'s playback pitch for the clap is not decoded: it ships at the rate its sound index gives (19644 Hz), slower
  than Open Tee 2's copy of the same sample (23361 Hz). Which Fore event plays it is not known either.
- Which Open Tee 2 / Fore! game event plays hakusyu (the gallery? the result screen?) isn't traced; it is the only clap
  SE in those banks, so it is the one used.
- Out of Bounds' clap is the lobby avatars' (a burst of several claps, 1.68 s), not traced to the putt gallery's
  `kuNN_cl` reactions; it's the only clap SE found in that game.
- When the sound starts within the clip: at the clip's first frame. The source games' own sync of the SE to the
  hands isn't known; not checked by ear in a running match (unattended run, no audio check possible).
- The clap rides the voice volume slider (it plays on the player's voice slot), not the effects slider.
- An online peer playing a modded opponent as a stand-in (no looks pack) draws the donor's clips, so no clap there.
- The original HST has no clap emote; nothing to compare against on PCSX2 (not launched).
- One mod (fore_pc17_tbone) failed my geometry scan (quaternion error in my throwaway script); irrelevant now that
  claps are picked by name, but its clip wasn't looked at by geometry.
