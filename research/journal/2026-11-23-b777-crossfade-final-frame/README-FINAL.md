# B777: the crossfade's last frame and the switch-tick bind (spec phase)

Status: PART. The spec is in `SPEC.md` beside this file. The port is a later session's job, from the spec.

## Task

B773's racket-z fit in the doubles stand test missed on two kinds of tick: the tick a crossfaded motion switches on,
and every tick from clip time 6 on inside a crossfade. This task finds what play does differently during a crossfade
and writes it down as behaviour, so the next session can port it.

## Method

1. Temporary instrumentation, reverted. A node-diff print in the stand test's same-tick read
   (`crates/hst-play/src/play/doubles_ai/scratch.rs`) compared the Racket chain's nodes against a clean bind plus
   sample of the current clip, tick by tick.
2. The diff showed play's fade ending one frame early, at the 1/8 blend. The right clavicle stays 0.079 off after
   stand→turn_fore and 0.055 off after run→turn_fore, and the pelvis is slightly off. Nodes the new clip doesn't
   animate frame by frame keep the blend value instead of returning to the new clip's held value.
3. Two env-gated hacks, both re-run through B773's fit scripts:
   - Keep the old clip for the weight-0 final mix (SPEC section 1).
   - Bind the new clip in the read on a motion switch (SPEC section 2).
   The fit runs were:
   - `cargo test --release -p hst-play --lib the_us_bot_match_plays_as_recorded -- --ignored --nocapture` and the same
     with `the_european_bot_match_plays_as_recorded`, with print enabled.
   - `research/journal/2026-11-23-b773-eu-racket-lead/scripts/b773_us_lead_fit.py` and `b773_lead_fit.py`, run on
     `context/b772/run6/eu_stack.bin`, rally window 38056–39692.
4. Both tests still pass with the hacks. The hacks are reverted; only this journal and the spec are kept.

## Results

| probe | base misses | final-frame mix | + switch bind |
|---|---|---|---|
| US (2648 entries) | 1765 | 755 | 717 |
| EU (589, rally window) | 282 | – | 12 |

The first fix alone takes US exact shift-0 matches from 340 to 611.

With the switch bind, US tick 888 matches (game 0.4028, play was 0.4199 before), and so do the other crossfaded switch
ticks 237, 243, 371, 391, 411, 426 and 427. US runs from clip time 6 on (226–236, 249–289, …) match after the
final-frame mix.

## Not verified / not 1:1

- The held fade's last frame (soft follow-through) is left out of the spec. By the same frame order the game may still
  sample the frozen old pose and then mix the new clip's frame 0 over the body tracks at weight 1, where play samples
  the new clip directly. That would differ only on single-key, untracked and extra bones. It's a prediction only: no
  probe exercises it, since no held fade feeds the Racket chain in either probe.
- The in-fade residual during stand→turn_fore frames 0–5 (1e-4 to 8e-3, with a bump mid-fade) is unexplained. It is
  filed as follow-up B779 (`agent-runner plan` id B779, after B777). EU's residual ticks are 7–14 (for example EU 11:
  game −0.2626, play −0.2599); US's are 220–225.
- EU's 4 trail-colour ticks (69, 188, 392 per B776; 547 shows the same pattern, motion 0x10 at step 7, game value 128) are B776's, not racket z. The other 8 EU ticks are B779's residual.
- US's remaining misses are other scratch writers (game values 0, 127, 65 and 39) and runs at a constant −5/−6 shift
  (idle clock offset, B775). Neither is the crossfade.
- The drawn pose wasn't checked. SPEC says it follows from the stored pose's motion step, but no screenshot or
  capture compares it.
