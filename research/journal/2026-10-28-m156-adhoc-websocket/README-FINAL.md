# M156 — adhoc guests without WebRTC, over a WebSocket

B689 found an iPhone (most likely in Lockdown Mode) saying `Can't find variable: RTCPeerConnection`. WebKitGTK has no
WebRTC either. Such a browser now joins the native host over a WebSocket on the host's adhoc http server, and plays.

## How it works

- `crates/hst-base/src/net/ws.rs` (new):
  - **Host end** (native): `upgrade(head)` picks `GET /adhoc/ws?name=<peer>` with `Upgrade: websocket` out of a
    request head. `accept` refuses with 400 for a bad name (1–32 of `[A-Za-z0-9_-]`) or 503 with no lobby open.
    Otherwise it answers 101 (`Sec-WebSocket-Accept`: a small SHA-1 here rather than a new crate, tested against
    RFC 6455's example and known digests), moves the stream onto the game's tokio runtime (nodelay) and hands it to
    the hosting lobby through the same inbox as M152b's signalled WebRTC guests (`rtc::arrived`). From there it's the
    lobby's ordinary `guest()` path.
  - **Framing**: text frames carry the lobby's JSON messages, binary frames rollback's datagrams. Fragments, ping/pong,
    close and 16/64-bit lengths are handled. Server frames go out unmasked, a client's masked. 64 MB per message at
    most. Over TCP a datagram is never lost, it just comes late on a bad link.
  - **Guest end** (browser): `web_sys::WebSocket` to `ws://<host:port>/adhoc/ws?name=ws<12 hex>`. The guest makes up
    its own peer name and sends it, so it knows the name the host knows it by. 10 s to open.
  - **Which transport** (`ws::join`, called by `connect` for a host code `rtch<host:port>`): in a browser without
    `RTCPeerConnection`, the WebSocket at once. With WebRTC, WebRTC first, and if that fails (e.g. a host firewall
    that lets the port in for TCP but not UDP, M152c1) the WebSocket. If that fails too, WebRTC's error is shown,
    since it says more. Natively it's still `rtc::dial`.
- `net.rs`: `Conn::Ws` beside `Conn::Rtc`, and `connect` dials through `ws::join`.
- `adhoc.rs`: `serve` hands an upgrade head to `ws::accept` before its own parsing. The module doc lists the route.
- `rtc.rs`: `arrived` and the tests' `SIGNALLING` lock are visible to `net::ws` (the WebSocket test shares the lobby).
- `web/index.html`: B689's stop for a browser with no WebRTC is gone. The page loads on and logs
  `hst: no RTCPeerConnection: joining the host over a WebSocket`, which reaches the host's log through B689's guest log.
- `tools/web_adhoc.mjs`: `NORTC=1` deletes `RTCPeerConnection` before the page's scripts run (`tools/web_guest.sh`
  passes the environment through). `tools/web_webkit.mjs`: the `hstNoRtcCheck`/`NORTC=stop` switch is gone with
  the stop.
- `Cargo.toml`: web-sys features `WebSocket`, `BinaryType`. No new crates.

## Verified

- `tools/check.sh -p hst-base ws::` (`net::ws::tests`):
  - `handshake_parts`: the accept key, SHA-1, and parsing the upgrade head.
  - `frames_carry_any_length`: 0/125/126/65535/65536-byte frames, masked and not.
  - `a_guest_joins_over_a_websocket`: refused with no lobby (503) and for a bad name (400). Then joined: the lobby gets
    the guest under its name, a message each way (one is 100 kB), datagrams each way, and a close is seen at both ends.
  - `rtc::` and `adhoc` tests still pass.
- `cargo check -p hst-base --target wasm32-unknown-unknown`: clean.
- **A full match, real host and browser guest without WebRTC** (`context/m156/run1`):
  `NORTC=1 tools/web_guest.sh "Hot Shots Tennis (USA).iso" context/m156/run1 90`. The host was a release `hst net host
  --relay off --games 1 --play` with the adhoc server. The guest was headless Chromium (Vulkan) with
  `RTCPeerConnection` deleted, opening only `http://<host ip>:8080/` and freshly built `web/pkg`.
  - The guest logged `this browser has no WebRTC: joining 192.168.0.118:8080 over a WebSocket` and sat in the lobby
    as `wsa841ee433126`.
  - The match started, and the rollback hashes were identical on both ends at ticks 0, 1800 and 3600 (`608a3e37…`,
    `902c72e6…`, `749e0ad7…`).
  - The guest drew the match through 90 s with no desync or disconnect (screenshots in `context/m156/run1`).

## Not verified / not 1:1

- **No real iPhone or Lockdown Mode**: none here. Chromium with `RTCPeerConnection` deleted stood in for it. Safari's
  WebSocket is standard, but Lockdown Mode wasn't tried, so that it allows plain `ws://` to a LAN address is
  assumed. The user's iPhone run is B689a.
- **WebKitGTK against a real host**: not run. `tools/web_webkit.mjs` plays the host itself and has no WebSocket, and
  the real host + MiniBrowser pairing has no driver.
- **The WebRTC → WebSocket fallback after a failed WebRTC connection**: not exercised in a browser (it would need UDP
  blocked while TCP is let in). It's ~30 s of WebRTC timeout before the WebSocket is tried.
- **Same machine only**: the guest's connection went to the host's LAN address on the same box. A slow or lossy Wi-Fi
  link wasn't tried. Over TCP, a lost packet holds up the datagrams behind it, so rollback sees more late inputs than
  over WebRTC. That's fine on a LAN, but it wasn't measured.
- **The host's join log line**: `online: guest … joined over a WebSocket` doesn't show in the `hst net host` CLI's
  output, which has no log subscriber until the match window opens. The guest's own line reaches the host log.
- Not 1:1: online play is our addition, so there's nothing in the original to compare with. No gameplay was touched.
