#!/usr/bin/env bash
# Ship origin/main to GitHub Pages now: cancel stale running/queued pages builds, start one on main's head unless one is already going.
set -euo pipefail
cd "$(dirname "$0")/.."
head=$(git ls-remote origin main | cut -f1)
id=
for s in in_progress queued pending waiting; do
  # a run already on main's head is kept (and reused); only stale ones are cancelled
  while read -r rid sha; do
    if [ "$sha" = "$head" ]; then id=$rid; else gh run cancel "$rid"; fi
  done < <(gh run list -w pages.yml -s "$s" --json databaseId,headSha -q '.[]|"\(.databaseId) \(.headSha)"')
done
if [ -z "$id" ]; then
  gh workflow run pages.yml --ref main
  sleep 5
  id=$(gh run list -w pages.yml -e workflow_dispatch -L 1 --json databaseId -q '.[0].databaseId')
fi
echo "building ${head::7} — run $id"
[ "${1:-}" = -w ] && gh run watch "$id" --exit-status && echo "live: https://opentennis.dev/"
