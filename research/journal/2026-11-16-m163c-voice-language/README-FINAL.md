# M163c — voice language option: already done as added3 (+ M165)

M163c has the same line as **added3**, which is already ticked in `plan/DONE.md`. The work is on main, and this branch
has it:
- `crates/hst-base/src/voices.rs`:
  - the languages: disc English, plus English (Europe), French, Italian, German, Spanish and Japanese from SCES-54535,
    and Japanese from SCPS-15113
  - `voices/<disc id>.pack` packs
  - lookup by bank name
  - a bank whose cues don't line up with the main disc's stays the main disc's
- `SoundBank::language` in `audio.rs`. The dub is what's heard. Timing (`centre_frames`, `take_ticks`) stays on the
  main disc's bank, so the sim and RNG never see the choice. Online, each player hears their own choice.
- Settings → Sound → Voice Language, and Add Voices (M165, in-game ISO picker with progress), in
  `crates/hst/src/play/main_menu/options.rs`.
- Release notes lines are already in `RELEASE_NOTES.md`.

This session changed no code. It checked that the above is present, and `tools/check.sh -p hst-base voices` passes
(3 tests). See `research/journal/2026-11-16-added3-voice-language/` and `2026-11-16-m165-add-voices/` for the
details.

## Not verified / not 1:1

As listed in added3's journal. In short:
- The language labels came from speech recognition plus the disc's language order.
- Nobody listened to a match by ear.
- A dubbed clip keeps the US clip's timing, so it can overlap the next sound or get cut short.
- A few banks whose cues don't line up stay US.
- Menu, prize and announcer banks outside the match aren't switched.
- If the main disc is EU or JP, "English" still names the main disc's own voices.

Nothing new was found in this session.
