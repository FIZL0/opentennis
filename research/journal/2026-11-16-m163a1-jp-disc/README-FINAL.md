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

## Spec round 2: the open questions answered
- **Ball-path hit flag:** it switches on the red landing marker at the aim point. Japan shows it on the serve too.
  It's presentation only: the CPU positions from its own shot target, not the marker. SPEC §5.
- **AIParam parser:** found in both builds by the file name string. The two row readers read cells the same way;
  Japan's row has two fewer plain-integer cells.
  - Which two: Japan's CPU reads its body-shot and low-shot rates from the record entries where the US keeps its
    lob and special-serve reactions. So exactly those two are missing, as the header said. SPEC §1.
- **The rest of the differing routines** are classified in SPEC §9. None changes play.
  - The check: compare, for every differing pair, the sequence of calls that reach `rand()` or the MT draw
    (`scripts/rngcmp.py`, call graph to depth 5, Japanese callees mapped back through the exact matches).
  - Only these sequences differ:
    - the already-specced serve coin and CPU reaction draws
    - the setup gallery-sound pick, which a reseed overwrites before it matters
    - a court-generator sound roll (presentation)
    - practice-only objects
  - The point-show timings are unchanged.

## Port phase (next session)
Implement SPEC.md as a per-release path (US unchanged and bit-exact):
- the CSV column maps
- serve timing on the rally table
- no strong-toss miss and no special serve
- no counter, no dull flag
- the Japanese aim narrowing, with no 90° shots
- no CPU lob or special-serve reaction draws
- the CPU smash search without its baseline check
- the red landing marker on the serve (display only)
- the km/h formula
- front-end fallbacks for the missing menu entries

Then record a bot match on the Japanese disc in PCSX2, compare it, and only then let `Release::playable` accept
Japan.

## Not verified / not 1:1
- **Nothing was ported. Japan stays refused.** This was the spec phase (PLAN.md *Clean-room ports*).
- **No PCSX2 run of the Japanese disc.** The spec comes from the decompile alone, so none of it is checked against
  the running game.
- **§9's classification is static.** It is based on which routines can reach a random draw and on reading the
  diffs, not on a run. Sim-touching paths were followed, presentation ones were not read line by line.
- **The red marker on the serve** comes from the decompile; nobody has seen it on a Japanese disc.
- **Presentation differences** (effect colours, sound priorities) only have to feel right, and weren't specced.
