#!/usr/bin/env python3
"""Task-named probes out of research/'s top level (T6): research/<id>_*.py (and .sh, and research/<id>/ folders) are
one-off scripts for one task; once it's done they live in that task's journal, research/journal/<dir>/scripts/.
The top level keeps the reusable tools (fn.sh, xref.py, …).

    tools/probes.py <ID>     move <ID>'s probes (tools/done.sh runs it on tick)
    tools/probes.py --all    move every task-named probe (a probe is <letter><digit>…_name)
    -n                       dry run: print the moves only

The journal: one named for the task (…-<id> or …-<id>-…), else one that mentions the script, else one named for a
parent task (b24d → b24); none → it stays and is listed. A move is a `git mv`, then the scripts' paths are fixed up:
`os.path.join(os.path.dirname(__file__), "..", "tools")`-style paths point at the same place from the new folder, an
import of another probe finds it where it now lives, a shell script's `cd "$(dirname "$0")/.."` goes to the repo
root, and every other tracked text file naming research/<probe> is pointed at the new path."""
import os, re, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RES = os.path.join(ROOT, 'research')
JOUR = os.path.join(RES, 'journal')
PROBE = re.compile(r'^([a-z]\d[0-9a-z]*)_\w+\.(py|sh)$')
TEXT = ('.md', '.py', '.sh', '.rs', '.toml', '.mjs', '.txt')


def probes():
    """{name: task id} for the top level: research/<id>_*.py|sh and research/<id>/ folders a task id names."""
    out = {}
    for n in sorted(os.listdir(RES)):
        if m := PROBE.match(n):
            out[n] = m[1]
        elif os.path.isdir(os.path.join(RES, n)) and re.fullmatch(r'[a-z]\d+[a-z]\w*|[a-z]\d+', n) and n != 'tools':
            out[n] = n
    return out


def journal(name, tid):
    dirs = sorted(os.listdir(JOUR))
    own = lambda i: [d for d in dirs if re.search(rf'-{re.escape(i)}(-|$)', d.lower())]
    def mentions(d):
        for base, _, fs in os.walk(os.path.join(JOUR, d)):
            for f in fs:
                if f.endswith('.md') and name in open(os.path.join(base, f), encoding='utf-8', errors='replace').read():
                    return True
    named = own(tid)
    if named:
        return next((d for d in named if mentions(d)), named[0])
    if ref := [d for d in dirs if mentions(d)]:
        return ref[0]
    for i in range(len(tid) - 1, 1, -1):  # parent task: b24d → b24, p17s2 → p17s → p17
        if named := own(tid[:i]):
            return named[0]
    return None


def git(*a):
    return subprocess.run(['git', '-C', ROOT, *a], check=True, capture_output=True, text=True).stdout


FILE_DIR = r'os\.path\.dirname\((?:os\.path\.abspath\(__file__\)|__file__)\)'
# the join's own closing paren stays where it is: only its first arguments, up to the last plain string, are replaced
JOIN = re.compile(r'os\.path\.join\(' + FILE_DIR + r'((?:\s*,\s*(?:"[^"{}]*"|\'[^\'{}]*\'))*)')
DIRNAME = re.compile(FILE_DIR)
HERE = 'os.path.dirname(os.path.abspath(__file__))'


def fix_py(path, old_dir, where):
    """`path` was in `old_dir`: re-aim its __file__-relative paths and probe imports (`where`: probe module → dir)."""
    src = open(path, encoding='utf-8').read()
    new_dir = os.path.dirname(path)
    def rel(target):
        return os.path.relpath(target, new_dir)
    def join(m):
        parts = re.findall(r'"([^"]*)"|\'([^\']*)\'', m[1])
        target = os.path.normpath(os.path.join(old_dir, *[a or b for a, b in parts]))
        mod = os.path.splitext(os.path.basename(target))[0]
        if os.path.dirname(target) == RES and mod in where:  # another probe, by file
            target = os.path.join(where[mod], os.path.basename(target))
        return f'os.path.join({HERE}, "{rel(target)}"'
    src = JOIN.sub(join, src)
    src = DIRNAME.sub(lambda m: m[0] if m[0] == HERE else f'os.path.join({HERE}, "{rel(old_dir)}")', src)
    def imp(m):  # `from x import …` / `import x, y`: a probe that isn't beside this file gets its folder on sys.path
        mods = [x.strip().split(' as ')[0] for x in (m[2] or m[3]).split(',')]
        dirs = sorted({where[x] for x in mods if x in where and where[x] != new_dir})
        return ''.join(f'{m[1]}import os, sys; sys.path.insert(0, os.path.join({HERE}, "{rel(d)}"))\n'
                       for d in dirs) + m[0]
    src = re.sub(r'^([ \t]*)(?:from (\w+) import\b|import ([\w, ]+?)\s*(?:#|$))[^\n]*$', imp, src, flags=re.M)
    open(path, 'w', encoding='utf-8').write(src)


def fix_sh(path):
    src = open(path, encoding='utf-8').read()
    src = re.sub(r'cd "\$\(dirname "\$0"\)(/\.\.)+"', 'cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"', src)
    open(path, 'w', encoding='utf-8').write(src)


def move(names, dry):
    found = probes()
    plan = {n: journal(n, found[n]) for n in names}
    left = [n for n, j in plan.items() if not j]
    plan = {n: j for n, j in plan.items() if j}
    for n, j in plan.items():
        print(f'research/{n} → research/journal/{j}/scripts/{n}')
    if dry or not plan:
        return left
    # where every probe module will be once this batch moves (the rest stay at the top level)
    where = {os.path.splitext(n)[0]: os.path.join(JOUR, j, 'scripts') for n, j in plan.items() if n.endswith('.py')}
    for n in found:
        where.setdefault(os.path.splitext(n)[0], RES)
    for n, j in plan.items():
        os.makedirs(os.path.join(JOUR, j, 'scripts'), exist_ok=True)
        git('mv', f'research/{n}', f'research/journal/{j}/scripts/{n}')
    for n, j in plan.items():
        new = os.path.join(JOUR, j, 'scripts', n)
        files = [new] if os.path.isfile(new) else [os.path.join(b, f) for b, _, fs in os.walk(new) for f in fs]
        for f in files:
            old_dir = os.path.join(RES, os.path.relpath(os.path.dirname(f), os.path.join(JOUR, j, 'scripts')))
            if f.endswith('.py'):
                fix_py(f, os.path.normpath(old_dir), where)
            elif f.endswith('.sh'):
                fix_sh(f)
    # still-top-level probes that import a moved one
    for n in found:
        if n not in plan and n.endswith('.py'):
            fix_py(os.path.join(RES, n), RES, where)
    # every reference elsewhere
    pat = re.compile(r'(?<![\w/])(research/)(' + '|'.join(re.escape(n) for n in sorted(plan, key=len, reverse=True)) + r')(?![\w.])')
    for f in git('ls-files').splitlines():
        p = os.path.join(ROOT, f)
        if not f.endswith(TEXT) or not os.path.isfile(p):
            continue
        src = open(p, encoding='utf-8', errors='surrogateescape').read()
        new = pat.sub(lambda m: f'research/journal/{plan[m[2]]}/scripts/{m[2]}', src)
        if new != src:
            open(p, 'w', encoding='utf-8', errors='surrogateescape').write(new)
    return left


def main():
    a = [x for x in sys.argv[1:] if x != '-n']
    dry = '-n' in sys.argv[1:]
    if len(a) != 1:
        sys.exit(__doc__)
    found = probes()
    names = list(found) if a[0] == '--all' else [n for n, t in found.items() if t == a[0].lower()]
    left = move(names, dry)
    for n in left:
        print(f'research/{n}: no journal for task {found[n]}; left in place', file=sys.stderr)


if __name__ == '__main__':
    main()
