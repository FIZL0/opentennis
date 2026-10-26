# B664: players are never culled (Suzuki's head and glasses in celebrations)

User report (2026-10-24): stock Suzuki's head and glasses disappear when he celebrates, and the Get-a-Grip Suzuki's
head goes see-through. The user added: other mods show the same missing parts, so fix the root cause.

## Fix
- `cull::Never` (hst-base `cull.rs`) marks a rig whose meshes are never culled. The `never` system runs before
  `apply` and turns every `Cullable` under such a rig into always drawn (`NoFrustumCulling`, no joint-following
  bounds, no longer `Cullable` so the setting and `Off` leave it alone). That covers the parts spawned with the rig
  and those added later: the second GS draws (`shade::own_materials`), the worn handicap pieces and the racket.
- The match marks each player figure `Never` where it gets `Figure(i)`, a one-line change in play.rs. The ceremony
  poses those same figures, so it's covered too. NPCs, the umpire and effects stay culled as before.
- Test: `cull::tests::a_never_rig_is_never_culled` (parts spawned with the rig and added later are drawn unculled
  with the setting on, through `Off` coming and going; another rig stays culled).

## What I found
- I couldn't make it happen here. I tried:
  - bot singles with `HST_CULL_CHECK=1`;
  - in-match celebration cut-aways at 640×480;
  - the end-of-match ceremony (`HST_STATS`) with stock Suzuki and with Get-a-Grip Suzuki (`--slot-mod`);
  - a human Suzuki on the close camera playing all four emotes.

  The head and glasses were always drawn (`context/b664/run1`…`run9`).
- `HST_CULL_CHECK` never found a vertex outside its box. But that run keeps meshes readable in the main world, so it
  doesn't build exactly what a normal run builds.
- Suzuki has no face UV tracks (checked with a throwaway probe), so the eye texture offsets aren't the cause.
- The part that would go missing is the face: five head layers on the `face` node, plus the sunglasses. These are
  the morphed parts. Bevy only extracts a morphed mesh's weights while it is in view, and B277 already hit a Bevy
  bug where a morphed mesh showing again draws without its morph data, or with another mesh's (B277's
  `reextract_morphed` only catches it from the second frame on). A celebration cuts to close-ups, which takes the
  head in and out of view. Never culling a player removes the whole class (wrong bounds and morph data lost on
  coming back in view) for any character or mod. The cost is four rigs drawn off-screen.
- Checked afterwards (`context/b664/run9`): the stock Suzuki ceremony on the close camera with the HD pack draws his
  head and glasses in every frame.

## Not verified / not 1:1
- I couldn't reproduce the reported vanishing on this machine, before or after the fix, so I can't prove this fix
  is what makes it go away. The user should check a celebration with stock Suzuki and with Get-a-Grip Suzuki.
- If it still happens with players unculled, the next suspects are:
  - the close camera's see-through fade (`alt_camera::fade`), which turns every part of the human blended and could
    sort Suzuki's layered head parts wrong;
  - near-plane clipping (0.1 m) in very close shots.
- Nothing is compared against the original here: this is rendering and visibility, not gameplay.

## Wrap-up state
The fix and its test are done (`tools/check.sh -p hst-base cull`: 2 passed). `tools/done.sh` couldn't finish: its
full suite fails only on `mods::tests::oob_hair_swings_and_settles` ("oob_pc00_jasmine: the hair never settled:
0.63"). That test reads the shared `mods/hst-mods/oob_pc00_jasmine`, which B663 rebuilt at 22:48 (the Out of
Bounds resize, opentennis-mods `Out of Bounds sized head to foot…`). B663's slot changes that test's settle frame,
and M147/M148 are parked on the same failure. Once B663 lands: unblock B664 and rerun
`tools/done.sh B664 "B664: players are never culled (heads/glasses missing in celebrations)"`.

## Close-out (2026-10-26)
B663 landed and `oob_hair_swings_and_settles` passes. The fix was already merged into main, but its second cull test
raced `off_unculls_till_it_goes` on the shared `ON` static: they ran in parallel and the B583 test failed now and
then. The two tests now take one lock.
