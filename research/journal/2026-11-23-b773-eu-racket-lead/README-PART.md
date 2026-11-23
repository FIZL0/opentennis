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

## Not verified / not 1:1

- Look-ahead 3 after the fix is predicted from the EU fit and the US 0x10 window. It has not been run.
- The ordering explanation (the per-frame update runs after the step) fits every case except the 0x2 ready motion at EU tick 257 (Open questions). Play has not been tried with the update moved. Lead 0 in the EU 555–589 window is predicted, not compared: play diverges before then.
- The US fit only covers ticks where slot 2's AI stored a racket direction. Many idle and run ticks show no match
  (the scratch holds other values there), and the idle loop's period makes those shifts ambiguous.
- The EU lead-4 window was fitted on ticks 396–421. B772 measured it on 393–423. The edge ticks weren't re-fitted.
