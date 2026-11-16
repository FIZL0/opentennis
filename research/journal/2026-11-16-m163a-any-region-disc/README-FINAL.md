# M163a — play from any region's disc

M163 (the research task) had no journal yet, so this task did that research too, on the three releases' images
(`fixtures/discs/SCES-54535.iso`, `fixtures/discs/SCPS-15113.iso`, the US ISO).

## Releases

| | US | Japan | Europe |
|---|---|---|---|
| serial / boot | SCUS-97610 `SCUS_976.10` | SCPS-15113 `SCPS_151.13` | SCES-54535 `SCES_545.35` |
| video | NTSC 60 Hz | NTSC 60 Hz | PAL 50 Hz, no 60 Hz option |
| GAME.BIN | MWo3 overlay, 0x100280 bytes, loads 0x322d00 | 0xfff80, loads 0x324900 | US data at file +0x180 |
| archives | `X.XB0` (+ `X.XB1`…) | `X.XB` (no language digit) | `.XB0`–`.XBA`, 11 languages |
| voices | `SND/VOICE/PC/PCnnVCEk.XB0` | in `PCANI/PCnnANIk.XB` (motions + set k) | as US |
| SG2IOPM1.IRX | — | identical | identical |

**Europe** is refused, with its own message. Every TRAJ table is retimed for 50 Hz (frames ×~0.8 in each cell's top
byte, speed ×1.25), and the remaster's match is the 60 Hz game. Its GAME.BIN data is the US data shifted uniformly,
except the shot-parameter base (151 words) and 2 camera-script words. Follow-up: M163a2.

**Japan** reads fully, but doesn't play yet. It is an earlier build, and its rules differ from the US game's:
- `TParam.csv` has 63 columns, not 67: the US's strong-toss and weak-toss IMP columns are missing (it has one
  rabbit/turtle pair).
- `AIParam.csv` has 43 columns: the US's lob and special-serve reaction times are missing.
- GAME.BIN has no missed-serve table.
- The CPUs' shot blend weights for characters 0–11 and 13 differ by a couple of words each.
- It has higher rain odds on courts 1, 2, 4, 5, 6 and 10.
- Its umpire word countdowns exist only for Japanese; they equal the US table's Japanese rows.
- TRAJ strk3/voly3 differ for some characters.
- Program code differs (the GAME.BIN layout around the tables moves piecewise; see `exe.rs` `JAPAN`).

Its menu archives also move or lack entries the front-end reads. `message0.dat` is missing (its text lives
elsewhere). `word.tm2` in `PRIZE/OTHER`, `inpane_Counter` and `fukidasi_dokuro` are missing. `Loading.tm2` and
`info.tm2` are in `MENU00B` rather than `MENU01`.

A JP match on the remaster panicked at setup (TParam column 55 isn't a number there), which is what showed the schema
difference. Follow-up: M163a1 (spec the JP rules from a JP decompile, port as a per-release path, verify with a JP
recording, then accept it).

## What landed

- `hst-data::exe`: `Release` (Us, Japan), read from SYSTEM.CNF's BOOT2 line. The boot ELF and overlays are checked
  per release. Each release has a `Layout`: boot/GAME/MENU sizes and load addresses, the MENU grades location, and
  piecewise shift tables from US locations (a `None` entry means "this release doesn't have the table").
  - Reads that span a place where the JP data inserts or drops bytes are assembled piece by piece, so they come out
    as the US bytes would. Before this, the shot-weight rows read 4 inserted JP words.
  - Pointers read from tables resolve in the release's own address space (NPC names, trigger model names).
  - `Release::playable` is Ok only for the US.
  - US reads are byte-for-byte what they were (one segment, shift 0).
- `hst-data::iso`:
  - US names resolve on the JP disc: `.XB0` → `.XB`, voices → `PCANI/PCnnANIk.XB`, motions → `PCnnANI0.XB`, and the
    boot program → the release's own.
  - The pack keeps the face blanking for `INPANE.XB`.
  - The JP pack hash is a known copy: 0d38064f5be1fdd5, "SCPS-15113 Japan 1.00".
  - `Iso::playable`.
  - `release_name(hash)`.
- App: the import (command line, file picker, start page) and a direct ISO launch refuse a readable but unplayable
  disc with the release's message, not "unrecognised disc contents".
- `crates/hst-data/tests/releases.rs`:
  - every `Game` table, the boot program's sound and court tables, and the grades: JP equals US except the listed
    tuning differences
  - JP aliases
  - EU refused
  - (ignored) JP import hash
  - Without `fixtures/discs/SCPS-15113.iso` (not the user's disc) it skips even under HST_REQUIRE_DISC.
- Checked: the US match still starts and plays (a `--shot` at 12 s, in `context/m163a/run3`). JP direct launch and
  `--import` print the refusal.

## Not verified / not 1:1

- **The Japanese release doesn't play.** Its rules and CSV schemas differ from the US game's, and porting them needs a
  JP decompile and recordings (M163a1). The task line asked for "play from any region's disc". This task delivers
  the reading layer (detection, offsets, files, pack) for JP, and the refusal.
- **Europe is refused** (50 Hz shot timing, M163a2). Its shift tables were found (+0x180 uniform plus the shot base),
  but they are not in `exe.rs`, because nothing can use them yet.
- **JP `serve_miss` returns zeros.** It has no such table, and what its serve does instead is unknown (M163a1).
- **The JP umpire word timings** for languages other than Japanese fall back to the Japanese rows. The disc has no
  others.
- **JP program code was not compared.** Only data was mapped. The trigger rows past type 52 and the surface rows past
  material 121 are program bytes beyond the tables, and they differ between releases; nothing reads them.
- **The camera shots' bytes 64..68** are a program pointer and differ per release. The sim doesn't read them.
- **No PCSX2 run of the JP or EU disc.** Everything is from the disc images.
