# T5 — tests that passed without checking

The audit (README, §5) found ~208 tests in hst-sim/hst-data/hst-oracle that `return` green when the ISO or a
`context/` recording is missing, and the ISO path hard-coded in 40 files.

## What changed

- `hst_data::fixture` (`crates/hst-data/src/fixture.rs`, behind hst-data's `testing` feature, on for the tests of
  hst-data, hst-sim and hst-oracle through their dev-dependencies): `iso()`, `read`, `read_to_string` (same return
  types as `Iso::open`/`std::fs`, so the tests' `let Ok(..) = .. else { return }` stayed), `exists`, `path`, `root`,
  `skip(what)`, `required()`. A missing file prints `skipped: no <path>`; with `HST_REQUIRE_DISC=1` it panics instead.
  One helper in hst-data serves all three crates (hst-oracle's `root()`/`program()`/`from_ram` now go through it, and
  its old `HST_REQUIRE_DATA` check is gone, as is the one in hst-sim's `tests/data.rs`).
- Every disc/recording read in `crates/{hst-sim,hst-data,hst-oracle}/tests/` (and texhash's disc test) goes through
  it; every explicit "skipped"/"absent" message became `fixture::skip(..)`. The ISO path is now in exactly one place
  in these crates (`fixture::ISO`).
- `tools/check.sh` exports `HST_REQUIRE_DISC=1` (override with `HST_REQUIRE_DISC=0`), so `done.sh`'s suite fails when
  a test can't check anything.
- **Worktree fallback** — running the suite with the flag found 18 tests failing in slot s5: their recordings were
  in *no* main-checkout `context/`; 13 of them exist only in some runner slot's `context/` (recorded there, never
  copied back), and the slots' own `context/` copies are stale (s5: 117 fixtures vs main's 192). So in the slots
  dozens of tests had been skipping quietly. `fixture::path` now takes a `context/` file this checkout lacks from
  the main checkout or another worktree (found through the git dir's `worktrees/`). With it all 341 tests of the
  three crates pass under `HST_REQUIRE_DISC=1`, the 18 included, checking against the real recordings.
- `prox_c08.bin`/`prox_c09.bin` were never recorded anywhere; `startled_creatures_match_the_game`'s doc already
  calls those courts extras, so they alone stay optional.
- texnames' check of the user's `mods/texture-replacements/` keeps a plain skip: user content, not game data.
- `receive_matches_the_game` now asserts every receive sub-state ran (as `_long` does); passes.
- cutaway.rs `debug_frame` deleted (ignored, asserted nothing, needed `K=`).
- The three tests T4's ci.yml skipped (`match_serve_walk`, `beach_plane_flies_over`, `serve_follow_through`) skip
  without data now; the `--skip` list is gone from ci.yml.

## Checked

- `tools/check.sh -p hst-data -p hst-sim -p hst-oracle --no-fail-fast` (flag on): 341 passed; slowest binary 4.4 s.
- CI shape: the crates copied to a scratch dir with no `.git`, ISO or `context/`, `cargo test --locked -p hst-sim -p
  hst-data`: all green without `--skip`; the same with `HST_REQUIRE_DISC=1`: 75 passed, 218 failed (the data tests,
  matching the audit's ~208).
- clippy `-D warnings` on the three crates, all targets: clean.

## Follow-ups

- T10: the Bevy crates' unit tests (hst-base, hst-play, hst) still open the ISO by their own path and skip quietly.
- T11: copy the slot-only recordings into the main checkout's `context/` (deleting a slot loses them today).

## Not verified / not 1:1

- No gameplay change, nothing to compare against the original.
- A *stale but present* file in a worktree's `context/` wins over the main checkout's (only missing files fall back);
  recordings aren't re-recorded under the same name in practice, so not handled.
- Reads that are optional by design (TRAJ A/B lookups, `.poke`/`.seed` side files, an optional second RAM dump)
  use `fixture::path` for the fallback but don't fail under the flag.
