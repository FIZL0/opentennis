# M163a1 — Japanese release rules: spec phase

This session built a decompile of the Japanese program and compared it with the US one, function by function. It
wrote `SPEC.md`: the rule differences in plain words, for the port session (which works with the decompile locked).

## How it was compared (scratch, in `context/m163a1/`, not in git)
- The Japanese overlay ELFs were rebuilt the way `research/README.md` does for the US, and a Ghidra project made
  from them, with the same dump scripts.
- The functions were matched with a masked-instruction hash (call targets, address high halves and memory/immediate
  offsets masked), then paired by link order between exact matches:
  - boot program: 2375 exact, 51 unmatched
  - match overlay: 1221 exact, 108 unmatched
  - About 200 US functions differ.
- Each differing pair was diffed as decompiled C with the addresses and offsets normalised (`scripts/cdiff.sh`) and
  as disassembly (`scripts/dis.py`). Most differences are only the smaller player record and shot record (the
  Japanese shot record is 0xe8 bytes, against the US's 0x118).
- The real differences are in `SPEC.md` §1–§6.

## Port phase (next session)
Implement SPEC.md as a per-release path (US unchanged and bit-exact):
- the CSV column maps
- serve timing on the rally table
- no strong-toss miss and no special serve
- no counter, no dull flag
- the Japanese aim narrowing, with no 90° shots
- no CPU lob or special-serve reaction draws
- the CPU smash search without its baseline check
- the path hit flag
- the km/h formula
- front-end fallbacks for the missing menu entries

Then record a bot match on the Japanese disc in PCSX2, compare it, and only then let `Release::playable` accept
Japan.

## Not verified / not 1:1
- **Nothing was ported. Japan stays refused.** This was the spec phase (PLAN.md *Clean-room ports*).
- **No PCSX2 run of the Japanese disc.** The spec comes from the decompile alone, so none of it is checked against
  the running game.
- **Not examined in detail** (listed under SPEC.md *Open questions*):
  - practice-mode routines
  - a cutaway/replay chooser
  - court-life/NPC routines
  - the scoreboard
  - some boot-program CSV helpers
  - the voice loader
- **The ball-path hit flag's effect** isn't tied to our code yet.
- **The Japanese AIParam parser** wasn't located. Its column map rests on the CSV header and the record layout.
- **Presentation differences** (effect colours, sound priorities) only have to feel right, and weren't specced.
