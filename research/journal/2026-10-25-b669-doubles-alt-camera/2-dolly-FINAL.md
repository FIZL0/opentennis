# B669 round 2: dolly, merged into main

User picked `dolly` (2026-10-25). It's now the doubles zoom itself (`alt_cam::move_in` after `aim_team`, so the
split halves get it too, with their `width`): the eye moves in along the ground until the nearest human's feet
sit 0.85 of the half-height below the middle, then the half-angle widens (at most 0.6) until every human fits in
0.9 of the half-width. The `HST_ALT_FRAME` options are gone.

User also asked: the match-start intro/banner shows on the normal camera, so a match on the close camera now
starts on the normal one and glides in (the Select glide) once play is live and the intro is gone; that switch
doesn't rewrite `camera.txt`.

Shot (HST_DATA with camera.txt = close, same test case as round 1): `context/b669/run2-far.png`. Not covered: the
glide at the start isn't in a --shot (the intro is off there and the first frames are still loading).
