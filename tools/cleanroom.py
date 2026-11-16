#!/usr/bin/env python3
"""`[plan] brief_extra` (runner.toml): the clean-room phase of a task whose journal holds a SPEC.md (PLAN.md
*Clean-room ports*). Open questions left → a spec phase; none → a port phase, and in a worktree it writes the
.cleanroom lock tools/guard_decomp.py reads (done.sh --spec clears it). Prints nothing for any other task."""
import os, re, subprocess, sys

tid = sys.argv[1]
root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
jr = os.path.join(root, 'research/journal')
own = sorted(d for d in os.listdir(jr) if re.search(rf'-{re.escape(tid.lower())}(-|$)', d.lower()))
specs = [f'research/journal/{d}/SPEC.md' for d in own if os.path.exists(os.path.join(jr, d, 'SPEC.md'))]
port = False
if specs:
    body = open(os.path.join(root, specs[-1]), encoding='utf-8').read()
    asked = re.search(r'^## Open questions\s*\n(.*?)(?=^## |\Z)', body, re.M | re.S)
    if asked and re.sub(r'^[\s-]*none\.?\s*$', '', asked.group(1), flags=re.M | re.I).strip():
        print(f'## Spec phase: answer the open questions in {specs[-1]}\n'
              'Read the decompile, answer them in the spec (behaviour in your own words), empty the section, '
              f'`tools/done.sh {tid} "<msg>" --spec`, stop. No Rust this session.')
    else:
        port = True
        print(f'## Port phase: implement from {specs[-1]} and the tests alone\n'
              'The decompile is locked for this session (fn.sh, xref.py, context/decomp, Ghidra). Prove the port '
              "with the PINE recordings and the oracle as usual. Something the spec doesn't say: write it under "
              f'`## Open questions` in the spec, `tools/done.sh {tid} "<msg>" --spec`, stop.\n\n{body.rstrip()}')
git = lambda f: subprocess.run(['git', 'rev-parse', '--path-format=absolute', f], cwd=root,
                               capture_output=True, text=True).stdout.strip()
if git('--git-dir') != git('--git-common-dir'):  # a slot's worktree, never the shared main checkout
    lock = os.path.join(root, '.cleanroom')
    if port:
        open(lock, 'w').write(tid + '\n')
    elif os.path.exists(lock):
        os.remove(lock)
