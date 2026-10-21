# B40g2 spec: Offbeat Rules (irregular bounce, slow motion)

This is the clean-room spec. It describes in words what the original does. The port session works from it, the
recordings and the oracle. The task is a **concept** (`concept/offbeat-rules`, AGENT.md *Concept branches*): the code
goes on that branch, not on main.

## Summary

The original's confirm screen has an Offbeat Rules page with four on/off rows:
1. **Irregular bounce.** The ball looks different and each bounce kicks it off in a slightly random direction.
2. **Slow motion.** From the start of each serve until the point ends, the match runs at about ⅔ speed.
3. and 4. Placeholders: "Change Special Rule 3/4." Nothing in the game reads them.

With both off, nothing changes. Neither rule touches the random generators: every draw described below happens in a
normal match too.

## The settings

- Two separate on/off values for the match (rule 1, rule 2). Each is 1 for on and anything else for off. Both last
  for the whole match.
- Menu text (message0.dat in MENU00B.XB0):
  - confirm row help: 327 ("Set handicaps and other offbeat rules. …", shared with the handicaps) and 331 ("Add
    some unusual rules to the match.");
  - per-row help: 343 (rule 1, "Turn on and the ball will bounce in unexpected directions."), 344 (rule 2, "Turn on
    to slow down all movement during the match."), 345/346 ("Change Special Rule 3./4.").
  The rule names are drawn art, not message strings. The front end is ours to design (PLAN.md). Name the rows
  ourselves ("Irregular bounce", "Slow motion") and leave out 3 and 4, or fill them with our own rules (see the
  proposal in `1-SPEC-PART.md`).
- Online, both peers must have the same rules. Rule 1 changes the sim. Rule 2 changes the frame pacing both peers
  run at.

## Rule 1: irregular bounce

### The deflection

Every ball launch already draws two uniforms from the **shared** generator. The port draws and drops them today: in
`play.rs`'s strike and in `toss_draws`. Each is the 32-bit word, read as unsigned, converted to float and times 2⁻³²,
i.e. `Mt::unit`. The first is the **pitch uniform**, the second the **yaw uniform**. They stay with the ball for
the whole flight. The CPUs' predicted path is a copy of the same flight, so it reads the same two values.

With rule 1 on, at a bounce the ball's new velocity (after the bounce response, before anything else turns it) is
turned like this:
- **Yaw spread:** 20° when the shot's kind is a lob, 15° for every other kind. The spread is turned to radians as
  `0.017453292 × spread`, rounded once. The yaw change is `(2 × yaw uniform − 1) × that`: the multiply by 2, the
  subtract, the final multiply, each rounded. That gives a range of ±spread.
- **Pitch change:** `0.017453292 × (pitch uniform × 15)`, i.e. 0°..15°. The product `uniform × 15` comes from a
  fused multiply-add onto zero (so it is truncated like the game's `madd`). The ×0.017453292 is a separate multiply.
- The velocity is then turned by that pitch and yaw. This is the **same turn** the port already has in
  `hst_sim::ball` (`turn(d, pitch, yaw)`: pitch measured from the horizontal with "down" positive, clamped to ±π/2;
  heading wrapped to ±π; the turned direction rebuilt from the rotation-about-X then rotation-about-Y matrices;
  length kept). A positive pitch change tilts the bounce **down**, so the ball comes off the court flatter, by up
  to 15°. The yaw change swings it left or right.
- The original has two fixed-angle debug overrides here. Both are 0 (off) in the retail data. Leave them out.

The two uniforms are fixed for the flight, so every deflected bounce of one shot turns by the same angles.

### When it applies

Exactly where the port applies the serve's first-bounce turn (`shot.bounce_turn`, both in `Flight::advance` and in
the single-contact plane step), under the same conditions **except** the bounce count and the turn value:
- a rally is on (`Flight::in_play`);
- the contact is a bounce, not rolling;
- it is the frame's first contact (the plane step has only one);
- rule 1 is on.

So every bounce of a shot is deflected: the first, the second, and the small ones before it rolls. A serve's
first bounce is too. Off-court contacts count as well (walls, props), since they go through the same response. On
a serve's first bounce, the irregular turn comes **first** and the serve's own first-bounce turn after it.

The landing point is unchanged, because the turn happens after the contact. Line calls, the bounce counters and the
landing record are unaffected. The CPUs' predicted path runs through the same code with the same two uniforms, so
the CPUs see the deflection coming. Rule 1 doesn't fool them.

### Looks and sound (by eye / by ear, PLAN.md)

- **Ball model:** `ball_irregular.mdl` (CMN GAME.XB, `data/hatsuyama/ball/`, next to `ball1.mdl`) replaces
  `ball1.mdl` for the whole match: the drawn ball only, with the same scale and outline rules. The shadow is
  unchanged.
- **Ball lighting:** the game tints the ball by court. Each court has a row of three light-strength multipliers and
  a light colour (RGB, ÷255). Rule 1 swaps in a second table. The normal table isn't ported either. Both, by court
  number 0–13 (multipliers x / y / z; colour r, g, b):
  - normal: x `1, 1.3, 1.2, 1.3, 1, 1, 1, 1.2, 1, 1, 1, 1.6, 1, 1`; y `1, 1.3, 0.9, 1.2, 1, 1, 1, 1.2, 1.2, 1, 1,
    1.6, 1, 1`; z all 1; r `255, 205, 255, 205, 255, 255, 255, 235, 255, 255, 255, 250, 255, 255`; g 255 except 215
    at court 8 and 200 at court 11; b `255, 205,
    255, 235, 255, 255, 255, 255, 235, 255, 255, 105, 255, 255`.
  - irregular: x `1, 1.5, 1.5, 1.5, 2, 1.5, 1.5, 1.5, 0.7, 1.5, 0.7, 1.5, 1, 1`; y `1, 1, 1, 1, 2, 1, 1, 1, 0.8, 1,
    0.8, 1, 1, 1`; z `1 ×8, 0.8, 1, 0.8, 1, 1, 1`; r `255, 55, 55, 55, 55, 240, 55, 55, 200, 55, 200, 55, 255, 255`;
    g `255, 815, 815, 815, 815, 605, 815, 815, 735, 815, 735, 815, 255, 255` (over 255: an over-bright green); b
    `255, 15, 15, 15, 15, 95, 15, 15, 105, 15, 105, 15, 255, 255`.
  - A row of all-1 multipliers and 255 colour changes nothing, and the game skips it. Both tables have a "use
    court 0's row everywhere" flag, off in both. Read the tables from the user's GAME.BIN through `hst_data::exe`
    (find them by these values) rather than writing them into code. Judge the result by eye against a screenshot of
    the original with rule 1 set on its confirm screen. The exact light model is up to the port.
- **Bounce sound:** the court bounce sound the port plays with key 0 (`sound::Bounces::frame`, bounces 1–3 on the
  court) uses **key 13** of the same bank and program instead. It is also never slowed: a hard smash's first bounce,
  which normally plays at half speed, plays at full speed. The material sounds and the extra "bounce too" key-0
  plays are unchanged. Check that key 13 exists on every court's bank, by ear.

## Rule 2: slow motion

Pacing only: the same ticks run with the same inputs and the same results. The original shows some ticks for two
frames instead of one. This is the instant replay's slow-motion block (`play/replay_pace.rs`, P0b4d3): after a
tick, `n` in-between frames are drawn, interpolated from that tick toward the next, the k-th at `k/(n+1)` of the way.

- **Armed** at the start of each serve phase (the point's set-up, before the walk-up and toss), unless the phase
  starts an instant replay. At the same moment the rule's state resets: not slowed, countdown 0.
- **Disarmed** the moment the rally phase ends (the point is decided: in, out, fault, let, net). The post-point
  phase, change of ends, instant replays and anything between points run at full speed.
- **Each match tick** (after the phase's own work, before the replay/pad handling):
  - armed: the countdown drops by 1. If it is now below 1, flip slowed. On: start the ×2 pace (one in-between frame
    after each tick), countdown = 1. Off: stop the pace, countdown = 1.
  - not armed but still slowed: stop the pace, clear slowed, countdown = 1.
- In practice, counting the serve phase's ticks from 1, every **odd** tick is followed by one in-between frame at
  the half-way point (fraction 0.5) and every even tick by none: 2, 1, 2, 1… frames. That is 3 frames per 2 ticks,
  about 40 ticks a second instead of 60. The alternation continues straight from the serve into the rally without
  restarting. The tick that ends the rally gets no in-between frame.
- Recorded on PCSX2 (slot 5, rule 2 poked on: `context/b40g2/run1/slow.txt`, script in this journal's `scripts/`):
  exactly that pattern, serve and rally, then full speed from the first post-point tick. The pace block reads
  start = end = ×2 (period 1.999999), ramp 2 ticks.
- The in-between frames are draws only. Sounds play on their ticks as usual. The pitch was not checked (see the
  journal).

## Open questions

none
