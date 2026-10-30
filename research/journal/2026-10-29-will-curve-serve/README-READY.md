# Will's curving serve: original vs port (READY)

User report (2026-10-29): "I just played against Will CPUs and never saw a curve at the same level of the original."

Result: the curve itself is exact. Once a serve has bend, the port flies it bit for bit like the original. The
difference is how often CPU Will gets a bending serve: 55 % of his serves in the original, 30 % in the port
(19 % of first serves). Two port bugs cause it, both in `crates/hst-play/src/play.rs`. A third difference (flat
rate) is still open. Research only: no gameplay code changed.

## Setup
- Original: PCSX2 copy 7, save slot 8. Singles on court 10, Ashley (P1, human, Cross mashed by `tools/vpad.py` every
  0.4–0.7 s) against CPU Will (character 11, outfit 0 → AIParam row 11, level 3). Five matches:
  `research/will_curve_rec.py 8 900 context/will-curve/singlesN.bin mash:<period>` records every serve window in
  the `frames_live` layout, and `research/will_curve_ana.py <rec> 11` summarises it. Then
  `research/will_curve_err.py` (four more matches) reads the singles AI's drawn serve error and its contact
  countdown live.
- Port: a temporary all-CPU singles test (Ashley vs Will outfit 1, which is also row 11, court 10, 300 000 ticks)
  and a temporary `eprintln` at the serve contact. The patch is in `context/will-curve/port_logging.diff`, and
  `research/will_curve_port.py` summarises its output. A temporary hst-sim test
  (`context/will-curve/will_curve_tmp.rs`, copy it into `crates/hst-sim/tests/` and set `WILL_REC=<rec>`) loads each
  recorded launch into `ball::Flight`. It steps once from every recorded frame and also runs free from the launch
  to 15 frames past the first bounce.

## Numbers

| | original (56 Will serves) | port (217 Will serves) |
|---|---|---|
| toss | always strong, second serves too (11 seen) | first strong, **second always weak** |
| kind | topspin 36 (64 %), flat 14 (25 %), slice 6 (11 %) | topspin 118 (54 %), flat 80 (37 %), slice 19 (9 %) |
| timing offset | 0: 41 (73 %), −2: 7, −4: 8, never > 0 | uniform −4..4, exactly −(serve error) |
| bent | 31 (55 %); 31 of 36 topspins (86 %) | 65 (30 %); first serves 28 of 151 (19 %), 35 % of their topspins |
| bend / turn | 0.506–0.577 (mean 0.554), −10° at the first bounce | the same formula, the same values per launch |

The original's bent serve: it bulges sideways by about the bend (≈ 0.55 m) mid-flight, then comes back to the
straight hit→target line at the bounce (off the line by 0.03–0.07 m on average, 0.14 m at most). It lands within
0.02–0.2 m of the target, and its heading turns ≈ 16.5° at the bounce (the 10° turn plus the bend's own
sideways motion). Curve frames 26–38.

**Path:** across the five recordings, the one-frame steps are 4989 of 4993 bit-exact, and the free flight from
the launch is exact for 109 of 112 serves. Every bent serve is exact. The four misses are a 1-ulp z difference on
two flat serves that hit the net (the net-cord contact, nothing to do with the bend) and one window cut short
at the end of a match. So the same launch gives the same path, and `ball.rs` is not the cause.

## Cause 1: CPU second serves are always weak tosses
`reset_positions` (play.rs ~1764–1786) rebuilds every player each point with `Player { …, ..Player::default() }`
and carries over only `ai`, `ai_hit`, `ai_mind` and `formation`. `ai_second` (the character's strong-toss chance
on a second serve, set once in `load_player` ~1546 from `exe::Game::second_toss()`) is reset to 0 before the
first point. The logged value was 0 on all 217 port serves. On the disc Will's chance is 100, and the
table is `[0,0,0,0,0,0,30,100,0,100,100,100,0,0]`, the same in the game's RAM. So characters 7, 9, 10 and 11
should always second-serve with a strong toss, and Carol (6) should 30 % of the time. `tests/ai_serve.rs` passes
because it reads the table straight from the disc, not through the app's `Player`.
- Original: all 11 of Will's second serves used a strong toss and bent like his first serves when timed well.
- Port: Will's 66 second serves were all weak tosses. Those bend whenever the grade is 1/2 (37 of 66 did), so this
  bug hides some of the gap overall, but the second serve looks wrong.
- **Fix:** keep `ai_second` across the reset like `ai`: at play.rs:1764,
  `let ai = (g.players[i].ai, g.players[i].ai_hit, g.players[i].ai_second);` and after the rebuild
  `(g.players[i].ai, g.players[i].ai_hit, g.players[i].ai_second) = ai;`. Also check the other fields
  `load_player` writes once (the rest are carried or rebuilt from `hand`/`stats` already). Doubles uses the same
  reset, so CPU second serves in doubles are affected too.

## Cause 2: the CPU's serve timing error makes it mistime twice as often
The bend fires only on a strong-toss topspin/slice serve with |offset| < 2 (`shot::special`). This part already
matches the original: no topspin with offset −2 or −4 bent.
- Port (play.rs ~3539, ~3564–3567): `bot_due = SWEET_FRAME − ai_timing.serve`. The AI presses on the first frame
  its contact pick (`serve::ai_search` over a `grades.len()`-frame path) is ≤ `bot_due`, and the swing's offset
  comes out as exactly −err (217 of 217). Row 11's serve error is ±0..4, so only err ∈ {−1, 0, 1} is clean: 40 %.
- Original, per frame (28 serves, `context/will-curve/poll*.log`, the singles AI object's serve state): the AI's
  countdown to its contact pick **always first reads 8 frames** when the pick is taken. It presses as soon as
  countdown + err < 9, so at k = min(8, 8 − err). A negative or zero error presses at once, on 8. The contact
  offsets that result:

  | err | −4..0 | 1 | 2 | 3 | 4 |
  |---|---|---|---|---|---|
  | press at k | 8 | 7 | 6 | 5 | 4 |
  | offset | 0 (15/15) | 0 (2/2) | 0 (4/4) | −2 (1/1) | −4 (6/6) |

  So an early error never makes the CPU serve early, and a late one only matters from 3 frames on. P(offset 0) =
  P(err ≤ 2) = 80 % for row 11 (observed 73 % over the 56 serves), against 40 % in the port.
- **Fix (needs a spec pass, clean-room):** the CPU's serve contact pick has to become available only 8 frames out
  (the original never sees it earlier), and a press at 6–8 frames out has to land on the sweet spot. The rule
  above (k = min(8, 8 − err); offsets 0/0/0/−2/−4 for k = 8/7/6/5/4) is the measured behaviour. The spec session
  should read the singles AI's serve state (the pick it takes once and then counts down) and the contact search it
  feeds, then write down why a press at k = 6..8 gives offset 0 and a press at k = 5 gives −2, before porting.
  The doubles CPU serve likely goes through the same press. Check it with `1p3goodcpus` (b681's setup).
- Expected effect of both fixes: about 80 % of Will's topspin serves bend (original 86 %), on second serves as well.

## Open questions
- **Flat rate tied to timing.** In the original, flats come mostly from mistimed (late-press) serves: offset 0 →
  32 topspin / 5 slice / 4 flat (10 % flat); offset −2/−4 → 4 topspin / 1 slice / 10 flat (67 %). The port's flat
  share doesn't depend on timing (40 % of first serves). Original flats all have a deep aim (+0x3e90 |z| 5.5–6.3,
  i.e. the stick points at the net, as `shot::stick_kind` expects), so the stick→flat rule matches. What differs
  is how often the AI's serve aim points forward, and in the original that goes with a late press. Unknown
  whether the aim zone draw depends on the press frame, or whether the original aims after the mistiming moves the
  target. To be read in the spec pass together with cause 2. If the zone draw follows the press, fixing cause 2
  alone may already bring the flat rate down.
- Of the original's 5 unbent topspins, 4 were mistimed (offset −2/−4, as expected). One had offset 0 and still
  didn't bend (v34924, hit −0.38, 11.93). That one is unexplained, maybe a quick serve. Worth a look in the spec pass.
- Doubles not re-recorded here. b681's doubles numbers (original 40 % topspin) predate this and used the
  King outfit (row 53, error ±2).
