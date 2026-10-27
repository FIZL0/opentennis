# M147 — Offbeat rule: everyone's stats (READY: rerun done.sh after B663)

User 2026-10-24: one Offbeat rules option that puts every stat category of every character at one grade, F–A;
default None.

## What landed
- Rules box (confirm screen, Select / C): new pill **Everyone's stats: None / F / E / D / C / B / A** (✕ cycles);
  the strip's Rules cell shows "All A" etc. Flag: `--offbeat stats-a` (`stats-f` … `stats-a`).
- `Offbeat::stats` (0 none, 1 F … 6 A; `#[serde(default)]`). With None nothing below runs, so the match is the
  original's (replay tests untouched).
- `hst-play/src/play/offbeat/stats.rs`: the select screen's five grades are only a summary on the original's menu,
  not sim inputs, so a grade has to stand for data. Each category maps to the TParam.csv cells behind it:
  - Serve: Serv POW, Serv CON, the serve near-miss scatter
  - Stroke: Strk/Lob/Lob2 POW, the three spins, Strk CON, Rizing ADJ, Strk GH/NH, Back DWN, LOW POW, Strk HI POW
  - Volley: Voley POW, Voley CON, Voley GH/NH, V DWN, V LOW POW
  - Impact: the timing-frame cells (rally, strong toss, weak toss) — the beginner characters with A Impact have the
    widest windows, Will (F) the narrowest
  - Footwork: SPE, STA, stamina costs, Agili
  A player keeps their own row (build, reach, contact heights, collision, body ADJ, hand; a mod its own row) with
  each category's cells set to the mean of the 14 stock characters' cells at that grade (from MENU.BIN's grades; every
  grade of every category has at least one character on the US disc; whole numbers rounded half away from zero).
  The graded row then goes through the same paths a row always feeds: run/stamina (`stats_of`), aim (`aim_of`), power
  pop-ups, rally timing grades and the timing bias (`Timing::restat`), the serve data (rebuilt like setup does).
  Applied at the rules' PostStartup, before Player speed paces the run.
- Means by grade (F→A): Serv POW 3, 4, 6.3, 8.7, 10, 13.5; Strk POW 4, 5, 7, 9, 12, 16; Voley POW 2, 4, 7.5, 7.25,
  11, 15 (the disc's D volleyers hit harder on average than its C ones); SPE 9, 9, 10, 11, 12, 12.

## Tests
- `play::offbeat::stats::tests` (mean, graded row, disc grades F<A), `play::offbeat::tests::everyones_stats_are_one_grade`
  (4 CPUs at A all get JJ's footwork and identical aim/timing/serve stats, a match plays 3000 ticks with hits),
  hst's offbeat menu test.
- Build note: this slot's incremental cache kept panicking rustc ("failed to open LTO bitcode file … pre-lto.bc");
  `CARGO_INCREMENTAL=0 tools/check.sh …` built fine.

## Not verified / not 1:1
- An Offbeat rule of ours, not the original's: no original behaviour to verify against. What a grade means is a
  design choice (the mean of that grade's characters' cells); the original never turns grades into stats.
- Trajectory tables, shot records (weight rows), serve variant tables and the computer's AI row stay the character's
  own, so characters still differ in shot shape and CPU skill; only the TParam stats are evened out.
- A mod's own run pace (`run_speed`) is dropped under the rule (its Footwork is the grade's); its swing pace stays.
- Not tried in a live window (headless tests only); the rules box grows to 8 rows (360 of 448 px, centred).

## Wrap-up
The work is done and its own tests, clippy and the audit pass. `tools/done.sh` couldn't finish, though. The full suite
failed only in hst-base `mods::tests::oob_hair_swings_and_settles` ("oob_pc00_jasmine: the hair never settled"). That
test reads `fixtures/mods/oob_pc00_jasmine`, a link into ../opentennis-mods/out, and task B663 (rescaling the
out-of-bounds mods) rebuilt that mod at 22:34 while this ran. The test doesn't touch this task's code. Committed by
hand and blocked; to unblock, rerun `tools/done.sh M147 …` once B663 has landed (its new mod and test agree).
