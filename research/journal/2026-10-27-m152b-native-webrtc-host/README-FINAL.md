# M152b — native WebRTC host for browser guests (FINAL)

A native build (desktop, phone) now hosts browser guests over WebRTC, on the same two channels as the browser's own
LAN (M2k): channel 0 reliable/ordered JSON lobby messages, channel 1 unordered with no retransmits for rollback's
datagrams, both negotiated up front. Once connected it's the same lobby protocol and `guest()` path as iroh guests.

## How it works

- **Stack**: `str0m` 0.24 (sans-IO WebRTC) with its pure-Rust crypto (`rust-crypto`: no OpenSSL/aws-lc, so it
  cross-builds for Android/iOS/Windows like the rest). It's driven on tokio in `net/rtc/native.rs`: one UDP socket and
  one host candidate per LAN IPv4 address (`netdev`), a reader task per socket, and a driver task that owns the `Rtc`
  (packets in, app sends out, timers). Reliable messages the SCTP buffer can't take yet wait in a backlog.
- **Signalling**: the guest offers and the host answers in the reply, so the host keeps no state between requests.
  The guest POSTs its offer code (`rtco…`, the M2k code format) to `http://<host>/rtc`, and the reply body is the
  answer code (`rtca…`, with `to` = the offer's ufrag). `native::signal` does that. `native::listen(port)` is a tiny
  HTTP/1.1 listener (POST /rtc, CORS preflight with `Access-Control-Allow-Private-Network`). M152a's page server can
  route `/rtc` to `signal` instead. When no lobby is hosting, the reply is 503 ("No lobby is open on this machine.").
- **Lobby hand-off**: `rtc::hosting()` gives `host()` a receiver for connected guests (a static sender, since the
  listener runs on tokio's threads). `host()` spawns `guest()` for each, like an iroh guest. The guest's peer name is
  its offer's ufrag.
- **Join code**: `rtch<host:port>` (`rtc::host_addr`). Join by Code / `?code=` take it, and `connect()` dials it. In
  the browser that's `web::dial` (fetch POST through reqwest). Natively it's `native::dial`, which is only used by the
  test so far (native↔native LAN stays iroh's).
- **For now the host listens when `HST_RTC_SIGNAL=<port>` is set** (dev builds only: a `ship` build drops `HST_*`
  variables). The menu toggle and showing the address belong to M152a.

### Quirks found on the way
- **DTLS role**: a code has no `a=setup`, and `Sig::sdp(false)` says `active` for the answerer. str0m answers an
  `actpass` offer as passive, which would leave both ends passive. So the host feeds str0m the guest's offer with
  `setup:passive`, and str0m then takes active, which is what the browser reads from the answer code. Natively
  dialling, the answer is rebuilt `active`, so the dialler is passive.
- **mid**: `Sig::sdp` always writes mid `0` (what Chrome uses). A str0m offerer picks its own, so `native::dial`
  swaps it into the rebuilt answer.
- **mDNS candidates**: the browser's candidates are `<uuid>.local` names that str0m can't resolve. They're dropped
  from the SDP str0m reads. The browser's checks reach our host candidates, and str0m learns its address as
  peer-reflexive. This worked first time in Chromium.
- **Closing**: a bare `rtc.disconnect()` is only noticed at the other end's ICE timeout (~20 s). We reset the
  message channel's stream first, so the other end sees `ChannelClose` at once.
- **The site page** (M96) now stands in front of the game on a fresh visit unless `hst-autoplay` is set in
  localStorage. The driver sets it. `tools/web_lan.mjs` probably needs the same now (not touched). In the lobby,
  synthetic Enter/J key events did nothing in headless Chromium, so the driver clicks Ready instead.

## Verified

- `net::rtc::tests::a_native_host_takes_a_guest_by_signalling` (loopback, both ends str0m, through the real `/rtc`
  listener): refused with no lobby; then connects, the lobby gets the guest under the dialler's name, a message
  each way (one is 100 kB, so SCTP fragmentation and the backlog are exercised), datagrams each way, and a close is
  seen at once. Runs in about 0.05 s. `host_codes_name_a_signalling_address` covers code parsing.
- **Native host + browser guest match** (`tools/web_adhoc.sh`, `context/m152b/run9`): release `hst net host --relay
  off --games 1 --play` with `HST_RTC_SIGNAL=8791 HST_AUTOPLAY=1` (window `HST_BG`). The guest was headless Chromium
  on Vulkan, with page `?code=rtch192.168.0.118:8791&webgl2&autoplay`.
  - The page opened straight into the host's lobby (5 ms ping) and clicked Ready. The host CLI started the match.
  - Rollback hashes were identical on both ends at ticks 0, 1800, 3600, 5400 and 7200: `62af84a5…`, `7792c48d…`,
    `d60d31fd…`, `f651af61…`, `6175f5ec…`.
  - The data channels carried about 130 datagrams/s each way (19027/19025 after 150 s), with no desync and no
    disconnect.
  - The screenshot shows the match on the guest.
- The `wasm32` build checks clean, and so does `cargo check -p hst-base` natively.
- README (Online play → *Adhoc play*) and the site's Online panel mention adhoc play, at the user's request
  (2026-10-27). They're worded as on its way, since the host toggle (M152a) and the served guest page (M152c) aren't
  in yet.

## Not verified / not 1:1

- **One machine only**: the guest's packets went to the host's LAN address on the same box. Two devices on Wi-Fi or a
  phone hotspot weren't available. AP/client isolation would block it, as with M2k.
- **Chromium only**: Firefox's and Safari's WebRTC weren't tried against str0m (M152c covers browsers on phones).
- **Mixed content**: a guest page served over https (opentennis.dev) can't fetch `http://<lan-ip>/rtc`. This is meant
  for the page served by the host over http (M152a/c). From `http://localhost` it works (the check).
- **Only the native host was played in a match**: native dialling (`native::dial`) was only exercised in the loopback
  test, not in a match.
- **IPv4 host candidates only**: no IPv6 and no loopback candidates outside tests.
- **No player-facing switch yet**: `HST_RTC_SIGNAL` is dev-only, and M152a adds the menu toggle (it should call
  `rtc::listen` or route `/rtc` to `rtc::signal`). That's why there's no release-notes line.
- **Dependencies**: str0m with rust-crypto adds about 60 crates to Cargo.lock (dimpl, RustCrypto ciphers, x509).
  Android/iOS/Windows builds of them weren't run here.
- Not 1:1: online play is our addition, so there's nothing in the original to compare with.
