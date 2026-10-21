# B628 — alternate camera: pull-back, tilt, smoothing (spec)

Clean-room spec (PLAN.md *Clean-room ports*). It describes behaviour only. It covers the three gaps B621 left in the
second match camera (shots 8 and 9, `hst_sim::alt_cam::Follow`): the eye backing away as the ball comes past the
human, the small tilt after the ball, and the longer gap with the human at the net. All three, and B621's "the turn
after the ball lags it", come from one per-shot hook that shots 8 and 9 share (and no other shot uses). The hook acts
at three moments of every frame of the shot.

Recording used for the black-box check below: `context/b621/alt_follow_s04.bin` (B621's, see
`research/journal/2026-10-21-b621-alt-camera-accuracy/README-FINAL.md` and the test
`follows_through_the_rally_as_the_original` for its layout).

## What changes against the current port

1. **The ball is not eased.** The ball the shot follows (its target frame) is the live ball's position. B621's
   `BALL_EASE` (5 % of the way to the live ball each frame) was a black-box stand-in for item 4 below and goes.
   - One rule about that ball: once the point is over (the match's post-point phase and anything after it), the ball
     the shot looks at stays where it was on the last frame of the point. It no longer follows the live ball.
2. **Pull-back along the view** (after the orbit is built, before anything else touches the view).
3. **Ball head-room tilt** (straight after the pull-back).
4. **Smoothing of the whole view** (last, after the framing turn and after the Select switch glide).

Order within one frame of the shot, as the original runs it (the parts already ported are marked):

1. the subject and look-at frames are filled in from the human (ported: `frames()`);
2. the script's channels advance (ported);
3. the orbit view is built: subject frame, yaw, pitch, framing offsets, distance (ported: the first part of
   `Shot::build`, up to and including the second-subject turn, which shots 8/9 don't use);
4. **pull-back** (new);
5. **ball head-room** (new);
6. the record's clearance, band and head-room checks (ported; all off for shots 8/9);
7. the framing turn after the target (ported: `frame_target`; it uses the view as it stands after 4–5, its eye
   included, and the possibly widened field of view from 5);
8. the Select switch glide from the other camera, while one runs (hst-play's `alt_camera.rs` today);
9. **smoothing** (new);
10. the final view is kept as "last frame's view" for the next frame's smoothing, and the shot's frame counter goes
    up by one.

So the hook has to sit inside `Shot::build` between the orbit and the clearance step, for shots 8/9 only. One way to
do this is an optional hook the alternate camera passes in. The smoothing can live in `Follow`.

## 4. Pull-back

Inputs:

- the view just built in step 3: its forward axis and its eye;
- the human's ground spot `p` (the subject's position);
- the ball's position `b` (the live ball, frozen after the point as above);
- the match phase and how many frames it has run;
- the number of shots so far this rally (0 until the serve is struck, 1 after the serve, and so on).

Work in "depth along the human's end".

- `s` = −1 if the view's forward z is negative, else +1.
- `P = s · p.z` and `B = s · b.z`.
- With the human at the −z end looking toward +z, `s` = +1, P ≈ −12.25 on the baseline, and B gets smaller (more
  negative) as the ball comes toward the human and past them.

The ball depth used, `D`:

- **Quiet case:** no shot struck yet this rally, or the rally phase has run fewer than 30 frames. Then
  `D = B + 0.3`. There is no other adjustment and no floor.
- **Otherwise:**
  - start with `D = B`;
  - if `D < P` (the ball is behind the human), push it further back by half the overshoot, at most 3 m:
    `D = D + max(0.5 · (D − P), −3)`. Compute `m = 0.5 · (D − P)`; if `m < −3` use −3, else `m`; add it to `D`.
  - Then floor it: if `D < −17.885` (that is −11.885 − 6, 6 m behind the baseline) use −17.885.

The limit `L`, from how far the human stands from their baseline:

- `r = P / −11.885`, capped at 1 (if `r > 1` use 1).
- `L = −3 · (1 − r)`.
- On or behind the baseline, `L` = 0. At the net the gap grows: at P = −5.20, r = 0.4375 and L = −1.6875. That is
  B621's 1.69 m longer gap.

The shift: `d = D − P`; if `d < L` the shift is `d`, else it is `L`. So the shift is never positive, and it is
"min(d, L)".

Apply it along the view's forward, flattened to the ground plane. Height is unchanged.

- `inv = 1 / sqrt(fz·fz + fx·fx)`. Sum it as the EE does: the product fz·fz first into the accumulator, then
  fused-add fx·fx; then sqrt, then a divide of 1 by it.
- `ux = fx · inv`, `uz = fz · inv`.
- New eye x = (0 + eye x) fused-plus `ux · shift` (accumulator loaded with zero plus the coordinate, then the EE's
  multiply-add). The same for z with `uz`.
- Eye y and the eye's w stay as they are. The original adds a zero product, which leaves them bit-identical.

The eye backs away by |shift| along the view. Typical values:

- the ball behind a baseline human: up to 17.885 − 12.25 ≈ 5.6 m;
- the human at the net: up to ≈ 12.7 m;
- B621 saw "~12 m", and its 15.7 → 12.7 m 3D distances.

The smoothing (item 9) makes the shift ease in and out.

## 5. Ball head-room

Straight after the pull-back, keep the ball inside the top of the picture. This is the same check the record's
head-room option makes (`Shot::head`: widen the vertical angle and tilt up so the point sits within `amount` of the
half-height above centre, vertical half-angle capped at 35°, only ever widening and tilting up).

- The point is the ball (the same ball as above).
- The amount is the shot record's pitch-framing dead-zone value. That is 0.9 for both shots 8 and 9: the second of
  the four pitch-framing floats, which the port already reads.
- Only when the ball would be above that line does anything change. Then the view tilts up a little (B621's forward
  height up to 0.011) and the field of view may widen. The framing turn after this sees the widened field of view.

## 9. Smoothing

From the shot's third frame on (frame counter ≥ 2, counting the first frame as 0), the view on screen moves 5 % of
the way from last frame's view to this frame's:

- **Rotation:** take the quaternion of last frame's rotation and of this frame's (`quat::from_matrix` on each), then
  `quat::slerp(last, now, 0.05)`, then `quat::to_matrix`. Those three are exactly the game's helpers.
- **Eye:** each coordinate = (0 + last) fused-plus `(now − last) · 0.05`; the eye's w is 1.
- **Field of view (horizontal half-angle):** the same form, (0 + last) fused-plus `0.05 · (now − last)`.
- On the first two frames of a shot (counter 0 and 1) there is no smoothing; the raw view shows.
- "Last frame's view" is the previous frame's final view: smoothed, and including the glide.

Each run of the shot starts its counter at 0. That covers each point's set-up, which restarts the shot (B621 6), and
each switch into the alternate camera. The counter does not advance while the game is paused. Our match doesn't step
the camera while paused, so nothing to do there.

Consequences, as seen in the original:

- **The turn after the ball lags.** B621 fitted this as an eased ball at 5 %.
- **The pull-back eases in and out** (B621's "~0.95/frame decay").
- **A Select switch into the alternate camera keeps settling after the 15-tick glide ends.** The smoothing runs on
  the glided view, so the picture trails it at 5 % a frame for a while longer. This is presentation, so it only has
  to feel right (PLAN.md). Smoothing the shown, glided view (step 8 then 9) matches the original; smoothing only
  `Follow`'s own view, before hst-play's glide, would make the switch settle sooner. The lock-step test doesn't cover
  switches, so either passes it. Prefer the original's order if it fits the code.

## Singles (shot 8)

Shot 8 has the same hook and the same constants (0.3, 0.5, −3, −6, −11.885, 0.05, the 0.9 head-room from its own
record, which has the same value). Only its script differs (already ported). It was not compared against the
original (no singles save state).

## Black-box check (this session)

A quick model checked the pull-back rule against the recording: 1861 frames of shot 9, duplicates skipped.

- The model: the rule above for the raw shift, smoothed at 5 % from each run's third frame, and a constant 11.74 m
  base gap.
- It predicts the eye's horizontal gap from the human with mean error −0.007 m and standard deviation 0.088 m.
- The remaining error is the model's constant base gap (the real one moves with the orbit). It is not the rule.

## Notes for the port

- **The recording repeats some samples** (39 of 3000: the same game frame read twice, e.g. 1801–1803). A repeated
  sample must not step the camera. Skip a sample whose game state is identical to the one before.
- **The recording has no shot count**, but the quiet case needs to know whether a shot has been struck this rally.
  In the match, use the rally's own shot count. In the test, "no shot yet" can be inferred: before the rally phase
  nothing is struck; in the rally phase, the serve leaving the server's hand counts.
  - Where the two disagree (the server holding the ball past the rally phase's 30th frame), the shift differs by a
    few centimetres at most: the ball sits at the human, so `D − P` is near zero either way.
- **The frozen ball after the point:** pass `Follow` a flag or freeze it in the caller. In the test, a sample in the
  post-point phase looks at the last rally-phase ball.
- **Acceptance** (the task line): `follows_through_the_rally_as_the_original` with no frames left out (drop the
  `pulled` exclusion) and the bound back to the view's rounding (the game stores shot views rounded to 0.001, so
  about 0.0015, as the settled-serve test uses).

## Open questions

none
