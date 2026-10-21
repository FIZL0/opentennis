# T10 — the Bevy crates' disc tests skip through `hst_data::fixture` too

T5 follow-up: hst-base, hst-play and hst unit tests opened the ISO by their own hard-coded path (40 files) and
returned green without it, so `HST_REQUIRE_DISC=1` (check.sh) couldn't catch a run that checked nothing.

## What changed

- hst-base, hst-play and hst take `hst-data` with its `testing` feature as a dev-dependency.
- `hst_data::fixture` gained `ISO_PATH` (the disc's full path, for tests that hand it to the app as `Args::iso` or
  to `--iso`) and `has_iso()` (the presence check, skip/fail like `exists`).
- Every `Iso::open(<repo path>)` in the three crates' tests is `fixture::iso()`; every `Path::new(ISO).is_file()` is
  `fixture::has_iso()`; the `ISO` consts are `fixture::ISO_PATH`; the "no ISO, skipped" prints are gone (the fixture
  says it). No hard-coded ISO path is left in the three crates.
- `context/` reads in those tests go through the fixture as well, so they fail under the flag and take a file this
  worktree lacks from the main checkout: `context/fixtures/round1.bin` (snapshot, tick_input), `context/ram/s03.bin`
  (serve_check), and the converted mods in `context/mods` (hst-base `modfs::tests::test_mod(id)`, used by mods.rs,
  modfs.rs, lod.rs; hst-play mod_match/preload; mods_online; hst online.rs).
- Left as plain skips (not disc/recording data): `mods/hst-mods` (the mods repo's builds) and `HST_MODS`. A test
  needing both the ISO and one of those still fails under the flag when the ISO is missing.
- `mutated_glbs_never_panic` keeps the ISO optional (it fuzzes with or without it): plain `Iso::open(ISO_PATH).ok()`.

## Found on the way

- With the fallback, `modfs`'s `browser_install_leaves_upscaled_out` and `same_id_mods_both_install` ran in this
  slot for the first time and failed: the main checkout's `context/mods/test_pc00` holds `mod.json.from-s3` and
  `mod.json.from-s4` (merge backups from 2026-10-08) that an install refuses. They fail the same way in the main
  checkout today. The two tests now zip only the mod's own files (`mod_files`, leaving `*.json.*` leftovers out);
  the leftovers themselves are still in main's `context/` (not touched from a runner slot).

## Checked

- `cargo clippy --tests -p hst-base -p hst-play -p hst`: clean.
- `tools/check.sh -p hst-base` (flag on): all pass after the fix above; `tools/check.sh -p hst-play -p hst`: 461
  passed; `HST_NETPLAY=1 tools/check.sh -p hst-play netplay`: 35 passed.
- With the ISO link moved away: `court_load` fails with `HST_REQUIRE_DISC=1` and skips ("skipped: no …iso") with 0.

## Not verified / not 1:1

- No gameplay change, nothing to compare against the original.
- The ignored tests (full-match netplay/rollback, `tick_cost`, …) were only compiled, not run; their changes are
  the same mechanical skip checks.
