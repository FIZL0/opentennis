# B733 — Direct / Relayed beside each player's ping

## What changed

- `net.rs`: `PathKind { Direct, Relayed }`. A QUIC connection's path is the one iroh has selected
  (`Connection::paths()`, `is_selected` → `is_relay`); the browser's WebRTC LAN link is always Direct. The host reads
  each guest's path in its 1 s ping tick and keeps it in `Room::path` (peer → kind, beside `Room::ping`); a change
  (a relayed link turning direct once hole punching gets through, or back) resends the room like a 10 ms ping move.
  Serde-default and skipped when empty, so older builds' rooms still read (they show "Online" as before).
- Lobby cards (`online.rs` `ping_line`, used by `roster.rs`'s card band): "Direct - 35 ms" / "Relayed - 165 ms"
  instead of "Online - 35 ms"; "Online" while the path isn't known yet. Same length as before, so the band lays out
  as it did; checked at 1280×720 and 720×900 (`HST_MENU=lobby` shots in `context/b733/`).
- No in-match network notice shows a ping, so nothing changed there.

## Test

`path_kind_reported` (hst-base): a local iroh relay (`iroh::test_utils::run_relay_server`, iroh's `test-utils`
feature as a dev-dependency only), a host on it; a LAN guest dialling its IP addresses reports Direct, a guest
with no IP transports (`clear_ip_transports`, what `HST_RELAY_ONLY` sets) dialling only the relay URL reports
Relayed; the room round-trips `path` through JSON and omits it when empty. `ping_line_shows_the_path` (hst) for the
card text.

## Not verified / not 1:1

- Not against the original: online play is ours (the original has none), nothing to match.
- Not tested over the real internet with a real failed hole punch: the test forces relay-only locally; iroh's
  selected-path flag is what both cases read.
