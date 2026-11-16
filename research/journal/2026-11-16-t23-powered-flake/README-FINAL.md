# T23 — `shot::powered::tests::powered_serve_table` flake

## Cause
M172's merge-gate log: `built in 5.649189477s` then a panic at the `took.as_secs_f32() < 5.0` assert. The test's
only nondeterminism was that wall-clock bound on building the powered table; under a loaded gate (other suites,
other slots' builds) the build ran past 5 s. Everything else in the test (fly frames, top speed, determinism, the
pinned hash, power-2 cap) is pure arithmetic on disc data: no ports, files beyond the read-only ISO, or threads.

## Fix
Dropped the wall-clock assert; the build time is still printed (`eprintln`) for anyone watching cost. The sibling
`probe_is_quick_and_pinned` had the same kind of bound (`< 1 s`) and would flake the same way, so it went too
(the fingerprint pin stays). No timeout was widened.

`tools/check.sh -p hst-sim powered`: 137 passed.

## Not verified / not 1:1
none (test-only change; no gameplay code touched).

## Also
`tools/done.sh` failed its web check on main's P139 vendored `bevy_transform` (the unused parallel walk warned as dead
code in the wasm build); one `#[allow(dead_code)]` on that module clears it.
