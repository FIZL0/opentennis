# B779: in-fade residual of the doubles stand test's racket z during a stand→turn_fore crossfade

## Task

After B777, the EU rally ticks 7–14 (B773's tick numbers, rally window 38056–39692) still missed B773's racket-z
fit by 1e-4 to 8e-3. Examples: tick 11 is game −0.2626, play −0.2599. The brief also named US ticks 220–225.

## Method

- Read the game's crossfade step, the mix routine and the slerp in the decompile. Read the instruction order of the
  mix's dot product and position mix in the disassembly (`research/dis.sh`).
- Temporary print of player 1's racket read. Env-gated hack that adds one frame to the outgoing motion's clock in
  the stand test's read only. Both were reverted.
- Ran `HST_B779=1 cargo test --release -p hst-play --lib the_european_bot_match_plays_as_recorded -- --ignored --nocapture`,
  then B773's `research/journal/2026-11-23-b773-eu-racket-lead/scripts/b773_lead_fit.py context/b772/run6/eu_stack.bin <log> 38056 39692`.
- Logs and fits are in `context/b779/run1/` (`base.*`, `old1.*`).
- The print's tick counter ran one ahead of B777's, so shift labels there are +1 relative to B777's.

## Finding

The crossfade's per-frame mix is the same as play's, step for step:

- The new clip is sampled first.
- Then each of the first 23 body tracks the outgoing clip keys is mixed from the node's sampled value toward the
  outgoing clip's sample at the step's weight.
- The weight is countdown / length. It is set to (length − 1) / length at the start and recomputed after each step.
- Rotation uses the same shortest-arc slerp. The end shortcuts apply at weight 0 and 1. The dot product is summed in
  the order y, x, z, w. A negative dot flips the sign. Then the VU0 slerp runs with the held value first.
- Position is held + (outgoing − held) · weight, computed by a multiply-add with the held value as the accumulator.
- The positions' w is set to 1.
- The outgoing clock is wrapped, sampled, then advanced by its speed.

Nothing needs porting, so there is no SPEC and no port phase.

Two details were seen that play does not need to change:

- The game skips resampling the new clip when its clock hasn't moved since the last sample. The result is the same,
  because the held values don't change.
- The mix has a second rotation path, a longest-arc slerp for one designated track. The motion setter always switches
  it off (it sets no designated track), so it never runs in a normal crossfade.

### The residual's cause

The cause is the EU probe's outgoing idle clock. Before the switch, the game's reads match play's one tick later:
EU ticks 1–6 fit at shift +1. That opening-idle offset is filed as B778. The switch itself is aligned: from tick 15
on, the fit is shift 0.

The game runs one more idle frame (game tick 7, value 0.3264), an idle time play never samples. So at the switch,
the game's outgoing idle clock is one frame ahead of play's.

With the hack adding one frame to the outgoing clock in the read, EU ticks 8–14 all match exactly (game 8 0.4461 …
14 −0.9290). Tick 7 stays a miss, because it is the idle frame play never plays.

So EU 7–14 should close when B778 starts the EU probe's idle clocks from the recording.

Ticks against fade frames: EU 8–14 are the fade's steps 0–6 (outgoing weight 7/8 down to 1/8; play's switch is
aligned with the game's on tick 8). Tick 15 is step 7 at weight 0, which matches. Tick 7 is not a fade frame at all:
it is the extra idle frame. The brief's "frames 0–5" was approximate.

The mid-fade bump: the miss is the outgoing weight times the change in the idle pose over one frame, pushed through
the racket chain. The weight falls 7/8 → 1/8, while the new clip turns the body so the racket z swings from +0.45 to
−0.93. So the error's effect on z changes sign between steps 2 and 3 (+0.0049, +0.0046, +0.0004, −0.0027, −0.0032,
−0.0019, −0.0003 game − play) and fades out with the weight. No separate cause is needed, and with the one-frame
clock shift every step matches to the fit's 6e-5.

### US 220–225

Already matched after B777 (`context/b777/run1/us_fit.txt`: 221–225 at shift 0). Not re-run here. Tick 220, the
switch tick, fits only at shift −1 there. That is a whole-tick timing offset on the switch, not an in-fade mix
difference, the same kind as the US probe's other constant-shift runs (B775's clock offsets). It is out of this
task's scope and listed under Not verified.

## Results

- Baseline EU rally-window misses: 14. They are ticks 7–14 (this residual), 69, 188, 392, 547 (trail colour, B776),
  and two new ones, 179 and 538, which were not in B777's run.
- With the hack, ticks 8–14 match. The total rises to 110, because the hack shifts every crossfade's outgoing clock,
  including ones that were already right. That is expected for a probe-only hack.
- Ticks 179 and 538: player 1's turn_fore switches to a stroke motion (0x12 at 179, 0x10 at 538). The game's read
  still sees the old motion one more tick. At 179 the game gives −0.7908, which is turn_fore at its next frame. Play's
  read already plays the new motion and gives −0.8610. This appeared since B777. B774 landed in between, and it moves
  the doubles player's first step to the press tick. Filed as a follow-up (see Follow-ups).

## Follow-ups

- B778 (EU probe idle clocks from the recording): its clock fix should also close the EU racket-z misses 8–14 and
  tick 7. Re-run B773's lead fit after the fix. Journal: `research/journal/2026-11-23-b779-in-fade-residual/`.
- Ticks 179 and 538 (player 1's switch from turn_fore to a stroke motion, 0x12 and 0x10): new task B781,
  which finds the tick the game's stroke start takes effect relative to player 2's read.

## Not verified / not 1:1

- The conclusion that B778's clock fix closes EU 8–14 is shown with a read-only hack, not with B778's actual change,
  because B778 has not landed.
- EU tick 7 is predicted to close with B778 as well, but that is untested.
- The longest-arc path's purpose was not explored, because it never runs.
- US 220 (the switch tick, shift −1) was not re-run or explained here; it is a timing shift, not the mix.
- The journal is a new B779 directory, not B777's (one directory per task, AGENT.md).
- Ticks 179 and 538 were not investigated. They are the follow-up.
