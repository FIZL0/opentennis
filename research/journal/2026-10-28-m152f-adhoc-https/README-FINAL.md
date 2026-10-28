# M152f — adhoc over HTTPS

Plain-http guests can't use OPFS or a service worker (not a secure context), so a phone held the whole disc pack in
memory (too much on an iPhone 15 Pro, user 2026-10-28). The adhoc host now serves https as well, and the card/QR give
the https address.

## What changed

- **One port, both schemes.** The accept thread peeks the first byte: a TLS handshake (0x16) goes through rustls
  (ring provider, ALPN http/1.1), anything else is plain http as before (native guests, typed http addresses). Same
  port list, same UDP pin for WebRTC. `crates/hst-base/src/adhoc/tls.rs`; `adhoc.rs` `take`/`serve` are generic over
  the connection (`Conn`).
- **Certificate.** rcgen, self-signed P-256, serverAuth EKU, valid a day back to a year ahead, fresh serial each time
  (Firefox rejects a reused serial from the same issuer), names = every LAN IPv4 + 127.0.0.1 + localhost. Kept in
  `<data dir>/adhoc-tls` (`cert.der`, `key.der`, `names.txt`); reused while it names every current address and has more
  than 30 days left, else remade (keeping the old names, up to 32, so a phone that already accepted it for an address
  isn't asked again for that one more than needed). Made when hosting starts.
- **WebSocket over TLS** (the no-WebRTC fallback, M152c): the upgrade is answered on the rustls stream, then a small
  async pump bridges rustls to a tokio duplex whose other end is the normal ws `arrive`. `net/ws.rs` split into
  `answer` + `arrive`.
- **Wasm**: `/rtc` signalling and `/adhoc/ws` pick https/wss when the page is https (`web::secure_page`), since mixed
  content is blocked.
- **Guest page** (`web/index.html`): on a secure adhoc page OPFS and the service worker are on (as on the site); the
  disc is streamed into OPFS `disc.pack` in 8 MB parts (resumes a cut-off download), keyed by the pack's hash in
  `info.json` (`disc.hash`) + size, and read back from there; the next visit reuses it. The service worker leaves
  `adhoc/*` and `/rtc` to the network. Plain-http guests unchanged (memory, no SW).
- Card: "Tap past the browser's warning once." after the address when it's https (`context/m152f/card.png`).
- `tools/web_guest.sh`: `SCHEME=https` default (Chromium `--ignore-certificate-errors`), `KEEP=1` keeps the profile.

## Verified

- `tools/check.sh -p hst-base adhoc` 14 passed (incl. https GET with Range over `localhost` and `127.0.0.1` with a
  rustls client trusting the kept cert; cert keep/remake rules); `-p hst-base ws` 12 passed (incl. a guest over a
  secure WebSocket, 100 kB down). wasm32 check clean.
- End to end, headless Chromium guest vs release host (`context/m152f/run1..3`):
  - run1 https, empty profile: disc went into OPFS (165 MB in the profile's File System), joined, played 40 s, host
    and guest hashes equal at tick 0 and 1800.
  - run2 https, same profile (`KEEP=1`): "the host's disc, kept from a visit before", no disc download, match hashes
    equal.
  - run3 `SCHEME=http`: old path (memory), joins and plays, hashes equal.

## Not verified / not 1:1

- **No real iPhone/Android**: whether Safari's warning tap works for the wss/fetches as well as the page, whether
  Add to Home Screen opens without a new warning (iOS home-screen apps have their own storage, so they'd fetch the
  disc once more), Android install prompt. Put in `plan/HUMAN.md`; fallback if the home-screen app can't get past
  the warning is a per-host certificate on opentennis.dev names (not done).
- Safari may evict a non-installed site's storage after 7 days without a visit: the disc then downloads again.
- Once read, the wasm side still holds the whole pack in memory (same as the site's path); what's gone is the
  page-side copy kept in JS during the download.
- The certificate is made when hosting starts: a LAN address that appears mid-session isn't on it until the next
  host.
- New feature, nothing in the original to match.
