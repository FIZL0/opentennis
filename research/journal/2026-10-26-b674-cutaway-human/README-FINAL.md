# B674 — cut-away/replay focus with one human

Spec phase: `SPEC.md`. The original reads no controller in who the post-point cut-away shows or in the win/loss
reactions; with one human it is the same last-hitter/winner rule as all-CPU. One gap: the every-third-replay swap
is held back for a player hit by the ball, not a diving one.

## Port phase
- `hst_sim::cutaway::roles`: the exception's predicate is now "hit by the ball"; `play/cutaway.rs` feeds it the
  point's body-hit target (`g.body_hit`, the same condition the reaction uses) instead of `dive`.
- `tools/record_cutaway.py`: `--slot N` (save other than 5) and `--press` (taps Cross every second during serve and
  rally, like `record_postpoint.py`), so a human-P1 match can be recorded.
- Recorded slots 3 and 4 (both are doubles, P1 human + 3 CPUs: the saves have no one-human singles match):
  `fixtures/b674/cutaway_s03.bin` (3000 frames) and `cutaway_s04.bin` (2500). Test
  `crates/hst-sim/tests/cutaway.rs` `director_one_human`: for every post-point cut-away it checks the shown/other
  roles from the port's rule, the shown player's reaction (win/loss/game/team/hit-by-ball from
  `motion::reaction`) and the picked shot and counters (from the previous cut-away's counters). Coverage: human lost
  by own shot 3+, CPU won by own shot 9+, game-ending points, the human's team winning points (a CPU shown losing),
  one body hit (the other subject in the hit-by-the-ball reaction). All pass.
- By eye: original (slot 4, `tools/screenshot.sh`) cut-aways show the CPU winners in their win takes and the game
  winner in the game-won take; the remaster (`tools/shot.sh … --play --singles --pads -,k`, human P2 idle) shows the
  CPU in its win take after its aces. Same choice. Shots in `context/b674/shots/` (o*.png original, r*.png ours).

## Not verified / not 1:1
- The changed case itself (replays shown a multiple of 3 with the other subject hit by the ball, or diving) didn't
  come up in the recordings (the replay count only reached 1); the fix follows the spec, not a recording.
- No one-human *singles* recording: save slots 3/4 are doubles; the rule has no players-count branch besides
  practice, and the all-CPU singles fixture already covers singles.
- The doubles same-team flag the director sets at the point is still not followed (spec: use unknown).
- The remaster shot's court drew untextured grey in this `shot.sh` run; unrelated to this task, not investigated.
