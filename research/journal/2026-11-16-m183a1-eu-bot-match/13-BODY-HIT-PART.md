# M183a1 part 13: a ball that hits a player's body (PART: blocked on a recording)

Continues part 12. Probe:
`HST_M183A1_PHASES=200 tools/check.sh -p hst-play the_european_bot_match_plays_as_recorded -- --ignored --nocapture`.

## Before
- 38 of 144 EU phases matched.
- Phase 38's rally (from recorded tick 9004) ran 92 recorded ticks, and play ran 138.
- At recorded tick 9094 the ball hits the net player's body (player 3). The harness never ran body-hit detection, so play carried on past the hit.

## Harness
- `bodyhit::sim_plugin` is new and test-only. It adds body-hit detection to `played_ticks_rng`.
- New `HST_M183A1_FLIGHT=from,to` prints the recorded ball's counts, slide and contact frames beside play's, for the given tick range.

## Stroke keeps the contact frame
- `play.rs` stroke: a hit used to reset the ball's contact frame. It now keeps it: a plane step with no slide reads it.
- With that, the ball matches bit for bit at recorded ticks 9094 to 9096. (The slide was kept too at first; the review found nothing reads it before it is recomputed, so that line is out.)

## Detection between the step and the rally check
- `simulate` runs `bodyhit::detect` after the ball's step and before its rally check (release and harness).
- A first version (531ad5a6) ran it as a system after `simulate` with a second check; the review found it judged a hit on a deciding bounce's tick as the bounce. Replaced.
- The `since_hit` guard and `HST_BODY_HIT`'s 30 read the count before this tick's increment, as before.
- Phase 38 matches, 92 of 92.

## Now
- 40 of 144 phases match. The floor in the test is raised to 40.
- Tests run: `hst-play` bodyhit (1 passed), `hst-play` fifty_hz (5 passed), `hst-sim` bodyhit (3 passed).

## Next gap: phase 40 (serve set-up)
- Phase 40 starts at recorded tick 9229. The game's set-up takes 147 ticks, play's 241.
- Player positions match at 9229 to 9231.
- From 9232, play's server (player 1, x about -4.76) walks to about -0.78, but the game's server stays where it is.
- That fits the game having picked the serve-level-0 wide spot (4.785), and play the 0.7 centre spot. So the pick, its serve level or the generators differ.
- Hypothesis only, not checked.
- `eu_point_reseeds` reports only through 5615 (offset -3, and -2 at 5615), because `fixtures/m183a1/eu-rng-long.bin` ends at 7000 ticks.

## Recording attempts for a longer generator run
Goal: a 9700-tick generator recording, via `scripts/eu_rng_rec.py 8 9700`. All three attempts failed.
1. PINE was not up yet: FileNotFoundError.
2. Timed out after 1150 s with 0 bytes. The "generators" line was never printed. The log then said PINE op 0x2 failed. A hand check after that found "already running" and no PINE socket. During this attempt I opened a second PINE client from outside the lock. It timed out and may have disturbed this run.
3. After a clean stop, I waited for PINE to report running ("ready after 2"). It timed out with 0 bytes, and the generators were never found.
- Slot 8's state is byte-identical to `fixtures/m169/eu-bot-start.p2s`.
- Cause unknown. Unverified guess: the state does not load in the slot copy's PCSX2 v2.9.52. To check: vsync and the game id after load, and a screenshot.
- Journal 3's successful run used `HST_SPEED=0.6`. Mine did not.

## Code
- `crates/hst-play/src/play.rs`: the stroke keeps the contact frame; `simulate` calls `bodyhit::detect`.
- `crates/hst-play/src/play/bodyhit.rs`: `detect` is a function of `simulate`'s; `sim_plugin` for the harness.
- `crates/hst-play/src/play/fifty_hz.rs`: `HST_M183A1_FLIGHT`, and the floor raised to 40.

## Status
Blocked on a 9700-tick generator recording (see the attempts above). Phase 40 waits on it.

## Not verified / not 1:1
- Whole-match parity is not proven.
- Phase 40's cause is not found. It is a hypothesis only.
- Phases 11, 14 and 21 (positional parts) are not diagnosed.
- The US comparison passed its existing floors in the fifty_hz tests; whether it now matches more phases was not looked at.
- `FIFTY_HZ_SIM` is not flipped, and its release note is not written (the body hit has its own line).
- A hit on a deciding bounce's tick follows the stated order; no recording shows one, and no suite test guards the hit tick (only the ignored probe).
- `HST_BODY_HIT`'s forced hit is now judged the same tick; not re-recorded.
- The earlier parts' items stand.
