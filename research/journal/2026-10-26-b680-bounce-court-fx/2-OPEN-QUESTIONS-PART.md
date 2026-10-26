# B680 — spec phase 2: open questions

User (2026-10-26, mid-session): the remaster has no instant replays yet, so leave the replay parts for the replays
task. SPEC.md now has a *Scope of the port* section: this task ports the live-play parts (wet-court spray, body-hit
ring scale, the smash clod burst on every court); the replay-only parts stay in the spec marked *deferred to B72*,
and B72's line has a note pointing at them.

## Answers
- **Blade size death** (replay-only, recorded for B72): the bounce blades die at size < 0, not the dive blades'
  0.01; with shrink 0.95 they only die on reaching the ground. Checked in the disassembly (compare against zero).
- **`rng.sound` order on a frame:** the effects run in creation order: foot/dive effects, hit sparks, then the bounce
  debris. The sparks' end-of-burst re-roll draws before the smash burst. Added to SPEC §4.
- **Does a replay restore `rand()`?** Only matters for the replay blade burst: moved to B72 as an open point.

## Next (Port phase)
Port the live-play scope from SPEC.md; verify per SPEC §5.

## Not verified / not 1:1
- Nothing run against the game in this phase (no PCSX2). The frame order comes from reading the program; §5 has the
  port phase prove it with `rng.sound`'s state after a smash landing.
- Replay-only behaviour is deferred to B72 by the user, including whether a replay restores `rand()`.
