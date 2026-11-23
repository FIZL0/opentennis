# B773: the European racket lead per swing

Spec phase. The spec is in `SPEC.md`.

## Method

- A temporary print in play's doubles AI hook (since reverted) logged player 1's motion, clip time, speed and racket
  z every tick of the EU and US bot-match probes (`context/b773/`).
- `research/b773_anim_show.py` decodes the anim captures (`fixtures/m169/eu-bot-anim.bin`, `fixtures/anim_s05.bin`):
  motion, sampled clip time, time, speed and crossfade per tick.
- `research/b773_fast_starts.py` lists every motion start with speed ≠ 1 and its first steps. Every fast start in both
  captures keeps its speed until the clock reaches 8 (Rule 1).
- `research/b773_lead_fit.py` (EU, from B772's dispatcher stack capture) and `research/b773_us_lead_fit.py` (US,
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
- US racket re-fit (`research/b773_us_lead_fit.py` against `fixtures/ai_net_s05.bin`, play's look-ahead-3 values per
  tick): the shift between the game's value and play's is one constant on every lined-up window (the early 0x10 swing,
  the 0x1b swing, idle, run, turns). Before it was 1 or 2, depending on the motion. So the clock part of the spec
  holds: the lead no longer changes per swing. Clip times 36-54 of the 0x1b swing still fit one further, as before.
- US bot match now parts at recorded tick 890 (was matched to 2354): computer player 2 stops one tick early. Cause is
  the open question in SPEC.md: player 1 switches from 0x1f to 0x5 at play's tick 888, and the stand test reads the
  new motion ahead of play's look-ahead.
- Look-ahead 2 with the update moved: the US match parts later, at 1827, near a motion switch (0x1f to 0x3).

## Not verified / not 1:1

- Look-ahead 3 with the update moved was run (*Port attempt*): steady windows line up, motion switches don't.
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
