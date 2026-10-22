# B651 — web app restarts on rotation to portrait

## Cause
In v1.3.0 a tall window split the match picture by itself (B633, then B637: two cameras, the second view's copies of
every camera-facing quad, each half its own camera), so turning the phone to portrait set up a second scene view in
the middle of a match. That is the likely memory spike that made WebKit kill and reload the tab. Main had already
dropped the automatic tall split before this task (commit 4bb195f8, "no automatic tall split", shipped in 1.3.5):
the split now starts only on 1P's Select when there's one human per team, and every match starts on the normal
camera. So in portrait the match keeps the normal camera and switches nothing. This task adds the hint.

## What landed
- `crates/hst-play/src/play/camera_hint.rs`: on a tall picture (`scene_aspect < 1`), in live play (serve/rally, no
  cut-away, replay, pause or intro), wherever Select's singles camera works (`alt_camera::human`: one human on this
  machine, offline), a pill under the score panel's top rows says "<Select> Singles camera". Its button follows the
  device last used: the touch pad's SELECT cap, the keyboard's camera key, or a pad's Select glyph in its style.
  Once the alternate camera is switched on (any window shape), `camera_hint.txt` ("seen") goes in the per-user data
  folder (localStorage on the web) and the hint never shows again.
- `legend::chip_image` (the legend's chip as a UI image), `AltCamera::chosen`, and `KeyC` in `HST_KEYS` (for shots).
- Tests: `camera_hint::tests` (when it shows, chip per device).

## Checked
- `--shot` 450×900 singles, keyboard human: the hint shows centred under the top panel; 1280×720: no hint.
- A run with `HST_KEYS=… 12:KeyC`: the alternate camera came on and `camera_hint.txt` was written; a portrait run
  with that file shows no hint, and one without it shows the hint again. Shots are in `context/b651/`.

## Not verified / not 1:1
- The reload itself: no iPhone here, so I couldn't confirm that 1.3.5+ no longer reloads on rotation. The automatic
  split that caused it is gone, but if the reload still happens on a current build, something else allocates on resize.
  `memory_log` (`hst: wasm memory …`) in the page console around a rotation would show it.
- A rotation mid-run: `HST_KEYS` `size=` didn't resize the floating shot window, so I only checked portrait from
  the start, not a turn mid-match.
- The touch pad's SELECT sending the camera switch in a match wasn't driven here (the default binding has Select on
  the camera; only the keyboard's C was pressed).
- The flag lives in its own `camera_hint.txt` beside settings.txt, not in settings.txt: the match crate can't name
  the menus' `Settings`, and settings.txt drops keys it doesn't know when the menu saves it.
- Front-end/HUD hint: ours, not in the original (no 1:1 needed).
