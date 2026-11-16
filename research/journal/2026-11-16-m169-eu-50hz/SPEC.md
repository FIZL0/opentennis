# SPEC — the European program at 50 Hz (Everybody's Tennis, SCES-54535)

Scope: every way the European program's match differs from the US program's, so that a "European build" switch
(next to the Japanese one in the rule-build module) can play a match exactly as the European disc does at 50 frames a
second. Source: M169's per-function classification of both programs, read function by function, plus a word-by-word
comparison of the data both programs carry (boot program and game overlay; M169's hash compared code only).

## 1. The general rule

The European program is the US program retimed for 50 Hz and relinked. The rules, AI decisions, shot choice, scoring
and the order of everything in a frame are the same. What changes is numbers, and only numbers tied to time:

- a count of frames becomes 5/6 of itself;
- a per-frame amount (speed, rate, step) becomes 6/5 (×1.2) of itself;
- a per-frame² amount (gravity, acceleration) becomes 36/25 (×1.44) of itself;
- a frames→seconds or frames→hours factor uses 50 in place of 60.

It is **not** one blanket rule applied by a tool. Each constant was rewritten by hand, and the rounding differs:

- Integer frame counts are mostly rounded to the nearest whole frame (220 → 183.33333 is the one count kept
  fractional; 80 → 66 is rounded down; 5 → 4 and 10 → 8 use ×0.8, not ×5/6).
- Float constants are mostly typed again as a decimal and then rounded to the nearest f32. They are not the US
  f32 multiplied. For example, the serve walk step 0.033333 becomes 0.0399996, which is 1.2 × the decimal 0.033333,
  not 0.04. The port must use the European bit patterns listed below, never compute `us × 1.2`.
- Some constants are not retimed at all (section 9). Those keep their US values and so run 20 % slower in
  real time. That is the European game's behaviour, and the port matches it.

The European build is a switch: every value below is the US value when the switch is off. With it off, the US
replay tests stay bit-exact.

## 2. Motion files: the 50 Hz switch

A global "50 Hz motion" switch exists. It is **on** by default.

Every loader of a keyframed track file honours it. That covers skeletal animation, face morph and UV animation,
each in its plain and variant forms: seven loaders in all. Each file starts with a ticks-per-frame value (80 in
every stock file). When the switch is on, the loader replaces that value at load with
`ticks_per_frame × 60 / 50`, in integer arithmetic (80 → 96). Keys keep their tick positions, so each clip plays
over 5/6 as many frames and the motion keeps its real-time speed at 50 Hz. Frame lookups, blends and lengths all
use the rewritten value.

One kind of track set (a presentation one: court objects) has a per-track rate float in its header instead. With
the switch on, it is scaled as `rate × 60.0`, then `/ 50.0`, as two f32 operations in that order.

### Exceptions: when the switch is turned off

The player loader loads motion indexes 0..47 of a character. Each index has a skeletal file, a face file and a
UV file from that character's motion folder. Indexes follow the motion table in the motion module: 16..31 are the
volleys, smashes and other swings in the range from the plain topspin swing to the "s_n" swing, and 37/38 are the
two serve swings. The switch is set per index, before that index's three files load:

- indexes 16 to 31: **off** (those clips keep 80 ticks a frame and so play 20 % slower in real time; their
  frame counts stay the US ones, which keeps every swing's contact frame unchanged);
- indexes 37 and 38: **off** (same reason);
- every other index from 0 to 47: **on**.

The four dummy clips after index 47 (s_n dummy, serve dummy, ball dummy, receive dummy) load with the switch off,
except the third one (the ball dummy), which loads with it on. The clips after the dummies (the serve ball
clip, the "gu" set dummy and anything later) load with it on.

The creature models (the special-court animals) load their motions with the switch off, then set it back on.

Sim relevance: any clip whose frame count the sim reads changes length with the switch: a swing's length, a
run or receive clip's length, the dummies whose lengths time the serve and ball. The swing clips the sim times
shots by keep their US counts. Everything else is 5/6 as long (in frames, after the nudge below).

## 3. Clip-length nudge (European program only)

Every routine that works out a clip's length in frames does it the same way. There are six, one for each clip
kind:

1. `length = largest last-key tick ÷ ticks_per_frame` (f32).
2. If `length` is not a whole number (it differs from itself truncated to an integer and converted back), it is
   multiplied once by **1.000001** (bits 3f800008), as an f32 multiply.

A whole-number length is left alone. The US program has no nudge. With the 96-tick rewrite, most lengths become
fractional, and the nudge pushes them just past the value, so a frame-count test that truncates or compares
lands on the next frame where the division rounded down. The port applies the nudge only in the European build,
in every place that computes a clip length.

## 4. Player movement

- **Run speed**: the base run speed per frame is 0.0717024 (bits 3d92d8b5) in the US and **0.08604288**
  (bits 3db03740) in the European build. It is used everywhere the US uses it:
  - the run-frames estimate;
  - the "ready at contact" check;
  - the press auto-approach step;
  - the AI's step toward a point;
  - the mode setter that stores a runner's speed.

  The formula keeps its shape: `((a / b) × 0.3 + 1) × run speed × character speed × stat / 100`.
- **Run counter**: the counter that rolls a whole "minute" of run frames rolls over at **49** instead of 59 (a
  "greater than" test). The elapsed-frames expression built from it, `(59 − n) + (m − 10) × 60`, becomes
  `(49 − n) + (m − 10) × 50`.
- **Soft follow at contact**: the squared-speed threshold below which the player softly follows the ball at contact
  goes from 0.2143347 (bits 3e5b7a8e) to **0.308642** (bits 3e9e0653), which is ×1.44.
- **Per-player frame counter**: the "less than 60" test in the player's top update becomes "less than **50**".
- **Path history**: the per-player/team allocation that holds a run of frames shrinks from 10816 to **9016**
  bytes. The path reset clears **2246** words instead of 2704. The path handler's countdown is **25** instead of 30.
  Its long counter limit is **1500** instead of 1800. These are buffer sizes and limits: the port sizes its
  buffers from them, and the limits must match.

## 5. The serve

- **Serve setup walk**: before the toss, the server walks along the baseline by pad × a step per frame. The step
  goes from 0.033333 (bits 3d08882f) to **0.0399996** (bits 3d23d69f).
- **Serve state value 45 → 37.5**: a serve-state frame value (a float), at two places: placement and the serve
  update.
- **Toss gravity**: the serve toss uses the gravity constant of section 7.

## 6. CPU players

- **CPU serve walk**: the CPU server walks to its target x and stops when `|target − x|` is below the walk step.
  That is the same constant as the serve setup walk, so it is **0.0399996** in the European build.
- **CPU serve waits**: every random wait the CPU draws before and around its serve is
  `(draw >> 16 & 32767) % 60 + 60` in the US. In the European build it is **`% 50 + 50`** (50..99 frames). The
  draw comes from the AI generator, as in the US, and the number of draws doesn't change. This applies to the
  singles serve routine, its doubles copy and the related pre-serve routines.
- **CPU AI frame waits**:
  - a "more than 29" counter test in the singles and doubles AI main routines becomes "more than **24**";
  - two helper waits change, "less than 60" → **50** and "less than 30" → **25**;
  - the "less than 60" tests in two more AI helpers become **50**.
- **AI ball-speed test**: two AI routines turn the ball speed into km/h to compare it with a character threshold.
  They compute `|v| × 50 × 3600 / 1000` instead of `× 60 × …`. The thresholds are unchanged, so the CPU reacts
  to the same real-world speed.
- **AI landing pick**: the AI's spot helper predicts a landing with the gravity of section 7.
- **AI step toward a point**: uses the run speed of section 4.

## 7. The ball

- **Gravity**: the per-frame² gravity changes from 0.0027222224 (bits 3b326750, 9.8/3600) to **0.00392**
  (bits 3b807358, 9.8/2500), everywhere it is used:
  - the live-ball response sub-step;
  - the serve toss;
  - the AI landing helper;
  - the "is it still bouncing" ball-speed test;
  - debris, puffs and footstep effects (those are presentation).

  An unrelated per-frame 0.003 term in the ball response is **not** changed (section 9).
- **Still ball**: a resting ball ends the point after more than **1500** frames (US 1800). The ball-speed helper's
  "more than 1801" test, where it has one, becomes **1501**.
- **Shot trajectory tables** (the shot parameter table already read from the disc): the European disc stores them
  retimed. In each 13-float record, fields 0, 1, 7 and 9 (the per-frame amounts) are ×1.2, written as decimals and
  then rounded. For example, 0.1 becomes 0.12 (bits 3df5c290), 1.18 becomes 1.416, 380 becomes 456 and 100 becomes
  120. The European build reads these values from the disc **as they are**: no conversion back to 60 Hz. Today's
  60 Hz European mode converts back ("regenerated shots"); the 50 Hz build doesn't.

## 8. Match flow, counters and readouts

- **Point handler**: a rally-length gate of "more than 900 frames" becomes "more than **750**". The same gate also
  needs 10 or more shots and a fast enough shot kind. It decides a fast-rally replay and its sound.
- **Instant replay**: the rally-length gate of "more than 1800 frames" becomes "more than **1500**". The same
  routine's mode-3 still-ball end uses 1500.
- **Ball-effect object**: its lifetime is **1500** frames (US 1800). Its fade is `(n << 7) / 1500`.
- **Post-point scoreboard update**: the first stage's wait goes from more than 30 to more than **25** frames (a
  float compare; the short variant's 1 is unchanged). If the next point waits on this stage, this is match
  timing (open question 1).
- **km/h readouts**: every place that turns ball speed into km/h uses ×50 instead of ×60:
  - the speed popup;
  - the replay gate's speed tests (`< 90`, `> 70`, `≥ 85`, `< 100` km/h, which are unchanged in km/h);
  - the sound and commentary pickers;
  - the court/NPC speed reader;
  - the AI test in section 6.

  The km/h value a given real-world shot shows is the same in both programs.
- **Wind**:
  - the court-state wind vector is `speed × 0.002` (bits 3b03126f) per frame instead of × 0.0016666667 (bits
    3ada740e);
  - the weather's per-second → per-frame factor is **0.02** (bits 3ca3d70a) instead of 0.016666668 (bits 3c888889).

  In our model, the weather wind moves footstep puffs and court objects only, not the ball (the shot's "wind" is
  the launch curve term, which is unchanged apart from the trajectory table above). So the wind is presentation
  unless open question 2 turns out otherwise.

## 9. What the European program did **not** retime

These keep the US values. The port keeps them too: that is the European behaviour.

- the post-point/game/set wait table in the game overlay's data (its 90, 45, 20, 16, 15, 10, 7 and 6 frame entries
  are word-for-word the US ones);
- the 0.003 per-frame term in the ball response;
- the rain velocities 0.0035 and 0.0005, and the leaf spawn interval 700;
- the 16 swing clips and the two serve clips (section 2: the switch is off for them).

## 10. One-player practice

Practice mode is the feeder, the coach and the drills, built only in one-player practice. It is not the doubles
AI, despite M169's label. Its constants change in the same way:

- **Drill waits**:
  - 30/20/30/20 → 25/16/25/16;
  - 60/10/10/20 → 50/8/8/16;
  - 5 → 4, 10 → 8, 30 → 25.

  Each of the many small state setters sets a 5 → 4 or a 30 → 25.
- **Drill clock**: 3600 → **3000** frames, clamped at more than 2999. The time-limit table in the overlay data
  goes 1800 → 1500, 1200 → 1000, 900 → 750 and 300 → 250. Seconds shown = frames / **50**.
- **Clock dial**:
  - per-frame turn 0.100027785 → 0.120040014, and 1.0027856 → 1.2040133;
  - wrap `% 3600` → `% 3000` and `% 360` → `% 300`;
  - a cycle of 1200 frames at 0.2502085 a frame → 1000 frames at 0.3003003.
- **Feeder and NPC drill states**: 180 → 150, 60 → 50, 30 → 25 (many), 240 → 200, 270 → 225, 220 → **183.33333**
  (kept fractional), 150 → 125, 120 → 100, 90 → 75, 3600 → 3000, 1440 → 1200.
- **Grade table** (overlay data): the score bands keep their bounds, but the grades are shifted down by one
  (5, 4, 3, 2 → 4, 3, 2, 1), with one band added and one removed. The bands should be read from the European disc
  as data.

Practice timing is sim (it draws from the shared generators), but the remaster only partly ports practice. The
port applies these where practice exists.

## 11. Presentation (feel-right only; listed so the port knows they don't touch the sim)

They need no bit-exact proof. They must not change the shared, AI or court generators' call counts (see
section 12).

- **Camera**:
  - cut countdown 80 → 66;
  - camera-table frame counts are scaled at use by `(int)(frames × 0.8333333)` (bits 3f555555);
  - camera-edit script durations get the same scale at load, for non-zero values only;
  - camera object thresholds 5/20/19 → 4.1666665/16.666666/16;
  - a turn step −0.011635529 → −0.013962635;
  - a new condition that sets one value to bits be32b8c2.
- **Aspect**: 0.7 → 0.8 (with 0.699219 → 0.796875) in about twenty draw routines, for the PAL screen height. This
  is video, not timing, and isn't specced for the port (the remaster renders at any aspect).
- **Speed popup**: shows for 50 frames (was 60), in steps of 5 (was 6), with frames/5 in place of frames/6.
- **Sound and voice**:
  - frames→seconds `/60` → `/50` (squared in one place);
  - a voice clip's length in frames becomes `len × 50 / 60` (integer);
  - bounce and hit sound windows 270 → 225 and 1440 → 1200 (sound generator);
  - a 120 → 100 threshold;
  - a fade call's 60 → 50.
- **Gallery and court life**:
  - 1440 → 1200, 120 → 100 and 60 → 50 in the crowd reactions;
  - the walker path tick 240 → 200;
  - the stream sound every 12000 ticks (was 14400);
  - emitter steps ±0.375 → ±0.45;
  - stereo loops every 1000/750/500/250 ticks (was 1200/900/600/300), with a pan slope 0.064 → 0.0768;
  - a serve-placement sound fade 180 → 150 frames at 1/150 (bits 3bda740e) a frame, offset 0.5 → 0.6;
  - a random repeat of 250..500 ticks (was 300..600);
  - one court-life routine drops its ±0.02 terms and reorders the sum to `(v − 0.04) × w`.
- **Effects**:
  - per-frame² terms ×1.44: bits 3ccb1c1e → 3d123d34, bits 3ff07d59 → 3feae774,
    bits 3ccd0f92 → 3d13a4d0, and bits 3ff2ccb9 → 3fed2918;
  - a wave filter 1.8888888 / 0.02777778 → 1.84 / 0.040000003;
  - a rotation rate 0.25 → 0.3;
  - hit-spark data: 60 → 50 frames and a drift of −0.01 → −0.012;
  - flight-effect data 0.015 → 0.018, 0.02 → 0.024 and 20 → 24;
  - debris and puffs use the section 7 gravity.
- **Weather** (boot program data):
  - rain: a 600 → 500 frame value, streak length 0.4 → 0.48 (bits 3ef5c290);
  - leaves: lifetime 400 → 333, fade 60 → 50 and 30 → 25, fall 0.015 → 0.018 (bits 3c9374bd, two fields), wind
    drift 0.011 → 0.0132 (bits 3c5844d0), eight 0.2 → 0.24 (bits 3e75c290) and 0.06 → 0.072 (bits 3d9374bd).
- **Results and front-end timers**:
  - the result object's 120 → 100 and 1440 → 1200 counters;
  - boot timers 60 → 50 (four sites);
  - an attract timer 5400 → 4500;
  - the court's intro timers 60 → 50.
- **Video driver**: the screen height 224 → 256 and 448 → 512, 3584 → 4096 buffers, a GS init 0xe0 → 0x100, other
  buffer sizes and a 448 → 512 overlay value. Not specced; the remaster has its own renderer.

## 12. Sort of every timing row: sim vs presentation

This covers M169's 84 untriaged timing rows plus its "sim?" rows, by what they drive.

**Sim** (must be exact; the port changes these):
- the motion switch and its exceptions, and the clip-length nudge (sections 2–3);
- run speed in its five users, the run counter, the soft-follow threshold, the per-player counter and the path
  buffers, countdown and limit (section 4);
- the serve walk step, serve state 37.5 and toss gravity (section 5);
- CPU serve walk tolerance, the waits `% 50 + 50`, the AI frame waits, the AI km/h test and the landing gravity
  (section 6);
- ball gravity, still-ball 1500 and the trajectory tables as they are on disc (section 7);
- the post-point scoreboard 25 if it gates the next point (section 8, open question 1);
- practice drill timing, wherever practice is ported (section 10).

**Presentation** (feel-right; must not change any generator's draw count):
- the point-handler gate and instant-replay gates 750/1500, and the ball-effect lifetime (section 8);
- every km/h readout except the AI's;
- wind (unless open question 2 says otherwise);
- everything in section 11.

**Draw-count risk**: some presentation routines draw from the court generator (gallery, walkers, emitters) or the
sound generator, and both are reseeded per point from the global `rand()`. Their retimes change *when* they draw,
not how many seeds they take. So they can't move the match's random numbers, as long as no presentation routine
draws from `rand()` itself or from the shared or AI generator. The port must confirm this against European
recordings: count the global `rand()` calls per point and check they match the US count for the same inputs.

## Open questions

1. Does the next point's start wait on the post-point scoreboard's first stage (more than 25 frames in the European
   build), or does the match flow's own wait (the unretimed table) always run longer? A European recording of the
   gap between two points settles it.
2. The court-state wind vector (`speed × 0.002`) is applied, ×40, to a list of objects. Is the ball ever in that
   list? Our model says no (only footstep puffs and court objects drift). Check a windy European recording for the
   ball's path.
3. A keyed parameter list in the overlay data has two entries changed by an odd ratio (6.4 → 6.34 and 8.7 → 8.58,
   not ×1.2 or ×5/6). Its reader isn't in the decompile. If it is a court or camera list, it is presentation;
   confirm from a European recording or the European disc's reader.
4. Which European clip lengths the sim actually reads after the switch. The list in section 2 comes from the
   loader, but the remaster's motion users must each be checked once against a European recording (swing contact
   frames should equal the US ones; run and receive clips should be 5/6 as long, after the nudge).
5. Sound files and the voice data on the European disc weren't compared (only the programs). Voice length is
   recomputed by the program (section 11), so this matters only for presentation.
