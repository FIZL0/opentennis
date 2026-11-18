# B714: menu cursor a black square after a match — blocked, no repro here

Report (opentennis-mods #1, v1.7.1 021d786, 2560x1600, one pad, Xbox glyphs): back on the main menu after a
finished singles match (Rematch card "Lola vs Glo…", Alpine, 1 set), the hand's image is an opaque black box the
size of the whole padded cursor image (`PAD`× the 64×32 quad), with a thin pink diagonal line ending in a small
knob near its bottom. It looks like the hand mesh drawn with garbage joint matrices (triangles stretched over the
whole view, one degenerate pink sliver), not like a missing clear: the cursor camera clears to `Color::NONE`.

## Tried (Linux, Vulkan, this machine's GPU; release build of d7ffaf03)

All via `tools/shot.sh` + `HST_KEYS`; shots in `target/b714/` (scratch).

1. `--play --pads -,-,-,-` → pause → Return to the Menu (Yes) → title → Enter: hand right.
2. Same with `debug.txt` `gpu_preprocess = off`, `uniform_skins = on` (B706's CPU-preprocessing / uniform-buffer
   skin path, the layout other drivers get): hand right.
3. `HST_MENU=rematch` with an empty preview cache (faces of the rematch players bake while the card shows): hand right.
4. `HST_MENU=rematch` → Rematch → match → quit → select → Escape to Main: hand right.
5. `--play --singles --pads -,- --sets 1 --games 1` played to the end (~200 s), ceremony, stats page, Enter back to
   the title and Main: hand right (`target/b714/full2/a239.png`).

## Read and ruled out (for this machine at least)

- `mode.rs` teardown despawns every mode entity and our crates' resources; the menu's Startup rebuilds the hand, its
  image, camera and light from scratch. Warm-up under the loading screen only toggles `is_active`/visibility back.
- Effects build their meshes whole (`skinned_parts_whole`: no Mesh-detail LOD), so `graphics::MESH` left at the
  match's value doesn't touch the hand.
- Bevy's skin extraction (`third_party/bevy_pbr/src/render/skin.rs`): a skin's joints are written in full only when
  it turns visible, then only joints whose `GlobalTransform` changed. A skin added while its joints' transforms are
  stale would stay mis-posed until they move — a candidate, but it reproduces nowhere here.
- Not ruled out: `graphics::lights` turns `shadow_maps_enabled` on and puts the court's cascades on *every*
  `DirectionalLight`, the cursor's own light included (fresh launch too, so not obviously match-dependent);
  the match's `Graphics` (Auto preset tuning) stays the base resource afterwards; `textures::set` from the match
  isn't undone for the menu. None of them shows here.

## What unblocks

The reporter's platform and GPU (OS, Windows DX12/Vulkan or macOS Metal, GPU model), their graphics settings
(preset, Auto, shadows, render scale) and whether mods / a texture pack were on, or `hst-log.txt` from a run that
shows it. With a machine that reproduces it, a PNG of the cursor image (Readback of `Cursor::image`) right after
the menu comes back would tell a mis-skinned hand from a bad clear.

## Not verified / not 1:1

- Everything: no fix made, the bug doesn't reproduce on this machine (Linux/Vulkan), so no test either.
