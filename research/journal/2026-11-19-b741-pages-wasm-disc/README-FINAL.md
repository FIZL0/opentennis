# B741 — pages.yml red: web build used the native-only disc module

`hst_base::disc` only exists off wasm. The iOS no-disc change read `disc::WAITING` from the touch pad (hst-play) and
cleared it in `menu_mode` (hst), so the web build in pages.yml stopped compiling.

Fix: the touch pad reads it through a small `disc_waiting()` helper, which is `false` on wasm (the web build never
waits for a disc). The clear in `menu_mode` is skipped on wasm. Checked with `cargo check --target
wasm32-unknown-unknown -p hst --features ship` (as `tools/web.sh` builds it) and a native `cargo check -p hst`.

## Not verified / not 1:1

- No full pages.yml run locally (release wasm build, wasm-bindgen, deploy). The wasm `cargo check` covers the
  compile error the job failed on. The real check is the next CI run on main.
- Build fix only, no gameplay change, so nothing needs checking against the original.
