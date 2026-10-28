# B693 — mod heads still see-through on iOS 1.5.2

## Cause (found, fixed)
Not morphs and not iOS: it shows on Linux/Vulkan too, for a **left-handed character the human plays**. It reproduces with
`tools/shot.sh … --play --chars 6,1 --outfits 0,0 --slot-mod 0 <left-handed mod> --shot-at 8`, P1 seen from behind.
Her face showed through the back of her head, the back hair was missing, and the front of the top showed as bare skin.
That is inverted back-face culling: one-sided parts kept their *back* faces.

How it happens:
- A left-hander's rig is mirrored (x scale −1). `shade::light` sets each of its GS materials' `key.mirror` so the
  pipeline culls front faces instead of back faces. It only writes when the rig's light key changes.
- `alt_camera::fade` draws the human faded. During the loading screen's pipeline warm-up it does so at full alpha. It
  snapshotted each material's whole `GsKey` before `light` had seen the flip (mirror false). When the fade ended, it
  wrote that whole key back. `mirror` went false again and `light` never rewrote it, so the human's one-sided parts
  were culled the wrong way for the whole match.
- Fix: the fade now changes only what it owns (alpha, TFX modulate, blend). Everything else in the key stays as it is
  now (`faded`/`unfaded`, test `the_fade_keeps_a_mirror_set_meanwhile`).
- Checked: with the same command, the right-handed copy of the mod and the fixed build both draw correctly. Before
  and after crops are in context/ (scratch). The `mod_log` census lists every part as "front faces culled (mirrored)".

Why B691 missed it: B691's desktop runs didn't look at the left-handed human from behind, and the CPU-morph change was
a red herring. With `mod_morph = gpu`, `cpu` or `off`, the fault looked identical.

Ruled out on the way: the CPU morph path (it runs on iOS; the registry fills for mod faces), depth/near plane
(reversed-Z infinite, Depth32Float on both backends), frustum culling (players are `NoFrustumCulling`), pipeline dedup
(compares full descriptors), and negative inverse binds in the mod (0 of 138).

## Debug switches kept for the device (`mod_debug.rs`)
`debug.txt` in the data folder (on iOS the Documents folder the Files app shows, beside `hst-log.txt`). One
`key = value` per line, `#` starts a comment:
- `mod_morph = gpu | cpu | off`: where mod faces morph (off = rest face).
- `mod_hide = face, mae_hair`: hides mod parts whose node/mesh/material name contains a piece.
- `mod_two_sided = on`: draws mod materials two-sided. It hid this bug, which pointed to culling.
- `metal_precise_math = on`: Metal compiles shaders without fast math (wgpu-hal patch, `PRECISE_MATH`).
- `mod_log = on | off`: on by default on Apple.

What to look for in `hst-log.txt`:
- `debug.txt: …`: the switches as read. A `sets nothing` line means a typo.
- `mod faces: morphed on the CPU|GPU`
- `mod <id>: <file> (<bytes> bytes, hash <h>)`: compare with the desktop copy.
- `mod rig <e> (mirrored, left-handed|unmirrored): N parts …`, then one line per part. Each line ends with how it
  culls. **On a mirrored rig every one-sided part must say `front faces culled (mirrored)`**. `back faces culled` there
  is this bug.
- `cpu morph: rig … draws its own face copy` and `… face written`
- `vertex patch: a write dropped …`: should never show.

If the heads still look wrong on a device with this build: put `mod_two_sided = on` in debug.txt. If that fixes it, it
is still culling; send the `mod rig` lines. If not, try `metal_precise_math = on`, then `mod_morph = off`.

Also in this branch: the skinned normal blend in gs.wgsl divides by `max(rest, 1e-6)`, so there is no 0/0 that fast
math may keep. This is defensive; it was not the cause.

## Not verified / not 1:1
- Not tried on an iPhone (no device here). The fault and the fix are platform-independent, and the iOS build checks
  (`tools/ios-build.sh check -p hst --features ship`).
- A stock left-hander (Carol, Will) as the human goes through the same fade and would have had the same fault. Not
  shot here; checked only with the left-handed mod.
- Rendering only (no sim change), so nothing to compare with the original.
