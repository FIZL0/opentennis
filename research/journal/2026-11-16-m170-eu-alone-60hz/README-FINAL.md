# M170: the EU disc alone at 60 Hz, close enough (offline)

The European disc (SCES-54535) now plays on its own. Its match runs at 60 Hz with the US rules
(`Build::Us`). The program data is read as the US data, and the shot tables are re-solved at read time by our own
solver. It's close to the US game but not the original, and the main menu header says so: "SCES-54535 (60 Hz: close,
not the original)".

## What the EU disc holds (follow-on from M163a2)

- **GAME.BIN**: the US program at file offset +0x180 (same load address, size 0x100400). Its data matches the US
  data except for these:
  - **Shot records**: fields 0, 1, 7 and 9 of every 13-float record are ×1.2. Every nonzero one goes back to the US
    value bit for bit via eu/1.2 in f64, rounded to 6 significant digits, then to f32 (`exe::sixty_hz`). Test:
    `releases.rs` `european_tables_read_as_the_us_ones` checks all 151 values.
  - **Two camera script times**: 6.4→6.34 and 8.7→8.58. Presentation only, so the remaster keeps the US ones.
  - **Pointers**: relocated.
- **Boot ELF**: 949_968 bytes, with two shifts against the US layout. Its sound, stereo, bank and court tables are
  equal to the US ones.
- **MENU.BIN**: the timing grades are equal to the US ones.
- **TRAJ**: all 485 tables were re-solved by the authors for 50 Hz. Speeds are about ×1.2 and frames about ×0.83.
  Three cells in four keep the US elevation.

## How the tables are rebuilt (`hst_sim::shot::regen`)

`Iso::read` goes through a retime hook (`iso::set_retime`, installed at startup and in the match setup). For a release
with `regenerated_shots()`, every `TRAJ/` archive is rebuilt once per run, about 2 s per archive on threads.

Each cell keeps the EU elevation. It is then handled one of two ways:

- **Reached cells** (the speed ÷1.2 lands within 0.5 m of the cell's target): the speed is re-solved so the shot lands
  nearest the target on our 60 Hz ball flight, flown with the shot record's spin. Serves and smashes use the fixed
  record spin; strokes and volleys use `rally_spin`. The cell's frames are the flight's.
- **Unreached cells**: keep the scaled cell (speed ÷1.2, frames ×1.2). These are short targets from deep in the court,
  which the authors' solver didn't reach either. Measured: there, the US cell's frames follow the scaled EU frames,
  not our flight's frames.

The rebuilt archive is written as raw entries (`xb::write`).

## How close it comes (`every_european_table_against_the_us_disc`, all 14 characters, 1,986,560 cells)

| Shot | km/h off: mean / p95 | Frames ±1 | Landing off: mean / p95 |
|---|---|---|---|
| Serves | 1.84 / 7.8 | 83.6% | 5.0 / 9.2 cm |
| Strokes | 0.90 / 4.0 | 92.0% | 7.8 / 27.1 cm |
| Volleys | 0.57 / 2.5 | 89.4% | 7.9 / 34.3 cm |
| Smashes | 1.61 / 9.3 | 87.4% | 4.4 / 9.6 cm |
| **All, regenerated** | **1.01 / 4.6** | 88.7% (65.1% same) | **7.0 / 25.8 cm** |
| All, merely scaled | 1.18 / 4.2 | 89.6% (53.7% same) | 9.9 / 24.3 cm |

11.6% of cells are bit-identical to the US cell.

- **Landing error**: measured by flying both cells on our model. Our model alone puts some US stroke and volley
  cells metres off (M163a2), and that shows in the max values.
- **Same elevation**: where the EU solver kept the US elevation, the regenerated speed is about 0.5 km/h off and
  frames are within one in about 90–100% of cells.
- **Different elevation**: the large errors (up to 90 km/h, all on serves and smashes) are cells where the EU solver
  picked a different arc. No 60 Hz solve from the EU cell can recover the US arc there (M163a2: the US solver can't be
  rebuilt bit for bit).

## Other changes

- **Disc import**: the EU disc's pack hash `5895f0a79053fe1a` is in `KNOWN_PACKS`, so it imports. The start page's
  wrong-disc message now names it.
- **Packs and the online digest**: both read the disc's raw bytes (`read_unlogged`, now documented as un-retimed).
  The pack writer used `read`, which would have written rebuilt archives at the wrong sizes. Fixed.
- **Online**: the digest differs from the US and Japanese ones, so an EU disc alone meets only other EU discs. Those
  rebuild identically, since the rebuild is deterministic (ps2 float model). M171 (streaming exact US data to an EU
  peer) has to bypass the retime hook (noted on M171).
- **Bot match**: `build_rules.rs` `a_bot_match_on_the_european_disc_alone` plays US rules, reads rebuilt TRAJ, and
  rallies play out.
- **Follow-ups**: notes on M166a (Rules row) and M169 (Match Speed row), so the "close, not the original" marker also
  shows there once those settings exist. Today only the main menu header shows it.

## Not verified / not 1:1

- **The match itself**: not exact by design. The regenerated tables are close, not the US tables (numbers above). No
  replay or PINE proof is possible: there is no 60 Hz EU original to record.
- **No PCSX2 run**: comparing against the real 50 Hz EU game would compare a different tick rate, so it was skipped.
  The measurement is against the US disc's tables instead.
- **Special-shot bends**: special shots' extra bends aren't in the plane flight model used to solve the cells, so
  those cells come out like ordinary cells.
- **Two EU camera script times**: not used (presentation only).
- **Not compared against the US files**: SAITOU/TRAJ (the remaster doesn't read it), 3 court HOL files and HSM.XB0
  (differ on the EU disc, not examined).
- **Two EU peers online**: they match each other, but no netplay run was made on EU discs.
- **Other platforms**: the rebuild was checked on x86_64 only. It uses only `hst_sim::ps2` arithmetic, so it should
  agree on other platforms, but that wasn't tested there.
