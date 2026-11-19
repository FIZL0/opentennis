# B734: replaying the iPhone 1.7.3 singles + mod desync

Capture: `context/iphone-desync/run1/` (guest's desync.json, hst-log.txt, mods-online/). Singles, court flag 11,
host's mod Wendy (char 12) vs stock char 11, rollback delay 11, ~315 ms ping. The first bad hash was at tick 13410
(local 40871fd2…, peer 8b858d62…), 13380 still matched. That window is right after the 2-0 game point, which was
rolled back 5 times. The guest was in Serve at 13410.

## What changed (netplay.rs tests only)

- `app_in` takes singles rooms: seat order `[0,1]` (it indexed 0,2,1,3 and panicked), `singles: !room.doubles`,
  and the mods are assigned in that order.
- `replays_a_desync` takes the room's mods from `mods-online/` beside the file, so Wendy loads with the looks the guest
  received.
- The match is seeded from `room.seed` as hst `main.rs` does (Seed::shared, Mt, weather schedule, Rngs,
  setup_rand(clouds), effects seed). The cloud count is searched against the log's tick-0 hash
  (`HST_DESYNC_CLOUDS` fixes it).
- `real_tick` adds production's FixedUpdate systems the harness lacked: flare_tick, react_first/reaction_roots,
  play_sounds with a real `Sound`, tick_trails, after_pose, emote lines, bounce debris, and the look/carts/tornado/
  offbeat/replay_pace/save_game plugins.
- `HST_DESYNC_RERUN=n` → `rerun_check`. Every tick it runs n ticks ahead on ggrs-style guesses (remote seats hold their
  last pad, local seats use the real pads). It restores as `rollback::handle` does, including the instant-replay queue,
  and compares each tick's hash with a straight run.

## Findings

- With the seed and all systems: rollbacks of 3, 8 and 19 guessed ticks (19 ≈ this match's ping) on every one of the
  13426 ticks all reach the same states as the straight run. One machine running this build rolls back
  deterministically, including sounds, jingles, the umpire's voice timing and the instant replays. So the split is
  something that differs **between the two machines**, not something the snapshot misses.
- An early harness false alarm (tick 2152, aim) was the instant replay's queue left in place after a restore. The
  harness now mirrors `rollback::handle`. This is not a game bug.
- Candidates that make match state depend on the machine, from reading the code (not confirmed):
  - `play_sounds`: `g.umpire_voice = (played.umpire != 0)`, where `played.umpire` is an audio play id (`Played`,
    not in the snapshot). `heard.voice_left` comes from `Sound::centre_frames` on that machine's umpire bank, and
    `play_as` prefers a bank's mod wav takes when it has any. A different umpire bank or takes on one machine changes
    the post-point timeline (`idle`). This fits a split right after a game call.
  - `Bgm::step` (`g.jingle.take()`, `g.music_hold`) runs only when the machine has `Sound` and the jingle bank.
  - Host and guest may resolve Wendy's data differently (host from installed mods, guest from mods-online).

## Not verified / not 1:1

- **The real match's timeline is not reproduced.** Our tick-0 hash (82a2e796… with 0 cloud draws) never equals the
  phone's 301c05ac… for any cloud count 0..64. Production's setup (intro flare ticks, PostStartup systems, the court's
  cloud models) lives in hst `main.rs` and can't be rebuilt from hst-play. By tick 13410 our replay is mid-rally, while
  the phone was in Serve, so the diverging field could not be named from this capture.
- Only the guest's state was saved, so there is no host state to diff against.
- The candidates above are unverified on a device.
- The v1.7.3 build itself wasn't replayed beyond one run. There are no sim/snapshot changes since that tag.

Follow-up: B738 (save the last in-sync snapshot plus both machines' states in the desync file, then replay from that
snapshot).
