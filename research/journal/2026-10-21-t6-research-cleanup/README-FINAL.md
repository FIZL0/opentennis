# T6 — research/tools cleanup (from the 2026-10-21 repo audit)

Findings: `research/journal/2026-10-21-repo-audit/README-FINAL.md`. Internal work only, no release note.

## What changed

- **Dev-stats ledger out of git.** `research/dev-stats.json` (612 KB, local session paths) is removed. `tools/dev_stats.py`
  keeps it in the *main checkout's* `context/dev-stats.json` (found through `git rev-parse --git-common-dir`, so
  every worktree shares one ledger). Until that file exists, the script starts from the last copy git had: the
  parent of the commit that deleted it (`old_ledger()`, checked: 2284 transcripts read back). `tools/release.sh`
  now commits only `web/stats.json`.
- **No hard-coded home paths.** `tools/pine.py` has `pcsx2_folder(key)` (reads `[Folders]` from this agent's
  `PCSX2.ini`: the copy's under `../<repo>-slots/pcsx2/sN` when `HST_PCSX2` is set, else the user's) and
  `state_file(slot)`. `tools/record_npc.py`, `p3e7_start`, `p14c3f_court8` use `state_file`; `b31e_glare_poke`,
  `b74b_hunt`, `p3f5_flare_hunt` use `pcsx2_folder("Snapshots")`. Checked: both modes resolve to the existing
  folders. `tools/single.sh` was already gone (agent-runner replaced it). The comment in `tools/guard_decomp.py`
  no longer names a home path.
- **Task-named probes into their journals.** New `tools/probes.py` (`<ID>`, `--all`, `-n` dry run) moves
  `research/<id>_*.py|sh` and `research/<id>/` folders into `research/journal/<entry>/scripts/`. Journal choice: an
  entry named for the task, else one that mentions the script, else one named for a parent task (b74d → b74;
  never p112 → p11). It `git mv`s, then fixes `__file__`-relative paths (folded to the same target from the new
  folder), adds a `sys.path` line where a script imports a probe that now lives elsewhere, turns a shell
  script's `cd "$(dirname "$0")/.."` into `git rev-parse --show-toplevel`, and rewrites every tracked
  `research/<probe>` reference (journals, plan, Rust doc comments, tools). `--all` moved 158 entries; every moved
  `.py` compiles. `tools/done.sh` runs `tools/probes.py <ID>` on tick (worktree mode; the main checkout commits
  only the named paths, so there it's skipped).
- **Unreferenced scripts.** `hud_rec.py` → P19's journal, `spark_rec.py` → P9's (task probes without an id prefix),
  `research/tools/bounce_model.py` removed (an early numpy model; `hst_sim::ball` is the ported one).
  `mdlvif/mdlwalk/mtlwalk/vifwalk` stay: `research/README.md` indexes them and `mdlwalk` is imported. 11 single-bug
  `tools/web_*.mjs` probes moved into the journal that used them (their `new URL('../…')` paths re-aimed, all pass
  `node --check`); `web_check`, `web_lan`, `web_touch`, `web_zen` (in `plan/REFERENCE.md`) and `web_webkit`
  (generic WebKit runner, used by another probe) stay in `tools/`.
- **`fixture_shot_tables.py`** runs `traj_inverse2.py` from its own folder (`research/tools/`).
- **One helper module, `tools/rec.py`:** the MIPS stub assembler (full o32 register table, encoders, `br`,
  `assemble`), `samples()`/`vsync()`/`cut()` for fixed-size fixture samples, `f32`/`u32`/`f32_at`, and
  `match_regions` (the match sample layout `record_p2m2.py` and `play_p2m2.py` both had). Converted: the five
  `tools/record_ai_*.py` (assembler), `record_aim`, `record_human_smash`, `record_live`, `record_npc`, `record_sound`,
  `trace_live`, `record_ai_rally` + research `bodyhit_rec`, `call_shots`, `marker_depth`, `marker_rec`,
  `smash_mark_rec`, `bounce_rec` (casts), research `bodyhit_parse`, `bounce_rec`, `flight_rec`, `fx_rec`,
  `trail_rec` (readers), `record_p2m2`/`play_p2m2` (regions). `record_ai_serve`'s `assemble(base, prog)` ignored
  `base`; its two calls now pass the program only. `record_ai_rally`'s local address-reading `f32` lambda is
  `ram_f32` now, so it no longer shadows the cast.
- `research/README.md`: the probes policy and the shared helpers.

## Checks

- Assembler: each recorder's old encoders (from HEAD, exec'd) vs `rec`'s on 3000 random argument sets per
  function, the register numbers each file used, and a branch/label program through `assemble`: identical.
  Static name check over every edited recorder: nothing undefined.
- Readers: `frames()` of the five research readers over `context/fixtures/{bodyhit,bounce,flight,trails,effects}_s05.bin`,
  old vs new, pickled and hashed: identical (200/1440/2400/2400/2279 samples).
- `match_regions` vs the old `regions` on 100 mock RAMs: identical.

## Not verified / not 1:1

- The recorders weren't re-run against PCSX2 (each needs a long capture); the equivalence checks above cover the
  code that changed (stub words, region lists, casts). The moved probes weren't run either, only compiled; they are
  archived one-offs for finished tasks.
- Archived journal scripts keep their own copies of the helpers (deliberately: they're the record of what ran).
- `tools/pick.py`'s `f32(p, addr)` (a different signature, imported by archived probes) and
  `balloon_anchor.py`'s tuple-returning `fl` were left as they are; `research/tools/ps2fpu.py`'s `bits`/`fromb` are
  that module's own API.
- Not in this task: the audit's `sys.path.insert` for pine in 156 files, the shell `set -e`/unbounded-wait items.
