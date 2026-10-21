# R11 — docs drift (from the 2026-10-21 repo audit, §9)

Docs only, no code.

## Fixed
- **PLAN code map**: `overnight.sh` was already gone (agent-runner line, 2026-10-21). Added the unmapped modules:
  hst-data `mor noi snd texhash uselog fixture` + bins `bankwav noidump texkeys texnames` and more test files;
  hst-sim camera extras (`instant`), AI (`ai aim rally position path`), court life (`npc umpire weather clouds
  shade lines court_anim`), effects/sound (`effect tornado foot bodyhit surprise sound face noise stats`); hst-base
  `android_pad disc jobs chat device vertex_patch effects court_anim weather noise spring inspect hud_gamma mips
  asset_use scene_census stall`. hst-play's `play/` stays summarised as "match parts" (each file's `//!` says what it is).
- "README *Character mods*" → `docs/DEVELOPING.md` *Character mods* (PLAN code map, REFERENCE).
- REFERENCE: the removed B524 backdrop bake paragraph now says the splash shows the disc's court photos, blurred;
  the "README *Steam*" sentence (X11 launch options) removed — no such section exists anywhere; the older duplicate of
  the court-mods (M8f6) paragraph and the half-duplicated web M9 lines removed (kept the newer, fuller copies).
- AGENT.md: `crates/game-sim` / `crates/kit-oracle` are named as the decomp kits' own repos, not this one.
- TODO header: tests are in `tests/` for hst-sim, hst-data, hst-oracle; in-module for hst-base, hst-play, hst.
- docs/DEVELOPING.md: lists all 7 crates; notes that `HST_REQUIRE_DISC=1` (check.sh) turns the disc skips into failures.

## Proposed for the user (not applied: rule wording) — also in plan/HUMAN.md
PLAN *Continue*, first bullet, now "Find it in the decompile (`context/decomp/`, `research/fn.sh <addr>`), port it,
prove it …". Proposed:

> **Match the original exactly** — no approximations or placeholders. Spec it from the decompile (`context/decomp/`,
> `research/fn.sh <addr>`), port it from the spec in a second session (*Clean-room ports* below), prove it against the
> real game: …

AGENT.md *Cheap context*, now "`research/xref.py <addr> --similar` (look-alike functions; `ported` ones have Rust to
copy the shape of)". Proposed:

> `research/xref.py <addr> --similar` (look-alike functions; `ported` ones already have a spec: a new spec can point to
> the behaviour they share instead of describing it again)

## Not verified / not 1:1
none (documentation only; nothing to check against the original).
