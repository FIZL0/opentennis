#!/usr/bin/env python3
"""Carry a worktree's `context/` files into the main checkout's `context/` (T11).

Usage: tools/context_sync.py [--all] [--since EPOCH] [-q]

A runner slot or `git worktree add` checkout often has its own `context/` directory, not the symlink to the main
checkout's that the runner makes for new slots. A recording made there stays there, and deleting the worktree loses
it. This copies every file the main checkout's `context/` lacks into it. On btrfs the copy is a reflink and takes
no extra space. An existing file is never overwritten. When two worktrees hold different files under the same new
path, the first one keeps the path and the other is kept beside it as `<name>.from-<worktree>`.

  (default)      this worktree's `context/`
  --all          every worktree of the repo (the backlog: run it from any checkout)
  --since EPOCH  this worktree's files written at or after EPOCH also replace the main checkout's copy: pine.py
                 passes its start time when a tool exits cleanly, so a re-recording under an old name reaches main
  -q             print nothing when nothing was copied

tools/done.sh runs the default form at the end of a task, and pine.py runs `--since` when a PINE tool exits. Tests
read through `hst_data::fixture`, which already falls back to the main checkout's (or any worktree's) `context/`
for a file this checkout lacks.
"""
import fcntl, filecmp, os, shutil, subprocess, sys

ROOT = os.path.realpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
FICLONE = 0x40049409  # linux/fs.h: share the source's extents (btrfs, xfs); falls back to a byte copy


def git(*a, cwd=ROOT):
    return subprocess.run(["git", *a], cwd=cwd, capture_output=True, text=True, check=True).stdout


def main_checkout():
    common = os.path.realpath(os.path.join(ROOT, git("rev-parse", "--git-common-dir").strip()))
    return os.path.dirname(common)


def worktrees():
    return [l[9:] for l in git("worktree", "list", "--porcelain").splitlines() if l.startswith("worktree ")]


def clone(src, dst, replace=False):
    """Copy src to dst (reflink when the filesystem can), keeping its times. Without `replace`, an existing dst is
    left alone and False is returned."""
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    if os.path.islink(src):
        if os.path.lexists(dst) and not replace:
            return False
        tmp = dst + ".sync-tmp"
        if os.path.lexists(tmp):
            os.unlink(tmp)
        os.symlink(os.readlink(src), tmp)
        os.replace(tmp, dst)
        return True
    tmp = dst + ".sync-tmp" if replace else dst
    try:
        fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_EXCL | (os.O_TRUNC if replace else 0), 0o644)
    except FileExistsError:
        if not replace:
            return False
        os.unlink(tmp)
        fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    try:
        with open(src, "rb") as s, os.fdopen(fd, "wb") as d:
            try:
                fcntl.ioctl(d.fileno(), FICLONE, s.fileno())
            except OSError:
                shutil.copyfileobj(s, d, 1 << 20)
        shutil.copystat(src, tmp)
    except BaseException:
        os.unlink(tmp)
        raise
    if replace:
        os.replace(tmp, dst)
    return True


def files(ctx):
    for d, dirs, names in os.walk(ctx):
        dirs.sort()
        for n in sorted(names):
            if not n.endswith(".sync-tmp"):
                yield os.path.join(d, n)
        for n in dirs:  # os.walk doesn't descend into directory symlinks; carry them as links
            if os.path.islink(os.path.join(d, n)):
                yield os.path.join(d, n)


def same(a, b):
    if os.path.islink(a) or os.path.islink(b):
        return os.path.islink(a) and os.path.islink(b) and os.readlink(a) == os.readlink(b)
    return filecmp.cmp(a, b, shallow=False)


def main():
    args = sys.argv[1:]
    quiet = "-q" in args
    since = float(args[args.index("--since") + 1]) if "--since" in args else None
    top = main_checkout()
    dest = os.path.join(top, "context")
    sources = [w for w in worktrees() if os.path.realpath(w) != os.path.realpath(top)] if "--all" in args else [ROOT]
    made = {}  # rel path -> worktree that created it in this run
    new = size = kept = fresh = 0
    failed = []
    for wt in sources:
        ctx = os.path.join(wt, "context")
        if not os.path.isdir(ctx) or os.path.realpath(ctx) == os.path.realpath(dest):
            continue
        name = os.path.basename(wt.rstrip("/"))
        for src in files(ctx):
            rel = os.path.relpath(src, ctx)
            dst = os.path.join(dest, rel)
            try:
                if not os.path.lexists(dst):
                    if clone(src, dst):
                        made[rel] = name
                        new += 1
                        size += os.lstat(src).st_size
                elif since is not None and not os.path.islink(src) and os.stat(src).st_mtime >= since:
                    if not same(src, dst):
                        clone(src, dst, replace=True)
                        fresh += 1
                elif rel in made and made[rel] != name and not same(src, dst):
                    kept += clone(src, f"{dst}.from-{name}")
            except OSError as e:
                failed.append(f"{src}: {e}")
    if failed:
        print(f"context_sync: {len(failed)} file(s) not copied:", *failed[:20], sep="\n  ", file=sys.stderr)
    if new or fresh or kept or not quiet:
        print(f"context_sync: {new} new file(s), {size / 1e9:.2f} GB, into {dest}"
              + (f"; {fresh} re-recorded file(s) replaced" if fresh else "")
              + (f"; {kept} same-name different file(s) kept as *.from-<worktree>" if kept else ""))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
