# M7 port: the alternate match camera

Ported from SPEC.md with the decompile locked; the camera's numbers checked against PCSX2 RAM (copy 2, slot 3
doubles, Select pressed, save state read back).

## What landed

- `hst_sim::alt_cam`: `Follow` runs scripted shot 8 (singles) / 9 (doubles) from the disc's shot table through the
  cut-aways' `Shot` runner, its two named frames filled with the human's ground spot facing the other end and a point
  100 m out on the court's long axis beyond the other end; the ball is the follow target. `glide_weight` is the
  switch's (t^(1−t))², `Fade` the 0..5 counter, `blocks` the 640×448 screen test.
- `hst-play/src/play/alt_camera.rs`: Select (the `camera` bind, C / pad Select) of the one local human toggles it
  (never online, never with two or more local humans); ignored during a glide, the change of ends and replays. The
  shown view glides over 15 ticks with `camera::swing` about the human's spot; switching off glides to a freshly reset
  match camera (a local copy; the shared `Game::cam` is untouched) shown until the next point's set-up. The fade
  draws the human's materials (body, face, racket) with alpha. The choice is kept in `camera.txt` (default off).
- `own_camera.rs`: the picture is served through `OwnCam`'s new `alt` views; with one human a passive `OwnCam`
  carries them and is removed when nothing is shown, so with the camera never selected nothing changes.
- Tests: `hst-sim --test alt_cam` (shot 9's eye, axes and fov against the RAM capture within 0.0015; glide and fade
  sequences; block cases), `hst-play alt_camera` (glide 15 ticks each way, view behind the human, the match and the
  shared camera identical tick for tick with and without it).

## Spec corrections

- The shots' FOV 35 is a full angle: the half-angle is 17.5° (0.3054 rad, as RAM shows), not 35°.
- The look-at frame is the point (0, 0, 100) (game space) with an identity rotation when the human is at the −z end
  (recorded); at the +z end it is taken as (0, 0, −100) facing back, by symmetry.
- The shot's frame 0 lookup must be a direction (w = 0), else the runner takes it for a second subject.

## Not verified / not 1:1

- Change of ends with the camera on: not recorded. Here the shot cuts to a fresh one at the human's new end the
  first tick they are there (no glide); the toggle is ignored while ends change.
- A long rally ending in the alternate camera, and the rally follow with the human moving: not watched live (only
  the serve set-up was read back from RAM).
- The fade: never triggered in the original's sample, and not seen on screen here (rules from the spec; the 30-tick
  rally settle is counted by our own rally tick count).
- The +z end look-at point (assumed mirror of the recorded one).
- The glide curve and its orbit blend: not frame-checked against the original; from the spec only.
- Switching off: the original resets its one match camera; ours glides to a reset copy and leaves the shared camera
  (which play reads) as it is, so the picture after the glide can differ from the original until the next point.
- The original rounds shot views to 0.001; not copied (eye matches the recording within 0.0015).
- Instant replays show the replay cameras; the alternate camera picks up again with a cut after them.
- The post-point cut-aways still run underneath (the sim reads them) but aren't shown while the camera is on, as
  observed.
- No in-match key hint for Select; the controls text names it.
- Not run in the remaster's window (no `--shot` with a human driving): the fade's look is unchecked by eye.
