# M159: iOS install probe

## What's in the game
Settings → Online & Mods → **Updates** (the row's side panel, under its description) shows on iOS only one
**Install info** line, read once a run from the app's own bundle and container, also written to `hst-log.txt`:

- `bundle <id>`: our bundle id as installed (AltStore/SideStore free accounts usually append the team id).
- `Added Info.plist keys:` every top-level key that `tools/ios-ipa.sh` doesn't write (names only, never values:
  `ALTDeviceID`'s value is the phone's id). `hst_base::install_info::OURS` is the script's keys plus the ones
  Foundation adds to the info dictionary; a test keeps it in step with the script.
- `URL schemes:` every `CFBundleURLSchemes` entry (M159a's `altstore-…`/`sidestore-…`).
- `Beside the app:` the names in the bundle container next to the .app (`_TrollStore`, `iTunesMetadata.plist`…). If
  the sandbox won't list it, `(can't list)` and the known markers checked one by one.
- `Profile:` `embedded.mobileprovision`'s kind (enterprise = ProvisionsAllDevices; development = device list +
  get-task-allow; ad-hoc = device list without it; App Store = neither), device count, team name, profile name,
  expiry day; `none` if there's no profile (TrollStore signs without one), `unreadable` if no plist in it.
- `Update opens:` which installer M159a's Update takes it for (or the release page), so the screenshot checks that too.

Nothing asks iOS about other apps, nothing goes over the network. The side panel's footnote now takes as many lines
as the panel has room for (never fewer than the old 5), so the long line isn't cut short.

Code: `crates/hst-base/src/install_info.rs` (wording, profile reading, tests), `crates/hst-base/src/ios.rs`
`install_info()` (gathering), `crates/hst/src/update.rs` `install_info()`, `play/main_menu/options.rs` (the note).

## Checks
- `tools/check.sh -p hst-base install_info`: 4 new tests (profile kinds from a wrapped plist, nested values, the
  line's wording, `OURS` vs `tools/ios-ipa.sh`).
- `tools/check.sh -p hst options`: passes. `tools/ios-build.sh check -p hst --target aarch64-apple-ios`: builds clean.

## Waiting on the user
Install the next .ipa (with this change) through each installer you use and screenshot Settings → Online & Mods →
Updates: AltStore, SideStore, TrollStore, Feather, Sideloadly (and anything else). Drop the screenshots (or the
`Install info:` line from `hst-log.txt` in the app's Files folder) in this journal's chat. The next round writes here
what each installer leaves and fixes M159a's detection if the marks differ from its source reading.

## Not verified / not 1:1
- Not run on an iPhone (no device or simulator here): the line's look in the side panel at phone sizes, whether the
  sandbox lets the app list its bundle container, and which Info.plist keys Foundation adds at run time (any extra
  `CF…`/`NS…` key it adds will show as "added" until it's put in `OURS`) are all unseen.
- The .app's own contents (an installer injecting dylibs or frameworks, e.g. Feather's tweaks) aren't listed.
- Front-end only: nothing to compare against the original game.
