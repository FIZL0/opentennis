# P134: court prop motions in the match

## What's in
- `play/carts.rs` (new) holds `Game::carts`, built from `hst_sim::court::Motions` (P133). Its per-tick state is
  `hst_sim::court::Phase` (first-pass flag, plus each cart's clock and resample flag). It goes into the state hash
  (Debug), the snapshots and rollback (clone), and a resync's bytes (serde). On decode, `adopt` binds it to the
  local motions and poses the world.
- Pass rule: one pass at the end of every match tick (FixedUpdate, after the cameras step), and one per intro tick
  (`intro::hold`). The first pass after the build steps nothing. Pause runs no tick, and slow motion only adds
  in-between frames, so neither steps. A cart the shared camera's eye test (`net_hide::tested` + `buried`) hides
  doesn't step.
- The ball sweeps `Game::world`, whose cart props the last pass posed. The `Const` world now has `make_mut`, and
  `mesh::World.models` is an `Arc`, so a copy-on-write costs only the 27 props, not the 7762 collision tris.
- An instant replay restores the serve snapshot but keeps the carts and the world (`carts::keep`/`put_back`): the
  original's draw passes go on through it.
- `court_load::moving` hands back the world plus its motions. A court mod gets none.

## Checks
- `hst-play carts::court_5_carts_turn_once_a_tick`: on court 5, 130 ticks give 129 steps, wrapping at 40. Only the
  carts' four props move. Snapshot restore and encode/decode both come back at the same phase and pose.
- `hst-play carts::a_replay_keeps_the_carts_going`. `snapshot`, `instant_replay`, `court_load` and `hst-sim world`
  still pass. Clippy is clean.
- Against the original on court 5 (`scripts/p134_intro.py`, from scratch slot 8 via Select Court; captures in
  `context/p134/intro1..4.txt`, repeatable): the clock steps once per intro match frame. Extra steps build up
  where the match frame stalls at the ends of the flyover's segments (near frames 100, 220 and 354; camera records
  100/120/130, flyover 354 frames), reaching 4 by frame 354. At the intro's last frame (446) the original has taken 449 steps
  (sampled 8), against 445 here. Into the serve phase it then steps exactly once per match frame (intro4: frames
  0→2…12 give sampled 8→10…20), which matches the per-tick rule.

## Not verified / not 1:1
- **The intro is 4 passes short on court 5.** The original draws extra court frames while its intro clock stalls at
  the flyover's segment ends. I didn't guess which passes those are. Follow-up P136.
- The ✕-skipped intro wasn't measured.
- The instant replay's skip ticks (run undrawn to catch up) step the carts here. The original wouldn't draw those
  frames. Not measured.
- The eye test uses only the shared camera (`g.cam.view`), never a player's own camera, to stay deterministic
  online. It wasn't checked against the original with a cart actually hidden.
- Slow motion and pause: inferred from the frame flow (no tick, no pass), not recorded.
- No replay fixture is on court 5 (they are on courts 10/11/4), so none needs the cart clock at its start. A future
  court-5 fixture sets it through `Motions::set_phase`/`Phase`.
- The ball hitting a moving wheel during a rally wasn't caught in a recording. The sweep uses P133's posed nodes,
  which were checked there.
