# M152d — Gamepads for adhoc guests: plain http is enough

**Decision (user 2026-10-27): stay on plain http, no HTTPS.** The Gamepad API is available to a plain-http LAN origin in
every browser tried, so M152c builds on http. HTTPS would only add a secure context (WebGPU, OPFS), and M152c loads the
disc into memory anyway, so that doesn't pay for a warning page every guest has to tap through. An HTTPS side (rustls +
rcgen self-signed cert, http page moving to it where `getGamepads` is missing) was built and tested, then dropped.

## Results (probe page polling `navigator.getGamepads()`, results sent back to the server as image requests)
| Browser | plain http (LAN IP) | https, self-signed, after the warning tap |
|---|---|---|
| iOS 18.7 Safari 27, iPhone (user), real Xbox Wireless Controller | pad seen, buttons 0–2 read | secure, WebGPU + OPFS, pad read; "visit this website" still needed every time a new cert came up |
| Chromium 154, Linux (headless, uinput virtual pad) | pad seen, buttons read; not secure | secure; pad, WebGPU, OPFS (tapped through the interstitial's Proceed via CDP) |
| Firefox 159, Linux (headless, uinput virtual pad) | pad seen, buttons read | secure; pad, OPFS (exception written to cert_override.txt) |
| WebKitGTK 2.54 MiniBrowser | API present, no pad seen over http or https: inconclusive (the window couldn't take focus without taking over the desktop) | – |

There's no way to skip the tap with a self-signed cert: if HTTPS is ever needed, it takes a CA-trusted cert for a
name that resolves to the LAN IP (Plex-style). iOS sent `certificate unknown` on every handshake before the tap; a cert
with and without the serverAuth EKU both got through once tapped.

## Found on the way
- The host PC's firewall (ufw here) blocked every phone until the user opened the ports. The game can't tell from its
  side: follow-up M152e.
- The user's Android phone is a work phone whose tunnel keeps it off the LAN, so Android wasn't testable.

## How to repeat
A static page polling `navigator.getGamepads()` and reporting `isSecureContext`/pads/pressed buttons, served by
`python3 -m http.server` on the LAN IP; open it on the device with a controller connected and press buttons. On the
desktop, an evdev `UInput` pad (like `tools/vpad.py`) stands in for a controller.

## Not verified / not 1:1
- Not 1:1: new feature, no counterpart in the original.
- Android Chrome/Firefox: not tested (work-VPN phone can't reach the LAN). Expected to work like their desktop engines
  (same Blink / Gecko gamepad code), but inferred, not tested. M152c's own phone check covers it.
- macOS Safari, and Chrome/Firefox on iOS: not tested (the iOS ones run on Safari's WebKit, so they should match).
- Desktop Chromium/Firefox read a uinput virtual pad, not a physical controller.
- Only the probe page was tested, not the game: no guest has played a match with a controller yet (the guest page is M152c).
