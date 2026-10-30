#!/usr/bin/env python3
"""Summarise WILLSERVE lines: a temporary eprintln at the port's serve contact plus an all-CPU singles test (not in
git; research/journal/2026-10-29-will-curve-serve/README-READY.md says how to add them). Per server: serve number,
toss, button kind and kind after the stick, bent or not. Usage: will_curve_port.py <log> [player, default p1]"""
import re, sys, collections
want = sys.argv[2] if len(sys.argv) > 2 else "p1"
rows = [l for l in open(sys.argv[1]) if f"WILLSERVE {want} " in l]
c = collections.Counter()
for l in rows:
    toss = re.search(r"toss (\w+)", l)[1]; kind = re.search(r" kind (\d+)", l)[1]; pre = re.search(r" pre (\d+)", l)[1]
    bent = "bent true" in l; faults = re.search(r"faults (\d)", l)[1]
    c[(faults, toss, pre, kind, bent)] += 1
n = len(rows)
print(f"{n} serves by {want}")
for k, v in sorted(c.items()): print(f"  serve {int(k[0]) + 1} toss {k[1]:6} button kind {k[2]} -> kind {k[3]} bent {k[4]!s:5}: {v}")
first = [l for l in rows if "faults 0" in l]
for lab, sub in (("all", rows), ("first", first)):
    k = collections.Counter(re.search(r" kind (\d+)", l)[1] for l in sub)
    b = sum("bent true" in l for l in sub)
    print(f"{lab}: {len(sub)} serves, kinds {dict(k)}, bent {b} ({100 * b / max(len(sub), 1):.0f} %)")
