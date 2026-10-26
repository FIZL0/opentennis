# Bigger Chief tune (user request 2026-10-25)

User: smaller, no sped-up animations, serves that land, much harder hits.

- Mod (opentennis-mods `tools/rerig/hst_bigger_chief.py`, v2): `scale` 2 → 1.5, `swing_speed` dropped (1),
  new `shot_power` 3. `run_speed` 0.25 kept.
- Serves: at 2× his toss contact was 5.7 m up, past the serve tables' heights; every serve landed 11–13 m out
  (probe in a bot match). The table lookup clamps, so it can't aim from there.
- New §6 `shot_power` (1–4), `Build::power`; `Game::shot_build` (power, scale) per player, set by
  `mod_match::apply`. `play.rs` `powered`: for a mod with power or scale ≠ 1, every serve/drive/volley/smash (not
  lobs or drops) launches at table speed × power, its elevation bisected on `Flight::step` so the first bounce lands at
  the aim's distance; if it can't clear the net (1.23 m at the crossing), the speed steps down toward the table's.
  The curve profile is re-stretched over the solved flight; a slice serve's side-angle wind is dropped.
- Stock characters never reach `powered` (shot_build (1, 1)); online refuses the field (`Build != default`).
- Test: hst-play `mod_match::tests::powered_serve_lands_in` (1.5×, power 3: a serve lands in, > 2× Carol's speed).
  Probe numbers: his serve ~2.9 units/frame vs Carol's ~0.6; in a 4-minute bot match his serves and drives land in,
  with Carol returning some.

## Not verified
- No look in the running game (user tests UI live). Feet still slide at ¼ run speed (M11 note).

## Follow-up: max stats instead of shot_power (user request 2026-10-25)
User: big characters hitched on every hit; use the max stats and drop the power solve.
- Cause: `powered`'s launch solve flew ~5–6k `Flight::step`s per hit (bisection × 11 speed steps), 70–100 ms in a
  frame. Tuning it got to ~10 ms; the stock tables top out at ~164 mph serves (Chief's own; pc10's special 200)
  against the 3× serve's ~390, so the user chose stock stats over the solve.
- Removed `powered`, `Game::shot_build`, §6 `shot_power`. New §6 `stats` ("F"–"A", `Build::stats`): the mod's row
  graded like M147's Everyone's stats, in `mods::tparam_row` (so the select's grades show it too); the grading
  moved to `hst-base/src/grades.rs`. Online still refuses it (`Build != default`).
- `scale` now grows only the reach columns (55–61), not the hit heights: with the heights grown and no solve every
  1.5× serve flew 8–9.5 m (probe); with stock heights his serves land in/out/net like a stock character's.
- Mod v4 (opentennis-mods): `stats` "A", no `shot_power`. Tests: `mod_match::tests::bigger_chief_serves_in`,
  `bigger_chief_plays` (heights now pc07's).

## Round 3: faster ball, best stats, bad stamina

- §6 `serve_speed` / `shot_speed` (1–4): the match runs that many ball steps a frame for the mod player's
  serves / other shots until the first bounce (or a net/post touch), advancing the frames-since-hit count with them
  so the CPUs' predicted path stays in step. Same path, so it lands as aimed; stock players run one step, unchanged.
- `stats` "S": each graded cell is the roster's best (largest or smallest, the way the A characters' mean lies from
  the F ones'). POW is capped at 16 inside the shot code, so ×2 cells would clamp anyway; the speed comes from the
  extra ball steps instead.
- Bigger Chief v5: S, serve 3, shots 2, run 0.2 (S takes the best SPE, so lower than 0.25 to keep him slow),
  STA 14 (tired below 10) and dive/backhand/smash costs 16/8/16.
- Still refused online (Build ≠ default).
