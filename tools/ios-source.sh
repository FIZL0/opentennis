#!/usr/bin/env bash
# tools/ios-source.sh VERSION IPA [OUT] — the iOS app as a sideloading source (M140, out of tools/release.sh): the
# AltStore v2 format, which AltStore, SideStore and Feather all read, pointing at release vVERSION's copy of IPA.
# OUT defaults to altstore-source.json next to IPA. tools/release.sh uploads it with every release, so the URL below
# always serves the latest release's; web/ios.html has the one-tap "add source" buttons for it.
# The app also carries the legacy top-level version/downloadURL fields, for older signers that predate `versions`.
set -euo pipefail
[ $# -ge 2 ] || { echo "usage: tools/ios-source.sh VERSION IPA [OUT]" >&2; exit 1; }
version=${1#v} ipa=$2 out=${3:-$(dirname "$2")/altstore-source.json}
[ -f "$ipa" ] || { echo "no such IPA: $ipa" >&2; exit 1; }
repo=FIZL0/opentennis
jq -n --arg v "$version" --arg date "$(date -u +%Y-%m-%d)" \
    --arg ipa "https://github.com/$repo/releases/download/v$version/opentennis-ios.ipa" \
    --argjson size "$(stat -c %s "$ipa")" \
    --arg src "https://github.com/$repo/releases/latest/download/altstore-source.json" '
  { name: "OpenTennis", identifier: "io.github.fizl0.hst", sourceURL: $src, website: "https://opentennis.dev/",
    iconURL: "https://opentennis.dev/icon-180.png", tintColor: "#2fae4a",
    subtitle: "A remaster of Hot Shots Tennis (PS2)",
    apps: [{ name: "OpenTennis", bundleIdentifier: "io.github.fizl0.hst", developerName: "FIZL0",
             subtitle: "A remaster of Hot Shots Tennis (PS2)",
             localizedDescription: "A remaster of Hot Shots Tennis (PS2). Plays from your own disc image (SCUS-97610): put the ISO in Files → On My iPhone → OpenTennis.",
             iconURL: "https://opentennis.dev/icon-180.png", tintColor: "#2fae4a", category: "games",
             version: $v, versionDate: $date, downloadURL: $ipa, size: $size,
             versions: [{ version: $v, date: $date, downloadURL: $ipa, size: $size, minOSVersion: "15.0" }] }],
    news: [] }' >"$out"
echo "$out"
