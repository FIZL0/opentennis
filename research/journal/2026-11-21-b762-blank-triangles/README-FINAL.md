# B762 — court batches drop the triangles no draw can show

## What

R13 found that 12 of the 119 triangles of court 1's fence `sclwall-01` (`wallnet@vert`) have vertex alpha 0 at all
three corners. In MODULATE the GS alpha is At·Af/0x80, so such a triangle is alpha 0 at every pixel. Neither half of
TEST 20..29 draws anything there: `A ≥ 0x70` fails, and `A < 0x70` blends alpha 0, which leaves the frame buffer as
it was. Yet both halves rasterized them, blended.

- `gs::drawn_triangles(indices, alpha, colour_a)`: per triangle, whether any vertex has Af ≥ 1 (vertex alpha ×
  material alpha, cut to 8 bits the way gs.wesl's vertex stage cuts it).
- `GsMaterial::blank_at_zero()`: whether a draw leaves an alpha-0 pixel untouched (colour, Z and shadow):
  MODULATE, and either a `Ge*` test (it fails, writes nothing, and the shadow pass discards the pixel too) or a
  blended colour-only draw (`Lt*`, `Never`). Not `Always`: a blended always-pass draw still writes Z. Not HIGHLIGHT2:
  its alpha ignores the vertex alpha.
- Court load (`gs_models_anim`, main.rs): a batch whose draws are all `blank_at_zero` keeps only its drawn triangles
  (`drop_blank_triangles`); a batch left with none is not built. Left out: the hole model (P32's own trimming and its
  `CLEAR_ZERO` already cover it), scrolling batches (`.UVA` track), models with a `.MTA` (it can raise a material's
  alpha), and the net (`znet*`: its shadow decal reuses the mesh and its own alpha in place of the material's).
- Shadows: unchanged. The dropped triangles cast nothing before either: the shadow pass runs the same alpha test,
  and the colour-only draws write no depth there. The sun-shade map reads the raw model, which is untouched.

## Check

- Unit test `gs::tests::drawn_triangles_leave_out_alpha_zero`: the Af cut (vertex × material alpha, no vertex
  colours), and which court draws count as blank at alpha 0.
- Fixed-camera A/B (`HST_EYE` at the court centre looking at both ends of court 1, `--stage 1 --shot-at 2`, a
  temporary off-switch that is not committed): before and after are pixel-identical, and two "before" runs are
  identical to each other, so the comparison is exact. Both end fences fill the frame. The ceremony itself
  (`HST_STATS=4 --play` shots at 8/13/22 s) differs from run to run (poses, confetti, the circling camera) even
  with a pinned seed, so it can't be compared pixel for pixel. By eye the fences look the same.
- iGPU bench (`HST_GPU=igpu HST_ENV='HST_PRESENT=immediate HST_PASSES=1 HST_STATS=4' tools/bench.sh LOG 30`), three
  interleaved before/after pairs. `main_transparent_pass_3d` ms per 5 s window (5…25 s):

  | run | 5 | 10 | 15 | 20 | 25 |
  |---|---|---|---|---|---|
  | before 1 | 1.86 | 2.85 | 0.96 | 1.59 | 2.21 |
  | before 2 | 2.01 | 2.02 | 1.59 | 1.06 | 2.53 |
  | before 3 | 1.81 | 2.89 | 1.02 | 1.40 | 2.35 |
  | after 1 | 1.44 | 1.58 | 1.46 | 1.04 | 1.60 |
  | after 2 | 1.06 | 1.14 | 1.02 | 0.92 | 1.40 |
  | after 3 | 1.35 | 1.85 | 1.00 | 1.09 | 1.65 |

  That is about 1 ms less at the ceremony's fence views (10 s) and ~0.8 ms less on the result page (25 s). fps was
  too noisy to read: the load average was 19-38 from other agents' builds, and the first fixed-camera shots
  overlapped "before 2" and "after 1".

## Not verified / not 1:1

- The bench machine was not idle (load average 19-38). The transparent-pass drop is consistent across all three
  pairs, but the fps gain couldn't be measured.
- The ceremony couldn't be compared pixel for pixel: it differs from run to run even before the change. Instead, a
  fixed camera on the same fences gave identical pixels, and the ceremony looked the same by eye.
- Only court 1 was looked at. Other courts' MODULATE batches are trimmed by the same rule, but their pictures and
  timings weren't checked.
- Rendering-only change (no sim, RNG or timing involved). The original still rasterizes these triangles; leaving
  them out draws the same pixels.
