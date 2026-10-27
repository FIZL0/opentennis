# B682 — ball outline missing over the backdrop on court mods

## Cause
Not the sky dome itself: Dubno's distant city and sea is a *blended* piece of the mod's `court.glb`
(`c20_bg_Enkei01..04`, a ring ~190–205 m out round the court), not part of `sky.glb`. Bevy sorts blended draws
back to front by mesh centre; the ring's centre is the court's middle. A ball beyond the net is farther than that,
so its outline draws *before* the ring, and the ring's blended draw then covers the outline's soft edge (the
TEST-25 "alpha < 0x70" half writes no Z). The ball model itself is opaque, so only the ring vanished. A near-side
ball is nearer than the court's middle and drew after the ring, which is why only the far end (the far player's
serve toss, far-side shots) showed it. Over the court floor (opaque) the outline was always fine. Disc courts'
backdrops already sort first (`BACKDROP`, B75); court mods only did that for `sky.glb`.

## Fix
`hst-base/src/court_mod.rs`: a blended court-mod piece whose box reaches 100 m out from the court on every side
(`surrounds_play`) gets its own material copy with sort offset `PANORAMA` (BACKDROP/2: after the sky backdrops,
before everything else), kept in the weather re-light and scroll lists like the original material.
Over all installed court mods the rule picks only distant-view rings: Dubno's city, Canada's mountains, the resort's
and Hiyoko's clouds (test `court_mod_panoramas`, `HST_MODS`). A 9.5/17.9 m "players' room" rule was tried first
and also caught fences, roofs, water and ground decals, which must keep their depth sort.

## Checked
- Remaster shots (`HST_EYE` low camera behind the near baseline, Dubno, all CPU): after the fix the far-side ball
  in front of the city strip shows its dark ring (context/b682/eyeA/s11.5).
- Unit test `court_mod::tests::court_mod_panoramas`.

## Not verified / not 1:1
- No exact before/after frame pair: the remaster's real-time-keyed shots don't replay the same rally twice, and the
  pre-fix run never put the ball in front of the city strip at a shot time. The cause rests on the sort rule
  (Bevy's `Transparent3d` key = view z of the mesh centre + depth bias) and the piece list above.
- Court mods only; the original game has no such courts, so nothing to compare against PCSX2. No gameplay touched.
