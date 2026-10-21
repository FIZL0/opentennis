# B628 — port phase

Ported from `SPEC.md` with the decompile locked, then proved against B621's recording `context/b621/alt_follow_s04.bin`.

## What's in

- `hst_sim::cutaway::Shot::step_with`: a shot's own hook, which runs on the orbit view after the field of view is set
  and before the record's clearance, band, head-room and framing. `step`/`build` behave as before.
- `hst_sim::alt_cam`:
  - `Follow::aim`: the shot's view. Its hook does the pull-back along the flattened forward and then the ball
    head-room, at 0.9, the record's pitch dead zone.
  - `Follow::show`: the 5 % slerp/eye/FOV smoothing from the run's third frame.
  - `Follow::step` does both.
  - `Play::{Quiet, Rally}` comes from the shot count and the rally's frame count.
  - `BALL_EASE` is gone.
- The ball is held after the point by the caller (`alt_camera.rs`: `held`, updated in Serve/Rally). It is kept
  across the shot's restarts, as the original's is (below).
- hst-play `alt_camera.rs`: `aim` → the Select glide → `show`, in the original's order, so a switch keeps settling
  after the 15 ticks.
- Also (the user's request mid-session): the match's keyboard hints (`key_hints.rs`) list "Camera" with its key
  (C by default), only when the switch works: one human on this machine, offline.

## Lock-step test

`follows_through_the_rally_as_the_original` now:

- compares every shot-9 frame, with no pull-back exclusion: 8 runs, 2181 frames;
- passes the bound 0.0015 (the views' rounding);
- worst error 0.0011, on one run's first frame (901);
- through the rallies and pull-backs, errors stay ≤ 2·10⁻⁴.

What it learned from the recording:

- **Recording layout:** match state +5 is the phase (2 set-up, 3 rally, 4 post-point, 1 the set-up after the change
  of ends) and +8 the frames in it. The rally counter is 1 on the rally's first frame.
- **Repeated samples** (identical game state) are skipped without stepping.
- **"Serve struck"** is inferred: in the rally, the ball's z moves more than 5 cm in a frame.
- **Set-up entry frames stay skipped.** On a set-up's entry frame the game still shows the old run's view; it
  restarts the frame after. Stepping that frame gives a 0.022 error.
- **Change of ends, first frame:** the game's camera saw the ball at the court centre (0, 0, 0). A probe with
  that ball gives forward x 0.050184 against the recorded 0.0501865; the live ball gives 0.0576, the last rally
  ball 0.0696. The ball is reset there for the change of ends, and the recorder read it later in the same frame. The
  test models that one frame.
- **The phase 1 set-up is not "after the point":** the ball there is live, not frozen.

## Not verified / not 1:1

- **The change of ends' first frame:** the original's camera looks at the ball reset to the court centre for one
  frame. Our match doesn't reproduce that. The first frame isn't smoothed and the next ones ease from it, so the
  difference is a ~0.007 rad yaw transient that decays. Presentation only.
- **The quiet case's shot count** comes from our `Game::shots` in the match. In the test it is inferred from the
  ball's motion (the recording has no shot count). The serve is struck at the rally's start in this recording, so
  the cases where the two could differ never come up.
- **Singles (shot 8):** same hook and constants, not compared with the original (no singles save state).
- **A Select switch's settle after the glide** (smoothing on the glided view) follows the spec's order. It is not
  checked against a recording of a switch.
- **The keyboard hint** is our own UI (not in the original). It is checked by the unit test; no screenshot was
  taken.
