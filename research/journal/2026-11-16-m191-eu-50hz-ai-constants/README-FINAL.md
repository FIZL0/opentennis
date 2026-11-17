# M191: EU 50 Hz AI constants left at 60 Hz

Ported from M169's SPEC (`research/journal/2026-11-16-m169-eu-50hz/SPEC.md` §6–7). I didn't read the decompile.

## What changed
- **AI landing helper (rally routines)**: when no path entry has bounced yet, the helper flies the last entry on. That
  fly-on now uses `Build::gravity()` (9.8/2500 at 50 Hz) and no longer the fixed US 9.8/3600 bits. SPEC §7 lists "the AI
  landing helper" among the places gravity changes. The old constant is gone.
- **Walk back's re-roll**: a failed return-to-centre roll is re-rolled every `Build::frames(60)` frames (50 at
  50 Hz). The new `position::Return::step_in` / `Single::step_in` take the build, and `step` stays as the US wrapper
  that the replay tests use. Callers that now pass the build: the rally routine's walk back (`rally.rs`), play's doubles
  formation walk back and its singles bot (`play.rs`, one call each).

## Which helpers SPEC §6 retimes
SPEC §6 retimes three "less than 60" AI waits to 50: one of the "two helper waits" and the "two more AI helpers". It
also retimes one "less than 30" wait to 25 and one "more than 29" test to 24. Our port has exactly these AI frame waits
(percentage rolls such as `u < 60` don't count):
- the "more than 29" re-pick tick: `rally.rs` and play's doubles AI, already `Build::frames(29)` (M183);
- the 30-frame wait in the rally routine: already `Build::frames(30)` (M183);
- the doubles rally routine's own walk-back re-roll: already `Build::frames(60)` (M183);
- `position::Return`'s re-roll, used by the singles bot and the doubles formation walk back: this was the last 60
  still fixed.

So every 60-frame AI wait in the port now goes through `Build::frames`, which matches the SPEC's count.

## Tests
- `position::fifty_hz::fifty_hz_walk_back_re_rolls_every_50_frames`: a failed roll re-rolls after 60 frames (US, JP)
  and after 50 (EU).
- The European recordings (`fifty_hz`, `build_rules` in hst-play) still pass. This change doesn't affect the EU bot
  match's first point, as M183a1 found when it tried these changes.

## Not verified / not 1:1
- **Neither change has been checked against the European game.** The EU bot match's recording parts before either
  one matters: at the server's partner's walk (M185) and at the extra AI draw (M190). Neither change touches that
  stretch. Once M185/M190 land and play follows the EU match further, the existing `fifty_hz` recording tests will
  cover these too.
- **Which helpers are retimed comes from the reasoning above, not from the original.** I matched the SPEC's count of
  retimed waits against the port's AI waits. I didn't check this against the European program. If a later recording
  shows a 60-frame re-roll at 50 Hz, change `Return::step_in` back. Its tests will show where.
- No unit test covers the landing helper's gravity on its own. It is private, and setting it up needs a full rally
  `Body`/`World`/path copy. It only uses `Build::gravity()`, which the build's own tests cover.
