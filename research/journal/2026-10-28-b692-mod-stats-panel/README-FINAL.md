# B692 — Mod page stat bars on a black panel

**Cause:** the Mods detail column drew the stats box as an ink-rimmed box *filled with ink* (`grid.rs`, the stats
card under the facts line). The meters write their labels in ink, so the labels vanished and the empty steps sat
on near-black. This was in every build, not just iOS: the code path is shared and has no platform branch.

**Fix:** the box is filled with the panel's paper colour inside the same ink rim, the background the character
select's stats sit on. The meters are now inset 8 px from the rim so the labels don't touch it.

**Checked:** desktop `HST_MENU=addmods HST_KIND=0 tools/shot.sh` (shots in `context/b692/`): Serve/Stroke/Volley/
Impact/Footwork labels, coloured steps, light empty steps and the grade letters are all readable on a light panel.

## Not verified / not 1:1
- Not re-shot on iOS (no device here). It runs the same drawing code, so it should match the desktop shot.
- The front-end is ours to design (PLAN.md), so there is no original to match.
