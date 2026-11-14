# T13 (+ T12, T14–T22): main_menu disc tests failing in B704's gate

## Cause (one, shared by all 11)
B704's gate log (`~/runs/opentennis/merge/context/notes/merge-gate.log`, 12:45) shows every failure as
`HST_REQUIRE_DISC: no /home/ryha/repos/opentennis-slots/merge/…/Hot Shots Tennis (USA).iso` (or `no fixtures/mods/…`,
which goes through the same `fixture::path`). That folder is the runner's old slot home; the merge worktree now lives
in `~/runs/opentennis/merge` and has the ISO and `fixtures/` links.

`hst_data::fixture::root()`/`ISO_PATH` are built with `env!("CARGO_MANIFEST_DIR")`, so the absolute path is fixed at
compile time. Cargo's fingerprints for workspace crates use relative paths, so when the worktree moved with its
`target/`, hst-data was never recompiled. The gate linked `libhst_data-b6b9…rlib` from 2026-10-21 (200 hits for
`opentennis-slots`, 0 for the new folder). That build stayed stale until a hst-data source change (97de7ce1, 12:52)
rebuilt it at 12:54, which is why nothing reproduced afterwards. Nothing was wrong in the code or the tests, so there
was nothing to bisect: at this branch's HEAD (which already contains e47146186a), `tools/check.sh -p hst main_menu`
passes all 291 tests, including the 11.

## Fix
`tools/check.sh` (`moved_checkout`) stores the checkout's absolute path in `target/.checkout`. When it changes, or
on the first run with no stamp when the newest hst-data rlib names another folder, it runs `cargo clean -p` on the
workspace's own crates (dependencies kept) before testing. Verified: with a bogus stamp it printed "checkout moved …",
rebuilt, the T13 test passed, and the stamp was rewritten.

The single fix commit (this task's done.sh commit) covers T12–T22. None of them needed its own code change.

## Not verified / not 1:1
- I didn't recreate an actual worktree move end to end (stale rlib → guard → pass). It was tested with a bogus stamp,
  which runs the same clean path.
- No gameplay was touched, so there was nothing to check against the original.
