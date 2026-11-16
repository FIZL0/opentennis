# M163 — other regions' discs: research

The research for the two retail releases was done inside M163a (`2026-11-16-m163a-any-region-disc`), and its
subtasks then made them play: JP M163a1 and its children, EU M170 / M174 / M169, crossplay M163b / M171, voices
M163c / M168, sibling discs M166, the Rules setting M175. This entry pulls the per-disc verdicts together and adds
the one disc nobody had looked at yet: the **January 19th prototype**.

Images are under `fixtures/discs/` (git-ignored): `SCES-54535.iso`, `SCPS-15113.iso`, and the new
`SCES-54535-proto-0119/` (its extracted file tree; the image itself is in `fixtures/discs-proto/`, because in `fixtures/discs/` the sibling-disc scan took it for the retail EU disc and `discs::tests::finds_the_other_releases` failed: noted on M179). Archive entries for EU,
JP and the prototype were extracted with `xbdump` into `context/m163/{eu,jp,proto}`. US entries are in `fixtures/xb`.

## Per disc

| | US | Japan | Europe | Jan 19 prototype |
|---|---|---|---|---|
| boot line / serial | `SCUS_976.10` | `SCPS_151.13` | `SCES_545.35` | `SCES_545.35` (the same as retail EU) |
| SYSTEM.CNF | NTSC | NTSC | `VMODE = PAL` | `VMODE = PAL`, VER 1.00 |
| volume dates | | | 2007-02-14 | ISO 2006-09-15, files 2007-01-19 |
| languages | English | Japanese | 11 (`.XB0`–`.XBA`: En Fr De Es It Nl Pt Sv No Da Fi) | the same 11 |
| file list | 646 | own names (`.XB`, voices in `PCANI`) | 2406 files | identical to retail EU (2406, same names) |

### Match data compared with US, entry by entry (`context/m163`)

- **Europe**
  - PCDATA (TParam/AIParam/…): identical. PC models, PCANI motions, CMN and IRX: identical.
  - SAITOU/TRAJ: all 211 entries differ (retimed for 50 Hz).
  - COURT: `plant_c03_h01_0.dat` differs (court 3 plant records, the main and SSN1 copies).
  - COURTSET: court 5's trigger creature model `trg_M-06` (MDL/MTI) differs.
  - GAME.BIN: the US data at +0x180. Exceptions: the shot-parameter base (per-frame amounts ×1.2) and 2
    camera-script words (M163a, M174).
- **Japan**
  - TParam.csv and AIParam.csv differ (63 vs 67 columns, 43 vs 45).
  - 14 of the 211 TRAJ entries differ.
  - The same court 3 plant file and court 5 creature model as EU, plus the camera scripts of courts 1, 3 and 9.
  - pc01's costumes 05, 06 and 09 (MDL/MTI) and the ASCII font differ.
  - GAME.BIN: piecewise shifts, with its own tuning in a few tables (M163a, `exe.rs` `JAPAN`).

### Prototype compared with retail Europe

- **Match data: identical.** PCDATA, SAITOU/TRAJ, COURT, PC, PCANI, IRX and CMN (except the ASCII font) match
  byte for byte. So the TRAJ tables are the same 50 Hz ones.
- **GAME.BIN**
  - Same size (0x100400). It loads 0x980 higher (0x338380, not 0x337a00).
  - In the data region the tables match retail EU at the *same file offsets*, i.e. the US data at +0x180. The only
    differences are 1,310 words that are pointers moved by exactly +0x980, plus two other sets:
    - 6 umpire word countdowns (two in each table for language row 3, one per table in row 4). Row 0, the one the
      remaster reads, is unchanged.
    - 33 words past the end of every table `exe.rs` reads.
  - Program code: every differing instruction is a relocation. 16,302 immediates or jump targets moved by the
    overlay's +0x980 or the boot program's shifts (+0x9e0 code, +0xa00 data). 77 calls go into boot-program
    library code that moved by other amounts. No opcode or register field differs, so it runs **the same game logic
    as retail EU**.
- **Boot program**
  - `SCES_545.35` is 952,400 bytes (retail: 949,968).
  - The tables `exe.rs` reads (court plates, sound pan/gain, stereo, bank volumes) are byte-identical to retail's.
    They sit at the US location +0x7b00 below 0x1ce970 and +0x156e0 from there on (retail: +0x7100 / +0x14d00).
- **MENU.BIN**
  - 470,912 bytes (retail: 468,864), same load address as its GAME.BIN.
  - The character grades are identical, at 0x3a3af0 (retail: 0x3a2970).
- **What differs, all of it presentation:**
  - the menu archives (188 files) and title walls
  - INPANE, PRIZE/OTHER and EFFCT.XB1 per language
  - HSM (`i_coatchange_00`, `hsm_indicator`)
  - all 48 umpire banks (English included)
  - the Spanish character voices (`*.XB3`, 43 files)
  - court 5's `trg_M-06` creature model (MDL/MTI/MTL)
  - the ASCII font
  - the two movies (`A/OPEN.PSS`, `A/ED01.PSS`)

## Video mode and timing

- US and JP run at NTSC 60 Hz.
- EU and the prototype run at PAL 50 Hz with no 60 Hz option. The game is retimed rather than slowed down: TRAJ
  frames ×~0.8 and speeds ×1.25, the shot parameters ×1.2, and the program's timing constants (M169's
  classification).
- So a 50 Hz EU match is a different simulation from the US one. The remaster plays EU in two ways:
  - at 60 Hz on the US rules, exactly when US data is present (M174), or close to exact on regenerated tables when
    the EU disc is alone (M170);
  - at 50 Hz as its own mode (M169a/M169a1).

## Verdicts

- **US**: the reference.
- **Japan**: plays, but differs. It is an earlier build with its own rules (CSV schemas, no missed-serve table,
  shot blends, rain odds, TRAJ). It is ported as `Build::Japan` and checked bit-exact against a JP bot match
  (M163a1a1). Done.
- **Europe**:
  - It plays, but differs at 50 Hz.
  - With US data present it plays the US game exactly (M174, open). Alone it plays at 60 Hz, close but not exact
    (M170, done).
  - A real 50 Hz mode is M169a → M169a1 (open).
- **January 19th prototype**: **can't today, but becomes exactly retail Europe** as soon as `exe.rs` knows its
  layout. Its SYSTEM.CNF names the retail EU boot program, so `Release::of`/`Iso::playable` take it for Europe.
  Then the GAME.BIN overlay check (load address), the boot-program size check and the MENU.BIN size check all fail,
  and the import stops with "GAME.BIN does not match the SCES-54535 Europe 1.00 overlay" (from code reading; not
  run). Once it is read, its sim data and the online gameplay digest match retail EU's: the same tables, the umpire
  language-0 rows `digest.rs` hashes, and the same TRAJ for M170's regeneration. Online it would meet EU and every
  other release under M171's rules. Its pack hash differs, so it needs its own `KNOWN_PACKS` line.

## Subtasks

Already made and mostly landed: M163a, M163a1 (+a1a, a1a1, a1a1a), M163a2 → M174, M163b, M163c, M166, M166a/M175,
M168, M169, M170, M171. Still open: M174 (EU exact with US data), M169a/M169a1 (EU 50 Hz).

New from this entry: **read the January 19th prototype as a second European build** (`agent-runner plan`, see
TODO). It needs a layout keyed by boot-program size or GAME.BIN load address under `Release::Europe`, using the
values above, plus a `KNOWN_PACKS` line and a `tests/releases.rs` case: every table equals retail EU's, except the
language 3/4 umpire countdowns.

## Not verified / not 1:1

- **The prototype was never run in PCSX2.** Every finding is from the image: file, entry, word and instruction
  diffs. "Same game logic as retail EU" rests on the instruction check (every differing word is a relocation).
  Constants loaded from boot-program data outside the tables we read weren't compared one by one.
- **The import refusal is from reading the code**, not from running it on the prototype.
- **Which language the umpire countdown rows 3 and 4 are** (probably Spanish and Italian, matching the changed
  Spanish voice banks) wasn't checked. The remaster only reads row 0.
- **Boot-program differences** outside the tables `exe.rs` reads (2,432 bytes larger) weren't analysed. The library
  code moved, but nothing showed it behaving differently.
- **US-vs-EU/JP "missing" entries** in the entry comparison are files that `fixtures/xb` holds and xbdump didn't
  extract (non-XB0 language archives, renamed JP archives). M163a handled the JP aliases; they weren't compared again
  here.
