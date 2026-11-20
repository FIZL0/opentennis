# T28 — voices::tests::other_releases_dub_the_us_banks failing on main

## Cause
Not a code regression, so no bisect: the gate's failure (merge/context/notes/check.log) and a fresh run in this
slot both panic at the first `import(...)` with `Os { code: 122, QuotaExceeded }`. The test writes the European and
Japanese voice packs (≈ 200 MB) under `std::env::temp_dir()`, i.e. `/tmp`, a quota-limited tmpfs; the session
scratchpads under `/tmp/claude-1000` had grown to ~12 GB, so the write ran out of quota before the test's logic
ran. The full suite showed two more victims of the same error: `releases::tests::registers_the_discs_beside_and_their_voices`
and `modstore::tests::package_then_install`.

## Fix
`tools/check.sh` (and so `done.sh` and the gate) exports `TMPDIR=<checkout>/target/test-tmp` before `cargo test`,
so every test's `temp_dir()` scratch lands on the main disk, in the slot's own target/ (covered by the target/ size
cap). One central change instead of editing each test. `cargo test -p hst-base` via check.sh: 647 passed, with the
discs present (HST_REQUIRE_DISC=1, nothing skipped).

Seen on the way: the `hst-rules-<pid>-*` dirs a test leaves behind (they used to pile up in /tmp; now in target/).

## Not verified / not 1:1
none (test tooling only; no gameplay touched).
