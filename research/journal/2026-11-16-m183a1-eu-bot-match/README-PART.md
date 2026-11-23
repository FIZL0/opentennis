# M183a1: the European bot match in play from the original's seed (PART: blocked on B768 and B769)

Status 2026-11-23 (part 9): B766, B767 and R14 landed. The US match matches 27 phases and parts at game 2's first-serve fault (B769). The EU match still parts at point 2's set-up (164 ticks against play's 154); R14 showed the set-up doesn't wait, and B768 maps the generator test to ticks to find what does differ. The table below is from the first parts.

The goal was to play M177's European bot match (`fixtures/m169/eu-bot-match.bin`) in play and match it phase for
phase, then tick for tick, with a US baseline from slot 5 (`match_s05.bin`). After that, flip
`match_speed::FIFTY_HZ_SIM` and add the release note. Both builds now play the first serve set-up as recorded. Both
part in the same place, the serve's toss and contact, and that gap needs the decompile. So the flip and the release
note wait. I added M184 (spec).

## Harness (`crates/hst-play/src/play/fifty_hz.rs`, tests only)
- `recorded_ticks`: one entry per vsync from the first serve set-up sample (phase byte 2), with the phase, the four
  players' positions and the live ball. A missing vsync repeats the sample before it.
- `played_ticks_rng` builds the match the way `main.rs` does: the weather schedule from the shared seed, then
  `Rngs::new`, `setup_rand` and the effects seed. It runs the FixedUpdate chain, including `flare_tick`, which draws
  24 `rand()` values a tick and was missing at first.
- Alignment: play's tick k+1 sits beside the recording's tick k. Play's setup state is the game's last frame before
  the set-up.
- `compare` reports the phases, plus where players and ball first part at shifts 0..2. `first_serve` measures the
  first serve set-up. A torn sample (the ball read a tick early or late) still counts as a match.
- Tests:
  - `the_european_first_serve_set_up_plays_as_recorded` and `the_us_first_serve_set_up_plays_as_recorded`
    (about 1 s, they run in the suite) assert the spans below.
  - The ignored probes `the_european_bot_match_plays_as_recorded` and `the_us_bot_match_plays_as_recorded` run the
    whole match. `HST_M183A1_TOSS_AI=n` adds n AI draws at each toss; `HST_M183A1_DUMP` prints the first 240 ticks.
  - `eu_match_seed_search` (ignored, one-off) is described below.

## Seeds
- **EU: the brief's seed was wrong.** `Seed::after(875_476)` is the reseed of the 3–3 point, as the late RAM dump
  holds it (globals +0xbc, the US match-seed slot, and again at +0x1fc). It isn't the match seed. I matched `eu-bot-late_ee.bin`'s whole
  weather schedule (65 weathers, plus wind speed and direction) against each earlier `rand()` output. The match seed
  is output 0x5bf93473, the 81 875th from boot: `Seed::after(81_874)`. With the old seed the server walked to the
  centre mark. With this one it walks to 2.76 like the recording.
- **US: slot 5's intro was skipped.** The schedule seed is 0x28c7c4a1 (P3d3). But at the first point `rand()` stands
  2642 draws past it. A whole intro's lens flare takes 394 × 24 = 9456, so the intro was cut short by a button. The
  probe starts `rand()` where play's setup (50 setup draws, the sound reseed, the whole intro) lands on the
  recording's state at vsync 7565. To do that it walks the LCG back (`rand_back`). It keeps the shared generator on
  the schedule seed. The EU match had its intro played out, so its seed needs no correction.

## What matches
| | US (60 Hz, slot 5) | EU (50 Hz, M177) |
|---|---|---|
| players, phase | ticks 0–215: whole set-up, to the serve press | ticks 0–134: whole set-up, to the press |
| held ball | 9–168, bit for bit | 8–112, bit for bit |
| `rand`, shared, AI generators | every tick to the toss (`rng_s05.bin`) | no generator recording |

Before these fixes, 0 phases matched in either build (113 US, 144 EU). The first gap was the same in both, so it is
general, not a 50 Hz one.

## Where they part: the serve (M184)
1. **Toss timing.** The game's toss flies the same curve as play's, bit for bit, but it starts later: 1 tick at
   60 Hz, 3 at 50 Hz. In between, the recorded ball sits at release positions play never produces, for two samples.
   Play releases at toss-motion frame 45 (37.5 at 50 Hz, which is 45 × 5/6). The EU capture (`eu-bot-anim.bin`) shows
   the server's toss clip (motion 0x24, length 93.3334) running one frame a tick with no jump. So the clip clock
   isn't the cause.
2. **AI draws at the toss.** In slot 5 the game's AI generator draws 60 values as the ball is tossed. Play draws none.
   The other draws line up: +1 at the press and +68 at the contact in both. With 60 stand-in draws the AI generator
   matches through tick 237.
3. **Contact timing at 50 Hz.** Play strikes the serve 3 ticks after the game (recorded contact at tick 143, play's at
   146).
4. **Serve launch at 60 Hz.** The shared generator parts at the contact. The launch differs: the recorded ball rises
   slightly, play's dips. So the serve kind or aim differs, and play's point 1 is a fault.

Experiment, reverted: moving the 50 Hz release to frame 40.5 lines up the toss, but it pushes play's contact to tick
150. That timing is part of a rule, not a constant, and needs the decompile.

## Not verified / not 1:1
- **Whole-match proof not done.** Neither match gets past its first serve: gaps 1–4 above, spec in M184.
  `FIFTY_HZ_SIM` is not flipped and there is no release note. Both wait for M184's port, then M183a1 resumes.
- **The first 8–9 ticks of the held ball differ in both builds.** That is the hand pose as the walk starts. From
  tick 8 or 9 on it matches, so I didn't chase it. It is in M184's files if the spec finds it matters.
- **No generator check for the EU match.** M177 recorded no `rng_*` file for it. The seed is proven only through the
  weather schedule and the server's walk spot.
- **The US baseline uses a computed `rand()` start**, not a recorded seed, because slot 5's intro was skipped. It is
  checked: the generators match the recording every tick up to the toss.
