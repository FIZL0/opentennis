# B738: replayable desync reports

B734's iPhone capture couldn't be replayed on the real timeline: the harness can't rebuild production's tick 0, and only
the guest's state was saved. Desync reports now carry what a replay needs.

## What changed

- `Netplay` keeps the last 12 hashed ticks (`kept`): the state text plus the state itself (`snapshot::Snapshot` and the
  instant replay's queue, the same `Arc` rollback puts in ggrs' cell, or a `snapshot::save` in lockstep's gate). A
  rollback re-run of a hashed tick replaces its entry, so the entry is the state ggrs' checksum came from.
- `save_desync(w, why, bad)` writes three new things into the report:
  - `snapshot_tick` / `snapshot_hash` / `snapshot`: the newest kept state before the bad tick, as `snapshot::encode`
    bytes in hex (the resync format, restored and put back as `rollback::confirmed` does).
  - `state_tick` / `state`: now this machine's text **at the bad tick**. Before, it was the last hashed tick, which
    under rollback is often later than the tick ggrs reports.
  - `peers`: `{peer: {tick, state}}`.
- On its first report each machine sends its bad-tick text as `Msg::Diverged { from, tick, text, last }`. It goes in
  96 KB pieces so it fits a browser data channel, and it is a match message, so the host relays it. When a peer's
  last piece arrives the report is written again with it. In the browser that means a second download.
- Tests:
  - `replays_a_desync` with a B738 report prints the first field where the machines differ (`first_difference`
    turns the Debug text's keys and nesting into a path like `rng.ai`). It then decodes the snapshot into a fresh
    match, asserts its hash equals `snapshot_hash`, runs the pads to the bad tick the way the room's ticks run them
    (lockstep `run_on` or rollback `handle`), and compares the result against this machine's state and each peer's.
    Older reports still take B734's path.
  - `rollback_desync_is_saved_and_play_goes_on` now checks both machines' reports (`report_replays`):
    - the snapshot is at 300, the bad tick is 330, and the peer's text is for the same tick;
    - the first difference is named `rng.ai`, which is exactly the field the test nudges;
    - the replay from the snapshot equals the host's state, not the nudged guest's.
  - `a_state_text_goes_in_pieces_that_join_back` covers the text chunking.

## Use

`HST_DESYNC=desync-….json tools/check.sh -p hst-play replays_a_desync -- --ignored --nocapture`. The first lines name
the field where the machines split. Then a straight run from the snapshot tells you which machine it agrees with.

## Not verified / not 1:1

- Not tried on a real two-device desync: there's no new capture. The iPhone 1.7.3 file predates B738 (no snapshot,
  no peer text), so it still takes B734's path, and that path wasn't re-run here (it is unchanged apart from the
  added "no peer's state" line).
- A snapshot put into a fresh harness match restores only what `snapshot::encode` carries. That is the resync format,
  which already keeps two live machines together. Entities outside it, such as the umpire's and walkers' motions,
  stay as the fresh match has them. If one of those ever affected the sim, the replay would go off from the snapshot.
  The hash check at the snapshot tick catches anything that is hashed.
- In 4-player rooms a guest's text reaches the other guests through the host's relay. Only the 2-machine case is
  tested.
- Netplay only; nothing to compare against the original game.
