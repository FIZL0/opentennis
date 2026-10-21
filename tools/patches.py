#!/usr/bin/env python3
"""The vendored crates (third_party/README.md): fuzz/Cargo.toml patches the same crates as the root Cargo.toml, every
third_party/ crate is in the README's table at its Cargo.toml version, and, where the upstream release is in the
cargo registry, every source file changed from it is named on the crate's rows and carries a patch marker.
Run by done.sh; prints nothing when all is well."""
import filecmp
import glob
import os
import re
import sys
import tomllib

ROOT = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))
MARKER = re.compile(r"\b(HST|hst|opentennis)\b[ (:]*(patch|[BCMP][0-9])")
# upstream files that aren't code or that the vendored copy leaves out on purpose
SKIP = re.compile(r"(^|/)(\.cargo_vcs_info\.json|\.cargo-ok|Cargo\.lock|Cargo\.toml\.orig|LICENSE[^/]*|README\.md)$")
# only these differ from upstream as a whole folder left out (ggrs' examples, tests, docs, …): files, not folders
CODE = re.compile(r"\.(rs|wgsl|toml)$")


def patches(path, prefix):
    with open(path, "rb") as f:
        table = tomllib.load(f).get("patch", {}).get("crates-io", {})
    return {name: os.path.normpath(os.path.join(prefix, spec["path"])) for name, spec in table.items()}


def readme_rows():
    rows, crate = {}, None
    for line in open(os.path.join(ROOT, "third_party/README.md")):
        cells = [c.strip() for c in line.split("|")[1:-1]]
        if len(cells) != 4 or cells[0] == "Crate" or set(cells[0]) <= {"-"} and cells[0]:
            continue
        if cells[0]:
            crate = cells[0].strip("`")
            rows[crate] = {"version": cells[1], "files": ""}
        rows[crate]["files"] += " " + cells[2]
    return rows


def changed(upstream, ours):
    """Paths under `ours` (relative) that differ from `upstream` or are new, code files only."""
    out = []
    for dirpath, _, files in os.walk(ours):
        for name in files:
            rel = os.path.relpath(os.path.join(dirpath, name), ours)
            if SKIP.search(rel) or not CODE.search(rel) or rel.startswith("target/"):
                continue
            theirs = os.path.join(upstream, rel)
            if not os.path.exists(theirs) or not filecmp.cmp(theirs, os.path.join(ours, rel), shallow=False):
                out.append(rel)
    return sorted(out)


def main():
    errors = []
    root = patches(os.path.join(ROOT, "Cargo.toml"), ".")
    fuzz = patches(os.path.join(ROOT, "fuzz/Cargo.toml"), "fuzz")
    if root != fuzz:
        errors.append(f"fuzz/Cargo.toml [patch.crates-io] differs from Cargo.toml's: {sorted(fuzz.items())} vs {sorted(root.items())}")
    rows = readme_rows()
    registry = glob.glob(os.path.expanduser("~/.cargo/registry/src/*/"))
    for name, path in sorted(root.items()):
        ours = os.path.join(ROOT, path)
        with open(os.path.join(ours, "Cargo.toml"), "rb") as f:
            version = tomllib.load(f)["package"]["version"]
        row = rows.get(name)
        if row is None:
            errors.append(f"third_party/README.md: no row for {name}")
            continue
        if row["version"] != version:
            errors.append(f"third_party/README.md: {name} is {version}, the table says {row['version']}")
        upstream = next((u for r in registry if os.path.isdir(u := os.path.join(r, f"{name}-{version}"))), None)
        if upstream is None:
            continue  # no upstream copy here to diff against
        for rel in changed(upstream, ours):
            # a new folder's files may be named by the folder (`src/platform/ios/`)
            named = set(re.findall(r"`([^`]+)`", row["files"]))
            if not named & {rel, *(rel[: i + 1] for i, c in enumerate(rel) if c == "/")}:
                errors.append(f"third_party/README.md: {name}'s {rel} differs from upstream but isn't listed")
            if not MARKER.search(open(os.path.join(ours, rel), encoding="utf-8", errors="replace").read()):
                errors.append(f"third_party/{name}/{rel}: differs from upstream, has no patch marker (HST (<task>): …)")
    if errors:
        print("PATCHES FAILED:")
        print("\n".join(errors))
        sys.exit(1)


main()
