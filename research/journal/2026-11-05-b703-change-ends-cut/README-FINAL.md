# B703: the odd zoom-out after a game's celebration

User: after the game-win celebration the camera "zooms out but then pops back in for the next screen"; it doesn't
have to match the original.

## What it was
Headless probe (a bot singles match, per-tick shared camera; also with player 0 owning the camera) and a frame
clip of our build (`HST_CLIP`, `context/b703/run1`):

- The game-end cut-away (shot 0x61 each time) is a calm close-up: eye drifts ~4 mm a tick, fov constant. Not it.
- At a change of ends the picture cut to the first view of scripted shot 2 (B174's port of the original): a
  telephoto (fov 0.175) from 6 cm above the ground, ~6 m to the side and ~8 m behind the server, who filled the
  frame tilted, then swung/pulled back ~30 m to the match camera over the 80 ticks. That's the zoom-out: from a
  close-up nobody had been shown into the wide view, and the cut into it read as the "pop".
- No pop at the serve: the move ends exactly on the view the serve continues from (eye and fov), with or without
  a camera owner. The ghosting in captures is the original's 25 % frame persistence (`hud_gamma::Persist`), intended.
- A game end without a change of ends already cuts straight from the celebration to the serve view.

## Fix
`change_ends::start` now starts the move at its end view, so the change of ends cuts to the match camera's view of
the new set-up on its first tick and holds it (`at` short-circuits equal views), the same cut as a game without a
change of ends. The callers keep their shape: the alternate camera and split-screen halves blend from the first
tick's view of their own shot to the shot as it moves (no ground start any more); the online/own camera holds its
own view. The change-of-ends tests now assert the cut and hold (`views all == to`).

Checked: `tools/check.sh -p hst-play change_ends` (3), `alt_camera` (35), own_camera tests pass; a new clip of
the release build (`context/b703/run3`, frames 2770–2893) shows celebration → cut → steady view under the
"Change Sides" banner → serve, no zoom.

## Not verified / not 1:1
- Deliberately not 1:1: the original glides from shot 2's ground-level view; we cut (user: doesn't have to match).
  Presentation only, the sim and replays are untouched.
- `split_view::tests::each_team_has_its_own_camera` / `each_human_has_their_own_camera` fail on main already
  (`hst_base::twin_view::shared_cascades` wants `DirectionalLightShadowMap`, from the split-screen shadow commit
  b7da0da0), so the split-screen change-of-ends path was only checked by compiling, not by its test.
- Not looked at on a real (drawn) court or with the alternate camera on in a live game; the clips draw no court.

## Wrap-up
`tools/done.sh` can't pass on this branch for reasons outside this task: `tools/patches.py` fails on main's
b7da0da0 (`third_party/bevy_pbr/src/lib.rs` has no `HST (<task>):` marker), and the split_view tests panic on
`twin_view::shared_cascades` (both noted on B704). With `--blocked` it runs the same checks, so I committed by hand
after `cargo clippy -p hst-play --all-targets -D warnings` and `tools/audit.sh` both passed, along with the
change_ends/alt_camera/own_camera tests. B703 is marked `[~]` so it can be reviewed and ticked.

## Outcome (user 2026-11-05)
The user preferred the original's glide from shot 2 after seeing both side by side, so the cut is reverted. The
zoom-out they had reported is the Game Set Match screen's transition after the match-end celebration (B705).
What they wanted fixed here: the next server (Cody in the clip) was drawn lying in his dive from the last point for
the first frames of the move, crossfading up into the serve stance. The idle players already cut to the stand on the
phase's first tick (B535); the server's stance now cuts in on that tick too (`play.rs` `motions`). Checked with a
before/after clip of the same seeded bot match (court 1, seed 1234, frames 4320–4740): standing from the first frame.
`change_ends` tests pass (3). Presentation only: motion commands and the sim are untouched.
