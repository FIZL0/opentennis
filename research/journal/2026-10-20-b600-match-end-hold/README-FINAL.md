# B600 — players flash back to their marks before the match-end ceremony

User (2026-10-20): at "Game, set and match" the players flash back to their default court positions for about a
frame before the match-end sequence.

## Cause

The tick the match ends on (the post-point phase's match-over step, once the umpire's match call has stopped) goes
straight on to the next match: score reset and `next_point`, which places everyone on the next serve's marks (at
the swapped ends) and cuts the camera. The ceremony then starts from Update: its dissolve first asks for a
screenshot of the frame on screen and waits for it, so a few frames of the new match's serve set-up were drawn
and the dissolve faded from one of them.

Logged in the real app (`--play --singles --games 1 --pads -,-`, a temporary per-frame probe of the drawn figure
positions, `context/B600/run1` with the fix, `run2` without): without it, 4 frames between the match's end and the
ceremony drew the players at (−3, −12.25) / (3, 12.25), the next serve's marks, against their end-of-point spots
(−0.39, 11.24) / (2.44, −9.57); with it, every frame until the ceremony draws the end-of-point spots and the same
camera, then the ceremony's own marks and camera.

## What the original does

From B38 (`research/journal/2026-10-12-b38-gameset-dissolve`): the match → ceremony cut is a 16-tick dissolve
from a still of the last frame before the cut. The players stop at the umpire's match call and stand still into
the match-over phase (noted at the post-point step in `play.rs`), so that last frame is the end-of-point picture.

## The fix (presentation only)

`crates/hst-play/src/play/match_stats/hold.rs`: through the match call, each tick keeps a copy of the match as
drawn (`Game`, the figures' `Motion`s, the online own camera). When the tick leaves the post-point phase, that copy,
stilled between ticks (prev = now for players, ball, camera, motions), is what the frames show: swapped in after
each fixed tick (FixedPostUpdate) and out before the next (FixedPreUpdate), so every tick, including rollback
re-runs (they run inside the gate, between the two), runs on the real match. It ends when the ceremony or the
result page is up (or after 120 frames). Only runs with a primary window: headless tests and tools never see it.
`OwnCam` got `Clone` and `freeze`.

Test: `the_match_end_holds_its_last_picture` (hold.rs): a 1-game bot match forced a point from its end, with and
without a window: the same state hash every tick, and for 30 ticks after the end the windowed app's `Game` shows
the end-of-point positions while the real match is on the next serve's marks.

## Not verified / not 1:1

- Not checked against the original by eye in this session: no PCSX2 save state sits near a match's end (slot 5 would
  need a whole bot match played out), and B38's held-dissolve captures of the original weren't in this checkout's
  `context/`. The picture the original dissolves from comes from B38's notes and the post-point step's own note that
  the players freeze at the match call.
- The held picture is the state after the tick before the match ended (one tick before the original's last frame).
  It is held completely still, crowd and umpire included, for the few frames until the ceremony (the original's
  crowd may keep moving under its dissolve).
- The port's players don't stop at the match call (an existing port difference noted in `play.rs`, not this task),
  so the held picture is wherever they stand when her call ends.
- Update-time writes to `Game` during the hold (a few frames) go to the held copy and are dropped. The only ticks
  that miss them are the hidden first ticks of the next match before the ceremony stops the clock; the app leaves
  the match after the result page.
