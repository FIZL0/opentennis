# B584 — see-through seam down modded characters' faces

## What it was
Not the geometry. Emma's (fore_pc02) `face` mesh is welded down the centre line (one position per centre vertex;
skin weights and all 16 morph deltas agree across the strip duplicates; no open border at x = 0 besides the mouth).
The centre-line vertices sit at **u = 0** of a half-face texture that the model mirrors, and the texture's sampler
is REPEAT (fore2gltf copies the model's wrap flag; psp2gltf/oob2gltf write one REPEAT sampler). With linear
filtering a sample at u = 0 is half texel 0, half texel W−1, the far edge of the sheet: on Emma's face that column is
near-transparent dark (mean alpha 35), so a dark/see-through line runs down the face. On top of that the game's mod
loader ignored glTF samplers entirely and forced Repeat on every mod image.

Survey (first costume of each mod): 63 of 81 character mods have REPEAT textures with UVs inside [0, 1] touching an
edge (faces, hair, shirts, gloves…).

## Fix
- Game (`hst-base/src/mods.rs` `materials_of`): each glTF texture wraps as its sampler says (repeat / clamp /
  mirror; none = repeat), one image upload per (image, wrap pair). Standard §2 already promised "as the texture
  sampler says" (§4c). Test `textures_wrap_as_their_sampler_says` (hand-built glb, four textures on one image).
  Disc characters untouched.
- Mods repo (`../opentennis-mods` 8c25048): `tools/rerig/edge_clamp.py` sets CLAMP_TO_EDGE on an axis of a texture
  when every UV that samples it stays in [0, 1] (±1e-4) and some vertex is within 4 texels of the edge (half a texel
  at full size, wider at mip levels). Textures on eye parts (skinned to a joint named like `eye`/`me`, a rigid mesh
  under one, or a manifest `eyes` material) keep repeating because the donor's `.UVA` scrolls them. Run by
  `package.py` per costume and by both umpire builders; by hand on packed mods it bumps `version`.
  Ran on all 77 character mods and 12 umpire mods in `out/mods`: every one changed, every version bumped.
  Standard §2 and README updated.

## Checked
- Viewer shots (`--mod DIR --radius`, `HST_VIEW_Y`/`HST_VIEW_YAW`), context/B584/run1/: Emma before/after
  (`ab.png`: the line between the eyes, down the nose and through the lip is gone), Phoebe with all samplers forced
  back to REPEAT vs clamped (`phoebe_ab.png`: the forehead line is gone; her hair parting is painted and unchanged).
- `loads_every_mod` with HST_MODS=../opentennis-mods/out/mods: every character mod loads (its 24 failures are the
  court/umpire mods it doesn't take, by kind, unchanged). Umpire glbs re-parse; `mods::` and `umpire` tests pass.

## Not verified / not 1:1
- The updated mods aren't uploaded to the mod store from this run (outward-facing; the versions are bumped so the
  store sees an update when the user uploads), and the mods repo commit isn't pushed (runner rule: no pushes).
  Until then installed copies keep REPEAT samplers and the seam.
- Only Emma and Phoebe were looked at in the game; the other 87 mods are covered by the rule (clamp only where a
  repeated and clamped sample can differ, at the edge), not by a shot each.
- Emma keeps a tiny grey notch at the middle of her lower lip: a one-triangle material (`231`) in the source model,
  not this seam.
- Texture faces (face.json PNGs, Get a Grip) carry no sampler, so they still repeat.
- Compared against the original only by reasoning: these are other games' models, HST has no reference; render
  match is by eye (PLAN: rendering only has to look right).
