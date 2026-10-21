# M143 — per-user data folder

From the 2026-10-21 repo audit: the desktop game kept its saved files beside the ISO (or in `.`), and the mod store's
files in whatever the working folder happened to be.

## What changed
- `hst_base::store` now owns where everything is saved. `store::init(iso)` runs in the desktop `main` after
  `disc::resolve`: it sets the folder to `disc::data_dir()` (`$XDG_DATA_HOME/hst` / `~/.local/share/hst`,
  `%APPDATA%\hst`, and now `~/Library/Application Support/hst` on macOS), which is where the imported disc pack
  already lives. `store::dir(iso)` / `store::path(iso, name)` give that folder, or the disc's folder when init never ran
  (tests, tools, iOS, Android, the browser: unchanged). `store::file(name)` is for the files that never had a disc
  (mods-store.json, mods-wishlist.txt, hf-token.txt): the per-user folder; else the bare name as before (the browser's
  localStorage key, Android's app folder as its CWD), or a per-process temp folder under `cfg(test)` / `testing`, so
  tests no longer drop mods-store.json into crates/hst and crates/hst-base.
- Moved onto it: online.txt (`net.rs`), hints.txt (`loading.rs`), controls.txt (`controls.rs`), mods-online/
  (`mods_online.rs`), the texture packs / texture-replacements root (`textures.rs`, `main.rs`), mods-store.json,
  hf-token.txt (`modstore.rs`), mods-wishlist.txt (`grid.rs`). settings, favorites, names, pipelines.json, rejoin.txt,
  mods/, cache/ and the audio settings already went through `store::settings(iso)`, which now follows.
- Migration: the first start copies each saved file/folder the new folder doesn't have yet, from beside the disc and
  from the working folder (fixed list `SAVED`). Copied, not moved: older builds (and other worktrees still on main's
  code sharing the same ISO folder) keep working; a symlinked folder (mods/ on another drive) is linked again, not
  copied. On macOS the old `~/.local/share/hst` (pack included) is renamed to the Application Support folder once.
- `HST_DATA=<dir>` pins the folder with no migration. `tools/shot.sh` sets it to the ISO's folder (keeps its
  "symlink in a folder without mods/texture-replacements shows the disc textures" trick); `bench.sh` and
  `select_stress.sh` too, so dev runs read the repo's settings as before. `slow.sh` (a real user's machine) doesn't.

## Verified
- `store::tests::migrate_copies_what_is_missing` (copies, keeps newer files, keeps the old copies, same-folder no-op).
- Smoke run of the debug binary with `XDG_DATA_HOME` in the scratchpad and the ISO symlinked into a folder holding a
  settings.txt and online.txt: both were brought over, controls.txt and cache/ were written to the data folder, nothing
  new beside the ISO.
- wasm32 `cargo check -p hst-base`, clippy clean on the touched code.

## Not verified / not 1:1
- macOS, Windows: not run here (no machine); the paths are the same `disc::data_dir` code the pack import already used,
  plus the macOS branch, untested.
- Not a gameplay port: nothing to compare against the original.
