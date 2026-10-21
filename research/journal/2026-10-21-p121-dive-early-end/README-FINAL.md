# P121 — dive slide early end, recorded (FINAL)

The dive's early end, when a blocked move cuts the slide short, had been ported from the decompile with no recording
behind it. It is now recorded from the real game, and the port is bit-exact there.

## How the case was forced
`scripts/p121_blocked_dive_record.py` (under `tools/pcsx2.sh`, slot 3, copy 2) drives P1 into dives the way P7e's
`p7e_dive_record.py` does. On each dive's first sample (counter 1, after the search frame) it moves the diver next
to the court clamp that the dive's direction points at most: the sideline here, or else the net or the back. It
writes the new point to the position, the last mover position and the dive's start, so the game carries on as if
the dive had started there. Each move goes to `<out>.pokes`. The distance to the clamp cycles through 0.01 m, half
the slide, and the slide + 0.02 m.

Run 1 (5400 frames, `context/p121/run1/`, copied to `context/fixtures/p121_blocked.bin` + `.pokes`) produced 4 dives,
all P1 = character 6:
- vsync 7372, k 11, dir (−1.000, −0.016): the sideline clamps x on frame 1. The angle from the start crosses 25° on
  frame 3. The counter jumps 3 → 13 (k+2), and the root phase is later stopped too.
- vsync 9009, k 8, half the slide: clamped mid-slide, but it stays within 25° until the root phase, which then stops.
- vsync 11023, k 13, slide + 0.02: the root path never reaches the clamp, so this is a plain dive.
- vsync 11382, k 9, dir (−0.939, −0.344): blocked on frame 1 and cut at once (counter 1 → 11).

## What the recording shows
- The counter jumps to k+1 on the cut frame (k+2 after the frame's increment), as `Dive::step` has it.
- The receive motion restarts at time 5. This character's root path is 0 through time 6 and 0.0903 at 7. After the
  frame-1 cut of the 4th dive, the game doesn't move on the first root frame. On the second it moves
  0.344 × 0.0903 = 0.031 m, so the motion was at time 6 and then 7. A motion that had kept its own time (≈ 2–3)
  wouldn't have moved.
- The body's position and the dive counter match `Dive::step` bit for bit on all 223 dive frames: slide, cut,
  root phase, root-phase stop and the end. Test: `tests/swing.rs blocked_dives`.

## Fix
`Dive::step` was already exact. The app's motion clock didn't jump: after a cut the receive motion played on from
its own time, about 3 frames behind the game's pose. `Dive::restarted()` marks the frame of a slide cut, and `play.rs`
`motions` then sets the clock to time 5 at speed 1. The app's dive animation timing is otherwise unchanged.

## Not verified / not 1:1
- The app's pose after a cut (time 5 on the cut frame) wasn't compared against a game screenshot. It may be one
  frame off from the game's, and it is presentation only (no sim effect).
- A contact dive whose slide is cut wasn't recorded: all 4 were miss dives. The port skips the contact after a cut
  because the counter jumps past k, which matches what the counter does here. The volley itself wasn't observed.
- Doubles-partner and net blocks weren't forced. Sideline clamps were enough to exercise the cut, and the mover's
  partner and net clamps are proven by `recorded_dives` and the locomotion tests.
- The game also treats a step as blocked when something else moved the player since its last mover call. Nothing
  in a normal match does that, so the port doesn't model it. The recorder writes the last mover position along with
  the position so that this branch isn't triggered.

## Also
- Clippy on main failed on the smash mode's `-(x as f32 expr) as i32` (float negation, `clippy::float_arithmetic`).
  It is now `-(… as i32)`, which gives the same value for every mode it can return (|x| ≤ 10.5).
