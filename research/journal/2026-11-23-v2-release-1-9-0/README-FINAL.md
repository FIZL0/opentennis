# V2: release 1.9.0

User 2026-11-22: this release is 1.9.0 and must include B764 and F2. User 2026-11-23: also wait for B654 (net hides
in close cut-aways + shortened RELEASE_NOTES.md) and use RELEASE_NOTES.md as it is on main.

- A first `tools/release.sh 1.9.0 publish` was stopped mid-build when the B654 note arrived (nothing pushed or
  released); its two local commits were reverted, and V2 was blocked until B654 landed.
- B654 reached main (3eac5638) at 00:23; main merged into task/V2 so the release commit contains B764, F2 and B654.
- `tools/release.sh 1.9.0 publish` from this worktree (as V1 did for 1.8.7): Cargo version 1.8.7 → 1.9.0
  ("Release v1.9.0"), dev stats, all builds, signed SHA256SUMS, GitHub release v1.9.0 published as Latest with
  16 assets, RELEASE_NOTES.md emptied after. Release notes = main's five shortened lines.
- Check: `gh release view v1.9.0` shows it published (not draft); the release-triggered pages run (site deploy)
  passed. This project has no CHANGELOG.md or separate release workflow: RELEASE_NOTES.md and tools/release.sh
  take those roles.

## Not verified / not 1:1

none (release only; no gameplay change)
