# T14: `a_mod_pick_goes_out_again_once_read_in` fails on the shared branch

## Finding
No code or test change was needed. No merge broke it, so there was nothing to bisect. Every gate that logged it
(B702/B703 on 2026-11-05, then B704/B705/B706 on 2026-11-14, `~/runs/opentennis/merge/context/notes/merge-gate.log`
and `gates/*.failed`) failed the same way, together with the other disc/fixture tests:

    HST_REQUIRE_DISC: no fixtures/mods/getagrip_pc00_emi
    HST_REQUIRE_DISC: no /home/ryha/repos/opentennis-slots/merge/crates/hst-data/../../Hot Shots Tennis (USA).iso

`/home/ryha/repos/opentennis-slots` no longer exists (it was the old runner home). sccache gave the merge
worktree a copy of `hst-data` compiled in that old checkout, so `fixture::root()`, which was built from
`env!("CARGO_MANIFEST_DIR")`, pointed at a deleted folder, and every test that reads `fixtures/` or the disc
failed. T12 (`245cdade`, on this branch) already fixed it: `root()` now finds the repo at run time, and
`ISO_PATH` is relative. The merge gates after T12 all passed (`gates/*.pass` at 12:57, 13:01 and 13:10, the last
one T13's merge).

## Verified
- `tools/check.sh -p hst a_mod_pick_goes_out_again_once_read_in`: passes (1 test).
- `tools/check.sh -p hst main_menu::online`: 43 tests pass.

## Not verified / not 1:1
none (test infrastructure only, nothing gameplay-related).
