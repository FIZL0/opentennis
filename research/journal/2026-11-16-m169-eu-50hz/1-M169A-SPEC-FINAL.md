# M169a — EU 50 Hz spec

`SPEC.md` in this directory is the clean-room spec for the European program's 50 Hz differences. The port is M181
(decompile locked).

## What was done
- Read every timing/motion/logic row of M169's `classify.tsv` (195 rows) as a normalised US/EU C diff. Changed
  float immediates were pulled out with their bit patterns. Scratch scripts and the combined report are in
  `context/m169a/`.
- Sorted M169's 84 untriaged timing rows and its "sim?" rows into sim and presentation (SPEC section 12).
- Compared the boot program's data word for word (`context/m169a/bootdata.txt`). The only timing changes are the
  rain and leaf rows (presentation).
- Also compared the **game overlay's** data (`context/m169a/ovldata.py`, `ovldata2.txt`): the words between
  functions, with relocation noise filtered out. M169's hash skipped this too. Found:
  - the shot parameter table ×1.2 in fields 0/1/7/9 (already known: `hst_data::exe` undoes it for the 60 Hz
    European mode; the 50 Hz build must not undo it);
  - the practice time-limit and grade tables;
  - hit-spark and flight-effect data;
  - a keyed list with odd ratios (open question 3);
  - and one thing not retimed: the post-point/game/set wait table is identical to the US one.

## Findings worth knowing beyond the spec
- The rounding isn't uniform. Floats were typed again as ×1.2 or ×1.44 of the US *decimal*, not the US f32 (the
  serve walk becomes 0.0399996, not 0.04). Counts are rounded to the nearest frame, except 80 → 66, 5 → 4 and
  10 → 8 (×0.8), and 220 → 183.33333 (kept fractional).
- M169 labelled one block "doubles AI", but it is one-player practice (feeder, coach, drills).
- The weather's wind doesn't reach the ball in our model, so wind 1/500 is presentation (open question 2).

## Not verified / not 1:1
- **No European recording was used**: no PCSX2 run of the European disc in this session. A spec phase needs none,
  and M181 proves the port against M177's European fixtures. All values are read from the two programs and their
  decompiles.
- **The bit patterns of constants loaded as immediates** were read from the binaries. A few constants the decompile
  shows only as decimals came from the binary where it mattered (1.000001 = 3f800008, 5/6 = 3f555555); the rest of
  the presentation decimals weren't pinned.
- **The disc's sound and voice files** weren't compared (only the programs).
- **Video and aspect rows** (screen height, buffers, 0.7 → 0.8) are listed but not specced: the remaster has its own
  renderer.
- **Practice** is only partly ported in the remaster, so its retimes apply only where practice code exists.
- **Open questions 1–4** in SPEC.md, each needing a European recording:
  1. whether the next point waits on the retimed scoreboard stage;
  2. whether the ball is in the wind list;
  3. the reader of the odd-ratio keyed list;
  4. the sim's clip lengths per motion.

## Wrap-up (second session, 2026-11-16)
Checked SPEC.md against every item in the brief (constants with their rounding, motion switch and exceptions, nudge,
km/h, wind, gravity, run speed, serve, AI, ball/replay counts, court life, the 84-row sort, boot/overlay data):
all present, audit clean. The open questions need European recordings, which the port task M181 gets from M177;
a note on M181 points at them. Ticked here: the port runs as M181, not as a second phase of M169a.
