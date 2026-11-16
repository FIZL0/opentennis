# M163a2 — play the European disc: needs a decision

Everybody's Tennis (SCES-54535) is the US game rebuilt to run at 50 Hz. The task line asks the user to choose
between (a) a 50 Hz match mode and (b) playing the EU disc at 60 Hz with the US timing. This session measured what
each choice needs, using the disc images (`fixtures/discs/SCES-54535*`, the US ISO). It wrote no code, because both
choices need their own work first, and which one to do is the user's call (cross-region online play, M163b,
depends on it).

## What differs from the US disc

Compared file by file (same path, same bytes):
- **Identical:** 481 of 645 shared files. These include every `PCDATA` file (TParam/AIParam CSVs, character
  data), every `PCANI` motion, `PC` models and nearly all courts. Motions are *not* retimed.
- **Different:** `TRAJ/` (28), `SAITOU/TRAJ/TRAJPCnn.XB` (6), `ZZBIN/` GAME/MENU/MOVIE.BIN, `MENU` (41), `SND` (77),
  `SAITOU/HSMODE/HSM.XB0`, `AZUMA` INPANE/OTHER, and 3 court files: `COURT/03/HOL01.XB`, `COURT/03/SSN1/HOL01.XB`,
  `COURTSET/05/HOLE01.XB`. Plus 1,761 files only on the EU disc (the other languages).
- **TRAJ tables:** all 485 tables differ. Over 1.99 M cells, the median speed ratio is ×1.207 (p5–p95: 1.16–1.22),
  and the median frames ratio is ×0.833 (0.81–0.87). That is 60/50 and 50/60, so it is a physical retiming, not the
  ×1.25 the task line says. The elevation also changed in 26% of cells (median 0, p95 +12 units of π/4096). So the
  tables were re-solved at 50 Hz and rounded again, not scaled. **The US tables can't be recovered from the EU
  ones.** One EU frame value maps back to as many as 12 different US values, and 1,400 of 1,462 EU speed values map
  back to more than one US value.
- **GAME.BIN data** (the tables `exe.rs` reads): the US bytes at file +0x180, except the shot-parameter base, where
  exactly 151 floats differ. Each one is the US value ×1.2: record fields 0, 1 and 7 (per-frame speeds and
  amounts), and field 9 in 9 records. These do round back to the US decimals (0.12 → 0.1, 1.416 → 1.18, 456 → 380).
- **GAME.BIN code:** the program differs throughout. Between the US 0x342d00 and 0x3f2d00, only 1–3% of words match
  at any one shift. It is a relinked build, and its 50 Hz frame constants and physics per tick are in the code, so
  they can't be read as data. How much of the logic changed is unknown without an EU decompile.

## The choices

**(a) 50 Hz match mode: play the EU game as the EU game.**
- Needs:
  - an EU decompile (a Ghidra project of its GAME.BIN and boot ELF)
  - a clean-room spec of every timing and physics difference against the US code
  - a per-release sim path
  - PCSX2 recordings of the EU disc at 50 Hz to prove it bit-exact
  - the remaster's fixed 60 Hz tick made switchable to 50, with rendering interpolated to the display
- Size: several tasks, on the scale of M163a1 (JP) or bigger.
- Online: an EU player can't share a sim with a US/JP player (different tick rate and tables). M163b would have to
  refuse that pairing, or both peers would need the EU disc's mode.

**(b) 60 Hz with the US timing: the EU disc plays the US game.**
- The US code is already ported, so this needs the US *data*:
  - shot parameters: ÷1.2 and rounded to the US decimals (looks exact; needs checking on all 151)
  - the 2 camera words
  - the 3 court files (not yet looked at)
  - `SAITOU/TRAJ` (not yet looked at)
  - **all TRAJ tables, which the EU disc can't give back.**
- The only way to get the TRAJ tables without shipping US data is to **regenerate them**: solve each cell
  (elevation, speed, frames) with our ported 60 Hz flight. That is only good if the solver reproduces the US tables
  bit for bit. That can be tested on the US disc alone: regenerate the US tables and compare.
- If it works:
  - EU players play exactly the US game.
  - Cross-region online works (M163b).
  - The EU disc gives menus, the other languages and voices (M163c).
- If it doesn't: (b) is only an approximation (EU tables scaled back), which breaks "match the original".

**Recommendation:** first try (b)'s solver (a research task). It is a clean yes/no that runs on data we already have,
and if it works, it gives the user's stated goal (every disc, players on different discs playing each other) at a
fraction of (a)'s cost. Fall back to (a) only if the regenerated tables can't match.

## Not verified / not 1:1

- Nothing was ported or changed. Europe stays refused, as M163a left it.
- The 3 different court files, `SAITOU/TRAJ`, `HSM.XB0` and the 2 camera words weren't examined byte by byte.
- How much of the EU program's logic (not just its timing) differs from the US one is unknown: no EU decompile.
- No PCSX2 run of the EU disc.
