# M183a1, part 5: after B719 and B720, the European match's first point plays as recorded; both matches start point 2 early (PART: blocked on B723)

B719 and B720 landed. I ran both whole-match probes again:
- US: parts at 855, the gap B722 is open for. Unchanged by this part.
- EU: parts at 303 (was 256). I found and fixed two gaps in play's doubles AI. Neither one needs the decompile. Now
  **every player and the ball match the recording through the whole first point** (to recorded 438). The AI
  generator matches every tick through 435.

| | US (60 Hz, slot 5) | EU (50 Hz, M177) |
|---|---|---|
| players | as recorded through 854 (B722 at 855) | **as recorded through point 1** (was 302) |
| ball | point 1 whole | **point 1 whole** (was 368) |
| AI generator | (B719/B720's checks) | as recorded through 435 (was 301) |
| rally phase of point 1 | 718 ticks, game 720 | 301, game 303 |
| after-point phase | 149 every point, game 154 | 134 every point, game 139 |

## Fixed 1: the server's frame stamp (EU 303)
- **Symptom:** at recorded 302, play's player 2 makes one AI draw the game doesn't make. It then walks back, where
  the game keeps it standing.
- **Where the draw comes from (backtrace):** player 2 is following its partner's shot. It finds the partner's shot
  record overdue (contact frame < now), holds its spot, and repicks the formation. The spot moves, so the walk back
  rerolls.
- **Why the record is overdue:** player 0's record put the contact at frame 300, but the contact lands at 301. Play
  counts the routines' frame up when a player steps a second time. Player 0 starts stepping only after its serve.
  So the count moved up on player 1's step each tick, and player 0's stamps stayed one frame behind its partners'
  for the rest of the point. That is the "frame label lags" B719 noted and left alone.
- **Fix (`doubles_ai.rs`):** the tick's first player to step moves the count up. That is any player at or after a
  seat that already stepped (`stepped >> i != 0`), in both places that count. Player 0 now shares its partners'
  frame.
- **Check:** the US probe is unchanged, and the US records already ended on their contact frame. The doubles, serve
  hand-over and smash-marker tests still pass.

## Fixed 2: a computer player's whiff lets go a frame later (EU 407)
- **Symptom:** player 1 whiffs. Play's routine leaves its swing state and enters wait (the walk-back roll, 1 draw),
  then searches (8 draws), each a tick before the game.
- **Motion captures, from miss pose to run:**
  - `eu-bot-anim.bin`: the miss pose is on the same tick in play and the game (recorded 376). The game runs from the
    miss motion straight into the run 32 frames after the pose, with no stance between.
  - `anim_s05.bin` (US), the same reader: both computer whiffs run 32 frames from pose to run.
  - EU: three clean cases out of six. The other three end in reactions.
  - So this gap is general, not a 50 Hz one.
- **Before the fix:** play let the whiff go at pose + 30 (`WHIFF_RECOVERY`), because the routine's preamble passed
  stick = true.
- **Fix (`whiff_stick`):** the routine holds the stick only from the frame after the recovery. The whiff now lets go
  at pose + 31, so the wait entry and the search land on the game's ticks. The rest of point 1 is exact.
- Human whiffs are unchanged.

## Where both matches part now: the point's end (B723)
- Play ends point 1's rally 2 ticks before the game (phase byte 3 → 4). Its after-point phase is 5 ticks short at
  every point.
- Both builds show the same counts, so the gap is general. The next point's set-up starts 7 ticks early. The lens
  flare draws 24 `rand()` values a tick, so point 2's serve can't match until this is fixed.
- I tried adding her voice timing to the harness (`play_sounds` on `Sound::test`, as `umpire_mod`'s tests do). The
  after-point length didn't change, so I reverted it. The voice isn't what's missing.
- I added B723 for it (files `play.rs`, `flow.rs`, `fifty_hz.rs`).

## Tests (`play::fifty_hz`, in the suite)
- `the_european_first_serve_set_up_plays_as_recorded`:
  - every player bit for bit on the ground from the serve's contact through recorded 437;
  - the ball bit for bit from the return through 437 (was 302/303).
  - Runs 450 ticks, about 1 s.
- `the_european_generators_play_as_recorded`: the AI generator as recorded every tick through 435 (was 301). Still
  leaves out the set-up's first sample and the serve's contact. New option `HST_M183A1_DRAWS=from,to` prints the
  draws a tick for another span.
- Probe commands:
  - `tools/check.sh -p hst-play the_european_bot_match -- --ignored --nocapture`
  - the same with `the_us_bot_match`
  - then read `context/notes/check.log`.

## Not verified / not 1:1
- **Whole-match proof not done.** Both matches start point 2 seven ticks early (B723). The US match also parts at 855
  in point 1 (B722). `FIFTY_HZ_SIM` is not flipped and there is no release note. Both wait until the EU match plays
  as recorded.
- **The whiff release:** I matched it on timing (the AI draws and the run motion's tick), not on the game's rule. For
  one tick (pose + 31), play shows the stance motion where the game's capture still shows the miss motion. Play's
  stick-free tick sets a stance before the run. Positions are identical, so only the motion differs. That's
  presentation, so I didn't chase it.
- **The frame count fix** comes from the recording, not the decompile: player 0's record has to end on the frame its
  partner sees the contact. Only seat order 0..3 is exercised. A human seat between bots counts through `human` the
  same way.
- **The EU shared and court generators** never match the recording (maybe the recorder), as before.
- **The first 8–9 ticks of the held ball** differ in both builds (the hand pose as the walk starts), as before.
- **The US baseline's `rand()` start is computed**, not recorded (slot 5's intro was skipped), as before.
