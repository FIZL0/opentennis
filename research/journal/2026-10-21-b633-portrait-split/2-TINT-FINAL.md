# B633 follow-up: blue players in the split's bottom half (→ B636)

Seen in `context/b633/run3/a.png`: in the second camera's half every player (A team seen from the front too) is
near-black with a flat blue; umpire, NPCs, racket, ball, court all right. Not the old mod hue shift.

Experiments (450×900, court 1, `HST_WEATHER=0`, `--shot-at 16`; shots in `context/b633/run4/`):
- Swap the rigs between the cameras (main camera gets the bottom view, the new camera the top one): the blue moves
  with the new camera entity, not with the view direction (`swap2.png`).
- Both cameras' components listed at runtime: the same set (Scene, Msaa, NoIndirectDrawing, Unclustered, …); the main
  camera only adds Orbit and a child (the lens flare).
- gs.wgsl fog off (`nofog.png`), `HST_CLEAR=0` (`noclear.png`), `--cull off` (`nocull.png`): still blue.
- gs.wgsl fragment = texture × 0.5, no lighting (`tex.png`): top half's players dim as expected, bottom half's
  pixels unchanged (e.g. (21,24,87) vs (29,32,94)): the players' colour draw doesn't reach the second view at all;
  their depth seems drawn (the court doesn't show through), so what shows is what the image held there.

Next (B636): check whether the players' opaque draws are queued for the second view (skinned + morph + patched
meshes), the second view's pipeline key / MSAA, and what the bottom region holds before the second camera draws.
