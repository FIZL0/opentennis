# M159a: iOS Update opens the installer that installed the app

M159 (the on-device probe that shows what each installer leaves) hasn't run on a phone yet, so its screenshots
weren't there. The marks below come from the installers' own public source code (GitHub, read on 2026-10-29) instead.

## What each installer leaves in an app it installs
- **AltStore** (altstoreio/AltStore, `ResignAppOperation`): adds a URL type with the scheme `altstore-<bundle id>`
  to the app's Info.plist (so it can open the app). `ALTDeviceID`/`ALTServerID` only go into AltStore's own copy, not
  other apps, so they're no use here.
- **SideStore** (SideStore/SideStore, its pipeline `ResignAppOperation`): removes any earlier `sidestore-…` scheme and
  adds `sidestore-<bundle id>`. It leaves an `altstore-` scheme alone, so SideStore is checked first.
- **TrollStore** (opa334/TrollStore, RootHelper): writes a `_TrollStore` marker file into the app's bundle container
  (beside the .app). TrollStore Lite writes `_TrollStoreLite` instead (not handled, see below). TrollStore's URL scheme
  is `apple-magnifier`, and `apple-magnifier://install?url=<ipa>` installs from a URL.
- **Feather** (khcrysalis/Feather, `SigningHandler`): changes only keys the user picks (bundle id, name, file
  sharing…) and adds nothing of its own, so it can't be told apart from our bundle. It does have
  `feather://install/<url>` and `feather://source/<url>`, but with no mark there's nothing safe to open, so Feather
  installs get the release page (their source shows the update anyway).

## Change
- `hst-base/src/ios.rs` `install_marks()`: the URL schemes in our own Info.plist (NSBundle's info dictionary, class
  checked at each step) and whether `_TrollStore` sits in the folder above the .app. It never asks iOS what's
  installed (no `canOpenURL`, B652).
- `update.rs`: `Installer` (AltStore / SideStore / TrollStore), `installer(schemes, troll)` (TrollStore marker first,
  then `sidestore-`, then `altstore-`; the prefix alone doesn't count), and the links: AltStore/SideStore
  `…://source?url=<encoded https://fizl0.github.io/apps/source.json>` (the source `web/ios.html` adds), TrollStore
  `apple-magnifier://install?url=<encoded release .ipa>`. A new `Way::Installer(link)`: the notice button says Update and
  opens the link. It never starts on its own (Updates: Always skips it, since that would switch apps unasked). No mark: Safari
  release page as before.
- `home.rs`: the notice's help line and key list name the installer ("Update opens SideStore, which installed this app…").
- `tools/ios-ipa.sh`: no change needed (`openURL` doesn't need `LSApplicationQueriesSchemes`).
- Release note line.

## Checks
- `tools/check.sh -p hst update`: 12 passed, including the new `the_installer_from_its_marks` (detection from the
  marks, links escaped like `encodeURIComponent`).
- `tools/ios-build.sh check -p hst --target aarch64-apple-ios`: builds, no warnings.

## Not verified / not 1:1
- Not tried on an iPhone (there's no device or simulator here), so the marks haven't been seen on a real install.
  They come from each installer's current source, not from M159's screenshots. When M159's probe runs, check that the
  Install info line shows the `altstore-`/`sidestore-` scheme and `_TrollStore`.
- Feather can't be detected (it leaves nothing), so Feather installs keep the Safari page. TrollStore Lite
  (`_TrollStoreLite`) isn't handled: its URL scheme wasn't confirmed, so it gets Safari.
- AltStore/SideStore open at our source (task's choice) rather than installing the .ipa directly: the player taps
  the update there.
- Front-end only, so there's nothing to compare against the original game.
