# G2 — Shadows: Original

## What
Settings → Shadows gets a fourth choice, **Original** (`--shadows original`, settings `shadows = original`).
Original Mode (`Graphics::original`, `--original`) draws it whenever shadows aren't Off (`Graphics::shadows_drawn`).
Default stays High. Auto-graphics steps Original down to Low like High.

## How
A single shadow map can't look like the PS2: the game has no shadow map at all. Each static caster gets its own
128² 4-bit texture framed on its own light-space extent (already ported for the shade map: `hst_sim::shade::texture`),
and the hole's ground triangles inside its shadow volume (`shade::receiver_tris`) are drawn again with it projected
along the sun, multiplying the frame buffer. Measured texel sizes on Wild Green (court 8): trees 10–48 cm/texel,
small props 1–5 cm; Bevy's 2048 map is ≈5 cm/texel. So lowering the map's resolution would blur the players and
still not match the trees.

`crates/hst-base/src/shadow/projected.rs` (+ `.wgsl`) builds one draw per caster at court load (`main::load`, under
the game-space root): the receiver triangles with each corner's (s, t) from the caster's light matrix, an R8 texture
of `min(index·0x20, 0xff)` filtered bilinear, a multiply blend (Dst × (1 − darken·texel)), depth-tested with a small
bias, sorted after the hole's draws. Darken follows the hole's own `uniform.shadow` (weather included). While shown,
the props leave the shadow map (`shadow::low`: new `Prop` marker for meshes under a `Caster`); the players stay in it
at High's settings; the spectator/NPC rigs (Minor) don't cast (the PS2's dynamic casters are the players only).

## Checks
- A/B on Wild Green vs the PS2 shot (`context/b157/orig_s08s.png` in the main checkout): `context/g2/run2/cmp.png`
  (PS2 top-left, Original top-right, High bottom-right). Original's tree shadows are soft blobs where the PS2's
  are and where High's are sharp-edged leaf patterns.
- First try showed a checkerboard: Bevy's back-face culling dropped half the strip's triangles → culling off.
- Tests: `graphics::tests::original_shadows` (round trip, flags, `shadows_drawn`, step-down), the options page
  tests, `hst-sim shade`.

## Not verified / not 1:1
- The PS2's shadow under the Wild Green trees looks a little darker/larger than ours by eye: the visible pass's
  CLUT wasn't captured, so the texel strength assumes the shade map's ramp (index·0x20) × the hole's darkening.
  By eye only (rendering rule), no capture comparison.
- The GPU's bilinear on 8-bit texels vs the GS's 4-bit weights; depth bias (16, slope 1) chosen, not from the GS.
- Overlapping casters multiply each other here as on the GS, but the order of draws isn't the game's.
- Players stay in Bevy's shadow map (≈5 cm/texel); the PS2 draws each player its own texture, not measured here,
  so their shadows may be sharper or softer than the original's; the umpire still casts into the map.
- Court mods and court 7 (no shade frame) have no projected draws, so their props keep the map's shadows.
- Only checked on Linux desktop (Vulkan); WebGL2/phones not run.
- Only court 8 was A/B'd against the PS2 and run; other courts weren't played with Original.
