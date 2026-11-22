# B756 — match end runs slower than a rally

## What it was

The integrated-GPU bench (`HST_GPU=igpu HST_ENV='HST_PRESENT=immediate HST_PASSES=1 HST_STATS=4' tools/bench.sh LOG 30`,
with the live `HYPRLAND_INSTANCE_SIGNATURE` and `WAYLAND_DISPLAY=wayland-1`) reproduced it: ceremony + result page
70-77 fps against ~98 for the same bench without `HST_STATS`.

The hidden cost was a **second 3D camera**. The "Game, Set, Match!" banner sat on the overlay camera's layer
(`popups::CALL_LAYER`, marked `OnCallCamera`), and `popups::call_camera_on` runs that camera whenever such a banner
shows: the whole ceremony and the whole result page. The pass log's `draws` line said so ("2 of 3 cameras active"
in the ceremony, 1 in a rally; a fourth shadow-view list), but its pass timings did not add up to the frame: the
second view's own work (light clustering, GPU preprocessing, its shadow cascades over the scene's casters, its pass
over the full-size MSAA scene image) went to "acquire". P33 had already moved the umpire's calls off that camera for
the same reason (5-7 ms a frame on a phone).

Measured with throwaway env toggles (not committed), 30 s benches, fps per 5 s window from 10 s on:

| run | fps |
|---|---|
| ceremony, as it was | 71-77 |
| overlay camera kept off | 89-92 |
| confetti off | 72-77 |
| both off | 90-96 |
| plain (rally) | 92-102 |

## What changed

- `match_stats.rs`: the banner draws on the match camera. Its space (`BannerSpace`) is a child of the match camera,
  x and y scaled by the match camera's half-angle tangent over the overlay camera's (as `popups::CallSpace` does for
  the calls), so it keeps its pixels and size; its materials are `effects::OnTop` (no depth test) sorted after every
  scene draw (`BANNER_SORT`, the calls' 1e5), no shadows. It no longer carries `OnCallCamera`, so `draw_banner` hides it
  itself when the HUD is off (`no_hud::OFF`, M98).
- `ceremony.rs`: the 400 confetti flakes were 400 entities with 400 blended materials (400 sorted, unbatched draws).
  Now one mesh (`confetti_mesh`): a quad per flake with its colour as a vertex colour, rebuilt each frame in `pose`,
  quads sorted farthest first so they blend as the separate draws did; one material, no frustum culling (the quads
  move), no shadow. Test `confetti_one_mesh`.

Screenshots before/after (`context/b756/before_*.png`, `after_*.png`, `HST_WEATHER=0 HST_STATS=4`, 6/8/13/16 s): the
banner sits at the same place and size in the ceremony and over the result page; the confetti looks the same.

## Result

Same bench after the fix, two runs each, interleaved, nothing else on the GPU:

| run | 5-10 s (ceremony) | 10-15 | 15-20 | 20-25 (result page) |
|---|---|---|---|---|
| ceremony | 80.3 / 82.6 | 87.3 / 89.0 | 91.5 / 89.5 | 85.8 / 83.9 |
| plain | 93.5 / 91.8 | 98.7 / 98.3 | 99.4 / 98.1 | (69.8) / 101.9 |

Second camera gone ("1 of 3 cameras active"), and the pass log now accounts for the frame. No slow-frame line at the
ceremony start, also under `HST_SLOW=1` (2 cores); no pipeline compiles at the ceremony start.

The other suspects:
- per-tick `Game` clone in `hold.rs`: runs in `FixedUpdate`, which the ceremony freezes (timestep 2^20 s), and it runs
  every rally tick too: not a ceremony cost.
- dissolve screenshots: two one-off captures (match → ceremony, ceremony → page), no slow frame logged.
- `ceremony::begin` reading the disc: no slow frame at the start even with 2 cores here.

## Not verified / not 1:1

- **The 5 % target is not met**: ceremony + result page average ~86 fps against ~97 plain (~11 % lower; was ~26 %).
  What is left is in the pass log, not hidden: the main transparent pass is 1.3-2.8 ms against 0.65 in a rally
  (the ceremony's close-ups fill the screen with the characters' blended parts, and on the result page the circling
  camera passes through the shown player, who draws faded), plus ~0.4 ms for the banner and ~0.2 ms of UI. Neither
  confetti nor banner is most of it (toggling each off moved the transparent pass by under 0.4 ms). Shrinking it
  means changing what the scene draws, not a bug in these files.
- The Steam Deck's own log (`~/.local/share/hst/hst-log*.txt`) was not read: `ssh deck` timed out (asleep).
- Bench numbers are from the integrated Radeon on this desktop, a stand-in for the Deck's GPU, and are noisy (other
  agents' benches share it; runs were started only when no other `hst --play` ran).
- Confetti within the one mesh is sorted per frame by depth, as before, but it now sorts as one draw against other
  blended draws (e.g. a faded player): a flake can no longer draw between two other blended objects. Not visible in
  the shots.
- Rendering only (presentation): nothing here reaches the sim.
