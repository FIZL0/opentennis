#!/usr/bin/env bash
# tools/ios-source.sh VERSION IPA [OUT] — the iOS app as a sideloading source (M140, out of tools/release.sh): the
# AltStore v2 format, which AltStore, SideStore and Feather all read. versions[] is every published (not draft, not
# pre-release) GitHub release with an .ipa, newest first, so clients show the history and can roll back; release
# vVERSION goes on top from the local IPA (it isn't published yet when tools/release.sh runs this), listed once.
# NOTES=file gives its "What's new" text (RELEASE_NOTES.md); older ones take their release's "## New in" section.
# OUT defaults to altstore-source.json next to IPA. tools/release.sh uploads it with every release, so the URL below
# always serves the latest release's; web/ios.html has the one-tap "add source" buttons for it.
# The app also carries the legacy top-level version/downloadURL fields (= versions[0]), for older signers.
# A failed GitHub API call fails this (and the release): never a source with only the new version.
set -euo pipefail
[ $# -ge 2 ] || { echo "usage: tools/ios-source.sh VERSION IPA [OUT]" >&2; exit 1; }
version=${1#v} ipa=$2 out=${3:-$(dirname "$2")/altstore-source.json}
[ -f "$ipa" ] || { echo "no such IPA: $ipa" >&2; exit 1; }
repo=FIZL0/opentennis
rels=$(mktemp); trap 'rm -f "$rels"' EXIT
gh api "repos/$repo/releases" --paginate --jq '.[] | select((.draft or .prerelease) | not)
  | {tag: .tag_name, date: .published_at[:10], body: (.body // ""),
     ipa: ([.assets[] | select(.name | endswith(".ipa"))][0])} | select(.ipa)' >"$rels"
[ -s "$rels" ] || { echo "GitHub lists no IPA releases for $repo" >&2; exit 1; }
notes=$( [ -n "${NOTES:-}" ] && sed '/^<!--/d' "$NOTES" || true)
jq -n --arg v "$version" --arg date "$(date -u +%Y-%m-%d)" --arg notes "$notes" \
    --arg ipa "https://github.com/$repo/releases/download/v$version/opentennis-ios.ipa" \
    --argjson size "$(stat -c %s "$ipa")" --slurpfile rels "$rels" \
    --arg src "https://github.com/$repo/releases/latest/download/altstore-source.json" '
  def trim_: gsub("\\A\\s+|\\s+\\z"; "");
  def desc: gsub("\r"; "") | (capture("## New in [^\n]*\n(?<n>[\\s\\S]*?)(?:\n## |\\z)").n // "") | trim_;
  def entry: {version: (.tag | ltrimstr("v")), date, downloadURL: .ipa.browser_download_url, size: .ipa.size,
              minOSVersion: "15.0", localizedDescription: (.body | desc)};
  ($rels | map(select(.tag == "v" + $v)) | first | if . then entry else {} end) as $old
  | ($old + {version: $v, date: $date, downloadURL: $ipa, size: $size, minOSVersion: "15.0"}
     + (if ($notes | trim_) != "" then {localizedDescription: ($notes | trim_)} else {} end)) as $new
  | ([$new] + ($rels | map(select(.tag != "v" + $v)) | map(entry) | sort_by(.version | split("-")[0] | split(".") | map(tonumber)) | reverse))
  | map(if .localizedDescription == "" or .localizedDescription == null then del(.localizedDescription) else . end) as $vs
  | { name: "OpenTennis", identifier: "io.github.fizl0.hst", sourceURL: $src, website: "https://opentennis.dev/",
    iconURL: "https://opentennis.dev/icon-180.png", tintColor: "#2fae4a",
    subtitle: "A remaster of Hot Shots Tennis (PS2)",
    apps: [{ name: "OpenTennis", bundleIdentifier: "io.github.fizl0.hst", developerName: "FIZL0",
             subtitle: "A remaster of Hot Shots Tennis (PS2)",
             localizedDescription: "A remaster of Hot Shots Tennis (PS2). Plays from your own disc image (SCUS-97610): put the ISO in Files → On My iPhone → OpenTennis.",
             iconURL: "https://opentennis.dev/icon-180.png", tintColor: "#2fae4a", category: "games",
             version: $vs[0].version, versionDate: $vs[0].date, downloadURL: $vs[0].downloadURL, size: $vs[0].size,
             versions: $vs }],
    news: [] }' >"$out"
"$(dirname "$0")/ios-source-check.sh" "$out"
echo "$out"
