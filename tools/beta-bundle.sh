#!/usr/bin/env bash
# tools/beta-bundle.sh WEB_TAR OUT SHA [BUILD...] — the whole of https://beta.opentennis.dev as one tarball, for
# pages.yml's beta job (B745: the beta lives there, not in a GitHub release): the website from WEB_TAR, and under
# download/ the beta's builds, a page listing them (download/index.html), files.txt (their names) and commit.txt (the
# full commit they were built from; Actions builds' update notice reads it, crates/hst/src/update.rs).
# Without builds (a tools/ship-local.sh prebuilt run: the website alone) download/ is copied from the live beta site
# (BETA_URL, default https://beta.opentennis.dev), so its downloads stay what they were.
set -euo pipefail
[ $# -ge 3 ] || { echo "usage: tools/beta-bundle.sh WEB_TAR OUT SHA [BUILD...]" >&2; exit 1; }
web=$1 out=$(realpath -m "$2") sha=$3
shift 3
live=${BETA_URL:-https://beta.opentennis.dev}
d=$(mktemp -d)
trap 'rm -rf "$d"' EXIT
tar xzf "$web" -C "$d"
dl=$d/download
mkdir -p "$dl"
ok() { [[ $1 =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; }

if [ $# = 0 ]; then
    if curl -sSfL "$live/download/files.txt" -o "$dl/files.txt"; then
        while read -r f; do
            ok "$f" || { echo "bad name in $live/download/files.txt: $f" >&2; exit 1; }
            curl -sSfL "$live/download/$f" -o "$dl/$f"
        done < "$dl/files.txt"
        curl -sSfL "$live/download/commit.txt" -o "$dl/commit.txt"
        curl -sSfL "$live/download/index.html" -o "$dl/index.html"
    else
        echo "no download/ on $live yet: the website alone"
        rm -rf "$dl"
    fi
else
    : > "$dl/files.txt"
    for f in "$@"; do
        n=$(basename "$f")
        ok "$n" || { echo "bad file name: $n" >&2; exit 1; }
        cp "$f" "$dl/$n"
        echo "$n" >> "$dl/files.txt"
    done
    echo "$sha" > "$dl/commit.txt"
    what() {
        case $1 in
            opentennis-windows-x86_64.zip) echo "Windows 10/11|Unzip, then drop your ISO on <code>opentennis.exe</code>" ;;
            opentennis-linux-x86_64.tar.gz) echo "Linux / Steam Deck|Unpack and open <code>opentennis</code>, then pick your ISO. For Steam (with artwork): run <code>add-to-steam.sh</code>" ;;
            opentennis-linux-aarch64.tar.gz) echo "Linux ARM64 (aarch64)|Unpack and run <code>OpenTennis/OpenTennis.sh</code>, with your ISO in the <code>OpenTennis</code> folder" ;;
            opentennis.zip) echo "Handhelds with PortMaster|Put the zip in PortMaster's <code>autoinstall</code> folder, then your ISO in <code>ports/opentennis/</code>" ;;
            *) echo "|" ;;
        esac
    }
    rows=
    while read -r n; do
        IFS='|' read -r plat how <<< "$(what "$n")"
        size=$(numfmt --to=iec --suffix=B "$(stat -c %s "$dl/$n")")
        rows+="<tr><td>${plat:-&nbsp;}</td><td><a href=\"$n\">$n</a> <span class=dim>$size</span></td><td>$how</td></tr>"$'\n'
    done < "$dl/files.txt"
    cat > "$dl/index.html" <<EOF
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>OpenTennis beta downloads</title>
<style>
:root { --bg: #fafafa; --fg: #1d1d1f; --dim: #6e6e73; --line: #d2d2d7; --accent: #2a7a2a; color-scheme: light dark }
@media (prefers-color-scheme: dark) { :root { --bg: #161617; --fg: #f0f0f2; --dim: #a1a1a6; --line: #3a3a3c; --accent: #6fcf6f } }
body { margin: 0; background: var(--bg); color: var(--fg); font: 15px/1.5 system-ui, sans-serif }
main { max-width: 60rem; margin: 0 auto; padding: 2rem max(16px, env(safe-area-inset-right, 0px)) 3rem max(16px, env(safe-area-inset-left, 0px)) }
a { color: var(--accent) }
.dim { color: var(--dim) }
.wrap { overflow-x: auto }
table { border-collapse: collapse; width: 100% }
td { border-top: 1px solid var(--line); padding: .6rem .5rem; vertical-align: top }
td:first-child { font-weight: 600; white-space: nowrap }
code { font-size: .9em }
</style>
</head>
<body>
<main>
<h1>OpenTennis beta</h1>
<p><strong>Rebuilt on every change to main (<a href="https://github.com/FIZL0/opentennis/commit/$sha">${sha::7}</a>). Newer than the latest release, less tested.</strong>
For the stable version, see <a href="https://github.com/FIZL0/opentennis/releases/latest">the latest release</a>.</p>
<p>No game data is included: you need your own <em>Hot Shots Tennis (USA)</em> (SCUS-97610) ISO.
Or play the beta <a href="../">in the browser</a>.</p>
<div class="wrap"><table>
$rows</table></div>
</main>
</body>
</html>
EOF
fi
tar czf "$out" -C "$d" .
echo "$out"
