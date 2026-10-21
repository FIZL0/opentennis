# C37 — hst-play's unused dependencies

Follow-up from C32 (`../2026-10-21-c32-unused-deps/`), whose closing note said hst-play's Cargo.toml lists gltf,
iroh, tokio, reqwest, the phone sections, ….

It doesn't, and never did: `git log` on `crates/hst-play/Cargo.toml` shows only P61 (which created it already trimmed)
and B503's lint section. C32's note must have been read off hst's or hst-base's lists. None of gltf, wgpu-types,
iroh, iroh-tickets, n0-future, tokio, data-encoding, reqwest, miniz_oxide or sha2 appear, and there is no
iOS/Android/native section.

Each dependency it does list is used in `src`:
- hst-data, hst-sim, hst-base, bevy: everywhere
- ggrs (+ its wasm32 `wasm-bindgen` line, which ggrs needs for its clock in the browser), bincode, web-time: `play/rollback.rs`
- serde: derives on match resources (`play/cutaway.rs`, `play/heard.rs`, …); serde_json: `play/netplay.rs`
- tiny-skia, ab_glyph: `play/touch_pad.rs`, `play/menu_art.rs`; image: `save_buffer` in `play/menu_art.rs`, `play/name_tags.rs`
- dev hst-base `testing`: the crate's tests

No change to the Cargo.toml, so no wasm/Android/iOS build check was needed.

## Not verified / not 1:1
none
