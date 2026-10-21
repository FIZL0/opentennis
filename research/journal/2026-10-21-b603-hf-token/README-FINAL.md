# B603 — Hugging Face token stored loosely

From the repo audit (2026-10-21). Before: `hf-token.txt` was written to the working directory with the default
umask (0644 typically), then chmod'ed 0600 by a second call on the same relative path: a window where the token was
world-readable, and the chmod could hit a different file if the cwd changed. The module doc said "beside
settings.txt", but settings live beside the disc. Android kept it in the shared external files dir.

## Change (`crates/hst-base/src/modstore.rs`, `crates/hst/src/main.rs`)
- `modstore::keep_token_in(dir)` (first call wins) sets where the token lives; `token_path()` is `dir/hf-token.txt`
  (bare `hf-token.txt` before it's set, and always in the browser, where it's the localStorage key as before).
- Desktop: `start()` calls it with settings.txt's dir (`store::settings(iso).parent()`).
- Android: `android_main` calls it first with `internal_data_path()` (app-private), so the later desktop-style call is
  ignored.
- Writes go through `write_private`: `OpenOptions::mode(0o600)` at creation, plus `fchmod` 0600 on the open handle
  before writing (an existing file keeps its old mode through `open`). Sign-out uses the same writer.
- Migration: a token an older build left in the working directory moves to the new place (if none there yet) and
  the old file is removed; an existing token file at the new place is tightened to 0600.
- Test: `token_file_is_owner_only` (fresh file is 0600; a 0644 file becomes 0600 with the new contents).

## Not verified / not 1:1
- The Android build wasn't compiled here (needs `tools/apk.sh`'s NDK env); `AndroidApp::internal_data_path()`
  checked to exist with that signature in android-activity 0.6.1. Desktop, wasm32 and Windows (`x86_64-pc-windows-gnu`,
  hst-base) type-check.
- Windows: no POSIX mode; the file sits in the settings dir with the directory's ACLs (as before).
- Not a gameplay change; nothing to compare against the original.
