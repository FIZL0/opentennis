# M177: European disc recordings at 50 Hz (for M169a1)

I booted the European release (SCES-54535, Everybody's Tennis) in PCSX2 copy 1. It runs PAL: I measured 49.999
vsyncs a second over PINE, and the picture has 512 lines. I drove its menus to a CPU-only doubles match, saved
states, and recorded:
- a bot match with `tools/record_p2m2.py`, in the US sample layout,
- a motion-clock capture with `tools/record_anim.py`.

Everything is in `fixtures/m169/` (git-ignored).

## Finding the EU addresses
The recorders depend on four fixed addresses:
- the frame clock,
- the pad manager,
- the match globals (they hold the match-object pointer),
- the rally block.

**How I mapped them.** M169's function matches (`context/m169/match.tsv`) pair each US function with its EU twin.
For every US instruction pair that builds one of these addresses (`lui` + offset), I decoded the same instruction
in the matched EU function. All the hits agree:

| what | US | EU | votes |
|---|---|---|---|
| frame clock | 0x1d5780 | 0x1ea480 | 34 |
| pad manager | 0x2efb00 | 0x304800 | 46 |
| rally block | 0x3165f0 | 0x32b2f0 | 697 |
| match globals | 0x422f80 | 0x437e00 | 1540 |

So the boot-program data moved up by 0x14d00 and the match globals by 0x14e80.

**Struct offsets.** fmatch masks memory offsets, so I compared the 1011 exactly matched overlay functions
unmasked. The only differing immediates are address low halves. No struct field offset moved.

**Checked live:**
- Court 6 and player count 4 sit at the US offsets from the globals.
- The characters are 0, 1, 1, 0 (Yelena, Yuki, Yuki, Yelena).
- Player positions (+0x3d70) stay inside the court all match.
- Clip lengths come out as 5/6 of the US value, nudged ×1.000001: 33.3334 and 166.6668. That is M169's motion
  rescale.

## Tool changes
- `tools/rec.py`:
  - `DISCS` lists each release's addresses (US, EU). `disc(p)` picks the entry by the running serial and points
    `pine.VSYNC` at that disc's frame clock.
  - `match_regions(p, d=None)` takes that entry. Called without it, it behaves as before (US).
- `tools/record_p2m2.py` and `tools/record_anim.py` call `disc(p)`, so they record either disc with no flag.
- `tools/record_anim.py` keeps the clip length's own 4 bytes. PINE reads whole 8-byte words, so since the batched
  reads it had been writing 8 bytes per player: a 4900-byte sample instead of the 4884 of `anim_s05.bin`. The
  layout now matches `anim_s05.bin` again. The EU capture was trimmed the same way.
- `tools/screenshot.sh` takes the copy's newest snap by time. Snap names start with the game's title, so the old
  name sort kept returning a stale "Hot Shots Tennis" snap once another disc was booted.
- `research/README.md`: an EU section (how to boot it, the addresses, the menu route, the fixtures).

## The match
- **Route** (also in `research/README.md`): English → Start → play data 1, named PLAYER, no card save → Tennis
  with Everybody → Doubles → four COMs → characters → Start the match.
- **Settings:** Alpine Heights (grass), umpire Chika, 1 set of 4 games. Yelena/Yuki (A) vs Yuki/Yelena (B).
  A won 5–3.
- **Speed:** recorded with the copy at 0.6× (`HST_SPEED=0.6`). At 1× the first take dropped about as many frames.
  The match recording ran under `HST_PCSX2_MAX_HOLD=1500`: at 0.6× the 32000 frames take about 18 minutes, which
  is past the default cap. This is my own copy, so no other agent waits on it.

## Fixtures (`fixtures/m169/`)
| file | what |
|---|---|
| `eu-bot-start.p2s`, `eu-bot-start_ee.bin` | State and RAM saved at vsync 37096, the first frame the match object exists (loading screen). Play starts around 37700. |
| `eu-bot-match.bin` | `record_p2m2.py` from that state: 32000 samples, vsync 37109–69198. 8324-byte `frames_live` samples (US layout). It ends at 3–3, before the match does. |
| `eu-bot-late.p2s`, `eu-bot-late_ee.bin` | State and RAM at vsync 71076, 3–3, same match. 1878 frames after `eu-bot-match.bin` ends, not continuous with it. |
| `eu-bot-late.bin` | `record_p2m2.py` from the late state: 15000 samples, vsync 71052–86068. The match object's phase byte (gm+0x55) turns 5 at 78164, game set match. Samples after that are the end screen. |
| `eu-bot-anim.bin` | `record_anim.py` from the start state: 12000 samples, vsync 37108–49192, 4884-byte samples (as `anim_s05.bin`). |

Gaps (frames that ticked mid-read; the recorder skips and logs them):
- `eu-bot-match.bin`: 90 frames missing in 53 gaps. 49 of the gaps fall in the first 1000 frames (loading screen).
  The rest are single frames at 39230, 47036, 47636 and 48743.
- `eu-bot-late.bin`: 17 frames missing. One gap falls before the match ends (73631). The rest are on the end
  screen.
- `eu-bot-anim.bin`: 85 frames missing, all in the loading screen.

## For M169a1
- `hst_sim::replay` hard-codes the US globals (`GLOBALS_ADDR`) and rally (`RALLY_ADDR`) addresses for its
  RAM-address accessors. EU samples need the EU base: globals 0x437e00, rally 0x32b2f0.
- The replay must step over missing vsyncs. `match_s05.bin` has none, so the existing tests may assume none.
- The bot match is long enough for a bit-exact run: 31000 gameplay frames, about 10 minutes at 50 Hz.

## Not verified / not 1:1
- **No continuous whole-match recording.** `eu-bot-match.bin` stops at 3–3. `eu-bot-late.bin` restarts from a
  state taken 1878 frames later and runs to game set match. Re-record from `eu-bot-start.p2s` with about 42000
  frames if one file is needed.
- **A few gameplay frames are missing** (listed above). `match_s05.bin` has none: it was recorded lock-step, which
  was removed in October. Slower copies (`HST_SPEED`) still drop the odd frame.
- **Start state taken during loading.** The state is saved at the frame the match object appears, mid-load. It
  reloaded and played cleanly three times (all three recordings). If a test needs a calmer start, take one at the
  first serve.
- **Struct offsets were checked statically** (the exact-matched functions) and on the fields used above. Not every
  recorded field was checked one by one against EU code.
- **Characters and court are what a fresh save allows** (only Yelena and Yuki unlocked). That is not slot 5's line-up
  or its court 10.
