# 02 — speed check, clips' own faces (FINAL)

User report after playing: the new reactions look too fast, and the eyes stay shut at the end of the celebrations.

## Speed: unchanged, matches the source
- The match starts reactions at speed 1, and a mod clip plays on its own clock at 60 Hz from key times in seconds.
- The source's key spacing is one key per 1/60 s at the assumed tick rate.
- Timed one celebration in a 60 fps recording of the source game against our export (crouch → landing, settle
  into the end pose). They match within the frame-sampling error (~0.35–0.4 s), nowhere near 2×.
- The source clips really are that snappy: a quick move, then a long held end pose. No speed change made.

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
- The tick rate is checked against a single clip only.
