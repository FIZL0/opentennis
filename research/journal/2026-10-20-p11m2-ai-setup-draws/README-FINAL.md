# P11m2: the AI generator's draws before the first point (spec phase)

This was the spec session. It read the decompile and wrote `SPEC.md`, but no Rust. The next session ports from it.

## How it was checked

- `research/journal/2026-10-20-p11m2-ai-setup-draws/scripts/p11m2_intro_probe.py <name> <slot> [follow s]` (under `tools/pcsx2.sh`) loads a Confirm Settings state,
  starts the match, and logs on every change: the AI generator's index, the match phase, and each player's
  state/AI-state bytes. It runs until P0 can serve, then optionally serves and follows on.
- The states came from P11m1's Confirm Settings saves (copy 4's slots 8/9), put into copy 3's scratch slots:
  singles Ashley (human) vs COM Cody, and doubles Ashley + COM Jun vs COM Cody + COM. A CPU-vs-CPU singles state
  (COM Ashley vs COM Jun, Queen's) was built from the singles one.
- Captures: `context/p11m2/{singles,doubles,doubles_follow,cpu_singles}.json`. Each one matches the counts in
  `SPEC.md`: setup leaves 36, the intro resets add 18 per AI object, then roles add 17 / 0 / 1. The first point
  adds 18 per AI object, the second 17.

## Found on the way (in the spec)

- The app never resets a human's AI object (a human beside a CPU partner); the game does at every point.
- The game draws the first point's net pick when the partner has an AI object; the app requires the partner to be a CPU.

## Not verified / not 1:1

- Doubles with two CPUs on P0's team, or a CPU serving first in doubles, wasn't captured. The counts follow
  from the same rules (captured for singles CPU serving, and for doubles rally/receive roles), but they weren't
  observed directly.
- Matches with a cut-scene flag set during the intro (the game then skips the AI and does partial resets) weren't
  seen. The flag stayed off in every capture, and the app has no such path.
- Only draw counts were checked, not the values drawn. The values feed AI state that the first point's reset
  overwrites; the port session should compare the AI generator's index (and state words) at the first point to
  these captures.

# Port phase (2026-10-20)

Ported from `SPEC.md` with the decompile locked: `crates/hst-play/src/play/ai_setup.rs`, hooked into `play.rs` in
three lines (setup's end, the human branch of `control`, the net-pick test in `ai_update`).

- Setup's end (after the server is known): with any AI object, 36 draws (the last one's two full resets), 18 per
  AI object (the intro's resets), then 17 for a CPU server and 1 per doubles CPU in the rally role.
- A human's AI object (beside a CPU partner) is reset with the others at each point, in player order (full at the
  first point, plain after), and takes no role.
- The match-start net pick is drawn when the partner has an AI object (always true for a CPU now), not only a CPU.

## Checked
`ai_setup::tests` builds the app's match from the disc and reads the AI generator's index:
- singles human serves to CPU: 54 at setup, 72 after the first point's reset (capture: 54, 72);
- singles CPU vs CPU: 89, 125, 142 (capture: 89, 107, 125, 142);
- doubles human + CPU vs two CPUs: 110, 182, 184 (capture: 110, 182, 184);
- doubles later point (placement forced with `reset_positions`): +68 (capture 505 -> 573).

## Not verified / not 1:1
- Values, not only counts: the test matches aren't the captured ones (other characters/court/weather, so other
  seeds), so only indices are compared. The intro's draws are taken in one go at setup instead of at the intro's
  moment; nothing else draws from the AI generator in between and the first point's reset overwrites what they set.
- CPU vs CPU singles: the capture's next index (144, the serve routine's spot + wait draws) isn't asserted: the spot
  draw depends on the serve level drawn, which differs with the seed. The app drew 143 (level not 0) in its match.
- Doubles' second-point rally entries (+2 after the resets) weren't seen in the forced-placement test (the doubles
  AI's own per-point state isn't reset by that shortcut); only the 4 × 17 resets are asserted there.
- Doubles with two CPUs on P0's team / a CPU serving first in doubles: not captured (as in the spec phase); follows
  from the same rules.

## Tests that moved with the new draws
The AI's choices in the first points now differ, so two scripted-play tests were made seed-proof rather than
re-tuned: `emote::an_emote_locks_then_plays_on` times its second flick off the point's measured end (half an emote
before it) instead of a fixed tick after the serve, and `smash_marker::every_lob_shows_the_marker` plays a 6-game
match (the 2-game one ended after 7 shots) and stops once it has seen two lobs.
