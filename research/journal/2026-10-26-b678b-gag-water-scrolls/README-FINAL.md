# B678b: Get a Grip courts' other water scrolls

Mods repo (`../opentennis-mods`, commit 77da718, pushed): `SCROLL` in `tools/psp2gltf/export_getagrip_courts.py`
gains courts 3, 6 and 13; rebuilt `gag_c03_denen` (version 3), `gag_c06_canada` (version 2, it had none),
`gag_c13_hiyoko` (version 3) in `out/mods`.

- court03: the "pond" is the irrigation ditch along the rice paddies (`c03_denen_water01`, planar, 12.5 m a repeat):
  a slow drift [0.004, 0.003]. The washtub's water (`c03_ob18_mizu`) stays still.
- court06: the falls (`water02`, two layers) pour down off the cliffs at the far end, `water01` is the river they
  feed, which runs past the court away from them, and `fog01@add` is the mist at their foot (its texture tiles in v
  only, so it only rises). Base and `@add` layers get different speeds/directions so they shimmer.
- court13: the river (`c13_hiyoko_water`, 20 m a repeat) and the sparkle sheet over it (`light01@add`, 25 m), each
  its own way.
- Directions: from per-triangle d(position)/d(uv) of each water mesh in the built glb (raw glb y is up in the
  drawn scene; v grows downwards on the falls), then checked with `HST_EYE` shots (`--play --court-mod`), two
  frames 1 s apart: the court13 river and the court06 lake/river change, and on a close view of the court06 falls the
  best match between the frames is the image shifted ~60 px down (falling). Shots in `context/b678b/run*`.

## Not verified / not 1:1
- All speeds and directions are set by eye: Get a Grip's program animates this water and its data has no track, so
  there is nothing to match (as for court04 in B678). The PSP game itself wasn't run to compare.
- The court13 river's flow direction (along its length) is a guess: nothing in the model says which way it runs.
- The three rebuilt mods aren't uploaded to the mod store yet (outward-facing; same as B678's court04).
