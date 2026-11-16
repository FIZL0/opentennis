# M178 — crossplay rejoin

A guest on another release's disc that dropped out of a crossplay match used to be refused when it came back: the
host sends a rejoiner its `Start` at once, and the guest would have started the match on its own disc's data before
the host's data arrived.

## What changed (`crates/hst-base/src/net.rs`)

- **Host (match relay):** the refusal is gone. A rejoiner whose disc plays differently gets the host's data
  (`crossplay_send`, the same patch the lobby sends) and then its `Start`, as any rejoiner does.
- **Guest (`join_as`):** a `Start` whose room plays another digest, received before the host's data has been laid
  over the disc, is **held** (`Crossplay::holds`). The status line says "Getting the host's game data to rejoin...".
  While it's held, messages from the host (the rejoin state that follows the Start, pings, chat) are queued and
  datagrams wait in their channel. When `take_patch` succeeds (`Crossplay::tried`, a `Notify`), the guest releases the
  Start, then the queued messages in order. If the patch is refused, the guest says why and closes, as in the lobby.
  If the host goes away, or no data arrives within 60 s (`CROSSPLAY_WAIT`), the guest gives up with a message.
- **M171 bug found along the way:** `off_thread` ran `crossplay_send`'s work on a bare `std::thread`, and
  `send_pack` then called `n0_future::task::spawn` with no tokio runtime. It panicked ("no reactor running"), so
  **the host's data was never sent at all**, in the lobby either. M171's tests built the patch by hand and never
  went over the network, so they missed it. `off_thread` now enters the caller's runtime (native only; the browser
  runs the work inline).

## Proof

`net::tests::a_crossplay_guest_rejoins_on_the_hosts_data` (hst-base, ~3 s; needs the US ISO and
`fixtures/discs/SCPS-15113.iso`, and skips without them): over real loopback iroh connections, a US host and a guest
with the Japanese disc's handshake (digest and manifest surveyed from the JP ISO):

1. the guest joins and receives the host's data in the lobby; the match starts;
2. the host marks the guest away, and the overlay is removed (a fresh process plays its own disc again);
3. a new guest connection with the JP handshake rejoins. The host reports `Rejoined` and sends a `State` straight
   after. The test checks the rejoiner's events: the "Getting..." hold, then "game data is here", then `Start` (its
   seat, with `rejoin` set), with the overlay installed by the time `Start` arrives, and the `State` after the `Start`.

Before the fix the rejoiner got `Refused`, and the test fails on that `Failed` event. `tools/check.sh -p hst-base`:
264 passed.

## Not verified / not 1:1

- The brief asked for the proof in `recover.rs`'s tests. The change is entirely in the network layer, so the test is
  in `net.rs`, where a real host and guest exchange messages. `recover.rs`/`netplay.rs`'s match-level rejoin test
  (`a_silent_peer_rejoins_and_a_silent_host_ends_it`) gives the rejoiner its room directly, with no network, so it
  can't exercise the hold. Their crossplay match tests already cover a JP disc with the host's data over it staying
  in sync.
- In the test, host and guest share one process, so the guest's overlay goes over the host's own disc (a temp
  symlink). The patch is still the one built for the JP manifest, but the check passes trivially. A real US↔JP
  overlay verify is covered by M171's netplay tests.
- No live two-machine run. This is online-only, and not a gameplay port, so there is nothing to check against the
  original game.
