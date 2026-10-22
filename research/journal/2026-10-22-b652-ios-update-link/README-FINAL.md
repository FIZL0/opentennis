# B652 — iOS update/download link asked to restore Magnifier

## Cause
M140's iOS updater picked the first sideload installer whose URL scheme `UIApplication.canOpenURL` said yes to,
trying `sidestore://`, `altstore://` and then `apple-magnifier://` (the scheme TrollStore claims). On stock iOS,
Magnifier is a system app that owns `apple-magnifier`, so `canOpenURL` said yes on every phone. Update then opened
`apple-magnifier://install?url=…`, and iOS offered to restore Magnifier (the user had removed it).

## Fix
The user's decision: the app can't update itself on iOS, so it opens the release page in Safari.
- `update.rs`: `way()` is always `Way::Page` on iOS. Removed the `Installer` way, the `INSTALLERS` list,
  `installer()`, `escape()` and the test for the installer links (the settings-word check stays as `the_updates_word`).
  The button now says Download and opens the release's `html_url` (plain https) through `modstore::open`, which
  calls `ios::open_url` (`UIApplication.openURL`).
- `hst-base/src/ios.rs`: removed `can_open`, which nothing uses now.
- `tools/ios-ipa.sh`: removed the `LSApplicationQueriesSchemes` list from Info.plist.
- `home.rs`: removed the iOS "Update through your installer" text, which can't show any more. The existing iOS
  Download text ("opens its release page: install its .ipa over this app…") shows instead.

## Checks
- `tools/ios-build.sh check -p hst`: builds with no warnings.
- `tools/check.sh -p hst update`: passed.

## Not verified / not 1:1
- Not tried on an iPhone: there's no device or simulator here. `open_url` is the same call the release-page fallback
  and other links already used, and the URL is the GitHub release page's https URL.
- Front-end only, so there's nothing to compare against the original game.
