# B576 — tiebreak score chime

User 2026-10-18: the 'ching' when the score changes in a tiebreak doesn't play.

## The original
The tiebreak point show's update (the one `hst_sim::flow::Show::step` ports for `Event::TiebreakPoint`) plays one
sound: when its roll step passes 7 and the show moves to the flash stage (countdown 3), not at deuce, it calls the
game's 2D sound helper with slot 0 (court bank), program 0, key 2, volume 0x80. The deuce branch of the same stage
sets no sound. The plain point show, the game/set shows and the scoreboard update call no sound at all (checked
their decompiles), so this chime is tiebreak-only. (Another caller of the same cue is a non-match screen.)

## The remaster
- `hst_sim::sound::TIEBREAK_CHIME` (court 0/2, full) and `flow::ShowState::tiebreak_chime()` (tiebreak show,
  flash stage, t == 3: only the entry tick; the deuce path leaves t at −1).
- `play/popups.rs` `tiebreak_chime`: plays it centred on the court bank, once per heard tick (`Played::first`), so
  rollbacks and instant replays don't double it. No sim state, RNG or timing touched.
- Tests: `hst-base audio::tiebreak_chime_is_heard` (every court bank 1–11 has it; court 1 peak 0.36),
  `hst-play snapshot::tiebreak_points_sound` now also asserts the chime ticks occur (>3) and start a sound.

## Not verified / not 1:1
- Not compared against a PCSX2 recording of the tiebreak (no per-sound decode of `sound_tiebreak.bin` done):
  the cue's bank/program/key/volume and its tick come straight from the decompile; presentation SFX only have to
  feel right (PLAN).
- Not listened to in the running app (unattended run); the headless test shows the sample is non-silent and is
  started on the chime ticks.
