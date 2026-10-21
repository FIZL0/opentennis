# B590 — web: game view inside the side/bottom safe area

**Problem.** B314 moved the web canvas below `env(safe-area-inset-top)` only. Held sideways, an iPhone's Dynamic
Island / notch is a *left or right* inset (and the home indicator a bottom one), which the canvas ignored, so at
Auto the picture ran under the island.

**Fix.** Reuse the phone apps' safe-area path (M106) instead of shrinking the canvas in CSS (which would shift touch
coordinates and the touch pad):

- `web/index.html`: a hidden fixed probe with `padding: 0 env(right) env(bottom) env(left)`; `window.hstSafeArea()`
  returns `[left, 0, right, bottom]` in CSS px from its computed padding (top stays 0: the canvas already starts
  under it). `viewport-fit=cover` was already set.
- `hst-base/src/web.rs` `safe_area()` calls it; `widescreen.rs` `follow_safe_area` now runs on wasm32 too and uses
  the canvas's CSS box (`page_size`) for the shares. `view()` then fits Auto / fixed ratios inside the safe rect with
  black strips, as on iOS/Android; taps go through the same `in_view` mapping.

**Checked.** `cargo check -p hst-base --target wasm32-unknown-unknown`; `tools/check.sh -p hst-base widescreen`
(8 pass). The probe in headless Chromium 154 with `Emulation.setSafeAreaInsetsOverride {left: 59, bottom: 21}`
returns `[59,0,0,21]`, `[0,0,0,0]` without the override (scratch script, not kept).

## Not verified / not 1:1
- Not seen on a real iPhone (no device here): the full web build wasn't run in a browser with insets; the native
  `view()`/`HST_SAFE` layout it feeds is M106's, already shot-tested.
- Stretch (B421) is still offered only in the phone apps, not on the web: the web canvas leaves the top inset out
  anyway, so a web Stretch would only reach the sides. Left as is.
- Front-end/presentation, so no original-game comparison applies.
