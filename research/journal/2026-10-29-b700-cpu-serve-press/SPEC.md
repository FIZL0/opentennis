# B700 spec: when a computer server presses its serve swing

Applies to every computer-controlled server, singles and doubles (both AI kinds run the same rule). Human serves
are unchanged.

## Background the port already has

- The server's serve timing error `err` (whole frames, signed): drawn with the AI's other timing errors
  (`ai::AiParams::timing`, the serve part). It is fixed for the serve; the press rule below reads it as is.
- The AI's own contact pick over the predicted ball path (`serve::ai_search` / `ai_pick`): the frame whose height is
  closest to the window's ideal height inside the toss's window, the ball falling (rising for a quick serve, never
  quick on an underhand toss), earliest on a tie, nothing from the first bounce on.
- The swing's own contact search, run on the press (`serve::search`), and its timing offset = contact frame − 8
  (frames counted from the press). The special-shot test for a strong-toss topspin/slice serve needs |offset| < 2.

## The rule (original behaviour)

1. **Look-ahead.** The AI's contact pick looks at the predicted path from **2 frames ahead** (frames 0 and 1 are
   never picked) out to **180 frames ahead**, or the end of the predicted path if that comes first, and stops at
   the first bounce as now. It is the best frame over that whole stretch, not just over the next few frames.
2. **Waiting for the pick.** Once the ball is tossed (the AI has nothing else to do in its serve), every frame it
   runs the pick on that frame's predicted path:
   - no frame qualifies → wait, try again next frame;
   - the pick is **9 or more frames away** → it is not taken yet; wait, try again next frame;
   - the pick is **8 or fewer frames away** → it is taken: the AI's countdown is set to that frame count, and it
     never searches again for this serve.
   Because the pick is the same ball frame each time and only comes one frame closer per frame, in practice the
   countdown is always first set to **8** (47 of 47 recorded CPU serves).
3. **Pressing.** On the frame the pick is taken, and on every frame after it:
   - from the second frame on, the countdown first drops by 1 (the frame it was taken it is not reduced);
   - then if countdown + err ≤ 8 the AI presses the swing on this frame (the swing kind is drawn at this press, as
     now), and the serve's press logic is done.
   So the press comes `min(8, 8 − err)` frames before the picked contact: a zero or negative error presses at
   once, on 8; err = 1 presses at 7, err = 4 at 4. A computer server **never presses early** (never more than 8
   frames out). With a very large error the countdown may go to zero or below before the press; the rule is the
   same (no clamp).
4. **What the press gives.** The swing's own search, run on the press, lands on the same ball frame the AI picked
   (live: offset = press countdown − 8 in every case), so the swing's timing offset is `−max(err, 0)`:

   | err | ≤ 0 | 1 | 2 | 3 | 4 |
   |---|---|---|---|---|---|
   | press countdown | 8 | 7 | 6 | 5 | 4 |
   | swing offset / grade (Will, strong toss) | 0 / sweet | −1 / clean | −2 | −3 | −4 / slow |

   The port must not force the offset: it comes out of the existing swing search. The table is what the tests
   should see.

## What changes against the port today

- Today the port allows the press as soon as its pick is ≤ 8 − err frames away, with the pick visible over the
  whole timing-table horizon, so a negative error presses early (positive offset, +1..+4) and only |err| ≤ 1 is
  special-eligible. The original presses at 8 at the earliest.
- Expected effect for a row with serve error ±4 (Will): special-eligible strong serves when err ≤ 1, i.e.
  1/2 + 1/2 · 2/5 = 70 % (port today 40 %). Offsets are never positive.
- The look-ahead starts at 2 frames and runs to 180 (port: from 0, `grades.len()` frames). The AI's pick has no
  timing-table gate (unchanged).

## Evidence

- Live, PCSX2 copy 1, CPU Will (row 11, serve error ±4) vs a mashing human Ashley, 19 Will serves
  (`research/b700_serve_press.py`, log in `context/b700/run1/press.log`, not committed): the countdown read 8 on
  the frame the pick was taken every time; presses at 8 (err −4..−1) gave offset 0 grade sweet; at 7 (err 1)
  offset −1 grade clean (3 serves, 2 topspins bent); at 4 (err 4) offset −4 grade slow. The pick's look-ahead
  start and length were read live (2 and 180).
- The will-curve research (branch `will-curve`, `2026-10-29-will-curve-serve`) logged the countdown on 28 more
  serves: always first 8, press at `min(8, 8 − err)`. Its "offset" column was the serve's **depth error**, not the
  timing offset (see README); the press frames it gives are right.
- Doubles: the doubles AI's serve state runs the identical wait / take / countdown / press steps. Not recorded
  live in doubles for this task.

## Tests the port should add

- A unit test of the press rule (pure): for err −4..4 and a pick that walks 12, 11, …, 0 frames away, the press
  frame is `min(8, 8 − err)` before the contact and the AI never takes a pick at 9+.
- The app test: an all-CPU singles match where the CPU's serve offsets are never positive and offset 0 follows
  every err ≤ 0 (log err and offset per serve). The replay tests stay exact (human serves don't change).

## Open questions

none (the flat-share question in the will-curve notes is a separate follow-up, B700a: it is not part of the press
rule).
