# B681: CPU Will in the King outfit (outfit 9) and his special serve (FINAL)

User report (2026-10-27): CPU Will King never seems to hit his strong serve. Result: the remaster already plays it
like the original. No code change.

## What Will's special serve is
Will's shot-effect row (GAME.BIN, read through `hst_data::exe`) has a bend only on the **topspin serve** (−0.8,
plus a 10° turn at the first bounce). Will has no special slice. The bend fires only on a strong-toss topspin serve
hit within a frame of the sweet spot (`shot::special`).

## How CPU Will King gets there
- Outfit 9 → AIParam block 3 (hardest), row 53 singles / 137 doubles. The row choice is already checked against the
  game's RAM (`tests/ai.rs` `bots_get_their_rows`, which includes Will in outfit 9).
- Row 53/137: serve level always 2, serve swing ✕ (topspin) 100 %, wide corner 50 %, serve error ±0..2 frames.
  First serves are always strong tosses.
- The serve toss, swing and aim already match the original call by call (`tests/ai_serve.rs`).
- At contact the aim stick still goes through the topspin→flat rule (stick within 60° of straight at the net → flat
  serve, so no bend). In the port, block-3 aims come out 50 % flat: every non-wide aim points forward and every wide
  one stays topspin. 20 000 `serve_aim` draws per side/court, both blocks 3 (rows 53/137) and 0 (rows 11/95, 46 %).
- So about half his first serves are topspin, and about two thirds of those are in time (|error| < 2). That makes
  roughly 1 serve in 3 a bent one.

## Checked against the original
`scripts/cpu_serves.py` (PCSX2 copy, `1p3goodcpus` save state: doubles, Carol human + Will/2/10 CPUs in outfits
9/9/4/9) mashes ✕ for Carol and logs every serve contact: each player's shot code after the aim frame (1 topspin,
8 flat) and the live ball's bend/turn.
- Original, CPU Will: run 5: 7 topspin / 12 flat; run 6: 7 / 9 (35 serves, 40 % topspin; port 50 %, p ≈ 0.16).
  The original flattens CPU serves with the stick too.
- Original ball at the serve (run 6, first five serves, read cleanly): topspin serves bent (bend 0.574, turn −10°),
  flat ones had bend 0.
- Port, same lineup headless (4 CPUs, 40 game minutes): Will 46 serves, 24 topspin / 22 flat, 13 bent (28 %).
  Singles Will King vs CPU: 5 of 12 bent.

So CPU Will King does use his bending serve, about a third of the time, in both games. Most of his other serves are
flat, because his AI aims them at the net, and a flat serve doesn't bend. That's probably what the user noticed.

## Not verified / not 1:1
- The bend rate (the original's 35 sampled serves vs the port's 50 % topspin) is compared statistically, not serve
  by serve. Getting serve-by-serve needs a frame-exact recording of CPU serves. The existing recordings (`1p3goodcpus*`)
  only have the human serving.
- Ball reads after run 6's 5th Will serve came too late (the ball had already reset). Only the first five ball
  values were used.
- Neither game logged any serves by p2/p3 (characters 2 and 10) in these runs. I didn't look into why; it doesn't
  matter for Will.
