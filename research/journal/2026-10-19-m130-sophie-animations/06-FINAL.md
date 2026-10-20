# 06 — emotes that don't move, racket clipping across every mod (FINAL)

User: "check for clipping for all modded characters, also some modded characters emotes they should be moving?"

## Moving
- The golf mods' reactions keep the root on its spot on purpose (`PIN` in the mods' package.py: the source
  celebrations walk off). Their bodies still move; only the floor travel is held.
- What looked frozen were takes: results-screen poses held almost still (no joint turning 30°), and clips that carry
  the body out of the shot (the pelvis metres up or down: starts or ends off-screen, rises out of the frame, drops
  through the floor). Both are now rules in the mods repo's takes.py, so those clips aren't takes any more.
- takes.py had the per-character take lists committed. Those are results of reading the disc, so they're gone: takes.py
  now picks a mod's takes at build time and caches them in `out/rerig/takes.json` (git-ignored). Mods 940aa78.

## Clipping
- Racket: the check (racket inside the head or hair, or deep in the body, for over 12 % of a clip → hidden for that
  clip) measured the pose before it was written. It now measures the written clip, which is what the game draws. On one
  take the written pose leaned further over than the one measured. Two mods changed.
- Survey over every mod's own clips at 30 Hz (scratch script, not committed): about 200 clips still show the racket
  inside the body for a moment (1–12 % of the clip), and none of the hands sink into the torso. Tennis motions (HST's
  own clips on mod bodies) weren't surveyed.
- 29 mods repackaged, versions bumped, installed.

## Merge
- Branch `m130-m131` (s2): merge-m130 (M130 plus the select's up-front motion check) and task/M131 merged in. The
  conflicts were field lists, `animate` (the off-hand grip, then the ease-in, so the ease blends the final pose) and
  the own-clip length test (merge-m130's `motion_of`). M131 is in DONE.md. Tests: hst-base 193, hst-play 168 and
  hst main_menu 231 all pass.
- Main moved again (3c8573d1) after that. Merging it in here was refused by the session's permission check, so it is
  left for the user.

## Not verified / not 1:1
- The 30° / one-pelvis-height cuts are judgement calls, not taken from any game.
- The brief racket passes below 12 % stay visible. Lowering the share would hide the racket in more clips.
- Not looked at in the running game.
