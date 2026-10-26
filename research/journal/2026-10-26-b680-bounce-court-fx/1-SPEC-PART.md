# B680 — bounce effects per court: spec phase

## What I found
- The bounce mark, puff and ring are the same on every court in live play: `ballkon_00` and `kemuri`, recoloured
  from the court look table. The grass mark the port draws is the original's.
- What grass (and clay) courts add is a separate **bounce debris** effect, thrown on the first bounce:
  - grass blades (`par/siba`) on courts 5, 6, 8 and 10;
  - clods (`par/tuti`) on courts 1, 2, 3 and 9;
  - **only during instant replays**, the same gate as the dive debris.
- A smash landing (85 km/h or more, more than one player) throws a clod burst on every court, in live play too.
- The material effect kinds 1..15 only drive the word pop-ups, which are already ported. No other bounce effect
  reads them.
- The bounce effect lacks:
  - wet-court spray (ring model, puff texture, bigger rising puffs, the wet smash puff);
  - the ring's 1.5 scale after a body hit (not "replay scale", as the P9f journal had it);
  - the marks being restored when a replay starts.
- Randomness: the smash burst takes 120 sound-generator draws in live play. Grass replays take 90 C-library
  `rand()` calls, which seed later points. The port has to spend both.

The full rules are in `SPEC.md`.

## Next (Port phase)
Implement from SPEC.md:
- the throw and aging in hst-sim, next to `effect::Bounce` (reuse `foot.rs`'s clod/blade throw shape);
- the bounce debris table read in `hst-data::exe`;
- the drawing in `hst-base/src/effects.rs`;
- the missing wet/body-hit parts of `Bounce`.

Verify per SPEC §5.

## Not verified / not 1:1
- Nothing was checked against the running game in this phase. Everything comes from reading the program. PCSX2 was
  not started. The recordings and screenshots in SPEC §5 are the port phase's.
- The blade size-death threshold (0 vs the dive journal's 0.01), where the trigger falls in the frame's
  `rng.sound` order, and whether a replay restores `rand()` are open (SPEC *Open questions*).
- The slow-motion variant is only outlined (offbeat rules).
