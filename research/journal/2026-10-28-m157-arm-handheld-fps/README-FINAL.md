# M157: ARM handhelds faster on Lowest (RG34XX SP)

The device is an RG34XX SP running muOS 2606, with the Mali-G31 blob and GL ES 3.2 on fbdev (the M155 path). Each
number below is the median frame time over 150 s of an all-CPU doubles match, measured with `HST_FPS=1`.

## Result
The frame went from 39.3 ms to 37.5 ms on Lowest, and to 35.7 ms on the new Potato preset (about 28 fps). The fbdev
swap only offers Fifo, so the frame rate snaps to whole multiples of the vsync interval. The 30 fps floor is close
but not reached, and 60 fps is far off.

Where the time goes: the main thread uses about 77% of a core and the render thread about 85%. The render thread's
~17 ms "submit" hardly moves with the draw count, the uploads or the render scale (going down to 33% changed
nothing). It is driver time spread thinly across libmali, and our code has no lever on it.

## Shipped
- Light visibility is checked only while something casts shadows (graphics.rs `casting`). 39.3 → 37.9.
- bevy_pbr `extract_shadow_lod_origin` returns early when there are no lights (third_party patch, in its README).
- The near-camera hide (`net_hide.rs`) runs again only when the view, the second-player state or a new box changes.
  NPC transforms are written with `set_if_neq`. Together these took off about 0.5 ms.
- New **Court Life** setting (`Graphics::scenery`, `--court-life off`, Options → Advanced) and a **Potato** preset
  (Lowest with Court Life off). Court Life off hides the crowd, the other figures (the umpire stays), the `shadow::Minor`
  props and the costume sway. It hides them through InheritedVisibility between propagation and CheckVisibility,
  because the NPC systems write Visibility themselves. Sway uses its own random generator, so turning it off leaves
  the sim untouched. 37.5 → 35.7.
- Auto picks Potato on fbdev handhelds. The auto-tune ladder turns Court Life off after mesh detail and before render
  scale. While Auto is on, the Preset row shows "Auto (<preset>)".

## Tried and rejected
- A faster ps2-float (inlined/branchless): slower on the A53 (39.3/40.4 against 37.x).
- Thin LTO with codegen-units 1: 36.7 ms, but each build took about 7 minutes longer.
- Merging a player's meshes into fewer draws: 38.8 → 36.0, but the user turned it down because the code was bad.
- Turning render scale down (33% in Potato): no measurable gain, so it was taken out.
- Turning off ball and hit effects: too few draws to matter.
- `HST_PRESENT=immediate`: the fbdev EGL can't do it.

## Follow-up ideas (exact, sim side)
- A broadphase for the court sweep (an XZ grid) in mesh.rs.
- A distance cull before `bodyhit::detect`.
- Skipping repeated pose sampling within a tick.
- Sharing `predicted_path` between its callers within a tick.

## Not verified / not 1:1
- Court Life toggled in the middle of a match on the device: only tested from the flag at start. When it is turned
  back on, the sway comes back only from the next match, as the option's about text says.
- A visual check of Potato on the device screen: only the draw counts were checked (0 crowd draws, no panics).
- The ~17 ms render-thread submit cost in the Mali driver is still unexplained.
- 30 fps is not reached on this device.
