# B614: mod face blinks (Get a Grip, Open Tee 2 texture faces)

## What was checked
- **The donors' face tracks.** I dumped Emi's (donor 0) `.MOR` faces per motion with a throwaway test: frames where
  each channel is at 0.5 or more. During play:
  - Stances 0–2 and serve stances 32–34 blink at frames 2–4 of their 120-frame clock. The face clock restarts
    whenever the motion is set, so the blink comes after every stop and turn of a rally.
  - Runs 3–7 hold `anger_eye` at exactly 0.5 the whole time. Strokes hold anger at 0.7–1.
  - The tired variants 8–15 hold `sorrow_mouth` at 1. Tired stances also hold `sorrow_eye`.
  - Team reactions 49–52 and 64 hold `joy_mouth` with no `joy_eye`. Team reaction 48 blinks shut from frame 14 to
    frame 92.
- **The textures.** I made a contact sheet of every Open Tee 2 mod in the scratch dir. Its face
  textures are eye-only patches: most characters' joy (`b`) and sorrow (`d`) eyes are drawn closed. Get a Grip's
  are whole-face textures. Their anger picks for Ban, Mitsuzane, Fan, Rachel and Gloria were xx, pained or
  eyes-closed faces.
- **The sources' 30 Hz face tracks.** Get a Grip's `face_tracks.json` and Open Tee 2's TATs, read with the mods
  repo's `opentee_data._tat`:
  - Blinks last 2–4 source frames (0.067–0.13 s). Idles blink once per 1–2 s loop. The closest pair is Get a Grip's
    stroke recovery `tb_end_s`, 0.83 s apart.
  - Get a Grip's shared motions show face 3 (the effort shout) on dash, every shot and serve, which are the moments
    HST plays anger. Stances show neutral.

## Causes
1. **Mouth-only weights shut the eyes.** A mouth weight on its own (tired panting, a team-reaction smile) picked
   the whole texture, and on Open Tee 2 that texture is just the eye. So the eyes closed on sorrow or joy.
2. **Wrong anger picks in Get a Grip.** The anger face is held at half through every run (B585 made 0.5 show).
   Five Get a Grip characters had an xx or eyes-closed anger face, so their eyes stayed shut all rally.
3. **Too fast.** HST's stance blink is above half for only three 60 Hz frames and restarts after every stop or
   turn. Faces also change at each step (run anger → stance neutral → blink → run). On a whole-face texture that
   is a hard snap every few frames, where a morph blink is only a faint flutter.

## Fix
- **hst-base `mods::texture_face`:** an emotion's `_mouth` channel picks no face when the emotion has an `_eye`
  png. It is still a weight. `TextureFace::blink` marks `blink_eye`.
- **hst-base `character::TextureFace::show`:** the `texture_faces` system keeps a `FaceShown` per rig and paces
  the swaps:
  - Every face holds at least 2/30 s, the sources' shortest blink.
  - A blink that would start within 0.8 s of the last blink's start is skipped, and the next strongest channel
    shows instead.
  - A held blink (an emote with the eyes shut) stays.
  - It uses virtual time, so it pauses with the game. Nothing in the sim or the RNG changes.
- **Mods repo (a7b6fbe):**
  - `export_getagrip_faces.py` maps anger to face 3 for every character, the source's own in-play face.
  - Standard §4b now describes the eye rule and the pacing.
  - I regenerated `faces.json` and rewrote the face.json of 10 packs in `out/mods`: Emi, Ban, Wendy, Brad,
    Mitsuzane, Paola, Fan, Rachel, Norma and Gloria. Their `version`s are bumped. The anger textures that are no
    longer used were removed from those packs.
  - Open Tee 2 data is unchanged: its anger textures (`c`) are all open-eyed.
- **Tests:**
  - New `texture_face_paces_its_swaps` (character.rs).
  - `texture_face_follows_the_donors_faces` checks the eye-only picks and still requires every donor face that
    reaches half strength to show something.

## Not verified / not 1:1
- Not compared side by side with the PSP games running. The comparison was against their decoded face tracks only:
  this repo has no PSP emulator driving. In-match presentation is judged by eye per PLAN, and I didn't take a
  `--shot` either, because the change is about timing and a still can't show it.
- The faces still follow the HST donor's `.MOR` weights, not the source games' own per-motion swaps (§4b design).
  So a Get a Grip character shows the effort face through whole runs, where Get a Grip shows it only at the shot.
- The 0.8 s blink gap and the 2/30 s hold are taken from the sources' tracks, not from any rule in the games.
- Tired stances still show sorrow, which is eyes-closed on most Open Tee 2 characters. That is what the donor plays
  there (its sorrow eye), so I left it.
- The mods aren't re-uploaded to the store and the mods commit isn't pushed (unattended run, outward-facing). The
  new versions go up with the next upload.
