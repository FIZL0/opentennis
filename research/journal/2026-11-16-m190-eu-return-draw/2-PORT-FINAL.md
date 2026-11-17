# M190 port: a doubles computer player's formation is set up at the start of every point

Ported from `SPEC.md` with the decompile locked.

## What changed
- `doubles_ai::set_up_doubles` writes `Formation::start` (lane, front, spot) into a doubles computer player's rally
  state once a point. Its inputs: the player's end and formation, whether it serves or receives this point, and
  whether the point is served from the ad court.
- It runs before the player's first rally step (`run`) and before its first message work (`at_message`), so it is
  in place before the serve's own-team re-pick. It reuses the per-point `set_up` flag, which the serve message
  clears. Singles keeps its own set-up.
- `fifty_hz::the_european_generators_play_as_recorded` now asserts that the AI generator *state* equals the
  recording at every tick through recorded 250, not only through 229 (140 is still skipped as before).

## Results
- The return's message (recorded 230) draws 68 as in the game; 249 draws 4 and 250 draws 1, as in the game.
- The generator state matches the recording through 250.
- The first difference left is at 251 (play 3, game 1): M185's stand scratch.
- hst-play's `fifty_hz` (4), `doubles` (21) and `replay` (11) tests pass, so the US match's generator checks are
  unchanged.

## Not verified / not 1:1
- **Players 0, 2 and 3's set-up values** follow the spec's rule, but only player 1's were compared with a snapshot
  of the game's AI state (spec session). The generator match through 250 checks them only indirectly.
- **Ad-court and attacking/defensive set-ups** don't come up in the recorded EU point. They use the same
  `Formation::start` rule, but no recording checks them.
- **251 onward** waits on M185.
