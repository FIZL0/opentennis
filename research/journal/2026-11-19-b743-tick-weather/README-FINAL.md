# B743 — match ticks read the snapshot-kept weather, not the drawn frame's

## Problem

`crate::weather::now()` is a global the render-side `weather::apply` (Update) sets from `Weather`. Tick code read it:
the flare's per-tick `rand()` draws (`play::flare_tick`), the dive thud and footstep whooshes pushed into `Game`
(`play.rs`, `foot_fx::steps`), and the NPCs' rain checks (`npcs::step` / `walk`: the gallery idles, walkers skip
their moves). A rollback's re-run ticks (and the second of two ticks in one frame) saw the last drawn frame's weather,
so a rollback across a weather change re-ran ticks before the change as if it were already raining: no flare draws,
`rand()` split between machines.

## Change

- `Game::weather` (serde default 0, so in every snapshot / encode): the weather the tick plays.
- `play::weather::tick_start` (FixedUpdate, before `remember`; the weather `step` now after it and `remember`) sets
  it from `Weather::today()` as the tick starts. That's the weather the previous tick ended with, the same one-tick
  lag single-machine play had (the last Update ran after the last tick), so a game's change still lands in the tick
  sims one tick later than `Weather` moves. With 2+ ticks per frame the second tick now sees the first one's end, not the
  frame before (more correct; unreachable in the replay tests, which never ran `apply` and so saw clear throughout).
- Readers switched: `flare_tick`, dive thud, replay footsteps, `npcs` gallery/walker rain checks (3 sites).
  Left on `now()` (render/presentation only): flare drawing, cheer-mark drawing (`npcs::draw_cheers`), hud glare,
  shade, the intro's gallery (pre-match, its own generator).
- Apps without the play weather plugin keep `Game::weather` = 0 (clear), as `now()` read in headless tests.

## Test

`play::weather::b743::a_rollback_across_a_weather_change_replays_the_same_ticks`: a bot singles on court 1 with game
1 rainy; every tick is followed by `apply` (as a drawn frame). When `Weather` moves to game 1 it restores the
snapshot 8 ticks before and re-runs 16 ticks without frames: per-tick state hashes match the first run. With
`flare_tick` put back on `now()` it fails ("split … 8 ticks before the weather changed (tick 4766)").

## Not verified / not 1:1

- No PINE check that the original's flare/NPC rain checks change on exactly this tick relative to the weather step:
  the task keeps the existing single-machine timing (one tick after `Weather` moves), which the replay tests pass
  with; whether the original switches a tick earlier was not re-examined.
- Not run as a two-machine online match; covered by the headless rollback test and the existing netplay rollback tests.
