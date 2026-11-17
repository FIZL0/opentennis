#!/usr/bin/env python3
"""M185: check eu_watch.py's records: for every dispatcher call, (a) the quad equals the one at its own player
update's entry, (b) for the player updated second..fourth, it equals the value the previous player's racket-matrix
hand-off spilled (the change logged at the hand-off's matrix call, return address RA_SPILL), (c) for the first player,
it is the walker's position. Usage: eu_watch_check.py <file>"""
import struct, sys, collections
PU, DISP, RA_SPILL = 0x35b530, 0x3e3ad0, 0x366b30
d = open(sys.argv[1], "rb").read()
r = [struct.unpack_from("<4I4I", d, k) for k in range(0, len(d), 0x20)]
stats = collections.Counter()
last_spill, pu_q, nth = None, None, -1
for x in r:
    h, ra, v, a0, q = x[0], x[1], x[2], x[3], x[4:]
    if h == PU:
        nth = 0 if last_spill is None or nth == 3 else nth + 1
        pu_q = q
    elif h == DISP:
        stats["same as at its player update's entry" if q == pu_q else "changed after the player update's entry"] += 1
        if nth > 0: stats["players 2-4: previous player's spill" if q == last_spill else "players 2-4: other"] += 1
        else: stats[f"first player: {struct.unpack('<4f', struct.pack('<4I', *q))}"] += 1
    if ra == RA_SPILL: last_spill = q
print(dict(stats))
