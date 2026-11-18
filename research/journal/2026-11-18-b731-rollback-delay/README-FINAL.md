# B731: rollback input delay follows the ping

Rollback always ran ggrs with a fixed 2-tick input delay, whatever the round trip. At 200 ms (6 ticks each way)
~4 ticks of the peer's pads were guesses, and every wrong guess re-ran ticks and warped the other player.

## Change

- `rollback::input_delay(room)`: the host's measured `Room::delay` (`net::delay_at` of the slowest guest's round
  trip, sent to every peer in Start, so every peer uses the same value) clamped to `DELAY`..=`MAX_DELAY` = 2..=4
  ticks. 4 ticks = 67 ms at 60 Hz, the cap so local controls stay responsive. A room with no measured delay (0, the
  tests' rooms) keeps the old 2.
- `Rollback::new` uses it for the P2P session and the `HST_NET_SYNCTEST` session; `Rollback.delay` keeps it (the
  10 s stats line logs it). Restarts (B196 `restart`) rebuild from the same room, so they keep the same delay.
- Lockstep is unchanged (it already used `room.delay`).

## Tests

- `rollback_delay_follows_room`: a 200 ms room (delay 7 → 4) and a delay-0 room (→ 2), each 600 ticks over a
  6-ticks-each-way link: both machines run with the expected delay, ran the same pads, checksums agree. Re-run
  ticks: 511 at delay 4 vs 984 at delay 2. ~5 s.
- `rollback_in_sync`'s `DELAY <= 2` assumption now also checks that a room with no measured delay gets the default.
- Existing rollback tests that use `room(delay_for(200 ms))` (doubles 4-peer, B197 drift) now run at delay 4; they
  pass.

## Not verified / not 1:1

- Not against the original: netplay is our addition (the original has no online play), so there is nothing to compare.
- Not tried over a real long-distance link; only the simulated lagged link in the tests.
