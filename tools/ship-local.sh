#!/usr/bin/env bash
# tools/ship-local.sh [-w] — put HEAD's website live on opentennis.dev from this machine instead of waiting ~20 min for
# Actions: build the browser version here (tools/web.sh), push HEAD to main, upload web/ and deploy it through
# pages.yml's `prebuilt` input (download + deploy, ~1 min). -w watches the deploy. The push still starts the usual
# pages run (which deploys the same website again when it ends). B745: web/ goes up as a draft release (hidden, never
# a version to launchers), which that run takes in and deletes. While SITE_FROM is release (pages.yml), opentennis.dev
# shows only releases, so this refuses: publish a release with tools/release.sh instead.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -z "$(git status --porcelain --untracked-files=no)" ] || { echo "commit first: what ships is HEAD" >&2; exit 1; }
git fetch -q origin main
git merge-base --is-ancestor origin/main HEAD || { echo "origin/main has commits HEAD lacks: pull first" >&2; exit 1; }
sha=$(git rev-parse HEAD)

# P147: no beta site any more; pages.yml deploys a prebuilt site only while opentennis.dev follows main
[ "$(gh variable get SITE_FROM 2>/dev/null)" = release ] && { echo "SITE_FROM is release: opentennis.dev shows only releases (tools/release.sh)" >&2; exit 1; }
site=https://opentennis.dev/
env -u HST_SITE tools/web.sh
mkdir -p dist
tar -czf dist/web.tar.gz -C web .

git push -q origin HEAD:main

# web/ for pages.yml to pick up: a draft, which nobody else sees and pages.yml deletes once it has it (an older
# leftover one goes first, and the published web-prebuilt release from before B745 if it is still there)
repo=$(gh repo view --json nameWithOwner -q .nameWithOwner)
for old in $(gh api "repos/$repo/releases" -q '.[] | select(.draft and .tag_name == "web-prebuilt") | .id'); do
    gh api -X DELETE "repos/$repo/releases/$old"
done
gh release delete web-prebuilt --cleanup-tag -y 2>/dev/null || true
id=$(gh api "repos/$repo/releases" -f tag_name=web-prebuilt -f target_commitish="$sha" -f name="Web build ${sha::7} (tools/ship-local.sh)" \
    -f body="The browser version as tools/ship-local.sh built it, for pages.yml to deploy. Not a game download." -F draft=true -q .id)
gh api -X POST "https://uploads.github.com/repos/$repo/releases/$id/assets?name=web.tar.gz" -H 'Content-Type: application/gzip' \
    --input dist/web.tar.gz -q .size >/dev/null

gh workflow run pages.yml --ref main -f prebuilt=true -f release="$id"
sleep 5
id=$(gh run list -w pages.yml -e workflow_dispatch -L 1 --json databaseId -q '.[0].databaseId')
echo "deploying ${sha::7} — run $id"
[ "${1:-}" = -w ] && gh run watch "$id" --exit-status && echo "live: $site"
