# B633 — tall windows split the match view per team

## What
`crates/hst-play/src/play/split_view.rs` (new): when the picture is narrower than 0.8 (width/height), the
original look is off, and the match is in live play (Serve/Rally, not the intro, no cut-away, no instant replay,
no alternate camera / own-camera glide), the picture splits:
- top half = the main (`Orbit`) camera, the team at the end the match camera looks from (the near team);
- bottom half = a second `Camera3d` (`SplitCam`, order 2, clears nothing; the main camera clears the whole image),
  the other team.
Each half's rig: looks toward the net from behind its team, pitch 28°, centred between the team's players (and the
ball's x when it's on their side), eye no further than 17.5 m from the net (fences; only the main eye gets
`net_hide`'s around-eye hiding), at least 7 m from the view centre; horizontal fit of the players ±1.6 m, vertical
so the deepest feet stand at 70 % below the centre, heads inside. Eased at 5/s, cut on a jump > 3 m (new point).
It runs after `camera` in Update and overwrites the main camera's transform/fov/viewport, so everything that
follows the main camera follows the top half. Leaving live play: viewport cleared, second camera despawned.
- Touch: the bottom half keeps its players above the face buttons (`touch_pad::face_top`).
- Score panels: while split, the near team's panel goes on top (`panel.rs`: `first_near` flipped by
  `split_view::on()`), so each team's panel sits over its own half.
- `own_camera::OwnCam::plain_view` tells it the picture is the plain match view.
Sim untouched: no `Game` field, RNG, timing or stick mapping changed.

## Checked
Shots (`tools/shot.sh`, `context/b633/run1..3/`): 450×900 doubles (court 1) — A team on top, B team bottom,
each panel on its half; 450×900 singles (court 3) — each player big from behind; 1280×720 unchanged.
Unit tests `split_view::tests` (framing from behind, eye limit, players in frame; feet above the buttons).

## Not verified / not 1:1
- Remaster-only presentation (no original counterpart); judged by eye only.
- Billboards (name tags, markers, balloons, sparks, rain…) face the top camera only: mirrored/edge-on in the
  bottom half; lens flare, crowd cull and around-eye hiding follow the top half only → B634.
- Local 2P with humans on both teams: the bottom-team human's stick still follows the shared camera (reversed in
  their half); fixing it needs an input-read-time turn, not `flipped` (sim state / replays) → B635.
- Between points, cut-aways, calls, replays and the change of ends show the one full picture (not split).
- Not tried on a real phone or with the touch pad on (`HST_TOUCH`); the original look (pixel aspect) never splits.
- User's follow-up request (Select-switched split with each human's own camera in 2P singles on mobile) → M146.
