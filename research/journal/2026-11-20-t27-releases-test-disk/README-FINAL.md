# T27: releases::tests::registers_the_discs_beside_and_their_voices failing

## Cause
No merge broke it, so there was nothing to bisect. The failure log said `voices: …: Disk quota exceeded (os error 122)`.
The test imports the Japanese and European discs' voices (packs of hundreds of MB) into a folder under
`std::env::temp_dir()`. `/tmp` here is a 16 GB tmpfs with a per-user quota, and agent scratchpads had filled it, so
the pack writes failed and the `.pack exists` assertion fired. The code is fine once it has disk room.

## Fix
The first attempt (wip/T27) moved the test's scratch folder to `target/`. It couldn't merge because of an unrelated
failing test (`net::rejoin::tests::a_kept_record_outlives_the_match_start`). Since then main has landed the same fix
(T29, 83ce2d18): `releases.rs`, `voices.rs` (`scratch_on_disk`) and `disc.rs` (`Tmp::on_disk`) write their big
imports under the build's `target/tmp/`. So this branch changes no code; wip/T27's version is dropped in favour of
main's, which keeps the merge clean.

## Verified
On this branch (main as of 7caad9b4): `tools/check.sh -p hst-base releases::` (2 passed), `voices::` (5 passed),
`disc::` (3 passed). /tmp currently has room (3% used), but the tests no longer depend on it.

## Not verified / not 1:1
none (test-only issue, nothing gameplay-related). Other tests that still write to `temp_dir()` (e.g. modstore) could hit
a full /tmp again. They write small files, so I left them alone.
