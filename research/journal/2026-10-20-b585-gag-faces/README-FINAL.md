# B585 — Get a Grip faces don't animate

## What was checked
- All 15 getagrip packs (`../opentennis-mods/out/mods`, and the user's installed copies, same face.json): both
  costumes list their face materials, every channel has a texture except the Helghast mask (null, by design);
  donors 0, 1, 6, 11, 12 per B561's map. `golf_faces_all` passes: every channel binds a donor track. Data was fine.
- The code path works: a debug log in `texture_faces` showed the donor's weights reaching the rig and the face
  material's two primitives swapping texture; a forced face showed in a match.

## Cause
The texture-face rule (standard §4b) showed a channel only when its weight was **strictly above 0.5**. The donors
hold their stance and stroke faces (ids 3–7, anger_eye) at exactly 0.5 for whole rallies, and many reactions peak at
0.5 too, so they never left the neutral face: in a 90 s match ~16 % of frames showed anything, mostly blinks.

## Fix
`character::strongest`: a channel shows at 0.5 or more. `texture_face_follows_the_donors_faces` now demands every
donor face that reaches half strength shows its expression on some frame (fails on faces 3–7 with the old rule).
Shots: Ban's win cutaway shows his grin texture; rally faces swap (tiny at match distance).

No mod data changed, so no mod version bumps. The mods standard's §4b wording ("above 0.5") needs the matching
edit in ../opentennis-mods: B593 (this run may only commit in its own worktree).

## Not verified / not 1:1
- Get a Grip's own per-motion face swaps (`face_tracks.json`) aren't used; faces follow the HST donor's `.MOR`
  weights mapped onto whole-face textures, by design of §4b. Judged by eye only (presentation).
- Partial weights below 0.5 (e.g. doki_mouth 0.3 peaks) still show neutral, and a blink at full weight still
  replaces an emotion's face for its few frames: a whole-face texture can't blend. Not compared to the original
  PSP game.
