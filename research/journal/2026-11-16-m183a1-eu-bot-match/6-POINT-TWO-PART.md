# M183a1 — 6: the US match through point 2's set-up (2026-11-17)

After B723, the US baseline (slot 5's `match_s05`) ended point 1 on the game's tick, but point 2 parted right away.
Its shared generator was reseeded from a different `rand()` output. Player 0 took one more step after the point. The
reaction's root motion started a tick early. Point 2's set-up ran 240 ticks against the game's 178. All four
causes are now fixed from the recording, and point 2's set-up plays its 178 ticks. Every player is where the recording
has them from the first serve to point 2's serve (recorded 1270).

## What changed (play.rs)

- **The sun's lens flare draws before the point's logic.** Its 24 `rand()` draws now run before `control`, where
  they used to run after the sounds. The next point's shared and court reseeds therefore read the same `rand()`
  output as the game's. Found by walking `rand()` back from the recorded reseed (`HST_M183A1_SEED`): play's
  shared generator was one tick's flare off.
- **The reaction reaches the players before anyone moves** (`react_first`, `ReactionRoots`). The game stops a
  running player on the tick the scoreboard's pause ends, where play gave them one more step. The reaction's root
  motion starts the tick after. `react_first` runs the post-point step on copies to see whether this tick reacts.
  If it does, it reacts before `control` and holds the roots until `control` has run. `simulate`'s own step then
  finds `reacted` already set.
- **The AI's routines don't step once the reaction has played** (`bot`: a reacting bot only stands). The game makes
  no AI draws from the reaction (recorded 967) to the next set-up. Play's player 2 still re-rolled its doubles walk
  back 61 ticks after its last roll (play 1011). That one extra draw moved the next serve spot: point 2's server
  walked to the centre where the game's stays wide.
- **A whiffed serve's fault is called once** (`serve_whiff`). The toss lying on the court set the phase back to the
  rally every tick after the fault, even in the point-over phase. With the change above, a bot whiffed on match point,
  and play scored a second "point" after the match was over and panicked. I saw this only with the new AI stop, but
  the old code could do the same whenever a whiffed toss rests during the point-over phase.

## Where it parts now

- **US, point 2.** On one set-up tick (recorded vsync 1226, around the AI server's toss) play draws 64 times from
  the AI generator and the game draws 60. All earlier draws in that set-up land on the game's vsync, mapped by
  `rand()` (play's tick 1176 lands on the game's vsync 1181). From there the serve pick differs: the
  receiver (player 3) runs from 1270 and the served ball is a few ulps off. Separately, the held ball falls a tick
  behind the recording at 1175 without moving a player (the hand-bounce runs one tick longer). The recording can't
  say which routine draws the extra 4 times → **B727** (spec, then port).
- **EU, point 2.** Its set-up still runs 154 ticks against the game's 164. The server tosses at 653 while the game's
  is still bouncing the ball, so the serve wait (an AI draw) differs. `eu-rng.bin` is indexed by vsync, and it has
  loading stalls after 463, so its "recorded" numbers past 463 aren't game ticks. The probe shows the game drawing 6
  AI numbers at vsync 473 that play doesn't. Map the EU vsyncs to ticks through `rand()` (as the US probe does)
  before reading anything into that. The shared generator matches nothing recorded after 465, as before (the EU
  shared and court generators never matched).

## Tests and probes (fifty_hz.rs)

- The harness chain has `react_first` before `control` and `reaction_roots` after it, and `flare_tick` comes first.
- `the_us_bot_match_plays_as_recorded` (ignored) asserts:
  - 4 matching phases;
  - every player on the ground from 219 to 1270, except 1084, the point-over phase's last tick, where play (a tick
    behind) has already placed them for the set-up.
- New probe options:
  - `part_from` (`HST_M183A1_FROM=k`): from recorded tick k, the first tick the phase, a player or the ball parts,
    with the ticks around it. The phase and ball comparisons are plain tick k, so a phase "part" a tick before a
    phase change is the usual one-tick level difference.
  - `HST_M183A1_KEEP=n` for `the_european_generators_play_as_recorded`: play n ticks so `HST_M183A1_DRAWS` can reach
    past 470.
- `tools/check.sh -p hst-play`: 265 passed.

## Not verified / not 1:1
- **Whole-match proof not done.** The US match parts in point 2 (B727), the EU match in point 2's set-up.
  `FIFTY_HZ_SIM` is not flipped and there is no release note.
- **The AI stop after the reaction** comes from the recording, not a spec. No AI draws happen from the reaction to
  the next set-up in the US recording. The rule's other effects (the stick, the walk-back state) aren't proven past
  "the players stand". The EU's 6 draws at vsync 473 might contradict it once mapped to ticks.
- **The whiff-fault fix** isn't checked against the game. I don't know what the original does with a toss still
  rolling after the point-over phase starts. Play now keeps the fault it already called. No release note: the
  double score was only seen with the AI stop.
- **The held ball's extra tick at 1175** (US set-up 2) isn't explained. The players don't move there.
- **The EU shared and court generators** never match the recording. The first 8–9 ticks of the held ball differ.
  The US baseline's `rand()` start is computed. All three as before.
