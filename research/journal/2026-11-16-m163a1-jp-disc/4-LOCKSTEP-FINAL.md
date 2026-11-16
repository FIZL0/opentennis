# M163a1a1: the Japanese build checked frame by frame and on the oracle

M163a1a let the Japanese disc play. This task checks the three Japanese rules a frame-by-frame recording or the
oracle can prove: serve grades (rally table), CPU reaction draws (AI generator in step) and aim narrowing. All of
them run against the Japanese program itself. All tests are in `crates/hst-oracle/tests/japan.rs`. They skip
quietly without the JP ISO (`fixtures/discs/SCPS-15113.iso`). With the ISO there, they need their fixtures.

## Recorder: `tools/record_jp.py`
- It loads a save state on `HST_ISO=fixtures/discs/SCPS-15113.iso tools/pcsx2-hst.sh`, then steps frame by frame.
  Each frame it writes the globals, the match object and two blocks per player: the TParam values plus timing
  table, and the swing state.
- It also writes a `.locks` sample (the server's path record) whenever the server's lock index turns non-negative.
- `ai` mode adds the AI generator (MT19937) and each player's AI record.
- The Japanese layout, measured on its RAM (`context/m163a1/` tables):
  - Globals sit 0x1900 above the US ones. The frame clock is a different word, also 60/s. There's a second
    counter that ticks only 20/s, so it is not the clock.
  - The player record has one timing table and no serve tables.
  - The swing state is 0x290 below the US one up to about +0x3c90, and 0x2a0 below past it.
  - The match's characters are in the globals. The player word I first read as the character is the player's
    index.
- Runs (scratch `context/m163a1a1/`):
  - run1: 1200 frames, a smoke test.
  - run2: 60,000 frames, 190 locks, 21 single frames lost to mid-read ticks.
  - run3: 30,000 frames with `ai`, 134 locks, 2 frames lost.
  - All runs start from `fixtures/m163a1a/jp-bot-intro.p2s` loaded as slot 8: Eleanor vs Jun, both CPUs, Aloha
    Beach.
- Copied (`cp --reflink`) into `fixtures/m163a1a1/`:
  - `jp-bot.bin` (run2)
  - `jp-bot-ai.bin` (run3)
  - `jp-bot-ai.rows`: the two CPUs' AI records from the same state's RAM. The test finds each CPU's `AIParam.csv`
    row with them.

## What was checked
- **Aim (`japanese_aim`, oracle, 20k inputs, bit-exact).** I ran the Japanese aim routine on `jp-match.bin`
  (`jp_aim_from` in `research/oracle.txt`) and compared it with `shot::aim_from` and `shot::aim` under
  `Build::Japan`. The routine runs from the base point on: deep button, drop halving, centred-stick nudge and
  plain smash's held depth.
  - Inputs: random hitters, ends, branches, kinds, offsets and depths (|z| 0.3–14), body shots, sticks, two and
    four players, incoming slices.
  - Every target, nudge and held value matches to the bit.
  - Coverage: 2469 smashes in the 6.4–11.885 narrowing, 5792 slices/lobs that never take the 90° widest angle,
    2187 off incoming slices, 5480 nudged, 2183 held.
  - The US rules miss 2722 of the 20k, so the test tells the two builds apart.
- **Timing tables (`japanese_timing_tables`, oracle).** I ran the Japanese table setup (`jp_timing_tables`) against
  `swing::timing` and against `serve::timing` over the widened row. Inputs: the disc's 14 rows plus 10k random
  counts.
  - Grades, length and depth bias all match.
  - The widened rows' serve cells equal the rally cells (28/29).
  - The setup's two memsets only clear the tables. The interpreter has no `pcpyh` (memset's halfword splat), so
    the test stubs memset (`jp_memset`) and zeroes the tables itself. This doesn't need a ps2-oracle release.
- **Serve grades in the match (`japanese_serves_grade_with_the_rally_table`, recording).** At each of the 43 serve
  locks in run2 (32 strong tosses, 11 weak):
  - The recorded grade and depth bias are the timing table's entry at the lock index.
  - That table is our rally table and our serve table for the character.
  - The recorded counts are the disc row's.
- **CPU reactions (`japanese_cpu_reactions`, recording, AI generator in step).** Each time a CPU's reaction wait is
  drawn in run3, the test replays the recorded generator through `AiParams::reaction`. It uses the row parsed from
  the Japanese `AIParam.csv` (widened as the app does) and the Japanese rules.
  - 250 of 251 reactions give the recorded wait in exactly one draw: 28 serve returns, 175 ground strokes, 34
    volleys/dives, 13 after smashes, 5 with the fast-ball extra.
  - That includes **41 incoming lobs**. Their draw sits at the same place as every other shot's, so the Japanese
    game draws no lob half.
  - The US rule would spend one draw more on each of the 41 lobs. The widened row's lob frames are 0, so the wait
    value wouldn't change, but every later draw would fall out of step.
  - Where the draw sits in the frame:
    - It is last in the receiver's own draws.
    - When player 0 receives, player 1's 15 draws follow it.
    - On some serve returns the returns-count draw follows it. The test allows that position only for serves.
    - Player 1's wait has already counted down once by the end of the frame.
  - The AI object's far/near bases are the row's singles reaction values.
- **Found on the way: the Japanese in-memory AI record is 8 bytes shorter.** It is the US one without the
  lob and special-serve reaction words at +0x68; everything after moves up 8 bytes. Our port keeps the US layout
  with those two values at 0 (`hst_data::columns` widening), which is what the game's behaviour needs.
  `AiParams::bytes()` is the US layout, and nothing in the app reads the Japanese RAM, so nothing changes.

## Not verified / not 1:1
- **One serve return's wait (run3, vsync 44645, player 1: 7 frames).** It is below any base (10/12), so no
  reaction draw sets it. It isn't the wrong-guess wait either (the row's is 32). The test lists it as a known case,
  and follow-up **M163a1a1a** finds the writer. The US may do the same: this was not checked against US captures.
- **The serve's lock index itself** (which frame of the path `serve::ai_search` picks) was not re-checked on the
  Japanese recording. The search is the same in both builds and is checked on US recordings (`tests/serve.rs`). I
  didn't decode the recorded path records (`.locks`) for it. Only the grade at the index the game picked is
  checked.
- **Underhand serves:** neither CPU threw one in 60,000 frames, so the grade check covers strong and weak tosses
  only. Underhand uses the same table (oracle check above), but its height window was not seen in play.
- **Doubles:** the bot match is singles. The doubles per-shot routine has the same reaction formula (decompile),
  but no recording covers it, and nor do four-player aims in play. Four-player aims are covered on the oracle
  (`four` is random in `japanese_aim`).
- **Fast-ball frames:** the recording doesn't hold the ball speed, so the test accepts a wait with or without the
  row's fast-ball extra (5 events took it). Whether the speed crossed the threshold is not checked here. That rule
  is unchanged from the US.
- The rally aim's random nudge is checked from the two coins it shows (read back from its result), not from the
  match generator's actual draws.
