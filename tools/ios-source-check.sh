#!/usr/bin/env bash
# tools/ios-source-check.sh FILE — fails unless altstore-source.json's versions[] is newest first, has no duplicate
# version and matches the app's top-level version/versionDate/downloadURL/size (tools/ios-source.sh; CI runs it)
set -euo pipefail
jq -e '.apps[0] as $a | $a.versions as $vs
  | ($vs | map(.version | split("-")[0] | split(".") | map(tonumber))) as $n
  | ($vs | length) > 0
    and $n == ($n | sort | reverse)
    and ($vs | map(.version) | unique | length) == ($vs | length)
    and [$a.version, $a.versionDate, $a.downloadURL, $a.size] == ($vs[0] | [.version, .date, .downloadURL, .size])' \
    "$1" >/dev/null || { echo "$1: versions[] not newest first, duplicated, or not matching the top-level fields" >&2; exit 1; }
