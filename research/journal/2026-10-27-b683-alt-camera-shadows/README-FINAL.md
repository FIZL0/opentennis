# B683 — alternate camera: shadows pop, background vanishes

## What was wrong
Several passes take a model out of the picture depending on where the camera is:
- the net, when the eye is right at it;
- a prop the eye is buried in or right in front of (`HideAroundEye`, `NearBox`);
- a court mod's `__clear` piece standing between the eye and the court;
- a court mod's near piece;
- a Grand Palais end house the eye stands inside (`HideInside`).

All of them set `Visibility::Hidden`, and Bevy's shadow pass skips models that aren't visible. Hiding a model therefore
also took its shadow off the court.

The default camera sits still at the far end, so each of these is either always on or always off there. The low
alternate ("close") camera follows the player in and out of those zones behind the baseline. On Fighill, the end
fences and walls (`__clear` boxes at z ≈ ±20, 3.4 m tall) flip whenever the eye backs past z ≈ 22, so their shadow
stripes on the court popped in and out. A debug log showed four or more flips per point.

A second problem: the Grand Palais houses carry both `HideInside` and `HideAroundEye`. The two passes wrote the same
`Visibility` every frame and fought over it: 143 toggles logged at one end with the alternate camera.

## Fix
`hst-base/src/shadow.rs`:
- **`EyeHidden`**: one bit per hiding pass, so the passes no longer fight.
- **`EyePasses`**: the system set the hiding passes run in.
- **`eye_layers`**: after those passes, it moves the meshes of a hidden model onto render layer 27 (`EYE_HIDDEN`),
  keeping their old layers so they can be restored. When the model is shown again it puts the old layers back.
- **`sun_layers`**: the match sun casts layers 0, TOP, BOTTOM and EYE_HIDDEN. No camera draws layer 27, but the sun
  still draws it into the shadow map, so a hidden model keeps its shadow. As a side effect, `__clear` pieces shown in
  only one half of split view now cast in both halves too.

Where each pass now lives:
- `net_hide.rs` (net, buried/in-front, near pieces) and `main.rs::hide_inside` set their bit.
- `court_mod::clear_view` puts the piece on `EYE_HIDDEN` through its existing per-split layer switch.

Test: `shadow::tests::eye_hidden_meshes_move_to_the_sun_only_layer_and_back`.

## Checked
- **Fighill (`--court-mod …/gag_c05_street`, `camera.txt` = close).** I took a 1/60 s clip across the fences' flip
  with the fix, and again with layer 27 left off the sun (which reproduces the old behaviour). With the fix, the fence
  stripes stay on the court once the fences hide. Without it, they vanish. Images are in `context/b683/clip_crop.png`
  and `clip_ab.png`.
- **Courts 5 and 7 with the close camera** (`context/b683/r13.png`). They look normal: the house's back wall doesn't
  show inside the house, and the background is drawn.
- `cull.rs` / `lod.rs`: `HST_CULL_CHECK` on court 5 with the alternate camera found nothing outside its box (worst
  vertex 0.0000 m out). Nothing in LOD switches by the alternate camera's view either. Neither file is changed.

## Not verified / not 1:1
- **Queen's Tennis Garden "background vanishes" was not reproduced.** I tried cull on/off, the eye-hide passes
  on/off, split view, and frame scans every 0.5 s (`context/b683/run8`). The only thing that hides there is the
  end-stand box behind the alternate camera (box02, z +28), which is out of view, and its shadow now stays. The dark
  band above the far wall at the box01 end is real geometry; the default camera shows it too. If the user still sees
  a vanish, it needs a capture of the exact moment.
- I did not compare against the original's alternate camera in PCSX2 (it needs menu driving to the camera option on
  court 5). Rendering only has to look right, and the shadows now behave like the default camera's.
- The shadow distance (cascades 80 m / 120 m) is unchanged. No pop came from it in these runs.
- The timeline before a `--shot-at` depends on frame rate, so the before/after clips are from different moments of the
  same kind of point, not the same frame.

## Side note
`plate_emote::a_mods_emote_voices_take_turns` failed in the full suite because the live packaged T-Bone mod now has
six takes of voice program 9. Main already updated the test's expectation, and the same two-line change is applied
here, so the merge is identical.
Main's matching fix for `mods::tests::oob_faces_all` (Toro has no face targets) is applied here for the same reason.
