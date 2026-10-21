# B617 — skybox check, stock and modded courts

User report (2026-10-21): some skies show texture seams that don't line up, and their animation jumps.

## What I found

### The jump: Get a Grip court mods (fixed in the mods repo)
- Every Get a Grip court's cloud ring (`sky.glb`) is a yaw-only spin. Its keys are evenly spaced, at a constant
  angle step (c01: 0.72°/s).
- The exporter (`opentennis-mods/tools/psp2gltf/anim.py`) resampled those keys with the i3m squad/hermite
  evaluation. That eases into and out of every key pair, so each ring slowed to a stop at its loop's wrap and then
  set off again. That is the visible "jump".
- c20 (city) was worse. Its keys sit at 0/180/360°, so the quaternion interpolation went the short way back and
  forth: a ±10° wobble plus two 176° one-frame jumps per loop.
- Fix: for yaw-only tracks (x/z quaternion parts ≤ 1e-4 on every key), `anim.py` now unwraps the yaw angle and
  interpolates it linearly before resampling at 60 fps (`spin_keys`, `sample_track(..., spin=True)`). The courts
  exporter passes `spin` for sky nodes only. After re-export, every sky's per-frame step is constant
  (min = max, no outliers), checked with a glb animation dump.
- c13, c14 and c18 had `bg03`/`bg05` cloud rings (200–270 m across) routed to `gimmick.glb`, so they stood in the
  world instead of following the eye. The sky regex now takes any `_bgNN_cloud`.
- The game side (`court_mod.rs` `Looped`: `clip.pose(now % length)`, linear keys, slerp) was already right. It
  just played the bad keys faithfully. No game code change.
- Mods repo commit `75704fa` (on origin/main). Courts 01 03 07 08 09 13 14 15 17 18 20 were re-exported
  into `out/mods` and their `mod.json` versions bumped (c01/03/07/08/09/13/14/15/18 → 2, c17 → 3, c20 → 5).

### Seams
- Stock: sweep of all 11 stock courts, 4 headings each at 25° up (`context/b617/sweep_d/`). Skies are continuous.
  Court 2 was compared with the original at the same camera (`context/b74c/c02_1.png` vs
  `context/b617/run1/port_c02_1.png`): the sky and clouds match.
- Mods: the same sweep for 13 outdoor Get a Grip courts (`context/b617/sweep_m/`), plus zoom crops of the
  cloud-heaviest ones (c09 stadium, c20 city). I found no seam that doesn't line up. The mod sampler is Repeat
  and stock is per material from the GS CLAMP setting (B74-era), both unchanged.
- So the seam the user saw was most likely the c20/c13/c14/c18 clouds. Before this fix they jumped or stood in the
  world as gimmicks, where the ring's ends cross in view. That is a guess from the symptoms; I couldn't reproduce a
  static seam.

### Stock clouds don't move in the original
- Live PINE read on court 10 over 54 s: the sky, `_clo` and `_bg` node matrices kept a constant yaw. The disc has
  no UVA/MTA for sky or cloud models either. Ours are static too, so nothing is missing (user question: "didn't the
  real courts have clouds?" — yes, and we draw them: both `_clo` domes and the singles cloud cards).

## Not verified / not 1:1
- Get a Grip's real spin can't be watched (the PSP EBOOT is encrypted). The constant rate is inferred from evenly
  spaced, equal-step keys.
- Only courts 2 and 10 were compared against the original at a matching camera. The other stock courts were
  checked in ours alone, for continuity.
- The re-exported mods are in the local `out/mods` (and the user's mods folder via the link) but not uploaded to
  the mod store: **the user needs to re-upload the 11 bumped court mods.**
- The mod cloud nodes rest at a 474 cm offset that their animation sets to 0, so the first frame moves the ring a
  little once. Minor, left alone.
- There's no free camera in the original. The hunt for the game's own up-looking cut-aways on court 10
  (`b74b_hunt.py`, slot 5) found three views, but F8 saved no PNG for any of them, so court 10 has no sky
  screenshot from the original. No free-camera follow-up (user: not needed; court 2 already matches).

## Files
- Screenshots: `context/b617/` (sweep_d, sweep_m, run1, drift5, orig).
- No release-notes line: the change is in mods only.
