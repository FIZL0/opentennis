# M152c — the adhoc guest page

The page an adhoc host serves (M152a) now runs as that host's guest. Someone opens `http://<host ip>:<port>/` and
the page gets the disc and the mods from the host into memory, then joins the host's lobby by itself. The guest runs
the full game in its own browser and plays the match over rollback netplay (M152b's WebRTC); it is not just a
controller.

## What changed

- `web/index.html`: on a plain-http page (not the site) the page fetches `/adhoc/info.json`. If that says
  `game: opentennis`, it's a guest (`window.hstAdhoc`):
  - no service worker, and no OPFS: it uses B457's `memoryDir()`, so nothing is left on the device;
  - no site landing;
  - `hostDisc()` streams `/adhoc/disc` into `pack_begin`/`pack_part`, with "Getting the disc from the host… X of Y
    MB". On 503 (the host is still packing an ISO) it retries every 2 s;
  - `hostMods()` registers every served mod. Texture packs, `.json` files and previews are fetched up front; the rest
    go through `add_mod_lazy`, and `hstModLoad` fetches them from the host when the game asks;
  - `hostLobby()` waits for `info.json`'s `lobby` ("Waiting for the host to open a lobby (Online › Host)…"), then
    `paste`s `rtch<host:port>`, which joins as `?code=` does.
- `crates/hst-base/src/adhoc.rs`:
  - `/adhoc/disc` is always a pack. A pack is served as is. An ISO is packed once, on a thread, into the data folder's
    `cache/adhoc.pack`; a valid existing one is reused. Until the pack is there, the disc answers 503.
  - `info.json` gained `disc.ready` and `lobby`.
  - `POST /rtc` on the same port hands the offer to `net::rtc::signal`, so a guest needs one address only. The body
    is capped at 8 KiB.
- `net/rtc.rs` `lobby_open()`. `net.rs`: `HST_ADHOC=<disc>` turns the server on while a CLI lobby is up (dev/tests).
- Touch → controller: on an adhoc guest, B656's touch hold is off (`seats::hold_touch`, `web::adhoc_guest`), so the
  first controller button takes the touch pad's seat (B247's path). This is at the user's request (2026-10-27): "they
  should be able to takeover the touch controls with a controller easily".
- Firewall (user asked 2026-10-27): the Adhoc card now ends with "Host a lobby to play. Page won't load? Let
  OpenTennis through this device's firewall." Windows and macOS prompt on the first listen. Linux ufw/firewalld drop
  the traffic silently. The WebRTC UDP sockets use ephemeral ports, so a port rule can't cover them yet. Follow-up
  **M152c1**: one fixed UDP port, and show the ufw/firewalld command.
- `tools/web_guest.sh DISC OUT [secs]`: a release host with `HST_ADHOC`, and a headless Chromium guest with an empty
  profile that opens only the host's address. `web_adhoc.mjs` takes `GUEST_URL`.

## How to try

`cargo build -p hst --release && tools/web.sh` (or an existing `web/pkg`), then
`tools/web_guest.sh "Hot Shots Tennis (USA).iso" context/m152c/runN 120`.
In the game: Online → Adhoc → On, then Online → Host, and open the shown address on a phone.

## Verified

- **run5** (`context/m152c/run5`): a native host started from the raw ISO, and a guest with an empty profile and only
  the URL.
  - The guest loaded the pack from the host into memory ("no OPFS … keeping files in memory") and joined by itself.
  - The host started the match at 17.4 s.
  - Rollback hashes are identical on both ends at ticks 0/1800/3600/5400: `9a3f1525…`, `d3f9ab94…`, `81fc789d…`,
    `2292c7ec…`.
  - 15046/15044 datagrams in 120 s; the shot shows the match on the guest.
- **run2/run3**: with the host's 81 mod characters served, the guest read them lazily from the host; the models
  loaded.
- `tools/check.sh -p hst-base adhoc`: parse (POST, Content-Length), and a live serve that includes `POST /rtc` → 503
  with no lobby. `tools/check.sh -p hst online` passes.
- **run4**: Chromium fell back to SwiftShader and was still on "Now Loading" at 60 s. `web_guest.sh` now asks for
  ANGLE/Vulkan.

## Not verified / not 1:1

- **No real phones and no hotspot**: everything ran on one machine (headless Chromium → the host's LAN address).
- **Chrome/Safari/Firefox on phones not tried**. Only desktop Chromium was. iOS Safari's memory for the in-memory
  pack (≈170 MB plus the wasm) is unknown.
- **Controller takeover not exercised** with a real pad in a browser; the change is in the code only. Real pads over
  plain http are M152d's question.
- **A guest's mod pick stalls the CLI host**: with mods served, the guest's random pick was a mod. The countdown
  showed and the match never started (run2/run3). `web_guest.sh` serves no mods for that reason. Follow-up **B688**;
  the menu host is unchecked.
- The guest's settings still go to the host origin's localStorage, so they persist on the device. Only the disc and
  mods stay in memory.
- Android/iOS hosts weren't run (`adhoc` is native-only code, as in M152a).
- Not 1:1: a new feature with no counterpart in the original.
