# B773: the European racket lead per swing

Spec phase. The spec is in `SPEC.md`.

## Method

- A temporary print in play's doubles AI hook (since reverted) logged player 1's motion, clip time, speed and racket
  z every tick of the EU and US bot-match probes (`context/b773/`).
- `research/journal/2026-11-23-b773-eu-racket-lead/scripts/b773_anim_show.py` decodes the anim captures (`fixtures/m169/eu-bot-anim.bin`, `fixtures/anim_s05.bin`):
  motion, sampled clip time, time, speed and crossfade per tick.
- `research/journal/2026-11-23-b773-eu-racket-lead/scripts/b773_fast_starts.py` lists every motion start with speed ≠ 1 and its first steps. Every fast start in both
  captures keeps its speed until the clock reaches 8 (Rule 1).
- `research/journal/2026-11-23-b773-eu-racket-lead/scripts/b773_lead_fit.py` (EU, from B772's dispatcher stack capture) and `research/journal/2026-11-23-b773-eu-racket-lead/scripts/b773_us_lead_fit.py` (US,
  `ai_net_s05.bin`) find, per tick, which play tick's racket z equals the game's leftover.
  - EU: lead 0 on the early-started 0x12 swing, 1 on lined-up motions, 4 on the speed-4 swing from 390.
  - US: lead 0 on the early-started 0x10 swing.
- The decompile confirmed the swing setter's speed (8 / frames-to-contact) and that the contact countdown resets
  the speed after the frame's motion step.

## Result

- The lead isn't 50 Hz stepping and isn't an extra pose pass. Two play-side clock differences set it:
  1. Fast swings lose their speed one step too early in play: play trails the game by 3 frames on speed-4 swings.
  2. Queued speed-1 swings start one tick early in play.
  Both come from one cause: play runs the per-frame swing update before the motion step, and the game runs it after.
- Both differences show up in the US match as well.

## Port attempt (2026-11-23)

- Built: play's per-frame swing update (a fast swing's return to speed 1, the queued swing's start 8 frames before
  contact) runs in a new system after the motion step. The stand test's look-ahead went from 2 to 3.
- Parked on `wip/B773` (commit e13fb188), not on the task branch, because the US match regresses (below).
- The non-ignored fifty_hz tests pass with it.
- EU bot match probe: 16 of the recording's phases now match (was 7, parting at point 3's tick 1585). The European
  acceptance point is passed.
- US racket re-fit (`research/journal/2026-11-23-b773-eu-racket-lead/scripts/b773_us_lead_fit.py` against `fixtures/ai_net_s05.bin`, play's look-ahead-3 values per
  tick): the shift between the game's value and play's is one constant on every lined-up window (the early 0x10 swing,
  the 0x1b swing, idle, run, turns). Before it was 1 or 2, depending on the motion. So the clock part of the spec
  holds: the lead no longer changes per swing. Clip times 36-54 of the 0x1b swing still fit one further, as before.
- US bot match now parts at recorded tick 890 (was matched to 2354): computer player 2 stops one tick early. Cause is
  the open question in SPEC.md: player 1 switches from 0x1f to 0x5 at play's tick 888, and the stand test reads the
  new motion ahead of play's look-ahead.
- Look-ahead 2 with the update moved: the US match parts later, at 1827, near a motion switch (0x1f to 0x3).

## Spec phase 2 (2026-11-23)

- The open questions are answered in `SPEC.md`: *When in the tick the pose is taken*, *Play's stored pose is one step
  behind, not two*, *What the port has to produce*.
- Decompile: each player's update runs in court-slot order: the AI and motion commands, one motion step, the pose layers
  and crossfade mix, the placement, then the racket hand-off. The hand-off evaluates the skeleton afresh and copies the
  racket node matrix (x row times hand sign) through the scratch slot. So player 2's stand test on tick T reads player
  1's pose from player 1's own step on tick T, including a motion player 1 started that tick.
- Checked against the captures: the game's read equals the game's own sampled clip time in the same frame at EU rally
  200, 289–290, 390, 396, 409, 411, and US play ticks 26 and 310.
  - EU 390: the switch to the speed-4 swing. The game is already on the new motion at clip 0 (−0.7529); play carries
    the old motion on (−0.8270).
- The earlier "lead" reading was backwards. On lined-up motions the fit shift is −1: the game equals the value play's AI
  read one tick earlier, so look-ahead 2 reaches one frame too far.
- The right read is the stored pose plus one step (look-ahead 1), plus this tick's motion command. Not look-ahead 3.
  That explains why the `wip/B773` look-ahead-3 attempt failed on motion switches.
- The US probe's opening idle has play's idle clock 3 frames behind the game's from the probe start (play starts the idle
  at clip 0). It is a test-harness start-state difference, filed as B775.
- Not verified: the new read was not run (spec phase, no Rust). The arm turn's place relative to the crossfade mix wasn't
  checked against a capture. The skip-drawing case (hand-off and sampling skipped) was not observed.

## Not verified / not 1:1

- Look-ahead 3 with the update moved was run (*Port attempt*): steady windows line up, motion switches don't. Spec phase 2
  explains the switch failures: the read should be look-ahead 1 plus the tick's motion command.
- The ordering explanation (the per-frame update runs after the step) fits every case, including the 0x2 at EU tick 257,
  which is the stance after the early-started swing ends (*After a swing ends*). Tried with the update moved on
  `wip/B773` (*Port attempt*). Lead 0 in the EU 555–589 window is predicted, not compared: play diverges before then.
- The stance pick one tick later (ball position read a tick later) is predicted to still give 0x2 at 257. Not run.
- The US fit only covers ticks where slot 2's AI stored a racket direction. Many idle and run ticks show no match
  (the scratch holds other values there), and the idle loop's period makes those shifts ambiguous.
- The EU lead-4 window was fitted on ticks 396–421. B772 measured it on 393–423. The edge ticks weren't re-fitted.
- The port is on `wip/B773` (e13fb188), not landed. The US bot match regresses at 890 with it (motion switch inside
  the look-ahead window).
- The stance after the EU 0x12 swing (0x2 at 257) and the crossfade were not checked against the anim capture.
- The look-ahead-1 read (*Spec phase 2*) is not implemented or run. The US probe's opening idle can't line up until B775
  fixes its start state.

## Port phase (2026-11-23)

Built from `SPEC.md` with the decompile locked.

- Per-frame swing update after the motion step: the parked `wip/B773` change, taken over as is (`swing_after_step.rs`:
  the fast swing's return to speed 1 and the queued swing's start are noted in the stroke update and handed to the
  motion command after `character::tick`).
- Same-tick read, no fixed look-ahead (`doubles_ai/scratch.rs`): after the pose, each player's `Racket` chain (local
  rows and held rotation and position) and its `Motion` are kept. At the next player's AI time the read runs play's
  motion setter for that tick on a copy (`motion_command`, split out of the `motions` system so both share it), then
  one motion step with its crossfade or held fade, under the player's current facing and arm turn.
  - Play runs each player's control in court-slot order, so player i−1's motion command for the tick is already set
    when player i reads it.
- `Nodes::restore` (hst-sim) rebuilds the kept chain into a pose to step.
- `swing_after_step::apply` also goes in netplay's real-tick harness, next to the plugin's schedule.

### Results

- `fifty_hz` tests, both bot-match probes included: all 7 pass.
  - EU: 16 of 144 phases match (was 7). Point 3's 1585 is passed with no no-contact fallback.
  - US: the 2354 player check holds. The parting at 890 from the look-ahead-3 attempt is gone.
- Shift fit, with a temporary print of player 2's read since reverted (`context/b773/port2/`):
  - EU (B772 run6 capture): 299 ticks at shift 0, 6 at shift 1 (the probe's opening idle, B775), no other whole-tick
    shift. Every swing matches on every tick: the queued 0x12 (181–256), the speed-4 0x10 from its cut at 391 through
    423, and 0x10 at 540–589, which is lead 0 as Rule 2 predicted. The first ticks of every other motion match too.
  - US (`ai_net_s05.bin`): shift 0 through the 0x10 swing (was −2), the 0x1b and 0x17 swings, and 889–894 across the
    0x1f→0x5 switch. Opening idle shift 3 (B775). Some 0x1b clip times (19, 35–47) still fit −1, as before.

### Not matched (follow-ups)

- Crossfaded motions (B777): on a switch with a crossfade the switch tick misses (US 888: game 0.4028, play 0.4199;
  was 0.4855), and from clip time 6 of every looping, stance or run motion play's value is off by 0.005–0.09 with no
  shift fitting (EU 7–22, 49–60, 307–353, 360–390). The old look-ahead-2 fit missed the same ticks. Recomputing with
  player 1's tick-end facing changes nothing, so the gap is in play's pose of the chain on crossfaded motions, not in
  the read's timing. Cut motions match throughout.
- Another scratch writer (B776): EU rally 69 and 188 (clip 7 of a queued swing) read 127 and 8, and 392 (second tick
  of the speed-4 swing) reads 128. Play reads the racket there, and the sign differs at 69 and 392.
- The stance pick one tick later (0x2 at EU 257) wasn't compared against the anim capture beyond the fit: 258–263 match
  at shift 0, and 257 itself is a crossfaded switch tick (B777).
- The skip-drawing case (no hand-off) isn't modelled. It was not seen in either capture.
