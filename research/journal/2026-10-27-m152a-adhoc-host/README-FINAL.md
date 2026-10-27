# M152a — Adhoc host server

Native builds can now serve the game to browsers on the LAN. Online screen → **Adhoc** row (under Join by Code, in the
big-pill column; native only, hidden on wasm). ✕ toggles it; the card shows `http://<ip>:<port>` and a QR code of it.
Leaving Online (Back) stops the server, since it hands out the disc and mods. User asked (mid-task) for the toggle to
live in Online as the Adhoc mode rather than Settings: moved there.

## What's on the branch
- `crates/hst-base/src/adhoc.rs`: std-thread HTTP/1.1 server (keep-alive, GET/HEAD, Range → 206/416, MIME incl.
  `application/wasm`), binds `0.0.0.0` on 80, 8080, 8000, 8888, else any free port. LAN IPv4s via `netdev`
  (private/link-local, virtual adapters skipped, Wi-Fi first). QR via `qrcode`.
  Endpoints (for M152c's guest page):
  - `/` and below: the web build (`index.html`, `pkg/…`). Found at `HST_WEB`, `<exe dir>/web`, `<data dir>/web`
    (stamped `.version`), or the cargo checkout's `web/`; release builds without one download the release's
    `web.tar.gz` into the data dir (503 auto-refresh page meanwhile).
  - `/adhoc/info.json`: `{game, version, web: ready|missing|fetching, disc {url, name, size}, mods [{id, name, kind,
    folder, url, files [{path, size}]}]}`; enabled mods only (texture packs that are on, characters not dropped/retired).
  - `/adhoc/disc`: the user's disc image (Range-capable). `/adhoc/mods/<folder>/<file>`: a mod's files.
  - Paths are confined to their roots (only normal, non-dot components).
- `crates/hst/src/play/main_menu/online.rs` + `online/adhoc.rs`: the row, card words, QR picture, `Fx::Adhoc`.
- Tests: `tools/check.sh -p hst-base adhoc` (parse, ranges, path escape, untar, live loopback serve, QR) and the
  Online cursor walk (Join by Code → Adhoc → Name and back).

## How to try
Run the game, Online → Adhoc → ✕. Open the shown address on a phone on the same Wi-Fi (or scan the QR). Until M152c
lands, the page served is the plain web build (it asks for its own disc import); `/adhoc/info.json` lists what's shared.
Screenshot: `context/m152a/online-on.png` (QR decodes to the address shown).

## Verified
Desktop (Linux): server on 8080, `index.html` 200 text/html, `pkg/hst_bg.wasm` application/wasm (full size),
`info.json` lists 105 mods + disc, Range on `/adhoc/disc` 206, mod file fetch 200 at the right size,
`/pkg/../../Cargo.toml` → 404. QR decoded with zbarimg.

## Not verified / not 1:1
- Not 1:1: new feature with no counterpart in the original (front-end, ours to design).
- Android and iOS: compiled in, not run on a device (port 80 needs privileges there → falls back to 8080 etc.; iOS may
  show its local-network permission prompt). No device was available to an unattended run.
- Release path that downloads `web.tar.gz`: only the dev checkout's `web/` was exercised; the untar is unit-tested.
- IPv6 addresses are not offered (IPv4 LAN only).
- No real guest browser load of the game yet: loading the pack and mods in the page is M152c; joining the host's
  lobby (WebRTC + signalling) is M152b; HTTPS (for browser features needing a secure context) is M152d.
