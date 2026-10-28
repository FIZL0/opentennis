# B691 — see-through heads on iOS (other-game mod characters)

## What the iPhone shots show
In the iOS screenshots (`context/ios-shots-1028/IMG_1426/1428/1429.PNG`) the other-game characters' heads let the
court through where the fringe and necklace should be. Up close, the fringe (`mae_hair`) and necklace are missing,
and the face is rounder and bigger than on desktop, with dark specks along its outline and mouth. The mod's face is
its own mesh (1425 vertices, 16 morph targets) lying just under the fringe and necklace, so a face pushed outward
covers those thin layers. Every material is OPAQUE (lace is MASK), so this is not a blend or sort problem.

## Ruled out on desktop
Desktop `--shot`s of outfits 0/5/9/11 drew correctly under all of these: the phone preset's flags (`--scale 67
--shadows low --msaa 2 --shading off`), `HST_TEXTURE_MAX=512 HST_LOW_MEMORY=1`, `--mesh 50` (decimation),
`--swap-hands`, and the face drawn unmorphed. None of the iOS-only render settings brings the fault back. What's left
is the morph itself: Bevy's storage-buffer morph path on Apple GPUs, the same fault B137a found for the effects in
Safari on the iPhone ("vertices displaced by another mesh's targets or weights").

## Fix
`crates/hst-base/src/cpu_morph.rs`: on Apple platforms (`target_vendor = "apple"`; `HST_CPU_MORPH=1/0` overrides it
anywhere) a mod's face mesh carries no GPU morph targets. The loader (`mods.rs`) keeps its targets in a registry.
`character::spawn` gives the part a `cpu_morph::Face` in place of `MeshMorphWeights`. Each rig then draws its own
copy of the face mesh. When the root's weights change, the copy's moving vertices (base + Σ weight·target) are written
straight into the vertex buffer (`vertex_patch`, no re-upload). The GS's second draws of the same mesh get the
copy as well. Lookups that find a part by its mesh (eye UV offsets in `character.rs`, `inspect.rs`, the ceremony) map a
copy back to its shared mesh through `cpu_morph::shared`. Character Shading still treats the face as a face
(`occlusion.rs`). Disc characters, and every platform other than Apple's, keep Bevy's morphs unchanged.

Desktop check: the same match shot with `HST_CPU_MORPH=0` and `=1` (Sophie mod, outfits 0/11, shot at 14 s, while
the faces were mid-morph per a temporary log). The two shots match apart from particle sparkles. The fringe is
present in both and the faces match.

## Not verified / not 1:1
- Not seen on an iPhone/iPad: no Apple device or Metal here. The fix sidesteps the GPU morph path; the user's next
  iOS build confirms it.
- The exact fault in Bevy's storage morphs on Apple GPUs isn't pinned down (as in B137a). Disc characters on iOS
  still use the GPU morph (the task says leave stock characters alone); their faces weren't reported broken.
- The browser build on iPhone Safari keeps the GPU morph for mod faces (no reliable way to detect an Apple GPU
  there); `HST_CPU_MORPH` can't be set in a browser.
- If a mod's face part were also a costume-sway part, the sway's copy would win and the face wouldn't morph on
  Apple platforms. No current mod has that.
