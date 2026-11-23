# B784: the extra rand() at the change of ends (PART: spec done, port next)

## Question
- The European build draws one more rand() as a change of ends starts than play does.
- Play draws only the sound reseed there (M183a1 part 12).
- The game draws 2 besides the flare's 24 on that tick.

## Method
- New script: `research/journal/2026-11-16-m183a1-eu-bot-match/scripts/eu_rand_calls.py <slot> <from vsync> <to vsync> <out.bin>`.
- It hooks rand()'s entry. It logs every call's vsync, return address, sp and 16 stack words over a vsync window.
- It runs from a save-state load, under `tools/pcsx2.sh`.
- It works on the European disc and the US disc.
- Outputs in `context/b784/`:
  - `run1`: EU, slot 8 (`fixtures/m169/eu-bot-start.p2s`), window 43470 to 43580.
  - `run3`: US, slot 5, window 14140 to 14250. The hook went in a frame late and missed the start frame.
  - `run4`: US, slot 5, window 14060 to 14170. It has the start frame.

## Findings
- EU change-of-ends start (vsync 43492):
  - Two non-flare draws. First the effects' message handler, which redraws the effects seed on the match-start and change-of-ends messages. Then the sound manager's reseed.
  - Then the flare's 48.
- EU setup frame (37518) has the same pair in the same order, after its 30 cloud draws.
  - That is the setup's effects seed and sound reseed #2, which play already makes.
- EU change of ends' end (43558) draws only shared and court, as play does.
- The US does the same.
  - `fixtures/b767/us-rng-game2.bin` counts 26 draws at the change of ends' start frame (14154).
  - The run4 caller log shows the same two callers in the same order.
  - So the draw is in both builds. It is not a 50 Hz or European rule, though the task line said European only.
- Play draws its effects seed once, at setup (`Rngs::setup_rand`), and never redraws it.
  - `rain.rs` restarts the effects LCG from that seed at every serve.

## Spec
- See `SPEC.md` in this directory.
- In short: at every change of ends' start, draw one rand() as the new effects seed, before the sound reseed.
- The port replaces the test-only `fifty_hz::CHANGE_ENDS_DRAWS` stand-in.

## Not verified / not 1:1
- Tiebreak and set-end changes of ends were not recorded.
  - The spec says the redraw follows every change-of-ends message, because the handler does not look at the score.
  - That comes from reading the decompile, not from a recording.
- What the new seed changes on screen (the next points' rain grid, leaves and ripples) was not compared against the game. It is presentation only.
- The port's effect on the probes (EU 24 phases with no env; US rand() through the change of ends) is for the port session to check.
