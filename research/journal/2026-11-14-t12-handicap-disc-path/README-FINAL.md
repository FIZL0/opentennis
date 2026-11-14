# T12 — handicap confirm-screen test failing on the shared branch

## Cause
Not a code regression, so there was nothing to bisect. The test failed in its disc part with
`HST_REQUIRE_DISC: no /home/ryha/repos/opentennis-slots/s4/crates/hst-data/../../Hot Shots Tennis (USA).iso`.
That folder is the old runner slot home, which no longer exists. `hst_data::fixture::ISO_PATH` and `root()` were
built from `env!("CARGO_MANIFEST_DIR")`, and sccache handed this worktree a `libhst_data` rlib compiled in the old
checkout (every `target/debug/deps/libhst_data-*.rlib` here holds the `opentennis-slots/s4` string). sccache doesn't
key on the crate's directory, so the baked-in path pointed at that other checkout.

## Fix
`crates/hst-data/src/fixture.rs`:
- `root()` is found at run time: the running test's `CARGO_MANIFEST_DIR/../..` (cargo sets this when it runs a
  test), else the compile-time dir, else an ancestor of the test binary. Each candidate must contain
  `crates/hst-data/Cargo.toml`.
- `ISO_PATH` is now `../../Hot Shots Tennis (USA).iso`, relative to the test's working directory (cargo runs tests
  in their own crate dir). It stays a `const &str`, so none of its ~37 users had to change.

Checked: the T12 test, all of hst-data, and hst-play `hit_fx` (disc tests) pass under `HST_REQUIRE_DISC=1`.

## Not verified / not 1:1
none (test infrastructure only; the original game isn't involved). If a test binary runs from a working directory
other than its crate dir (not cargo), `ISO_PATH` won't resolve. No test does this today.
