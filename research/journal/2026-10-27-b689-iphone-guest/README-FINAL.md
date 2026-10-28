# B689 — iPhone Safari adhoc guest crashes and asks for an ISO

## What it was

Reproduced in WebKit (`tools/web_webkit.mjs`, new `GUEST=1` mode: the tool plays the adhoc host itself — info.json,
the disc with byte ranges, a mods folder — and samples the browser processes' RSS into `OUT/memory.txt`).

With the user's host mods (the `hst_hd` texture pack on, 1559 MB, plus a character), the guest page took the texture
pack in whole, like an OPFS install does:

| run | wasm heap | WebKit page process | all browser processes |
|---|---|---|---|
| before (`context/b689/run1`) | 2083 MB | 2995 MB | 3235 MB |
| after (`run2`) | 283 MB | 1190 MB | 1437 MB |

iOS Safari kills a tab well before 3 GB, even on an 8 GB iPhone 15 Pro. Then it reloads the tab and kills it again
("A problem repeatedly occurred"). The pack itself was already streamed straight into wasm memory (M152c/B468), with
no second full copy.

The user's screenshot mid-task showed the game's bar saying `Can't find variable: RTCPeerConnection` on the iPhone.
WebKitGTK says the same. That Safari has no WebRTC, so the page loaded and joining the host failed. On an iPhone this
is most likely Lockdown Mode, which also turns off JIT, so the 75 MB wasm runs on the interpreter: slow and heavier.
Not confirmed: needs the user's phone (B689a).

## What changed

- `web/index.html`
  - Texture packs on a guest: taken whole only while they come to ≤ 64 MB (smallest first). Bigger ones are left out,
    said on screen and in the console. After a load that never reached the first frame (sessionStorage
    `hst-guest-loading`, which survives WebKit's crash-reload), all of them are left out and the page says the last
    try was cut off. Mods' `textures/upscaled/` files are never fetched (the browser never reads them anyway).
  - No ISO picker for a guest: info.json is asked with an 8 s timeout. A tab that was a guest (`hst-guest` =
    host:port) keeps asking, with "Can't reach the host at …" on screen. A first visit asks three times; a 404 (a plain
    http server, tools/web.sh) still means no host. The error screen's "Settings & files" and "Pick the ISO again"
    links are hidden for a guest.
  - Disc download: a cut-off or failed read goes on from the byte it stopped at (Range, 206), says why, retries every
    2 s. A refusal from the game itself (string thrown by `pack_part`) still fails.
  - No `RTCPeerConnection` on a guest: it stops before downloading anything, saying WebRTC is off and how to turn
    Lockdown Mode off for the page. `hstNoRtcCheck` (set by the test tool) loads on anyway.
  - The guest's log to the host: load steps (a MutationObserver on the boot text), console info/warn/error,
    exceptions, rejections, the game's WARN/ERROR/`hst:`/`online:` lines, and a first line with the UA, screen,
    `WebRTC:` and `wasm:`. They go to `POST /adhoc/log` once a second (6 KB a post, 3000 lines a load), the rest by
    sendBeacon on pagehide.
  - "mods read from the host (0.0 MB)": the count was never added for the host's mods. It now is, and a line says
    what's read now and what's left for later: `1 mods from the host: 0.0 MB now, 58 files (79.6 MB) when the game
    wants them`. 0.0 MB with mods listed is right: a character's or court's mod reads only its manifest and pictures
    until it's picked. `hstModLoad` logs each mod read from the host.
- `crates/hst-base/src/adhoc.rs`: `POST /adhoc/log` prints each line as `adhoc guest <ip>: …` (control characters
  out, 400 characters at most, test `guest_log_lines`).
- `crates/hst-base/src/web.rs`: `pack_begin` lets go of an earlier pack before allocating, never two at once.
  Unreached now that the page resumes instead of starting over, so it's a guard only.
- `tools/web_webkit.mjs`: `GUEST=1 MODS=dir`, `FLAKY=N` (N info.json 503s plus a disc cut at 20 MB),
  `OUT/memory.txt`, `OUT/guest-log.txt`, `NORTC=stop`.

## Verified (WebKitGTK MiniBrowser, `context/b689/`)

- run2/run8: a guest load with the HD pack on gets to the menu at a 283 MB wasm heap, with the pack left out and said.
- run8: two 503s, then a disc cut off before any bytes arrived: retried and played. run9: cut at 8.5 MB, resumed from
  there with a Range request, and the game started and drew its menu from the resumed pack.
- run5: `NORTC=stop`: the WebRTC message shows. The host log got the UA line and the steps.
- `tools/check.sh -p hst-base adhoc`: 12 passed. `cargo check -p hst-base --target wasm32-unknown-unknown`: clean.

## Not verified / not 1:1

- Not run on a real iPhone (none here). WebKitGTK stands in for Safari's engine, but iOS's tab memory limit and
  Lockdown Mode can't be reproduced on Linux. That the iPhone's crash was the texture pack, and that it has Lockdown
  Mode on, are inferences. B689a: the user's iPhone run, read through the new host log.
- The 64 MB texture budget is a judgement call, not measured on a phone.
- A guest with WebRTC off still can't play: it's told why and how to fix it. M155 would carry the match over a
  WebSocket through the host for those browsers.
- The wasm in `web/pkg` wasn't rebuilt, so the `pack_begin` guard ran only through `cargo check`.
- Menus and front-end: no 1:1 requirement (PLAN.md), and no gameplay was touched.
