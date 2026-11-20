# B742: the iPhone 1.7.3 desync at tick 13410, host side

The user sent the host's report (`desync-1795100188-host.json`, pre-B738 format: pads, room, state at 13410, no
snapshot). B734 had only the guest's file. For tests, both are copied into `fixtures/desync-b742/` (`host.json`,
`guest.json`; git-ignored). Run copies are in `context/b742/run1/`.

## The diff

The host's and the guest's state texts at 13410 differ **only in the random generators**. A token diff by field:
`rng.rand` (1 value), `rng.court` (all 624 words), `rng.sound` (all 624 words), `rng.flare.bright` (22 of 24 rays).
Everything else is identical: ball, players, score, phase, `rng.shared`, `rng.ai`, every motion and pad.
`replays_a_desync` with `HST_DESYNC_PEER` names it too: "first differ at `rng.rand`".

How to read that:
- At a new point, `rand()` seeds the shared generator, then the clouds' draws come (5 each, only if the weather
  change remakes the clouds), then it seeds the court generator. The sound generator reseeds after that.
- `shared` is equal on both machines, so `rand()` was still in step when the last point started.
- `court` differs, so the machines split **between those two seeds**. The only thing there is the clouds' draw
  count, `point_clouds` → `Sky::made_count` when `weather::clouds_remade(Sky::made, w)`.
- `sound`, `rand` and the flare (`flare_tick` draws `rand()` every clear/cloudy tick) differ as a result.

## Cause

`Sky` lives in a process global (`stage::SKY`), outside the world. `point_clouds` reads `Sky::made` (the weather the
clouds were last made for) and then writes it. No snapshot kept it. Say a rollback goes back over the first point of
a game with new weather. The re-run point then finds `made` already set to the new weather, takes no cloud draws, and
seeds the court generator from a different `rand()`. That sequence depends on the machine: it happens only on the
machine whose rollback crosses that point, and the 2-0 game point here was rolled back 5 times. The state hash doesn't
cover `made`, so nothing showed until the generators fed a hashed field.

**B734's work doesn't cover it.** B734 only fixed the harness: singles/mod rooms, seeding, the systems. Its
`HST_DESYNC_RERUN` check found nothing because the hst-play harness never sets up `SKY` (main.rs does), so no clouds
are drawn there. B738 only changed what reports carry.

## Fix

- `stage.rs`: `sky_made()` / `set_sky_made()` read and write `Sky::made`. `point_clouds` reaches the sky through
  `with_sky`, which under `cfg(test)` is a thread-local, so a test's sky can't leak cloud draws into other tests'
  matches running in parallel. Production still uses `SKY`.
- `snapshot.rs`: a new `CloudsMade` Wire resource sits last in the module walk. It is refreshed from the sky before
  `save`/`encode` and put back after `restore`/`decode`. So rollbacks, resyncs, instant replays and save games all
  restore it. Older encoded states lack the part and leave the sky as it is.
- `netplay.rs` (test only): with `HST_DESYNC_PEER=other.json`, `replays_a_desync` diffs a pre-B738 report against the
  other machine's own report.
- Test: `snapshot::b742::a_rollback_over_a_weather_change_draws_the_clouds_again`. Save, then a new point on a game
  whose weather changes (4 clouds, so the court seed moves), then restore, then the same point again: all generators
  are alike. With the put-back disabled it fails (`made` stays 2).

## Found on the way (queued, not done)

B743: match ticks read `weather::now()`, which the render-side `weather::apply` sets in `Update`. That covers
`flare_tick`'s `rand()` draws, footstep whooshes into `Game`, the dive thud and npcs. In rollback re-runs it is stale,
so it is a second possible split across a weather change. It wasn't this desync: that one sits between two seeds in a
single `point_clouds` call. Changing when the flare draws happen needs the replay tests, so it's left for its own task.

## Not verified / not 1:1

- **Not reproduced on devices.** The cause is inferred from the state diff (shared equal, court split) plus code
  reading. The regression test shows the mechanism on one machine. No two-device match was replayed. The old report
  has no snapshot, and the harness can't rebuild production's tick 0 (B734), so `replays_a_desync` still stops at
  "no cloud count … starts at the logged hash" after printing the field.
- Which weather change and which rollback set it off isn't pinned down. The reports hold only the 13410 states.
- The hst-play netplay harness still has no sky, so its rollback re-run checks can't catch cloud-draw bugs. A test
  sky is a thread-local, and the Apps' systems may run on other threads.
- Netplay only; nothing to compare against the original game.
