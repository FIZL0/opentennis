# B669 — doubles alt camera zooms out too far (concept, slug doubles-alt-camera)

Runner slot s1, 2026-10-25 22:00: not started, to avoid a collision.

- `concept/doubles-alt-camera` is checked out in another worktree, `../opentennis-doubles-alt-camera`, where a
  live Claude session (started from the main checkout) is working on this exact task: uncommitted framing
  options in `crates/hst-play/src/play/alt_camera.rs` (`HST_ALT_FRAME=base|pin|dolly|crane`), and it was running
  `tools/check.sh -p hst-play alt_camera` at the time.
- Git won't let this slot switch to a branch checked out somewhere else, and editing that worktree or building a
  second version of the branch would clash with that session's work. So nothing was changed here.

What unblocks it: let the other session finish and push the concept branch (or close it), then unblock B669 for
the next round (screenshots of each option in the serve-and-partner-runs-back test case).

## Not verified / not 1:1
Everything: no work done in this slot (see above).
