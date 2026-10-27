# B678a Plants' `.MOR` sway on the disc courts

## What binds
`hst-sim/tests/court_anim.rs` (`plant_morphs_bind`) reads every `tree/*.mor` in courts 2, 3, 4, 6 and 9 (both
seasons) with its model:
- Every track binds a morph target of its own model on court 2 (glass3, glass4, hibis, yashi-02, yashi-03), court 4
  (midtree01, sakura), court 6 (lowtree, shirakaba01–03) and court 9 (midtree01).
- court 3's aroe and kabeki-2, court 4's uekomi and court 6's sugi ship a `.MOR` but their models have **no morph
  targets** (no names after the packets, none in the file's strings): nothing of theirs can move. The test pins this
  list.

## Port
The library already built `court_anim::Morph` meshes for any model with a `.MOR` (B678), but only the hole's
`Playing` got them. `spawn_court`'s placed-structure loop now inserts the model's `Morph` on its `Playing` too
(`crates/hst/src/main.rs`). The meshes are the library's shared ones, so every placed copy of a plant moves in step,
on one clock started when the court spawns.

## Phase in the original (PCSX2, court 2 state 10, RAM)
- `leafs\x01leafs01` occurs 4 times in EE RAM: the two palm models' `.MDL` and `.MOR`, loaded once each, not per palm.
- Each loaded plant `.MOR` (header 80 ticks/frame) is pointed at only by one small object just in front of it (two
  pointers to it 0x18 apart), and by nothing else: one player per model, none per placed plant. So all copies of a
  plant share one clock: in step, as the port does.
- I couldn't find the clock or weight value itself in two RAM images 0.4 s apart (a scan for weight runs was too
  noisy). Scripts: `context/b678a/names.py`, `scan.py`, `clock.py`.

## Checked
- `--stage 2` at 7 s vs 9 s: palm fronds, grass tufts and hibiscus differ, trunks/hut/hotel identical (diff image
  `context/b678a/run1/diff.png`); fronds bend plausibly (`palms.png`).
- `--stage 6` / `--stage 4`: birch, shrub, maple and cherry canopies move; cedars still (`context/b678a/run2/*_cmp.png`).

## Not verified / not 1:1
- In-step phase rests on the pointer evidence above (one player object per loaded `.MOR`), not on a read clock value
  or a side-by-side of two copies of a plant in PCSX2. The start phase (court spawn) wasn't compared with the original's.
- Sway amplitude and speed were checked only by eye in our renderer, not side by side with PCSX2 frames (the match
  camera shows little of the palms). Presentation: by eye per PLAN.md.

## Blocked on an unrelated test
`tools/done.sh` failed only on `hst-base mods::tests::oob_faces_all`: the installed `oob_pc18_toro` mod (shared mods
folder, rewritten 15:16 by another session's rerig work) "plays [] … no target" for every face clip; the same failure
that blocked B678. This task touches neither mods nor characters. Rerun `tools/done.sh B678a` once that mod is rebuilt.
Other gates, run by hand: audit passes; clippy fails on main's own code (B678's `court_anim.rs` type complexity, fixed
here with a `MorphTrack` alias; `hst-base` audio.rs:882 and spring.rs:217 from other merges, left alone); the web
check fails on a warning in `third_party/bevy_audio` (not this task's).
