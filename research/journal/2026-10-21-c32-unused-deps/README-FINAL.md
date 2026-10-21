# C32 — dependencies left behind by the crate splits

From the repo audit (`../2026-10-21-repo-audit/`). Each crate's `src` was grepped for every dependency it lists.

- `crates/hst`: dropped ggrs (incl. its wasm `wasm-bindgen` feature line), bincode, gltf, serde, tiny-skia, ab_glyph,
  iroh-tickets, data-encoding, miniz_oxide, iroh-mainline-address-lookup, n0-mainline, iroh-mdns-address-lookup,
  wasm-bindgen-futures, rfd, the whole iOS and Android dependency sections. `iroh` itself is only named by a test
  (`online.rs`'s paste test) → moved to dev-dependencies. web-sys trimmed to the three features hst's own code
  needs (Window, Location, Storage; hst-base still turns on the rest). Kept log + tracing (only for `ship`'s
  compile-time cap), and the clipboard bevy feature.
- `crates/hst-base`: dropped ggrs (both lines), bincode, tiny-skia, ab_glyph, minisign-verify (hst's `update.rs` is the
  only user, and hst still lists it).
- `crates/hst-oracle`: hst-sim moved to dev-dependencies (only `tests/` name it).
- Comments fixed: hst's updater deps now say `update.rs`; hst-base's pointed at `main.rs` window_icon,
  `play/main_menu/previews.rs`, `mode.rs`, `play/touch_pad.rs`, `net.rs` relay_put (moved/renamed) → now name
  `android.rs`/`android_pad.rs`, the mod PNG files, hst-play's `mode.rs`/`touch_pad.rs`, `relay_get`/`modstore.rs`.

Cargo.lock only loses those crates from hst/hst-base/hst-oracle's dependency lists: every dropped crate is still built by another workspace crate.

Checked: `cargo check --workspace --all-targets`; wasm32 `-p hst --features ship` and `ship,webgpu`; Android
`--example hst --features android,ship` (aarch64, NDK as tools/apk.sh sets it); iOS `tools/ios-build.sh check -p hst
--features ship`. All clean.

## Not verified / not 1:1
- Builds were `cargo check`, not full linked release builds of the APK/IPA/web bundle (linking is unaffected by
  removing unused direct deps; every one is still in the graph through hst-base/hst-play).
- hst-play's Cargo.toml wasn't in scope; it too lists unused crates (gltf, iroh, tokio, reqwest, the phone deps, …) —
  left for a follow-up so this merge stays clean.
