# P117 spec — smash deep mode, powered shots' unscaled margins, smash mode blend

Behaviour of the original, in my own words, for the port phase. Three linked pieces: (A) the smash's mode
pressure and its blend, (B) which margin pair the court pull uses (`low`), (C) when the pull keeps its margins
unscaled (`special`), plus (D) what the smash launch does with the blend. Names below are the port's
(`hst_sim::shot::Margins::inside`, `swing::smash_scatter`, `swing::smash_scale`, `swing::timing_launch`,
`shot::special`, `shot::effect`, `ReachStats`).

## A. The smash's mode pressure (set up at the swing, with the scatter)

The original builds a smash's timing error in the same routine as the strokes': side, depth and a *mode
pressure* (an integer, tenths, like `TimingError::mode`). For the smash branch:

1. The pressure starts at 0. The smash branch gets none of the stroke/volley/dive additions (no backhand,
   no mistimed-volley, no dive 10).
2. The body-shot addition applies to every branch: if the hitter's current motion is the body-shot swing
   (either side, `anim::BODY` / `BODY + 1`), add the character's Body DWN. A smash is never played from that
   motion, so in practice this never fires for a smash; the port may leave it out for the smash.
3. Deep addition — only for a **plain** smash (not the △ smash, i.e. the same `plain` test as
   `smash_scale`) whose timing offset is beyond ±1 (`|offset| > 1`, again the same test as `smash_scale`):
   - `d` = |the hitter's z where the swing started| (the same value `smash_scale` reads, `aim_from[1]`),
     held to [6.4, 11.885].
   - add `trunc( ((d − 6.4) / 5.485) × 10 + 0.5 )` (f32 EE ops in that order: subtract, divide, multiply,
     add 0.5, then truncate toward zero to an integer; the pressure is added as a float to the running total
     before the truncation, i.e. `trunc(total + ((d − 6.4)/5.485)·10 + 0.5)`, with total 0 here).
   - This gives 0 at or inside 6.674 m (6.4 + 0.05·5.485) and grows to 10 at the baseline.
4. The mode is the **negative** of that total: `mode = −pressure` (0 … −10).
5. None of the serve / lob-button overrides that zero the mode for other branches apply to the smash.

At the launch, the mode becomes a blend like the strokes' (`timing_launch`), with the smash's own stats:

6. Held to [−10, 10], divided by 10 (f32): `blend = mode / 10`.
7. Power scaling, smash row: the *base* power is the character's **Serv POW** (`ReachStats::power[0]`; the
   smash shares the serve's power stat) and the *add* is **SM LOW POW** (`ReachStats::smash_low_pow`). The
   high-power branch never applies (the smash's mode is ≤ 0); for `blend ≥ 0` the rule is the strokes': top =
   min(base + add, 16), `blend × (top − base)/(16 − base)` when base < 16 else 0 — the result is 0 either way
   for a zero blend.
   For `blend < 0`: `low = max(base − add, 0)`; if base < 1 the blend is 0, else
   `blend × ((base − low) as f32 / base as f32)` (int subtraction, then two int→f32 conversions, divide, then
   multiply).
   Quirk: a negative blend times a zero factor is −0.0, which counts as "not negative" in B.
8. The launch then holds the blend to [−1, 1] (no effect for these values, but it is the order).

`smash_scatter` should return this blend next to its (side, depth); the strike needs it for B and D.

## B. The margin pair (`low`)

The court pull (`Margins::inside`) picks its (across, along) pair by the **sign of the launch blend**:
the second pair when the blend is strictly below 0, else the first.

- Strokes and volleys (classes 1, 2) always hand the pull a blend of 0, whatever their table mode is (their
  mode is passed to the launch separately as the table choice). So `low = false` for them is right; only the
  smash can be low.
- Smash: `low = blend < 0.0` with the blend from A (after step 8). −0.0 is not low.
- On the disc the smash rows are (0.15, 0.15) high and (0.9, 0.75) low for both smash kinds, so a deep,
  off-timing ✕/○ smash aims inside 0.9 m of the sideline and 0.75 m of the baseline. (The △ smash never has
  a mode, so it always takes the high pair.)

## C. Unscaled margins (`special`)

When the shot heads across (the existing `across` test), the pull scales both margins by the shot direction's
share unless the shot is a *special* one. Special here means: the character's special effect would apply to
this shot, judged as follows (all on the launch's final class and kind, i.e. after a framed hit turned into a
lob and after the high ground stroke turned into a volley):

1. The special gate holds — exactly `shot::special` for the striking player (the hitter's own timing grade /
   offset / toss; for rally shots: grade 1 or 2). A game-wide debug switch can force the gate on; it is off on
   the retail disc (ignore it).
2. The character used is the one whose tables and records launch the shot (`src`: the incoming hitter's on a
   counter, else the hitter's), the same character `shot_effect` takes its effects from and the pull's
   `angle` row uses. (Every character on the disc has its "has specials" flag set, so no character falls back
   to character 0's row; the port need not model the fallback.)
3. Look up the effect by (class, kind), using the character's raw table values (before any distance scaling):
   - stroke slice: special if the rally-slice bend (`ShotEffects::bend[2]`) is non-zero;
   - stroke lob: special if the lob curve (`ShotEffects::curve`) is non-zero;
   - anything else (topspin, flat, drop, every volley, every smash): not special. (Serves don't go through
     the pull.)
   On the disc every character's lob curve is non-zero (−1 or −2), so **every clean-graded stroke lob is
   special**; the slice is special only for the two characters with a rally-slice bend (characters 3 and 8,
   ±0.8).
4. Practice quirk: with one player on court and the hitter being player index 1, the slice is never special
   and the lob is judged by character 0's lob curve (non-zero, so the lob is still special). This is the same
   exception `shot::effect` documents as left out; only take it if the port's practice mode can have
   player index 1 hitting.

Equivalently: `special = shot::special(..) && effect-row(class, kind) has a non-zero bend or curve`.
Note the gate and the effect are read even though the pull only uses the result inside the across branch;
there are no side effects, so evaluating it lazily is fine.

## D. The smash launch with a blend

- The smash's table is always its **base** table (`smsh0` / `smsh1`), blend or not: the launch looks the base
  table up twice and mixes the two results, so the table result is unchanged. The port's base-table lookup is
  already exact.
- What the blend does change is the launch's spin-group record (launch spin, first-bounce spin, side angle,
  restitution and its low value: record fields 7..12; angles converted to radians before mixing):
  - only when the class is smash and the blend (after A.8) is non-zero;
  - the second record is the character's variant record for (smash, kind) flagged for variant 1 (`dw1`); if
    the character lists none for that kind (every character's list has only kind 0), the base record;
  - the factor: `blend / w`, where `w` is that variant entry's weight (negative on the disc, −0.18 … −0.67),
    held to [0, 1]; when there is no entry (w = 0) the blend is not divided or held; then the absolute value;
  - each field: `f = t·(v − b) + b + 0` (f32 EE ops in that order) with t the factor, b the base record's
    field, v the variant's.
- On the disc all three smash archetype records (kind 0 and kind 1) have the same spin group (spin 5°, the
  rest 0), so every character's dw1 smash record has the same spin group as its base record and the mix
  returns the base values exactly. The observable effect of D is nil on stock data. The port should either
  implement the mix or keep `smash_spins` and add a disc test asserting the dw1 smash record's spin group
  equals the base record's for every character (making the shortcut provably exact).

## Order inside the strike

1. Contact: build the smash's (side, depth) scatter and its blend (A).
2. Strike: pull the aim inside the court with `low` from the smash blend (B) and `special` from C.
3. Then the existing smash scatter along the pulled aim, the base-table lookup from (hit − scatter) to the
   pulled aim, and the spin (D).

## Verification ideas for the port phase

- Function oracle (`research/oracle.txt`: `court_pull`, `special_effect_check`, `timing_error_setup`,
  `timing_error_read`): the pull over ≥10k generated (class 1..3, kind, blend sign, player count, character,
  hit, aim, effect gate) inputs against `Margins::inside`; the error setup + read on the smash branch over
  generated (z, offset, plain/△, Serv POW, SM LOW POW) against the port's blend.
- Recordings: the existing smash fixtures (`match_s05.bin`, `human_smash_s04.bin`, `new_recording.bin`,
  `1p3goodcpus.bin`) — the ball's launch target should equal pull(aim, low from the blend) + scatter for every
  smash, including the off-timing ones from deep. Clean lobs heading across (any character) and clean slices
  by characters 3/8 exercise C; record some with `tools/vpad.py` if no fixture has one.

## Open questions

none
