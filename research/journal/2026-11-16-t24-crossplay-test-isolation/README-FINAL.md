# T24 — crossplay rejoin test vs other tests' end_crossplay

**Problem.** `net::tests::a_crossplay_guest_rejoins_on_the_hosts_data` points the global handshake disc at its own
symlink. Each net test that left a lobby through `join()` called `handshake::end_crossplay()` without condition, which
took the overlay off whatever disc the global named. That included this test's link, mid-test.

**Fix (lobby-keyed).** `join()` now owns the lobby's `Crossplay` (the new `join_in` takes it; `join_as` is a thin
wrapper that still makes its own) and calls `Crossplay::leave()` when it's done. That removes the overlay only if
*this* lobby laid the host's data over the disc (`done`). If the data lands after the lobby is over, it comes off
right away (`left` flag, SeqCst on both sides so at least one side sees the other). A lobby that never took data leaves
the disc alone. The test goes back to `assert!(crossplay::remove(&link))`, since nothing else can take its overlay off now.

Before, `end_crossplay` also ran for lobbies that took no data. It did nothing there in the real app, where only one
lobby runs at a time, so players see no difference.

Verified: `tools/check.sh -p hst-base net::tests` (689 passed, including the test with the Japanese disc there); the full
suite runs in done.sh.

## Not verified / not 1:1
none (test-only isolation fix; online lobby code, no gameplay port)
