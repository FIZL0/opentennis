# B749 — browser touch pad waits for the splash's tap

B748 kept the native apps' on-screen pad away (no gamepad, nothing drawn, no touch read) until the splash is tapped,
via `touch_pad::Tapped`. The browser's pad is `web/touch.js`, which the game never gated.

## What changed
- `web/touch.js`: a `started` flag, set by the new `window.hstTouchStart()`. The pad's `want` (shown) now needs it, so
  before the tap the pad's root stays `hidden` (display none: touches fall through to the canvas, i.e. the splash,
  whose tap counts in the middle 75% only, B748's shared `pointer.rs` rule) and it isn't added to `getGamepads()`.
- `hst-base::web::touch_start()`: calls it, once.
- `touch_pad::plugin` on wasm: inits `Tapped` (the splash's `tick` and `main.rs`'s no-menu start already set it, as on
  native) and a `Last` system `page_start` that calls `web::touch_start` once it's set.

## Checked
- `cargo check -p hst-play --target wasm32-unknown-unknown` builds.
- Headless Chromium on a stub page with `touch.js` and `?touch`: before `hstTouchStart` the pad is hidden and
  `getGamepads()` lists 0 pads; after it, shown and 1 pad.

## Not verified / not 1:1
- Not run on a phone or in the full web build (needs a disc import in the browser); only the stub-page check above.
  Front-end behaviour, no original to match.
