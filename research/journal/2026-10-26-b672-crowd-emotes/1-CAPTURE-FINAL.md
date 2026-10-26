# B672 — what the walkers do after a point, in the original (FINAL)

User report: every spectator claps after each point; in the original some jump or do excited moves.

## Where the remaster stood

`npc::Walker` already had the ordinary reaction, proven against recordings (P14b): the gallery picks three walkers
(all of them when fewer than four), the picked ones clap (clip 5, a ~11-frame loop), the rest play clip 2 (the
~120-frame reaction whose root dips and rises 0.2–0.8 m depending on the set: the "excited"/jumping move). What it
was missing, found in the decompile and checked below:

1. **The gate.** The walkers react to a decided point only when the call was a plain point or the point won a game or
   set. The remaster reacted to every decided point, so after outs, double faults and net-outs (most points between
   computer players) everyone reacted, the clappers in front.
2. **Set and match end** (result kind 2): match not over, or a favoured winner → every walker claps from frame 0
   (picked or not, no draw); an unfavoured winner → every walker plays clip 6 (never used by the remaster) and turns
   to player 0's end.
3. **The match-over message**: a second reaction by the same rule.

Clip lengths/shapes by file (court 11's sets): clip 2 110–120 frames with the root dropping and rising up to 0.65 m
(npc05/npc07), clip 4 the bumped stop (a 0.3–0.6 m root rise), clip 5 9–14 frames flat (the clap), clip 6 54–70
frames with a 0.1–0.26 m rise.

## Captures (`scripts/b672_watch.py`, slot 5: court 10, four computer players, run from the load to the match's end)

Logs: `context/b672/watch_s05b.txt` (plain), `watch_s05c.txt` (`pokeend`). The run is deterministic from the load:
both logs agree point for point up to the poke.

| vsync | call | result | what the walkers did |
|---|---|---|---|
| 8502 and most points | plain | 0 | picks 0,2,3 clap; 1 plays clip 2 (to 8615, then idle) |
| 15045 | net-out | 0 | pick redrawn, **no walker reacted** (mode stays idle) |
| 19529 | net-out | game | three clap, one clip 2 |
| 20111, 21222, … | fault | −1 | nothing |
| 33592 | plain | set (1st) | all four clap, the unpicked walker 3 too |
| 33819 | plain | set (match) | all four clap (all computers → favoured) |
| 33928 | — | match over msg | all four back to "react", clip 5 restarted at 34009 |

With `pokeend` (player 0 made to count as human from 33592 on, after that point's reaction): at 33819 all four play **clip 6**, facing z turned from the winners' side (0.47, …, −0.59) to player 0's
(−0.78, …, −0.91); at 33928 (match over) clip 6 again, and the gallery's pick flags reset to all set (its unfavoured
match-over reset).

Not captured: the one-player mode (the remaster has none), an instant replay's carried-over pick, a human in the
match without the poke.

## Not verified / not 1:1

- The poke stands in for a real human player: only the gallery's "favoured" test reads that word in this window
  (player 0's controls weren't needed after the last point). A real human-vs-computer match end wasn't played.
- The one-player branch and the instant-replay pick carry-over are from the decompile only (no remaster path uses them).
- Spec only: the port (next session) does the side-by-side `tools/screenshot.sh` check.
