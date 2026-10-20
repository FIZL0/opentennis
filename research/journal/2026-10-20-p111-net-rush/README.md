# P111 — do the bots lob a human at the net? (FINAL)

User's report (2026-10-20): the original's bots lob when the human is close to the net; ours don't.

**Result: they do, in both.** Against a net-rushing P1 the port's bots lob at the same rate as the original's. Their
shot choice reads the opponent's position, and it replays bit for bit against the original on every recorded call.
Nothing was missing, so there's nothing to port. The P11l6 stand-ins (shot target, the hit message's spot) don't
come into it. The lob against a net player comes from the opponent's depth zone (the front third → lob, unless the
contact is high above the stroke height). The `near` test that uses the hit message's spot only runs when the
opponent is *not* in that zone.

## Original (PCSX2 copy 3, `tools/record_ai_rally.py`, P1 driven by `HST_DRIVE=1`)

New recorder switch: `HST_NET=<m>`, how far off the net the driven P1 holds (default 4, as before). Recordings are
in `context/p111/run1/`, copied to `context/fixtures/ai_rally_{singles_,}rush{,25}.bin`.

| Run | Setup | P1 holds | CPU ground strokes (contact kind 0) with P1 inside 4.1 m | Lobs |
|---|---|---|---|---|
| singles_rush | slot 8 = `singles_1p_serve.p2s` (Ashley vs COM row 1 lvl 3), `HST_SINGLES=1`, 5000 f, tags 2,3 | 4 m (at 3.8) | 3 | 3 |
| singles_rush25 | same, 6000 f | 2.5 m (at 2.6–2.7) | 4 | 4 |
| doubles_rush_s3 | slot 3, 6000 f, tags 1,2,3 | 4 m | 8 | 2 (5 topspin, 1 slice) |
| doubles_rush25_s3 | slot 3, 6000 f, tags 2,3 | 2.5 m | 7 | 1 (3 topspin, 3 slice) |

The singles CPU's other presses against the net player were volleys (plan 12) from mid-court. The parser
(per-press button/plan/kind/positions from the exit records) is a scratch script; the same reading is in the test
below.

## Port (`doubles_ai.rs` `net_rush_probe`, ignored)

The same driver against the app (1P human by `Replay` pads, closed loop): hold `P111_NET` m off the net, steer to
the ball coming its way, ○ once within 2.5 m. 30000 ticks each, court 0. Output in `context/p111/port_*.txt`.

| Run | Characters | P1 holds | CPU rally ground strokes (kind 0) | Lobs |
|---|---|---|---|---|
| singles | 0 vs 1 | 4 m (at 4.3) | 22 | 19 |
| singles | 0 vs 1 | 2.5 m | 15 | 15 |
| doubles | 0,1,2,3 | 4 m | 46 | 16 |
| doubles | 0,1,2,3 | 2.5 m | 42 | 13 |

Singles: about 90–100 % lobs in both. Doubles: 20 % (3/15) in the original, 33 % (29/88) in the port. With the
original's 15 samples that is within chance (binomial, p ≈ 0.2), and if anything the port lobs a little more. The
app's △ press launches a lob (`kind::LOB` on the swing).

## Verification

`ai_rally.rs net_rush_matches_the_game`: the four new recordings, every receive/NET/BASE call through
`hst_sim::rally` bit for bit (≈ 29 000 calls), and the lobs over a net player counted from the game's own exits (9).

## Not verified / not 1:1

- **Same rallies, different players.** The port's run isn't a replay of the original's: P1 is closed-loop on both
  sides, the pad timing in PCSX2 is wall-clock, and a frame-exact replay of a point through the app's `step` still
  waits on P11l6. So the comparison is of rates over matched scenarios, not per hit. The decision functions
  themselves are proven exact on the recorded inputs.
- **Characters and courts differ slightly.** Port runs use roster 0/1 (0–3 in doubles) on court 0. The original's
  are the saves' (singles Ashley vs character 1; slot 3's four). AI rows are table-driven and bit-exact (P11a), so
  only which rows were sampled differs.
- **P1 stops at 4.3 m in the port, 3.8 m in the original, with the same "stop within 0.4 m of 4 m" driver.** Most
  likely the driver's cadence (every tick in the port, about every 2 frames plus PINE lag in PCSX2), not
  locomotion. At 2.5 m both sat at 2.6–2.7. Not chased.
- **Doubles sample is small** (15 ground strokes in the original). A bigger original sample would tighten the 20 %
  vs 33 % comparison, but both shot pickers are proven exact on the recorded calls.
