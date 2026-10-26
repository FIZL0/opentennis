# B668 — the game's files beside the executable

## What changed
- `disc::data_dir()` is now the game's own folder on Windows and Linux (`disc::beside_game`: the `.AppImage`'s folder
  under an AppImage) when a probe file can be written there; else `disc::app_data_dir()` (the old per-user folder, same
  rules as before). Never beside the game for: Flatpak, a Steam library (`steamapps/`: Steam owns it, an uninstall
  wipes it), an updater copy under app-data `update/`, a build inside a cargo checkout (an ancestor holds `Cargo.lock`,
  so dev/runner builds keep using `~/.local/share/hst`; `HST_DATA` still overrides everything). macOS/iOS/Android/web
  unchanged.
- `store::init`: when the old app-data folder exists and differs, its saved files are copied over once each (missing
  ones only, old copies stay for older builds), except `mods/` and `mods-online/`. The beside-disc / working-folder
  migrations then don't run (they were folded into app-data already). `camera.txt` joined the migrated list.
- Mods: `modfs::moved(new, old)` registers the old `mods/`; `mods::dirs` lists the old folder's mods after the new
  one's (a folder name in both: the new one wins), sorted together. `folder_for` treats a name the old folder holds
  for another mod as taken; an install (update) of a mod still in the old folder lands in the new folder and deletes
  the old copy; `keep` (online peer mod) counts an old copy as kept. `installed_by_game` and the texture-pack order key
  (`mods_textures::folder`) accept the old folder.
- `mods/texture-replacements` is read from the old folder while the new one has none.
- Disc pack: read from the new folder, else the old one; a re-import writes into the new folder and removes the old pack.
- Updater: `Way::Copy` releases go to app-data `update/` always (never beside the game); `Way::Swap` already only
  touches the exe, `exe.old` and `opentennis.new`.

## Verified
- `old_folder_mods_move_when_updated` (hst-base modfs): listing both folders, update moves the mod, name clash skip.
- Live: debug build copied to a scratch folder with a fake `XDG_DATA_HOME` (settings + `pipelines.json` + test_pc00):
  settings/pipelines copied beside the binary, cache/controls written there, "1 custom characters" listed from the old
  `mods/`, mod select shot fine. Same with the folder `chmod 555`: everything stayed in the app-data folder.

## Not verified / not 1:1
- Windows not run (no Wine check): the code paths are the same std calls; `%APPDATA%` fallback unchanged.
- AppImage path not run (no AppImage build exists; uses `$APPIMAGE`).
- `mods-online/` (cached peers' mods) and `mod-pictures/` aren't carried over: refetched/rebaked.
- A Steam install keeps app-data on purpose (see above); a user decision could change that.
- Front-end/tooling only; no gameplay touched, nothing to match against the original.
