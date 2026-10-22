#!/usr/bin/env python3
"""Delete what `context/` holds that nothing needs any more.

Usage: tools/context_prune.py [--delete | --empty] [--days N] [--cheap BYTES] [-v]

Without --delete it only reports what it would delete. --delete moves those files into `context/.trash/<date>/`
(a rename, instant, same layout), so a test run can prove nothing needed went; a mistake goes back with `cp -al`
or `mv`. --empty then deletes the trash for good. A file in the main checkout's `context/` is kept when:

  live        a tracked file outside the journals (code, tests, tools, research scripts, AGENT/PLAN/REFERENCE, TODO)
              names it, or names its folder, or a glob/`{name}` path into its folder: the whole folder is kept,
              since code often names one file and reads its neighbours by bare name
  open task   it sits under a folder named after an open task (`context/b646/…`), or an open task's journal or
              plan file names it
  cheap       a finished task's journal (or DONE.md) names it and it's at most --cheap bytes (default 1 MB):
              the journal's evidence stays readable, big captures behind finished work go
  recent      it was written in the last --days days (default 3): work in flight nobody has written up yet

Everything else goes, then any folder left empty. Symlinks are never followed. The disc-derived setup
(`xb`, `iso`, `decomp`, `elf`, Ghidra, save states) is kept because the tools and research/README.md name it;
anything deleted by mistake that came from the disc is rebuilt by research/README.md, a recording only by
recording it again.
"""
import os, re, subprocess, sys, time

ROOT = os.path.realpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
REF = re.compile(r"context/[A-Za-z0-9_.{}*$/-]+")


def git(*a):
    return subprocess.run(["git", "-C", ROOT, *a], capture_output=True, text=True).stdout


def open_ids():
    out = git("show", "HEAD:plan/TODO.md")
    return {m.group(1).lower() for m in re.finditer(r"^\s*- \[[ ~]\] \*\*([A-Za-z0-9]+)\*\*", out, re.M)}


def refs(paths):
    """Every `context/…` path the files name, cut at the first glob or placeholder."""
    out = set()
    for p in paths:
        try:
            text = open(os.path.join(ROOT, p), encoding="utf-8", errors="ignore").read()
        except OSError:
            continue
        for m in REF.finditer(text):
            r = re.split(r"[{}*$]", m.group())[0].rstrip("./-")
            if r.count("/") >= 1 and r != "context":
                out.add(r)
    return out


def main(argv):
    delete, verbose = "--delete" in argv, "-v" in argv
    days = float(argv[argv.index("--days") + 1]) if "--days" in argv else 3
    cheap = int(argv[argv.index("--cheap") + 1]) if "--cheap" in argv else 1 << 20
    ctx = os.path.join(ROOT, "context")
    trash = os.path.join(ctx, ".trash")
    if "--empty" in argv:
        if os.path.isdir(trash):
            subprocess.run(["rm", "-rf", "--", trash], check=True)
        return print("trash emptied")
    ids = open_ids()

    tracked = [p for p in git("ls-files").splitlines() if not p.startswith(("third_party/", "context/"))]
    def open_journal(p):
        slug = p.split("/")[2] if p.count("/") >= 3 else ""
        return any(slug[11:].lower().startswith(i + "-") or slug[11:].lower() == i for i in ids)
    journal = [p for p in tracked if p.startswith("research/journal/") or p == "plan/DONE.md"]
    live_src = [p for p in tracked if p not in set(journal) or open_journal(p)]
    live, past = refs(live_src), refs(journal)

    # a live reference keeps its whole folder: a file's parent, or the folder itself
    keep_dirs = set()
    for r in live:
        full = os.path.join(ROOT, r)
        keep_dirs.add(r if os.path.isdir(full) else os.path.dirname(r))
    keep_dirs.discard("context")

    cutoff = time.time() - days * 86400
    gone, kept, why = 0, 0, {}
    doomed = []
    for top, dirs, files in os.walk(ctx):
        dirs[:] = [d for d in dirs if not os.path.islink(os.path.join(top, d)) and os.path.join(top, d) != trash]
        rel_top = os.path.relpath(top, ROOT)
        for f in files:
            full = os.path.join(top, f)
            if os.path.islink(full):
                continue
            rel = os.path.join(rel_top, f)
            st = os.lstat(full)
            first = rel.split("/")[1] if rel.count("/") >= 2 else ""
            if any(rel == d or rel.startswith(d + "/") for d in keep_dirs) or rel in live:
                reason = "live"
            elif first.lower() in ids or any(first.lower().startswith(i) and not first[len(i):len(i) + 1].isalnum() for i in ids):
                reason = "open task"
            elif rel in past and st.st_size <= cheap:
                reason = "cheap"
            elif st.st_mtime >= cutoff:
                reason = "recent"
            else:
                reason = None
            if reason:
                kept += st.st_blocks * 512
                why[reason] = why.get(reason, 0) + st.st_blocks * 512
            else:
                gone += st.st_blocks * 512
                doomed.append((rel, st.st_blocks * 512))

    by_top = {}
    for rel, n in doomed:
        k = "/".join(rel.split("/")[:2])
        by_top[k] = by_top.get(k, 0) + n
    gb = lambda n: f"{n / 1e9:.2f} GB"
    for k, n in sorted(by_top.items(), key=lambda kv: -kv[1])[: None if verbose else 25]:
        print(f"  {gb(n):>10}  {k}")
    print(f"{'moved to context/.trash' if delete else 'would delete'} {len(doomed)} files, {gb(gone)}; keeping {gb(kept)} "
          f"({', '.join(f'{k} {gb(v)}' for k, v in sorted(why.items()))})")
    if delete:
        bin_ = os.path.join(trash, time.strftime("%Y-%m-%d-%H%M%S"))
        for rel, _ in doomed:
            dst = os.path.join(bin_, os.path.relpath(rel, "context"))
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            os.rename(os.path.join(ROOT, rel), dst)
        for top, dirs, files in os.walk(ctx, topdown=False):
            if top != ctx and not top.startswith(trash) and not os.path.islink(top) and not os.listdir(top):
                os.rmdir(top)


if __name__ == "__main__":
    main(sys.argv[1:])
