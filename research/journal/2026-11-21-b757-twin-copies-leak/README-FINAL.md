# B757: split screen kept piling up copies of the backdrop meshes

The cause was traced in R12 (2026-11-21-r12-split-screen-cost). `backdrops()` in `hst/src/main.rs` marked every
descendant mesh of a backdrop as a `Billboard::TURNED`. That included the second view's own copies, because
`twin_view::sync` spawns them as siblings of their originals. A copy with a Billboard drops out of sync's copies
query (`Without<Billboard>`), so sync spawned a new one each frame and the old one stayed drawn on BOTTOM.

## Fix
- `backdrops()`: its parts query now also has `Without<TwinOf>`, so the copies are never marked or posed.
- `twin_view::sync`: belt and braces. When the query can't see a tracked copy, sync now despawns it before
  spawning the replacement, so any other system that marks copies can't leak them.
- Test `twin_view::tests::twin_copies_dont_pile_up`: 3 billboards, second view on for 50 frames, every copy marked
  as a Billboard every other frame. The TwinOf count must equal Twins' copy count and stay at 3. Without the sync
  change it fails at frame 1 (6 vs 3).

## Not verified / not 1:1
- I didn't rerun the igpu bench in this session. The fps numbers on the task line (split 89/79 vs 96 unsplit, mesh
  count flat) are R12's measurement with the same `Without<TwinOf>` change. Presentation only: no sim, RNG or
  timing is touched, so there's no comparison to the original.
