# M152: Adhoc mode (umbrella)

All the subtasks have landed: a (host server), b (native WebRTC host), c (guest page), c1 (one fixed port for the
firewall), d (gamepads over http), e (firewall hint), f (https opt-in), g (native guest with no disc). Each one has its
own journal (`2026-10-27-m152*`, `2026-10-28-m152f-*`). This entry is the final end-to-end check of all of them together on
current main, over **plain http** as the task line asks, **with mods served**. M152c had to serve none because of
B688, which is now fixed.

## Run (`tools/web_guest.sh`, `SCHEME=http`, host's `mods` → the user's mods folder)

Release host (`hst net host --play`, `HST_ADHOC`) and a headless Chromium guest with an empty profile that opens only
`http://<lan ip>:8080/`. Native and web were both built from this branch.

- **run1** (90 s): the guest said "no OPFS (… plain http), keeping files in memory". It drew a mod character at random
  (read from the host), the match started, and the host's and guest's hashes agree at ticks 0/1800/3600.
- **Found**: the guest's Chromium **disk cache held every mod file it fetched** (models, motions, voices: ~300 MB of
  the 455 MB cache). The host sent `/adhoc/` files with `Cache-Control: no-cache`, which still lets the browser store
  them, so "nothing left on the device" didn't hold for mods. The disc was not in the cache: the page fetches it with
  `cache: 'no-store'`, but the wasm's mod fetches don't use that.
- **Fix** (`hst-base/src/adhoc.rs` `file`): everything under `/adhoc/` (disc and mods) is sent with `no-store`. The page
  and the wasm keep `no-cache`, so a returning guest still revalidates them instead of downloading 155 MB again. Over https
  (M152f) the disc's copy is the OPFS one, so nothing there relied on the HTTP cache. The `serves_over_http` test checks
  both headers.
- **run2** (70 s, rebuilt host, fresh profile): the guest drew a mod again and read it from the host. Hashes agree at
  0/1800/3600. The cache now holds only `/`, `pkg/`, `touch.js` and `site.css` (149 MB, the wasm), with no `/adhoc/` entry.

## Not verified / not 1:1

- Not 1:1: adhoc play is our own feature, with no counterpart in the original.
- **No real phones, no second device, no hotspot**: the guest is headless desktop Chromium on the same machine
  (traffic to its own LAN IP). Phone checks over a hotspot (Safari/Firefox/Chrome on iOS/Android) are in
  `plan/HUMAN.md` (M152f item). Each subtask's journal lists what it couldn't run (Android/iOS hosts, firewalld, a
  real gamepad in a guest match).
- Only Chromium's disk cache was checked for leftovers. Safari's and Firefox's caches weren't, but they follow
  `no-store` too.
- The guest's settings still go to the host origin's localStorage (by design, as in M152c: they aren't game data).
- Touch controls on the guest were exercised in M152c/M152d's runs, not again here.
