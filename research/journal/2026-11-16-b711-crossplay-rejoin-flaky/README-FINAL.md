# B711 — flaky `net::tests::a_crossplay_guest_rejoins_on_the_hosts_data`

## Cause
The overlay of the host's data sits on the disc path in a process-wide table (`hst_data::crossplay`), and
`handshake::disc()` is process-wide too (this test sets it to its own symlink). Every `net::join` ended with an
unconditional `handshake::end_crossplay()`, so when any other net test's guest left its lobby (tests run in
parallel threads of one process), it took the host's data off this test's disc: either before the old
`assert!(crossplay::remove(&link))` (that assert had already been dropped by M174, T24 comment) or, still possible
after that, between the rejoiner laying the data and `assert!(crossplay::installed(&link))`.

## Fix
`join` now keeps the session's crossplay state (`join_with`, `join_as` is a test-only wrapper) and ends crossplay
only where that lobby waited for or laid the host's data (`Crossplay::involved`). That is also the right behaviour in
the game: leaving a lobby that never laid anything has nothing to take off. Other tests' guests (same disc as their
host) never wait for data, so they no longer touch the overlay.

## Checked
- `cargo test -p hst-base --lib net::tests` 3/3, the crossplay test alone with `--nocapture` (ran, not skipped:
  both discs present), full `hst-base` lib suite 5/5 with `HST_REQUIRE_DISC=1`.

## Not verified / not 1:1
- The original failure never reproduced here (it was rare), so the fix is proven by reasoning about the race plus
  the repeated green runs, not by a before/after reproduction.
- Not gameplay: nothing to compare with the original game.

## Also
`tools/webcheck.sh` failed every done.sh on a deprecation warning in vendored `third_party/bevy_ecs`
(`world/identifier.rs`: `fetch_update` renamed `try_update`; it shows since bevy_ecs is a path dependency). Renamed
the one call (same signature on the pinned 1.99 toolchain) so the web check passes again.
