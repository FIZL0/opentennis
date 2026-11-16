# T26: `rules_choice::tests::another_release_beside_the_main_disc` flaky

## Failure
P141's merge gate: `lay(&disc, r)` returned `Err("No such file or directory (os error 2)")`; passed alone.

## Cause
Shared global state between tests in the same process. `lay` finds the sibling disc through the global
`SIBLINGS` list, and `sibling(r)` returns the *first* entry for a release. The test pushed its own entry
(the real fixture path), but `releases::tests::registers_the_discs_beside_and_their_voices` (and the
European one) call `releases::find`, which calls `rules_choice::add_sibling` with symlinks in
`/tmp/hst-releases-<pid>/home/` and then deletes that dir. Whenever the releases test ran first, `SIBLINGS`
already held `(jp, <deleted temp path>)` ahead of ours, so `lay` opened a file that no longer existed. Under load
the order changes, hence the gate-only failure. It had nothing to do with timing or timeouts.

## Fix
`lay` now splits into `lay(disc, r)` (looks the sibling up) and `lay_from(disc, path)` (the work). The test calls
`lay_from` with the fixture path and no longer touches `SIBLINGS`, so it neither reads other tests' stale entries
nor leaves its own behind for others (e.g. the options test that checks `available()`). No behaviour change for
the game.

## Checked
`tools/check.sh -p hst-base -- rules_choice releases` (both modules in one process, as in the gate) 3× passed;
each filter alone passed.

## Not verified / not 1:1
none (test-only robustness fix; no gameplay touched).
