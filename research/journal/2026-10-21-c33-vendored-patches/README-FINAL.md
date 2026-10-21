# C33 — document the vendored patches (2026-10-21)

From the repo audit (`2026-10-21-repo-audit`, *Vendored crates*).

- `third_party/README.md`: one table of the six crates — upstream version, every file changed from the crates.io
  release, the task and what it does, upstream status (none sent upstream; B385's journal found no Bevy issue; the
  audit found wgpu 30.0.1 still has the B328 problem), and how to move one onto a new release.
- Found by diffing each folder against `~/.cargo/registry/src/*/<crate>-<version>`, beyond what the audit listed:
  bevy_render's GPU-path B395 change (gpu_preprocessing.rs) and winit's M108 Android lifecycle hook were also
  missing from the root `Cargo.toml` comments; winit's M91 change also touches `ios/event_loop.rs`; ggrs'
  `Cargo.toml` drops upstream's examples/tests/dev-dependencies. All in the README and the root comments now.
- Markers: every changed code file now names its task (added to ggrs `Cargo.toml`/`lib.rs`/`protocol.rs`, gilrs-core
  `platform/mod.rs`, winit `ios/event_loop.rs` and the pad/lifecycle hunks of `android/mod.rs`). The existing
  markers come in several spellings (`HST (…)`, `HST patch`, `hst B…`, `opentennis P…`); left as they are, the
  check accepts all of them.
- Stale paths fixed in the root `Cargo.toml`: `hst-base/src/android_pad.rs`, hst-play's `mode.rs`.
- `tools/patches.py` (run by `done.sh`): fuzz/Cargo.toml's `[patch.crates-io]` resolves to the same crates and
  folders as the root's; every vendored crate has a README row at its Cargo.toml version; and, where the upstream
  release is in the local cargo registry, every changed `.rs`/`.wgsl`/`.toml` file is named on its crate's rows
  and has a marker. Tried: removing winit from the fuzz list and renaming a README entry both fail it.

## Not verified / not 1:1

- Upstream status is "not sent"; I didn't check each upstream's current tracker for a fix (no network lookup in
  an unattended run). Where the registry lacks a crate's upstream release, the file-level check is skipped.
- No gameplay or rendering change, so nothing to compare against the original.
