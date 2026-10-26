# B676c — Carol's off hand in the square stance, side by side with the original

User report (2026-10-26): stock Carol standing square (motion 0) shows her off hand's fingers through the strings
at the head's bottom rim. B676 had matched the clips, frames and the off hand's place in the racket's frame against
live nodes, but had never put the two games next to each other on screen up close.

## Result

**The original shows the same thing.** Seen from the same camera behind her, the original and ours both have the off
hand cradling the head's lower rim: the thumb lies over the strings on her side, and the fingers wrap round the rim
so that the fingertips show through the strings from the far face. The racket's angle, the hand's place on the rim
and the way the strings draw over the fingers match by eye. Nothing after the pose differs: not the skin weights or
finger bones, not the racket model or its scale, not the string alpha or draw order, and not the post-sample head
and leg turns (B675, B676b). No fix needed.

Images (in `context/b676c/`, git-ignored):
- `compare.png`: the original from behind (`orig_hand.png`, cropped from `back_close.png`), ours from the same
  camera (`ours_hand4.png`, from `ours_c0.png`), and our front view (`ours_front_hand.png`). The front view is the
  user's look: fingertips and nails behind the strings.
- `hand_ab4.png` / `close_ab4.png`: the same pair, a wider crop and the full frames.
- `back_2.png`: the original from 2.2 m, height 0.8.
- `front_ours.png`: our front view in outfit 9 and in stock outfit 0. The grip is the same in both; the outfit
  doesn't change the hands.

## Method

The original has no free camera, so `scripts/closeup.py` keeps writing the match camera's eye (all three copies of
its matrix; only the first drives the main view) while the game runs, during the player's motion-0 span. It then
presses the copy's L3+R3 screenshot hotkey through `vpad.py` at a set frame. `scripts/grab.sh` waits until PCSX2 has
finished writing the PNG and copies it out.

- Slot 4: p0 is right-handed Carol (char 6, outfit 9) at (-2.70, 0, -12.25), with an identity player matrix. Her
  50-frame motion-0 span starts about 67 frames after the load.
- Original shots:
  - `HST_PCSX2=2 tools/pcsx2.sh research/journal/2026-10-26-b676c-square-grip/scripts/grab.sh context/b676c/back_close.png 4 0 0 1.3 60 0.6 0 6`
  - the same with `… back_2.png 4 0 0 2.2 60 0.8 0 6`
  - These give an eye 1.3 m behind her and 0.6 m above her point, with the game's own rotation (forward
    (0, .2924, .9563)). The camera's field-of-view field reads 0.417. Motion time at the shot: 47.
- Ours: `tools/shot.sh context/b676c/ours_c0.png --character 6 --outfits 9 --motion 0 --shot-at 0.783` with
  `HST_EYE=0,-0.9801,-1.2432,0,0.2924,0.9563,0,0.9563,-0.2924,0.45`, which is the same eye relative to the player at
  the viewer's origin. t = 0.45 matches the original's framing. For the front view, mirror z:
  `HST_EYE=0,-0.9801,1.2432,0,0.2924,-0.9563,0,0.9563,0.2924,0.45`.
- `scripts/probe.py` (players, motions, camera) and `scripts/spans.py` (one player's motion spans after a load) found
  the slot and the frames to use.

Pitfalls:
- Camera writes while paused do nothing: the pause screen is a frozen still.
- `tools/screenshot.sh` is too slow to catch the span, so the shot is pressed from inside the write loop.
- The F8 shortcut via hyprctl can't find the copy's window. Use the pad hotkey instead.
- The PNG is written late, so wait until its size settles.

## Not verified / not 1:1

- **No front view of the original.** Turning its camera by RAM writes is unreliable:
  - the game's own camera update races the writes, and the view jumps;
  - at a half turn Carol disappears;
  - writing the turn into the matrix's columns instead points the camera at the sky;
  - pinning the field-of-view field throws the camera onto another player.

  Only moving the eye (with the game's own rotation) works, so the comparison is from behind. The fingertips showing
  through the strings from behind is the same overlap the user saw from the front.
- **Framing matched by eye.** In the original, the eye write races the game's camera, and the game blends each frame
  with the previous one (a faint ghost is visible). The sideways shift (`HST_CAM_SHIFT`) didn't take effect. So our
  `HST_EYE` field of view was matched to the original's framing by eye, not taken from the camera's field.
- **Only outfit 9 and right-handed Carol were captured in the original.** The save states have no stock-outfit or
  left-handed Carol. Our outfit 0 grips the same way as our outfit 9. B676 checked the mirror at node level.
