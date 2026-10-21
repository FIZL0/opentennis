# P113 — the reaction voices' view gate (FINAL)

The audit (2026-10-21-repo-audit) flagged `play/reaction_voices.rs`: the post-point reaction voice draws from the
shared generator only for a player in the camera's close view, and the app fed the player's position into that test
for both points it takes (the model origin and the spot). A different verdict changes how many draws happen, and so
the AI's later decisions.

## Recording

`context/p3d1/view_s05.bin` was gone from every checkout, so `hst-sim` `rng.rs` `reaction_views_like_the_game` and
`shared_draws_like_the_game` had been passing by skipping. Re-recorded with
`tools/pcsx2.sh python3 research/journal/2026-10-08-p3d1-reaction-voices/scripts/p3d1_view.py context/p3d1/view_s05.bin 3650` (slot 5, 1 min). Both tests now run
again and pass ("2 in close view, 1 voiced").

## What the two points are

- **Spot:** the player's position for every character except 13. In the recording the other source (used only for
  character 13) holds data that has nothing to do with the player. So the app had this one right.
- **Origin:** not the position. A probe of slot 5 showed the origin is the world matrix of the node straight after
  `Bip01` in the model's node array (0x120 bytes on; the head is 6 nodes on: Bip01 → Pelvis → Spine → Spine1 →
  Spine2 → Neck → Head, the same chain as P17o1), i.e. **`Bip01Pelvis`**: about 0.5–0.9 m above the feet, ±0.15 m in
  x/z, on every frame recorded (14600 player-frames, none equal to the position).
- Effect: the origin is only tested against the near limits (z > 0.001 and focal / z < 1000, i.e. z > 0.24 m). So
  the two points give different verdicts only when a reacting player is within ~1–2 m of the camera plane. The
  recording's closest reacting player in front of the camera was 1.73 m away (origin 1.46 m), and none of its 14600
  player-frames came out differently. Rare, but post-point cameras do get that close, so it is now ported.

## Port

`reaction_voices::pelvis`: the player's `Bip01Pelvis` in game space from `bodyhit::Posed` (the nodes the motion
player last posed; `foot_fx` reads the same bone), through `player_matrix`. It is the view test's origin. Before the
first pose it falls back to the position. The spot stays the position. No play.rs change, no new draws, same
system order.

## Tests

- `tools/check.sh -p hst-play reaction_voices` (matrix_is_local) passed.
- `tools/check.sh -p hst-sim --test rng reaction_views` / `shared_draws` passed, on the fresh recording.

## Not verified / not 1:1

- **Which frame's pose:** `step` runs before this tick's `bodyhit::pose`, so it reads the pelvis posed one tick
  earlier. The recording samples at the end of each frame, so it can't tell whether the game's test sees this
  frame's node or the last one. The two differ by a frame of reaction movement (cm), which only matters inside the
  near-limit band above. Not proved against a lock-step capture of the app.
- **Character 13's spot:** it tests a point of its own. No save state or recording has character 13, so it still uses
  the position (`ponytail:` in the code). Follow-up: **P115**.
- No app-side end-to-end check that the gate gives the game's verdict at each of view_s05's three point ends. The
  sim test checks the gate with the game's own recorded origin, spot and view.
