# P124 — umpire tick order (T8 ledger)

Question: the app steps the umpire at the top of `simulate`, before the match's phase messages. Her call line
gates the post-point wait (`idle`), so does this move the next serve by a tick?

## Result: no shift, order kept, ponytail replaced by the real account

- **The game's order inside a tick** is pinned by the `umpire_s05`/`umpire_tiebreak` recordings
  (`crates/hst-sim/tests/umpire.rs`, her whole state compared every tick): phase-entry messages (start, serve,
  rally, point over, match over) → her step → the scoreboard's calls (score words, game/set announcement).
- **The app**: her step → the scoreboard's calls (same as the game) → the deciding tick's own message (verdict →
  `point_over`, `Next::Serve` → `next_point` → `serve`, `Next::ChangeEnds` → `start`). The game delivers that
  message as the *next* tick's phase entry, before that tick's step. Either way her sequence is
  `step, message, step`, so her state follows the same path. Only her voice goes out one tick sooner, and
  `VOICE_LAG` already accounts for that (its doc says so; it was measured on the integrated timing).
- **What gates the post-point wait**: `PostPoint::step_with` reads `voice_idle` in two places only, the line-call
  sprite's settle and the match-over wait after `calling_match`. Both voices (`point_over`'s line call,
  `call_match`) are played straight from the message, not queued for her step, so step order can't move them.
  Score words and announcements are queued (played by the next step) but nothing waits on them.
- I worked through the integrated arithmetic (play.rs voice block: `voice_left = clip + VOICE_LAG`, decremented
  the same tick, read by the next tick's `idle`). Line call: idle from post tick clip + 5, which is what
  `match_s05_post_point` measures per call. Match call: idle from the call + 58 + 4, which is the tick
  `match_s05_match_over` asks for. Both tests pass.
- **Match point**: can the set announcement (`announce` once `showing()`) cut into the match call and hold the
  wait open? No. With slot 5's timing the show starts at post tick 122, but the match call starts at 32 and
  match-over is asked at ~94, so it never plays first. (Checked with a throwaway PostPoint run, deleted afterwards.)
- `rally()` reaches her before her step on the serve-contact tick, while the game delivers it at the next phase
  entry. It only feeds `seen` (ball within 6.4 m of the net), and at serve contact the ball is at the baseline,
  so that tick looks the same.

Change: the `simulate` ponytail comment ("she hears them a tick later") was wrong and is now the account above.
No code change.

## Not verified / not 1:1

- No app-level test drives the voice timing end to end: the voice countdown only runs with an audio device
  (`play_sounds` returns early in tests), so the integrated check is the arithmetic above plus the two
  recording tests on `PostPoint`.
- The line call, tiebreak and change-ends announcements play one tick before the game's (presentation only,
  inside `VOICE_LAG`).
- `call_match`'s position against her step isn't in the umpire recordings (they don't reach match-over).
  It's played straight from the message and `match_s05_match_over` pins its timing, so order doesn't matter for it.
