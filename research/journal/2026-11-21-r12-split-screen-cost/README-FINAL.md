# R12 Split screen: measure, then fix or recommend removal

**Verdict: salvageable, with a one-line bug fix.** Split screen was slow because of a leak, not because drawing
two views costs too much. Every frame it spawned a new copy of each sky/distant-scenery mesh for the second view
and never removed the old ones. So the second view drew more and more each second: on the igpu bench it fell from
~40 fps to 7 fps within 35 s, and kept falling. With the leak fixed (tried locally only, not committed), split
holds 89 fps side by side and 79 fps stacked at 100 % scale and 4x MSAA on the igpu bench. Unsplit gets 96 fps
in the same scene. No scale or MSAA cap is needed while split on desktop. Queued: the fix (B) and a bench mode that
really splits (T). No removal task.

## Why the earlier split bench didn't split

`tools/bench.sh` always sets `HST_AUTOPLAY=1`. That makes `Pads::slot_of` return no human for any seat, so
`split_view::eligible` is false and `HST_SPLIT2=on` does nothing. `--pads k,-,k,-` itself was fine: in doubles,
1P is player 0 (team 0) and 3P is player 1 (team 1).
`research/split/bench_split.sh` is `tools/bench.sh` without `HST_AUTOPLAY`. The `--pads` humans get no input,
so a bot's serve plays the point and a human's serve leaves everyone standing until the run ends. That scene is
fixed and the same split or unsplit, so the two compare fairly. Screenshots (scratch, not kept)
show both halves drawing: side by side, and stacked.
The stacked layout (the Split Screen setting) isn't applied in a `--play` run: `STACKED` is only set from the menu's
settings (`fit_grid`, pause menu), and `--play` skips the menu. To measure stacked I added a temporary local
`HST_SPLIT2=stacked` switch. It isn't committed; the T task below adds one.

## The leak

`backdrops` (`crates/hst/src/main.rs`, from fa2356e6, 2026-10-26: "the sky and distant scenery stand at each half's
own eye") walks every descendant mesh of each backdrop model and inserts `twin_view::Billboard::TURNED` on any
mesh that lacks it. `twin_view::sync` spawns each original's copy as a sibling (`ChildOf` the original's parent),
so the copies are descendants of the backdrop too, and they get `Billboard` as well. `sync`'s copy query is
`Without<Billboard>`, so from then on it can't find the copy: it spawns a fresh one and overwrites its map
entry. The old copy keeps `TwinOf`, so it is neither an original nor tracked. It stays forever, drawn by the
second view (`BOTTOM` layer, `NoFrustumCulling`).
I probed it with a temporary logging system (not committed). Mesh entities went 706 → 2661 → 4581 at 2 s
intervals, about 32 more each frame. The top groups were 158 copies each of the same four backdrop meshes under
one parent, all `TwinOf`, layer 29. The tracked copy count (`Twins.copies`) stayed at 49–65, and the twin
meshes' vertex counts stayed flat.
The fix I measured with: add `Without<hst_base::twin_view::TwinOf>` to `backdrops`' `parts` query filter. The
mesh count then stayed flat and so did the frame time.

## Numbers

Bench: `HST_GPU=igpu` (Radeon 610M-class Raphael iGPU, the Deck stand-in), `HST_PRESENT=immediate HST_PASSES=1`,
1280×800 window, court 1, doubles, 60 s runs (`HST_GPU=igpu HST_ENV="HST_PRESENT=immediate HST_PASSES=1 [HST_SPLIT2=on|stacked]" research/split/bench_split.sh LOG 60 --pads k,-,k,- [--msaa off] [--scale 67]`; the 4-bot row is `tools/bench.sh` with the same env). Values are averages of the 5 s reports from 15 s on
(`research/split/summarize.py`). "1 % low" is the worst 5 s window's.

| run | fps | 1 % low | CPU ms/frame | game ms |
|---|---|---|---|---|
| unsplit, 4 bots (`tools/bench.sh`) | 91.1 | 27 | 3.16 | 1.02 |
| unsplit, `--pads k,-,k,-` | 96.1 | 47 | 5.40 | 2.06 |
| unsplit, no MSAA | 111.4 | 55 | 4.19 | 1.44 |
| **split side by side, today (leak)** | 10.0 (38.9 at 5 s → 6.9 at 35 s) | 6 | 12.54 | 3.57 |
| split side by side, leak fixed | 89.2 | 42 | 5.83 | 2.21 |
| split stacked, leak fixed | 79.4 | 36 | 5.88 | 2.10 |
| split side by side, fixed, no MSAA | 112.2 | 12* | 5.63 | 2.25 |
| split stacked, fixed, no MSAA | 105.9 | 77 | 3.87 | 1.16 |
| split side by side, fixed, no MSAA, 67 % | 170.2 | 54 | 4.44 | 1.72 |
| split stacked, fixed, no MSAA, 67 % | 163.9 | 112 | 3.62 | 1.02 |

\* one hitch at 15 s; the other windows' lows were 77–100.

Per-pass GPU ms (`HST_PASSES=1`), leaked split: `main_opaque_pass_3d` grew 3.8 → 11.5 → 25.9 → 36.2 → 44.7 →
52.1 → 58.6 ms at 5…35 s. The other passes stayed flat (transparent ~1, msaa_writeback 0.84, shadow 0.2). Unsplit:
opaque 4.7, transparent 0.6, msaa_writeback 0.84, shadow 0.44–0.5, upscaling 0.35. Fixed split: opaque 2.4
(side) / 2.9 (stacked), transparent 0.26 / 0.45, msaa_writeback 0.86 (the first view's only; the second view skips
its own as designed), shadow 0.22 (one pass, the second view reuses it as designed).
Caveat: Bevy's pass report holds one value per pass name, so under split these are likely one view's figures, not
the sum of both. Frame times are the reliable number here.

Two earlier no-MSAA split runs sat at exactly 60.0 fps with 11–14 ms idle a frame. That was a cap, not load:
reruns gave 112 and 106 fps uncapped. I didn't find what capped them: no frame limit was set and `LOW_POWER` is
iOS-only. Probably outside interference (window or compositor state during the run). The table uses the reruns.

Split no-MSAA (112 fps) matching unsplit no-MSAA (111) doesn't mean split is free. On this machine, in this
scene, the frame isn't bound by the GPU's draw work. Unsplit opaque is 4.7 ms of a ~9–10 ms frame. The rest is
spent waiting in acquire/present and on fixed per-frame work, so halving each view's pixels (each half is half the
window) and doubling the draws nearly cancels out. With 4x MSAA, the resolve and the extra view's per-sample work
do show: 89 vs 96. Each run's fps moved ±3–5 between its own 5 s windows, so differences under ~5 fps are noise.

## Verdict

Salvageable. Fixed, split on the igpu bench holds 79–89 fps at full settings (12.6 / 11.2 ms a frame). That is
60 with 4–5 ms headroom, and 1 % lows of 36–42 against unsplit's 47. Split adds ~0.4 ms CPU a frame. Turning MSAA
off while split would add another ~20 fps, but the fix alone already keeps 60 with headroom. I'd hold that knob
for the Deck itself if it falls short. The user's "still so laggy" matches the leak: a split match gets slower
every second, whatever the hardware.

Queued:
- B: the leak fix (`backdrops` skips twin copies) with a test that the copy count stays flat over frames of split.
- T: `tools/bench.sh` mode for split benches (no autoplay, humans on both teams, and a way to pick the stacked
  layout in a `--play` run). It waits on B757 because, until the leak is fixed, a split bench measures the leak's growth rather than split's cost.

## Not verified / not 1:1

- The leak fix was measured locally only. It isn't in this branch (research task); the B task lands it.
- Split screen is the remaster's own (not in the original), so nothing to compare against the original.
- The bench scene is a standstill while a human serves (humans get no input). It isn't a moving rally. Unsplit
  autoplay (91 fps, moving) vs unsplit standstill (96 fps) suggests the scene is representative to within ~5 %.
- The CPU side was measured on the 9800X3D only, not under `HST_SLOW=1` (`bench_split.sh` doesn't do that) or
  on a real Deck. The igpu is the stand-in for the GPU only.
- Per-pass GPU figures under split likely show one view (see caveat above), so the split vs unsplit per-pass comparison is partial. Frame time and CPU ms are complete.
- The stacked numbers used a temporary local switch (not committed). They can be rerun once T30 lands.
- The two 60 fps-capped runs' cause wasn't found (reruns uncapped).
