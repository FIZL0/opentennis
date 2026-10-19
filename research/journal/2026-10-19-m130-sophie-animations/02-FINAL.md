# 02 — speed check, clips' own faces (FINAL)

User report after playing: the new reactions look too fast, and the eyes stay shut at the end of the celebrations.

## Speed: the source time base was 2× fast (fixed in the mods repo)
- First check (one celebration, coarse frame sampling) said 1×. That was wrong. The user still saw the waves and
  claps as far too fast.
- Rechecked against footage at 60 fps: claps cycle every 0.1 s in our export and every ~0.2 s in the source. The
  results jump is airborne ~0.2 s for a ~0.4 m rise, where gravity needs ~0.55 s.
- Every source clip keys every 80 ticks. The tick rate is 2400/s, not 4800/s. The extractor now uses 2400. Every
  clip of that game is twice as long, faces included. Its mods are re-exported, repackaged and version-bumped.
- No game-side change: a mod clip still plays at speed 1 on its own clock.

## Faces: a mod clip now plays its own
- Before, every mod clip played the donor's `.MOR` face, which ran on its own clock and held its last frame. A
  reaction's face therefore had nothing to do with the body playing.
- motions.py (mods repo) now copies each source clip's face weights into motions.glb, frame for frame with the
  body, on a mesh-less `face` node whose `extras.targetNames` are the source names (the costumes keep those
  targets). Standard §6 *Faces*.
- `own_motions` reads a `weights` channel into a `Face` per motion and take (`CharacterData::own_faces`).
  `drawn_face` picks the one for the drawn take. `Cache::face` samples it at the drawn clock's frame × 80 in
  place of the donor's face; a clip without one keeps the donor's.
- The OOB and Fore! mods (41) were repackaged with faces and bumped. Get a Grip and Open Tee have none (texture
  faces): unchanged.

## Tests
- `takes_load_and_take_turns` checks that each take gets its own face, bound by name.
- `own_motions_are_visual_only` skipped `NAME.K` (it broke on M131's takes).

## Not verified / not 1:1
- No in-game screenshot of the new faces. Checked through the loader tests and the data only.
- Eyes may still close at the end of some emotes: some source clips end on a laughing (closed-eye) face. That is
  the source's own look.
- The donor's eye texture scroll (`.UVA`) and blinks still play over a mod's own face.
- The tick rate (2400/s) is inferred from the clap rate and jump physics, not read from the source executable.
