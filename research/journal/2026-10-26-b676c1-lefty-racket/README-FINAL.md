# B676c1 — lefty Carol's off hand through the strings

User (2026-10-26): "still clipping, it should match original". In the original the off hand's fingers curl over the
bottom rim; in ours the hand sat inside the lower strings.

## Method

- Original: doubles on my PCSX2 copy, characters 0, 6 (Carol, COM, left-handed by default), 2, 5; Carol in costume 3
  (R1 three times at select: display "4"); picks confirmed with ✕. `scripts/start.py` starts the match and saves scratch
  slot 8; `scripts/point.py` plays one point from it and saves slot 9 the moment Carol stands at the far net
  (−2.7425, 0, 5.2) in motion 0. Shots at 6× upscale (copy's per-game ini `upscale_multiplier`, `ScreenshotSize = 1`;
  both put back after), copied from `snaps/` once the PNG stops growing (`scripts/grab.sh`; `tools/screenshot.sh`
  often copied a truncated file).
- Camera: `scripts/cam.py` reads the match camera matrix (eye (0, −11.8896, −39.2382), forward (0, .2924, .9563),
  fov .1745). Ours with `HST_EYE` = the same eye, shifted by Carol's x so the relative view is the same
  (`--play --chars 0,1,3,6 --outfits 0,0,0,3 --pads -,-,-,- --shot-at 2`: Carol at the far net at 2 s), once at the
  match's fov and once aimed at her with fov .035.
- Nodes: `b676_nodes.py` (B676's) dumped slot 8 → `fixtures/b676c1/s8.bin` (60 samples). Our pose chain matches it:
  `stance_poses_match_the_game` now runs over it too (off hand in the racket frame within 5 mm).

## Finding

The pose was right; the racket model was mirrored. Side by side at the net (context/b676c1/ab_net.png, ab_fix.png,
ab_match.png) the original's racket head has the frame's dark edge at the bottom-left and the star decal the other
way round from ours; ours had the dark edge at the top. Flipping the racket model in x under its joint gives exactly
the original's look (dark edge, star, fingers over the bottom rim, nails at the rim's front). So the original draws
the racket model unmirrored on a mirrored body: its matrix's x row times the player's hand (the ceremony already did
this for Carol and Will's turned stance). The racket head is not symmetric about the handle axis, and in the
mirrored model the knuckles fall 7–30 mm inside the strings; unmirrored, on or just outside the rim (−1…+20 mm).

## Fix

- `character::RacketPart` on the racket meshes (and on the GS second draws beside them, `shade::own_materials`);
  `character::unmirror_rackets` (PostUpdate, after transform propagation) sets the part's x scale to the sign of its
  joint's determinant, so every racket draws unmirrored: match, select preview, ceremony. Right-handers: no-op.
- Culling: `shade::light` gives the racket draws their own mirror flag from their own matrix (the rest of the rig keeps
  the body's), else the unmirrored racket would cull its front faces.
- Tests: `drawn_pose::lefty_fingers_wrap_the_rim` (slot 8 fixture: knuckles on/outside the strings' outline when
  drawn unmirrored, ≥7 mm inside when mirrored), `character::tests::rackets_draw_unmirrored`.

## Not verified / not 1:1

- The rule (racket x row × hand) is read off screenshots, not the decompile: the dark edge, the star and the hand
  all match the flip, and the ceremony's turned lefties already did it, but no clean-room spec backs it.
- The select-screen preview and the ceremony's left-handed mods over right-handed donors follow the same rule; not
  compared against the original (no lefty mod in the original; the select preview wasn't shot).
- Strings: at the match's zoom ours still show the forearm behind the head a little more than the original's do
  (ours over a dark backdrop here, no court in this worktree; the original's strings blend with the sand). Rendering,
  left by eye.
- Righties: unchanged by construction (determinant > 0 → scale 1); not re-shot.
