# B744 — keep cosmetics out of the online hash

## What the original does (why some cosmetic draws stay)

The match has `rand()` and four MT generators (`hst_sim::rng`). In the original, cosmetics draw from two places:

- **Gameplay sources.** The clouds (5 `rand()` each when a weather change remakes them), the lens flare (24 `rand()`
  a tick in clear/cloudy weather), and the court's and sound manager's reseeds all draw `rand()`. `rand()` reseeds
  the shared generator every point, and the shared generator feeds placement, mis-hits, aims and the AI's reseeds.
  Shouts and voice banks draw the shared generator directly. These draws are the original's, so they **stay** as they
  are: moving them would break the replay tests. They stay in the hash too, since a different count really does change
  the next point. B742 and B743 already made their counts depend only on synced state (snapshot-kept `Sky::made`,
  `Game::weather`).
- **Their own generators.** The court generator (spectators, gallery, emitters), the sound manager's (hit keys,
  bounces, footstep dust, hit sparks, debris) and the flare's ray brightnesses already have their own generators in
  the original. I checked every consumer (`npcs`, `foot_fx`, `bounce_debris`, `effects` sparks, `stroke`'s random
  bit, `intro`, `ceremony`): none reads them back into gameplay, and none of their draws calls `rand()`, `shared` or
  `ai`. They are reseeded from `rand()` at fixed, gameplay-timed points.

So the original already gives pure cosmetics their own generators. What was wrong here is that the state hash
covered them, along with the state only they feed.

## Change

`snapshot::state_text` (so `state_hash`, the rollback/ggrs checksum, the lockstep hash and desync reports) now leaves
out `Game`'s pure cosmetics, `snapshot::Cosmetic`:
- the court and sound generators, the flare rays, the hit sparks' kept generator (`respark`);
- what only they and the sound system feed: the sound queues (`sounds`, `whooshes`), the ambient `emitters`, the
  `gallery`, its `cheers` and the `applause` it takes.

They are swapped out of `Game` while the text is formatted, then put back, with change detection bypassed. Each
machine keeps its own copy. `save`/`restore` still copy them, so a local rollback re-runs them the same way. A
resync still sends them, which does no harm.

## Test

`snapshot::tests::a_rollback_with_other_cosmetics_keeps_the_hash`: machine A and machine B play round1.bin's pads.
At tick 300 B's cosmetics are knocked off (7 court draws, 3 sound draws, flare rays at 0.5, a fresh gallery). B then
rolls the last 8 ticks back every tick. For 900 ticks the state hashes match A's tick for tick, while the cosmetic
texts differ on >100 ticks. B's rollback re-runs reproduce both its state and its cosmetics. Control: with the
cosmetics hashed again it fails at tick 300 ("the cosmetics split the hash").
`tools/check.sh -p hst-play` snapshot (11), rollback (9), netplay (45), weather (6), replay (12) pass. The full suite
runs in done.sh.

## Notes

- Hashes changed for everyone. A pre-B744 desync report's logged tick-0 hash (`replays_a_desync`'s cloud-count
  search) won't match any more. Set `HST_DESYNC_CLOUDS=n` for those, or use a B738+ report with a snapshot. Two builds
  from either side of B744 can't play each other without desync reports (the same was already true for B743).
- The local rollback synctest (`Rollback::sync`) compares `state_text`, so it no longer catches a cosmetic re-run
  bug. This test checks the cosmetics for its own run.

## Not verified / not 1:1

- No PINE check was needed or done. Gameplay draws are untouched, and the replay tests prove they're still exact.
  The claim that the original's court, sound and flare generators feed nothing in gameplay rests on reading our ports'
  consumers, not on the original.
- Not run as a two-device online match. Covered by the headless two-machine rollback test above.
