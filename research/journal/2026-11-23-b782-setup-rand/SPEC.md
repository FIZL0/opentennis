# B782 spec: the match setup's cloud draws and the intro's length at 50 Hz

Two things decide how many shared `rand()` draws a match makes before its first point's shared reseed:
- the clouds the setup makes (5 draws each);
- the intro's length in game ticks (24 lens-flare draws each, in clear or cloudy weather).

The European bot match (court 6 doubles, 50 Hz) made 6 clouds (30 draws) and 327 intro ticks. Play made no clouds
and 394 ticks. This spec gives the rules behind both numbers. They replace the probe's `SETUP_CLOUDS` and
`INTRO_TICKS` hooks.

## 1. The intro's length

### Rule (both programs)
The intro is the court's camera flyover, then the "Start the match!" banner. The phase ends this many ticks after it
starts:

    ticks = flyover + banner + 2

- **Flyover:** the court's camera table (`COURT/NN/CMN.XB` → `cam_cNN_00.dat`: up to 20 records of 40 bytes; the
  first field of each record is a signed 16-bit frame count). Add up each record's frames. A record of 0 frames after
  the first is a cut and costs 2 ticks. The first record's 0 costs nothing.
- **Banner:** the banner animation's length in frames (`AZUMA/INPANE/INPANE.XB0` → `i_gamestart_00.ANI`: its last
  key's tick divided by the ticks per frame).
- **+2:** the camera-done hand-off and the banner-done hand-off, one tick each. The 2 is the same at both rates.

At 60 Hz this is the current rule (`Rngs::intro_ticks_of`), unchanged.

### What the 50 Hz program changes
1. **Camera frame counts are scaled when the table loads.** Each record's frame count that isn't 0 becomes
   `trunc(frames × 0.8333333)`:
   - the count is turned into a float;
   - it is multiplied by the single-precision constant 0.8333333 (bits `3f555555`) with the EE's chopping multiply
     (`hst_sim::ps2` mul, which rounds toward zero);
   - the result is truncated to an integer.

   A 0 stays 0, so a cut still costs 2 ticks. The chop matters: 150 × 0.8333333 is just below 125 and chops to 124.
   Rounding to nearest would give 125.
2. **The banner is 5/6 as long: 75 frames, not 90.** It reported done 76 ticks after the camera, against 91 on the US
   game. That fits its last key (7200) read at 6/5 the ticks per frame (80 → 96), which is what the 50 Hz program's
   animation loaders do. So take it as the last key's tick ÷ (ticks per frame × 6/5).
3. The cut's 2 ticks and the two hand-off ticks stay as they are.

### The counts per court
Retail camera tables; the European disc's tables give the same 60 Hz lengths as the US disc's:

| court | records' frames | 60 Hz | 50 Hz scaled frames | 50 Hz |
|---|---|---|---|---|
| 1 | 0, 160, 0, 110 | 364 | 0, 133, 0, 91 | 303 |
| 2 | 0, 185, 0, 115 | 394 | 0, 154, 0, 95 | 328 |
| 3 | 0, 180, 0, 120 | 394 | 0, 149, 0, 99 | 327 |
| 4 | 0, 120, 0, 180 | 394 | 0, 99, 0, 149 | 327 |
| 5 | 0, 100, 0, 120, 0, 130 | 446 | 0, 83, 0, 99, 0, 108 | 371 |
| 6 | 0, 150, 0, 150 | 394 | 0, 124, 0, 124 | 327 |
| 7 | 0, 130, 0, 170 | 394 | 0, 108, 0, 141 | 328 |
| 8 | 0, 120, 0, 180 | 394 | 0, 99, 0, 149 | 327 |
| 9 | 0, 155, 0, 145 | 394 | 0, 129, 0, 120 | 328 |
| 10 | 0, 120, 0, 180 | 394 | 0, 99, 0, 149 | 327 |
| 11 | 0, 70, 0, 190 | 354 | 0, 58, 0, 158 | 295 |

50 Hz = scaled flyover + 75 + 2. Same in singles and doubles. Courts other than 6 come from the rule, not from a
recording.

### Seen on the European game (court 6 doubles, from the bot-match save state)
- The camera shot reported done at intro tick 250 (124 + 2 + 124). On the US game it is 302.
- The banner reported done at tick 326 (75 frames later, seen one tick after it reached 75). On the US game it is 393.
- Phase 2 came at tick 327, the first point's set-up.
- The `rand()` trace counts exactly 327 ticks of 24 flare draws (M183a1 part 11).

M183a1 part 11 split the 327 as 125 + 125 + 2 + 75 and asked why only one +2 remained. In fact the camera moves are
124 each and both +2s remain: 124 + 2 + 124 + 75 + 2.

## 2. The setup's clouds

### Rule (the same in the US and European programs: this code didn't change)
The setup can make clouds twice. Each time, it first removes every cloud, then spawns a count of clouds with the
spawner P3f3 ported (5 draws each, `Rngs::clouds`). The count is the first game's weather's count, worked out the
way a weather change works it out (`clouds_remade`): the court's envir count for that weather, or the exe's
heavy-rain constant in weathers 3 and 5. Neither spawn happens when the court has no in-season cloud models.

1. **Setup spawn (singles only).** Partway through the setup, a singles match spawns the first game's clouds. No
   "old weather" check applies here. A doubles match skips this step.
2. **First weather set (singles and doubles).** As its last step, the setup sets the court's weather to the first
   game's. The weather holder remembers the weather it last set the clouds for, and compares that with the new one.
   If they differ, the clouds are remade exactly as at a weather change between games:
   - nothing happens when both the remembered and the new weather are heavy rain (3 or 5);
   - otherwise all the clouds are removed and the count is spawned again.

   In singles, the second spawn replaces the first: the clouds on screen are the second set. Then the holder
   remembers the first weather.

**What the holder remembers at the start of a match is never set.** The holder is a fresh heap block, and nothing
writes its remembered-weather field before step 2 reads it. So the value is whatever that memory last held, and it
depends on what earlier screens allocated and freed. It doesn't depend on the court, the weather or the build:
- In the European bot match, the field read 8 before the setup. That was the low byte of a stale pointer left in the
  reused block, the same before and after the block was allocated. 8 isn't a weather, so step 2 remade the clouds:
  6 clouds (court 6's doubles count for clear weather), the recorded 30 draws, at the end of the setup.
- P3f3's two US court 10 doubles setup logs had no cloud draws, so there the stale byte happened to equal the first
  weather.
- P3f's singles menu logs had 100 draws. That fits a setup spawn plus a remake of 50 draws each; the logs didn't break
  the 100 down.

### What the port does
The remake at step 2 is real game behaviour in both modes. Which way it goes depends on a heap byte, so it isn't a
function of the match. The port takes the remembered weather as a setup input, "the weather the clouds were last made
for", whose value is either a weather or *none*:
- **Default: *none*.** No weather equals it, so step 2 always remakes the clouds: doubles draws the count once,
  singles twice. This matches the European bot match and P3f's menu-started singles matches, which are the matches
  started from the menu that were seen.
- **A recording's probe gives its own value.** The European bot match is *none*. A recording whose setup made no
  remake gives the first weather.
- **The rule is the same at 60 Hz.** The remake isn't a 50 Hz change, so the default changes US doubles play too
  (today it makes no setup clouds in doubles). Tests built on US recordings keep passing by giving the value their
  recording had:
  - a setup whose trace shows no setup cloud draws gets the first weather (P3f3's two court 10 doubles logs);
  - one whose offset shows the draws gets *none*. M183a1 noted that US slot 5's offset fits 50 setup draws, which is
    a doubles remake of 10 clouds.
- **Online, both peers use the default.** Today netplay's test harness (`seed_match`) is handed the cloud count by its
  caller (a test-local cell), while `main.rs` works out its own count (singles only). Both must take the setup's cloud
  count from one shared function with the same default. The count also needs the same in-season-models check and the
  same singles/doubles count on both peers. Neither peer may read anything from its own machine's state for it.

The draws go where the setup's cloud draws are now (`Rngs::setup_rand`): after the lens flare's 48 and the change of
ends, and before the setup's later draws, which play already makes (the effects seed and the rest of the "+2" M183a1
saw 3 frames after the +30). M183a1 part 11's experiment put 6 clouds at that spot, and the shared seeds landed
exactly on the recording's. The singles setup spawn comes first, then the remake. No singles setup was traced here,
so nothing shows a draw between them, but nothing rules one out either. P3f's 100 draws were reported as one run of
cloud draws.

### Order check against the European trace
The +30 came on the same frame as the end of the setup. The 2 draws play already makes came 3 frames later, then the
327 intro ticks. With 6 clouds and 327 ticks, the shared seeds land on the recorded outputs (M183a1 part 11's
experiment).

## Open questions
none
