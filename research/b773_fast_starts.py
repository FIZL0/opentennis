#!/usr/bin/env python3
"""B773: every motion start in a record_anim.py file whose start speed isn't 1: the time stepped on its first three
ticks and the speed after each, per player. Usage: fast_starts.py <file>"""
import struct, sys
d = open(sys.argv[1], "rb").read()
REC = 4 + 0x100 + 4 * 0x484
rows = []
for k in range(0, len(d) - REC + 1, REC):
    v = struct.unpack_from("<I", d, k)[0]; gm = k + 4
    rows.append((v, d[gm + 0x55], struct.unpack_from("<I", d, gm + 0x58)[0],
                 [struct.unpack_from("<I3f", d, gm + 0x100 + P * 0x484 + 0x400 + 0x20) for P in range(4)],
                 [struct.unpack_from("<3f", d, gm + 0x100 + P * 0x484 + 0x400 + 0x38) for P in range(4)]))
from collections import Counter
c = Counter()
for n in range(1, len(rows) - 3):
    for P in range(4):
        samp, t, spd = rows[n][4][P]
        if samp == 0 and spd != 1 and rows[n][3][P][0] != rows[n - 1][3][P][0] and all(rows[n + j][0] == rows[n][0] + j for j in range(4)):
            steps = tuple(round(rows[n + j][4][P][1] - rows[n + j - 1][4][P][1], 3) if j else round(t, 3) for j in range(3))
            spds = tuple(round(rows[n + j][4][P][2], 3) for j in range(3))
            c[(round(spd, 3), steps, spds)] += 1
for k, v in sorted(c.items()): print(v, "start speed", k[0], "steps", k[1], "speed after", k[2])
