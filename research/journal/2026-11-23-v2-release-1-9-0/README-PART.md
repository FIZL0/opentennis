# V2: release 1.9.0

User 2026-11-22: this release is 1.9.0 and must include B764 and F2 (both on main as of e7941be2).
User 2026-11-23: also wait for B654 (net hides in close cut-aways + shortened RELEASE_NOTES.md) to be on main, and use
RELEASE_NOTES.md as it is there.

- Started `tools/release.sh 1.9.0 publish`; the B654 note arrived during the native builds, before anything was pushed
  or released (no v1.9.0 tag on origin, no GitHub release). Stopped it; its two local commits (version bump, dev stats)
  are reverted on this branch so main doesn't claim 1.9.0 before the release exists.
- B654 is finished (task/B654 3eac5638) but not merged into main yet, so V2 is blocked on it.

Next: once task/B654 is an ancestor of main, from an up-to-date V2 branch run `tools/release.sh 1.9.0 publish`
(it bumps the version, builds, tags, publishes, and empties RELEASE_NOTES.md), then check `gh run list` and
`gh release view v1.9.0`.

## Not verified / not 1:1

none (release only; no gameplay change). Release not made yet.
