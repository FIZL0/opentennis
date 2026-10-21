# B626 — Web app: instant Settings category switch

Settings' tab switch (B40e2's card slide through a dimmed panel, `main_menu/slide.rs`) is skipped in the web build:
`Slide` gets an `instant_tabs` flag, on under `cfg!(target_arch = "wasm32")`; a switch between two Settings pages
then lands on the finished state at once (no dim panel, no fade). Every other screen change still slides, and native
builds keep the tab slide. Test `slide::web_tabs_switch_at_once` forces the flag on.

`hst-base/src/web.rs` needed nothing: a compile-time cfg is enough.

## Not verified / not 1:1
- Not run in a browser (unattended run, no web build/serve here); the logic is covered by the unit test.
- Front-end, ours by design (menus need not be 1:1), so no comparison to the original.
