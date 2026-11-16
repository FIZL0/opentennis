# M183a: EU 50 Hz play, the rest

Port phase from `research/journal/2026-11-16-m169-eu-50hz/SPEC.md` (sections 4 and 8), after M183.
`match_speed::FIFTY_HZ_SIM` stays **off**: the whole-match proof isn't there yet (below), so the flip and its release
note move to **M183a1**.

## What changed

- **Instant replay decision** (`hst_sim::instant`): `decide_in(point, build)` beside `decide` (= `Build::Us`).
  - The fast-rally gate (10+ shots, a fast kind) replays only within `build.frames(900)` ticks: 900 at 60 Hz, 750 at
    50 Hz.
  - Its speed tests read km/h through `speed_kmh_in(v, build)`: ×50 instead of ×60 at 50 Hz, in the same order of
    operations as the US expression. The thresholds (90, 70, 100 km/h) stay, so the same real-world speed passes.
  - `decide` keeps its signature, so the oracle test (`hst-oracle/tests/instant.rs`) is untouched.
- **The pad recorder's limit** (`play/instant_replay.rs`): a rally cancels its replay at `g.build.frames(1800)`
  ticks: 1800, or 1500 at 50 Hz. The point message calls `decide_in(.., g.build)`.

The spec's "path history" (10816 → 9016 bytes, 2704 → 2246 words, a long limit 1800 → 1500) fits the replay's pad
recorder: 1800 frames × 6 bytes plus a header. The port's recorder is a `Vec` (a serve snapshot plus the pads
since), so it has no fixed buffer to resize. The limit that counts is the cancel above.

## Tests

- `instant::tests::fast_rally_gate_follows_the_tick_rate` (hst-sim): the gate at 750/751/900/901 ticks under US and
  Europe; the same real-world speed gives the same km/h (0.5 m a frame at 60 Hz = 0.6 at 50 Hz); 0.5 m a frame at
  50 Hz (90 km/h) doesn't pass a fast kind's "more than 90".
- `play::fifty_hz::tests::the_european_bot_match_plays_as_recorded` (hst-play, `#[ignore]`, a probe for M183a1):
  the European bot match in play, seeded from the original's match seed, listed phase by phase beside
  `fixtures/m169/eu-bot-match.bin`. It asserts nothing yet.
  - The seed: the recording's later RAM (`eu-bot-late_ee.bin`) holds 0x5d6f8eaf, the 875 477th `rand()` output
    from boot, so `Seed::after(875476)`. The weather schedule and `setup_rand` follow as main.rs does. Line-up 0/1/1/0,
    court 6, doubles, 1 set of 4 games, all CPU.
  - Result: **0 of 144 phases match.** The first serve set-up lasts 135 recorded vsyncs and 215 ticks in play.
    The recorded rally then runs 303 vsyncs; play's runs 39 (a fault) before its point is over.

## Not verified / not 1:1

- **The 50 Hz bot match is not bit-exact** against `eu-bot-match.bin`; the probe diverges at the first serve
  set-up. No US match has ever been compared this way in play either (the US proofs are per system: ball, AI calls,
  flow, motions), so nothing yet tells a 50 Hz gap apart from a general play-vs-original gap. Possible causes:
  play's phase boundaries vs the game's phase byte, what the bots do before the serve (the walk, the serve wait's
  draws), the setup draws, or the start alignment. Moved to **M183a1**, which starts with a US baseline (slot 5).
- **`FIFTY_HZ_SIM` not flipped, no release note**: they wait for that proof (M183a1).
- **The path handler's countdown 30 → 25** isn't ported. The port's replay recorder has no counterpart for it, and
  the spec doesn't say what it counts down. Its 1500 limit is ported (the cancel).
- **The gates aren't checked against a European recording.** `eu-bot-match.bin` has no instant replay of a 750..900-tick
  fast rally. The values come from the spec.
- **The km/h operation order at 50 Hz is assumed** to be the US expression with 50 for 60, as the spec describes it.
  No European oracle run backs it.
- **The ball-effect object's lifetime 1800 → 1500** (`effect.rs`'s `MARK_LIFE`) isn't changed. It is presentation
  and not in this task's files.
