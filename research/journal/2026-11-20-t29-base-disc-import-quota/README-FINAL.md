# T29: base_disc_imports_an_image fails on the shared branch

## Cause
Not a merge. The test imports the 1.3 GB Japanese disc image into a pack in `std::env::temp_dir()`. On this
machine `/tmp` is a tmpfs mounted with `usrquota` (16 GB, ~13 GB in use by other runs' scratch), so the import
died at ~20% with `Os { code: 122, QuotaExceeded }`. `crates/hst-base/src/disc.rs` is unchanged since M180, so a
bisect would only find the gate run where `/tmp` filled up, not a commit. I skipped the bisect for that reason.

## Fix
The test-only `Tmp` gets `Tmp::on_disk`: the scratch dir goes under the build's `target/tmp/` (found from the
test binary's path), and falls back to the system temp dir if there's no `target` ancestor. Only the full-size
import test uses it. The other disc tests write a few bytes and stay in `/tmp`. The full suite hit the same quota error in
`voices::tests::{other_releases_dub_the_us_banks, add_voices_reads_a_disc_with_progress}` and
`releases::tests::registers_the_discs_beside_and_their_voices` (voice-pack import fails silently, so the pack is
missing). Those now write under `target/tmp/` too. `tools/check.sh -p hst-base`: 282 passed.

What fills `/tmp`: about 12 GB of other projects' agent scratchpads (`/tmp/claude-1000/...`). They aren't ours, so I
left them alone.

## Not verified / not 1:1
none (test infrastructure only, no gameplay)
