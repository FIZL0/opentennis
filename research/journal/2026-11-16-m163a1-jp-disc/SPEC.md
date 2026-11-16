# M163a1 — the Japanese release's rules (SCPS-15113), as they differ from the US game

Clean-room spec from a decompile of the Japanese program (boot program and match overlay), compared function by
function with the US one. The Japanese build is an earlier build of the same game. Almost all of its match code
is the US code with a smaller player record and a smaller per-character shot record. Below are only the
differences that change what happens in a match. Anything not listed here behaves as in the US game, apart from
the parts listed under *Open questions*.

US play must stay bit-exact. Every rule below is a per-release switch that is off for the US.

## 1. Data files

### TParam.csv (per-character shot/serve data)
- 63 columns, against the US's 67. The four US columns for the serve's own timing counts are missing. These are the
  strong-toss pair (rabbit, turtle) and the weak/underhand-toss pair (rabbit, turtle): US columns 30, 31, 32 and 33.
- Columns 0–29 are the same as the US, including the rally timing counts (rabbit, turtle) in columns 28 and 29.
- US column c, for c ≥ 34, is Japanese column c − 4. Examples:
  - the ideal serve heights (over 65, under 66) are 61 and 62
  - the smash heights (64) are 60
  - speed/stamina/agility/costs (40–43) are 36–39
  - column 62 is 58
- Cell types and unit conversions (tenths, hundredths, centimetres) are the same per column.
- Values differ from the US in many cells (for example Lob POW). That is data, read from the disc. Use it as is.

### AIParam.csv (CPU parameters)
- 43 columns, against the US's 45. Two US columns are missing: the lob reaction time (column 23) and the
  special-serve reaction time (column 24).
- US column c, for c ≥ 25, is Japanese column c − 2. Columns 0–22 are the same.
- Same 168 rows, in the same layout: character + 14 × level block, with doubles in the second half. The row chosen
  at match start uses the same rule as the US.
- Values differ from the US. That is data.
- With those two columns gone, the lob and special-serve reaction times don't exist. See 4.

### Program data (already read per release by the disc layer)
These are tables inside the Japanese program. Read them from the Japanese program, not the US one, and use them as
the game's tuning:
- the CPUs' shot blend weights (characters 0–11 and 13 differ)
- the rain odds (courts 1, 2, 4, 5, 6 and 10 are higher)
- the umpire word countdowns (Japanese rows only)
- the TRAJ tables come from the disc's own TRAJ archives (strk3/voly3 differ for some characters)

There is no missed-serve table (see 2).

## 2. The serve

### Contact timing
- The US game has three timing tables, chosen by the toss:
  - a strong toss uses its own table, built from the strong-toss timing counts
  - a weak or underhand toss uses its own table, built from the weak-toss counts
- The Japanese game has one table. **Every serve, whatever the toss, is graded with the rally timing table**: the
  same grade windows and depth-bias table that ground strokes use, built from the rally counts (TParam columns 28
  and 29).
- The ideal contact-height windows still depend on the toss (over-arm / underhand heights, as in the US).

### No mistimed strong-toss miss
- In the US, a mistimed strong toss adds a miss to the serve's aim. The miss comes from the missed-serve table by
  character group: depth and side from the stick scaled by two thirds, or a random side drawn from the RNG.
- **The Japanese game has none of this.** The aim never adds that miss, the missed-serve table isn't consulted, and
  **the random side draw doesn't happen**, so the generator isn't advanced there.
- The launch error step that, in the US, adds the stored stick miss to a serve's side and depth is also absent.
- Everything else in the serve aim is the same as the US:
  - widths 2.0575 / 3.4 / 2.4 (with |offset| > 1 and not a strong toss)
  - the same box aim
  - the same launch error otherwise

### No special serve
- The US flags a "special serve". The conditions are: a strong toss, a clean grade (|offset| < 2), and one of
  character 11 or 12 serving right-handed, or character 10 serving left-handed.
- **The Japanese game never flags one.** No special serve exists: no special-serve message to the receiver, and no
  special-serve reaction (see 4).

## 3. Strikes

### No counter (overpowering)
- In the US, a flat or topspin stroke (with two or more players, not an inside-high hit) or volley can be a
  "counter": it answers a flat or topspin rally shot with a clean grade, and the hitter's power is lower than the
  incoming shot's. When that happens the US:
  - zeroes the launch blend
  - sets the counter flag and the power gap
  - launches the ball with the **opponent's** character shot record
- **The Japanese game has no counter at all.** Every shot launches with the hitter's own record and its normal
  blend. The incoming shot's details aren't fetched for this.
- So there is also:
  - no counter pop-up (the menu archive has no "Counter" panel)
  - no overpower racket sound
  - no "counter winner" named point (see 5)

### No dull-hit flag
- The US marks a shot as "dull" when the random power multiplier is chosen.
- The Japanese game makes the **same RNG draws** and applies the same multiplier, but never sets the flag. So there
  is no dull hit sound (presentation only; the sim state and RNG are unchanged).

### Aim: the widest angle off straight
Two differences in the per-shot aim, both affecting where every rally shot lands:
1. **Smash narrowing.** A plain smash with |offset| > 1 narrows its widest angle by its depth.
   - US: depth clamped to 3..11.885, factor = 1 − 0.4 × (depth − 3) / 8.885.
   - Japan: depth clamped to **6.4..11.885**, factor = **1 − 0.3 × (depth − 6.4) / 5.485**.
   - The constants are code, not data.
2. **No 90° shots.** The US gives a ground slice, a ground lob and any rally lob a widest angle of 90° (so they are
   never pulled straighter). Japan has no such case: those shots take the normal angle like every other shot.
   - It is scaled by the body and rising-ball multipliers when there is no incoming slice.
   - It is scaled by the incoming-slice multipliers 0.6 / 0.45, or by the drop's 0.5.
   - Its floor and the acos/tan steps are the same.
   - The order in which the multipliers apply is the same as the US's for the other shots.

## 4. CPU players

### Reaction time
- In the US, when a CPU's reaction delay is drawn for an incoming shot, it adds an extra delay:
  - for a lob (kind lob, branch ground/volley/dive): half its lob reaction time plus a random amount up to that half
  - for a special serve: the same with the special-serve reaction time
- **Japan adds nothing, and makes no RNG draw for it** (in both the singles and the doubles routine). The rest of
  the reaction draw is the same.

### Smash search
- In the US, the CPU's smash search skips a candidate ball when its standing spot (the ball offset by half the smash
  reach, along the hand and depth) is at or beyond the baseline (|depth| ≥ 11.885).
- **Japan has no such check**: a candidate behind the baseline is accepted. All other smash search rules are the
  same: falling, smash height window, ideal-height score, in time to run.

## 5. Match flow and judging

### Named point
- The judge's named point for the scoreboard (ace, return winner, smash winner, counter winner) has no "counter
  winner" in Japan (there is no counter).
- A point the US would name "counter winner" is not named. The other names follow the same rules.

### Ball path on a hit
- When a shot is hit during the rally, the path/prediction manager sets a flag. In solo play it also sets a
  30-frame count.
  - In the US that happens only if one of these holds: solo play and not practice mode 3, or more than one player
    and not the serve (shots this rally > 1).
  - **Japan sets it on every rally hit**, without that condition.
- What reads the flag isn't yet tied to our code. See *Open questions*.

## 6. Ball speed in km/h

- Everywhere the game turns ball velocity into km/h, it uses the length of the velocity per frame:
  - the strike's speed pop-up
  - the smash-bounce check (≥ 85 km/h)
  - the CONK sound (≤ 50 km/h)
  - the per-shot statistics
- US: length × 60, then × 3600, then ÷ 1000, each step a chopped float operation.
- **Japan: length × 60, × 60, × 60, then ÷ 1000.** That is three separate chopped multiplies by 60, so results can
  differ from the US by an ulp, and a threshold can flip at the edge.
- The length itself (the sum of squares in y, x, z order, then square root) is the same as the US.
- In Japan some callers compute it inline rather than through a shared routine. The result is the same.

## 7. Same as the US (checked)
- The serve toss.
- The player and CPU construction, apart from the record sizes.
- The ball launch (apart from the counter's record override; Japan adds debug prints and a debug-only branch that
  never runs).
- The smash speed clamp.
- The AI messages.
- The match init, apart from table sizes.
- The ball object's bounce handling, apart from the km/h formula.
- The ball path restart (message "new path").

## 8. Front-end (ours to design; no exactness)
The Japanese menu archives lack or move entries our front-end reads:
- `message0.dat` is missing. Its text lives elsewhere.
- `word.tm2` in PRIZE/OTHER is missing.
- `inpane_Counter` is missing. It isn't needed: there is no counter.
- `fukidasi_dokuro` is missing.
- `Loading.tm2` and `info.tm2` are in MENU00B.

On a Japanese disc:
- leave out what doesn't exist (the counter pop-up)
- fall back or hide the rest gracefully, without panicking

Presentation differences in the program (effects colours, hit sound priorities, cutaway/replay picks, the
scoreboard) only have to feel right. They were not examined in detail.

## Open questions
- **Ball-path hit flag (5):** which of our path/AI parts reads it, and what it changes in play. The port session
  should find the matching flag in our path/AI code and confirm against a Japanese recording.
- **Not examined in detail** (differences seen, kind not yet classified). Most look like presentation:
  - the practice mode routines
  - a cutaway/replay chooser that uses ball speed
  - court-life/NPC routines (box-overlap reactions)
  - the scoreboard
  - a few boot-program CSV helpers and the voice loader (Japan keeps voices inside the motion archives; the disc
    layer already handles that)
  - A Japanese PCSX2 recording would show whether any of them touch the sim.
- **Where AIParam's parser lives in Japan** wasn't confirmed directly. The column map above comes from the CSV
  header and the record layout. It should be verified by reading the Japanese CSV with the US parser's types.
- **No Japanese PCSX2 recording exists yet.** The port must be verified against one, a bot match on the Japanese
  disc, before `Release::playable` accepts Japan.
