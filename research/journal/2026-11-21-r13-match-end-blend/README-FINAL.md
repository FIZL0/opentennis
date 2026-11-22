# R13 — what the match end's blended draws cost

## Question

After B756 the ceremony and result page still ran ~11 % under a rally on the iGPU bench
(`HST_GPU=igpu HST_ENV='HST_PRESENT=immediate HST_PASSES=1 HST_STATS=4' tools/bench.sh LOG 30`), with
`main_transparent_pass_3d` at 1.3-2.8 ms against ~0.6 in a rally. B756 guessed at the close-ups' blended character
parts and the result page's faded player. Which blended draws are they, and can they be made cheaper without
changing the look?

## How

A throwaway diagnostic (not committed; the patch is in `context/r13/throwaway-census.patch` and
`context/r13/r13_census.rs`), on with `HST_R13=1`:
- every 5 s, the blended GS draws in view grouped by court object / figure, with their test half (`Ge70`, `Lt70`, …);
- `HST_R13_HIDE=…` hides chosen blended draws (by kind, by the root's component, or by the court model's name), to
  time each group;
- at court load, for each alpha-tested (TEST mode 20..29) court material, how many triangles can show a see-through
  texel (P32's `gs::hole_triangles` test) and the texture's alpha spread.

All runs: 30 s benches, court 1, same window and settings, started only when no other `hst --play` ran on the iGPU.
Logs in `context/r13/run1`…`run6`. fps per 5 s window; the ceremony runs to ~17 s, the result page after it.

## Findings

**It is court 1's school fence, not the figures.** Hiding each group's blended draws in the ceremony bench:

| hidden (blended draws only) | fps in the 5 s windows ending at 10 / 15 / 20 / 25 s | transparent pass ms |
|---|---|---|
| nothing | 82.9 / 89.3 / 92.4 / 86.8 | 1.3-2.8 |
| figures (all four, every part) | 83.3 / 89.7 / 92.6 / 86.7 | 1.2-2.7 |
| the backdrop (`Backdrop`) | 82.4 / 90.1 / 93.2 / 88.8 | 1.1-2.8 |
| the eye-tested court objects (`HideAroundEye`: the fences) | 94.2 / 98.4 / 100.4 / 102.5 | 0.4-1.0 |
| `sclwall-01` alone | 92.5 / 94.5 / 93.2 / 94.4 | 0.9-1.6 |
| `sclwall-02` alone | 82.2 / 88.7 / 94.8 / 87.1 | 1.1-2.8 |
| the fences' `Lt70` halves only | 88.1 / 93.0 / 94.0 / 90.4 | 1.0-1.8 |
| the fences' `Ge70` halves only | 83.8 / 89.5 / 92.3 / 89.5 | 1.1-2.3 |
| every blended draw | 91.6 / 96.1 / 100.4 / 100.1 | 0.4-0.6 |
| (plain rally bench, nothing hidden) | 93.8 / 101.3 / 98.4 / 101.4 | 0.6 |

With the fences' blended draws gone the match end runs about as fast as a rally; nothing else in the transparent pass
matters measurably (hiding the figures' blended parts, the faded result-page player or the backdrop moves it by
≤ 0.3 ms and fps by less than the ±2-3 fps run-to-run noise). That holds
for the result page too (the window ending at 25 s): the B756 guess about the faded player was wrong.

**What the fence is.** `sclwall-01` (material `wallnet@vert`, its vertex alpha 0..1) and `sclwall-02` (`wallnet`,
`wallnet2`) are two wire-net fence models standing 0.1 m apart behind one end of court 1 (and `sclwall-01/-03` at the
other). Their material is the original's two-pass alpha test (TEST mode 20..29), drawn ABE: an `A ≥ 0x70` draw (the
wires, colour and Z) and an `A < 0x70` draw (the faint mesh between them, colour only), both blended. The ceremony's
close-up cameras and the result page's circling camera look level across the court, so the fences fill much of the
background behind the figures, every pixel blended, MSAA. In a rally the camera looks down at the court and the fences
are a thin band. Most of the cost is `sclwall-01` (hiding it alone gets back most of the gap; hiding `sclwall-02` alone
moves nothing beyond noise). Part of why: 12 of its 119 triangles have vertex alpha 0 at all three vertices (a faded
edge of the `@vert` net), so neither half can draw a pixel of them, yet both rasterize them, blended (below).

**What can and can't be cut without changing the look.**
- *Dropping triangles by texture* (P32's trick for the court hole model): every fence triangle can show both wire and
  hole texels (119 of 119, 117 of 117, 110 of 110 see-through triangles; the texture is 13.5 % fully opaque and 86 %
  below 0xe0), so by texture neither half has a triangle it can skip.
- *Dropping triangles by vertex alpha*: in MODULATE the GS alpha is At·Af/0x80, so a triangle whose three vertices
  have Af 0 is 0 everywhere: the `A ≥ 0x70` half fails the test and the `A < 0x70` half blends alpha 0, which leaves
  the frame buffer exactly as it was. `sclwall-01` has 12 such triangles (the other fences none). Leaving them out
  (throwaway toggle, two clean interleaved pairs, logs `context/r13/run8*` b/c) took the transparent pass from
  2.90/2.85 to 1.92/1.93 ms in the window ending at 10 s and from 2.52/2.59 to 2.33/1.98 at 25 s; fps was too noisy
  to read (other agents' builds held the load average near 30, and this iGPU shares the CPU's memory). Exactly the
  same picture, so it is queued: **B762**.
- *Skipping the zero-alpha fragments before the blend* (P32's `CLEAR_ZERO`, turned on for every blended draw with a
  throwaway toggle): output identical in theory, but no gain measured (two interleaved pairs: 84.9 vs 85.4 fps over
  5-25 s; transparent pass within ±0.2 ms). The fence's texture has its own averaged mip chain (B161) and is sampled
  by its size on screen, so almost no sample is exactly 0: the faint mesh *is* what the `Lt70` draw paints.
- *Per fragment* each half is already a texture sample and an alpha test before anything else; lighting is per vertex.
  The cost is fill: the number of fence layers on screen times the pixels they cover.
- *MSAA off for the fence layers only*: not possible inside one render pass (the sample count is the target's).
- *Merging the two halves into one draw* would get back at most about a third of the gap (hiding the `Lt70` halves,
  an upper bound since a merged draw still paints those pixels, gives ~91 fps over 5-25 s against ~88 and a rally's
  ~99), and only by changing what the original draws: one draw can't write Z for the wires
  and not for the mesh between them, so either the faint mesh starts hiding what's drawn behind it later or the wires
  stop hiding it. That changes the look and still misses the 5 % target, so it isn't filed.

## Verdict

**Mostly inherent, one exact saving queued.** The remaining ~11-13 % at the match end on the bench is court 1's
see-through fences, drawn as the original draws them (two blended passes per fence model), filling the ceremony and
result-page background. The match end's own additions (banner, confetti, figures' blended parts, the faded player)
cost nothing measurable any more. One look-identical saving: B762 leaves out the court triangles no draw can show
(vertex alpha 0), up to ~1 ms of the transparent pass here. Its fps gain wasn't measurable under load, and with the
rest of the fence inherent the 5 % target likely stays out of reach on this court.

The bench also always picks court 1: on a court without big see-through pieces behind the ceremony's view the match
end should match a rally; on others (chain-link fences, e.g. Academy Grounds) it will depend on how much of the frame
they fill. Not measured.

## Not verified / not 1:1

- Only court 1 (the bench's court) measured; other courts' fences and nets not checked.
- iGPU (Radeon on this desktop) only, a stand-in for the Deck's GPU; not run on the Deck itself.
- The "merge the halves" saving is estimated from hiding the `Lt70` halves (the same pixels go undrawn), not from an
  actual merged draw.
- Bench noise: windows differ by ±2-3 fps between identical runs; each figure is from one or two runs. The B762
  measurement ran under a load average of ~30 (other agents' builds): its transparent-pass drop is consistent in
  both pairs, its fps effect isn't readable.
- Research only: no product code changed.
