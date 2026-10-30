#!/usr/bin/env python3
"""B700a: flat share of the CPU's topspin serve swings against the swing's real timing offset, from
research/b700_serve_press.py logs (args). Only CPU serves (a press tuple) with a topspin swing (+0xb2 == 3)."""
import re, sys
from collections import Counter
rows = Counter()
for path in sys.argv[1:]:
    for line in open(path):
        m = re.search(r"press \((\d+), (-?\d+), (-?\d+), (-?\d+), (-?\d+), (\d+)\) toss (-?\d+) offset (-?\d+) grade (\d) depth (-?\d+) side (-?\d+) kind (\d)", line)
        if not m: continue
        v, first, cd, err, _, swing, toss, off, grade, depth, side, kind = map(int, m.groups())
        rows[(swing, off, kind)] += 1
print("swing offset: kinds (0 topspin, 1 slice, 2 flat)")
for sw in sorted({k[0] for k in rows}):
    for off in sorted({k[1] for k in rows if k[0] == sw}, reverse=True):
        ks = {k[2]: n for k, n in rows.items() if k[0] == sw and k[1] == off}
        print(f"  swing {sw} offset {off:+d}: {dict(sorted(ks.items()))}  flat {ks.get(2, 0)}/{sum(ks.values())}")
