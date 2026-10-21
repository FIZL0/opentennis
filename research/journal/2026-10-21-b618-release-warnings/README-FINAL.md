# B618 — release build warnings

The four warnings from the 1.3.0 `tools/release.sh` log, each fixed and re-checked on the target that showed it:

1. **main.rs in two build targets** (every platform, Cargo's manifest warning). The Android `[[example]] hst` now
   points at `crates/hst/src/android_so.rs`, a git symlink to `main.rs`. Cargo compares target paths lexically
   (`src/./main.rs` still warned), and rustc resolves `mod x;` from the symlink's own directory, so it builds the same
   crate. Tried first: `include!("main.rs")` from a real file. It fails: `mod play` inside an `include!` is a
   macro-expanded item and may not shadow the `pub use hst_play::*` glob (E0659). A `[lib]` split of the game crate
   would also work but moves a lot of code; not worth it for a warning. The Android build runs only on Linux
   (`tools/apk.sh`), so a Windows checkout without symlinks doesn't matter.
2. **gilrs-core** (Android, iOS): `#[macro_use] extern crate log` only where a backend logs (not android/ios), and
   the default backend's `set_ff_state` takes `_`-named arguments. Listed in `third_party/README.md`; `tools/patches.py` passes.
3. **update.rs `Due` never constructed**: that was the wasm32 build (the browser's `offer` is always `None`). It's
   `allow(dead_code)` on wasm32 only. The type can't be cfg'd out because `offer`'s signature, which the menu
   calls on every platform, names it. Confirmed: taking the attribute out brings the warning back on wasm32 only.
4. **HstActivity.java deprecations**: `getInstallerPackageName` (API < 30) and `getParcelableExtra(String)`
   (now `getParcelableExtra(name, Intent.class)` on API 33+). The old calls are only reached on Android versions
   that have nothing newer (minSdk 26), so each sits in its own small helper with `@SuppressWarnings("deprecation")`.
   `tools/apk.sh` now compiles with `javac -Xlint:all -Werror` instead of `-nowarn`, so a new one fails the build.

Web target guard: `tools/webcheck.sh` (`cargo check -p hst --target wasm32-unknown-unknown --release --features ship`
into web.sh's `target-wasm/`) fails on any error or warning in our code. `tools/done.sh` runs it after clippy, and
`tools/release.sh` runs it before the native builds. Checked both ways: it passes now, and fails with the `Due`
warning put back. About 3 s warm; the first run in a slot is about 2 min.

Checks run (release profile, `--features ship`): wasm32, aarch64-apple-ios (`tools/ios-build.sh check`),
aarch64-linux-android (the example, NDK env as apk.sh), x86_64-pc-windows-gnu: no warnings in our code.
`javac -Xlint:all` on the activity: clean.

## Not verified / not 1:1

- No full `tools/release.sh` or `tools/apk.sh` run: those sign and publish, and the Linux build needs the docker
  container. Each target was type-checked (`cargo check`) instead, which shows the same Rust warnings, plus `javac`
  on its own. Link-time warnings (none were reported in 1.3.0) weren't looked at.
- Not run on a phone: the Java change (install confirm on Android 13+, the store check below Android 11).
  It has the same behaviour through the newer API.
