# B760 Match win: pill gap, "size jump", hand-over hitch

Three faults the user saw at the match's end (2026-11-21), plus "a bad transition to that point" (the hand-over
from the last frame of the match into the ceremony). Looked at with `HST_STATS=4 tools/shot.sh --play --shot-at T`
(forced end) and `HST_CLIP=N` frame runs (60 Hz steps) of both the forced end and a real last point (a throwaway
score poke so the first game ends the match; removed). Frames are in `context/endscreen-concept/` (scratch).

## 1. Pill gap

The winner pill sat at y 320..352 of the 448-unit screen, the banner's row starts at 360 but its letters only at
371.2 (every one of the six rows the same): 19 units of air, about 50 px at 1152 tall. The pill and its team text
now sit at y 335 (bottom 367): 4 units above the letters. Both are laid out by `widescreen::put`, so it holds at any
aspect (checked 16:9 and a square window).

## 2. "The banner jumps in size"

Not the still. The still is a window screenshot of the window's own size (2048x1152 for a 2048x1152 window) drawn
by a 100% x 100% UI node: players and court sit on the same pixels in the last live frame and in the still. The
still holds no banner at all. What reads as a 1.6x, offset banner at 4.3 s is the live banner's own intro: the
"Game, Set, Match!" effect model starts at the ceremony's first tick and its big blurred layers close in on the
final letters, seen through the fading still. The banner is the same size and place through the ceremony and over
the result page. Nothing to fix in the size; what was wrong in the still is under 3.

## 3. Hand-over: stale still, hitch

- Stale content: the screenshot was asked for on the first frame the result existed. What hides at the match's end
  goes off the next frame, but the picture keeps 25 % of the last frame a tick (`hud_gamma`, the original's frame
  persistence), so it lingers for a few ticks. Forced end (it cuts in during the serve set-up, which a real match
  never does): the COM/1P name tags were back inside the still. Real end: the red bounce marker was back in the
  still though the held picture no longer had it. The cut now lets the picture settle for three ticks (3 frames and
  50 ms, a fast display draws several frames a tick) before it asks for the still, so the still is the picture the
  screen already holds. A one-frame settle was not enough at 240 fps (tags still in the still at ~25 %).
- Hitch: `ceremony::begin` read the disc on the frame of the cut (camera clip, win/lost motion archives, faces,
  rackets, prize voice, result music): 20-62 ms on the main thread, one frame of 20.6-22.7 ms against 4.2 ms. The
  read now runs on a job (`crate::jobs`) from the first frame the ceremony is pending, while the still is on its
  way; `Dissolve::cut` takes `ready` and the cut happens when both are here. Longest frame across the cut after:
  8.6 ms (the screenshot read-back's frame); the rest 4.2 ms. Not pinned down: 2 of 4 forced `HST_FPS=1` runs still
  showed one 35.5-37.5 ms frame just before the ceremony starts (while the picture is held; the real-path run and
  the other two had none). Suspect the read job or the read-back competing with the frame.
- What remains: the picture is held for the three settle ticks (50 ms) plus the GPU read-back of the still (3-4
  frames here, 10 at most) before the dissolve starts. On the real path that picture is frozen anyway (hold.rs); the forced path
  keeps running. On a real last point the outcome label ("Smash Ace") goes off at the match-over tick, a few frames
  before the dissolve; not changed.
- No black or doubled frame, no camera snap besides the dissolve itself, either path. The 16 and 60 tick lengths
  are untouched; nothing here touches the sim RNG or match timing (the ceremony starts a few real frames later at
  most, after the match is over).

Test: `cut_waits_for_still_and_caller` (dissolve.rs).

The ceremony → page dissolve is as on main (user 2026-11-21: it's good): only the cut into the ceremony settles
and waits for the read; the page cut takes its still at once.
