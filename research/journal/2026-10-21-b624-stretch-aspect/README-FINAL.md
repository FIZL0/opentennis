# B624 — Stretch aspect ratio back — FINAL

## What removed it

B421 (7c4b72fe, 2026-10-15) added Stretch to Aspect Ratio on the phone apps only (or with `HST_SAFE`): the view is
the whole window, the safe area ignored, nothing distorted. B608 (51caa41d, 2026-10-21) made the 3D picture always
fill the window and added the Safe Area setting; it took Stretch out of `aspect_choices` everywhere (a saved Stretch
read back as Auto) and dropped its case from `view_in`.

## Fix

- `widescreen.rs`: `aspect_choices` offers all of `ASPECTS` on every platform; `view_in` returns the whole window at
  `STRETCH`, so the HUD and menus ignore both the device's insets and the Safe Area margin (the picture already
  filled the window, `scene_view`). Nothing stretches non-uniformly: same rule as B421 (the 3D view widens, the HUD
  keeps its 4:3 screen).
- `options.rs`: the Aspect Ratio help text names Stretch; the settings test checks it's offered and a saved
  `aspect = Stretch` reads back.
- Test `views_in_the_safe_area`: Stretch is the whole window past insets + a 5 % margin, sideways and upright.

Shots (`context/b624/Auto.png`, `Stretch.png`, 852×393 with `HST_SAFE=59,0,59,21`, settings.txt `aspect = …`): at Auto
the Settings screen keeps inside the 59 px sides; at Stretch it reaches the window edges and the row reads Stretch.

On a desktop with Safe Area at Auto or None, Stretch looks like Auto (no device insets); it differs once a Safe Area
margin is set.

## Not verified / not 1:1

- No real phone: insets faked with `HST_SAFE`, as in B421/B608.
- No in-match shot; the match HUD uses the same `view` rect as the menus.
- "Stretch" kept B421's meaning (whole screen, undistorted), not a PCSX2-style non-uniform stretch of a 4:3 image:
  PLAN.md's "scale uniformly, never stretch" and B421's own definition. If the user meant a distorting stretch, that's
  a new task.
- Front-end setting, not part of the original: nothing to compare against.
