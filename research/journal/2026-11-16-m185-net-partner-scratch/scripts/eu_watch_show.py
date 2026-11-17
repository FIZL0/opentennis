#!/usr/bin/env python3
"""M185: print eu_watch.py's records around the dispatcher calls of one player.
Usage: eu_watch_show.py <file> <dispatcher hook> <player slot> <skip> <count> [before]"""
import struct, sys
d = open(sys.argv[1], "rb").read()
disp, want, skip, count = int(sys.argv[2], 16), int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5])
before = int(sys.argv[6]) if len(sys.argv) > 6 else 8
recs = [struct.unpack_from("<4I4f", d, k) for k in range(0, len(d), 0x20)]
slot_of = {}
seen = 0
for n, r in enumerate(recs):
    h, ra, v, a0 = r[:4]
    if h != disp: continue
    seen += 1
    if seen <= skip: continue
    print("----")
    for m in range(max(0, n - before), n + 1):
        h, ra, v, a0, *q = recs[m]
        print(f"{v} {h:x} ra={ra:x} a0={a0:x} q=({q[0]:.4f} {q[1]:.4f} {q[2]:.4f} {q[3]:.4f})")
    count -= 1
    if not count: break
