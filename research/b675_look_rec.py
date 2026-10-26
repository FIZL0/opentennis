#!/usr/bin/env python3
"""B675: sample the players' head look-at state from the running game, one line per frame seen (JSON).
Per player: motion, motion time, clip length, look latch, look flags/angles/counter, frames since contact, stroke
branch, z, and the head node's local matrix; globals: match phase, shots this rally, server, the marker's hold.
usage: b675_look_rec.py <slot> <frames> <out.jsonl>"""
import json, struct, sys, time
sys.path.insert(0, "tools")
from pine import Pine, VSYNC
p = Pine()
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
p.load_state(slot); time.sleep(3)
u = lambda b, o: struct.unpack_from("<I", b, o)[0]
i = lambda b, o: struct.unpack_from("<i", b, o)[0]
f = lambda b, o: struct.unpack_from("<f", b, o)[0]
gm = p.read32(0x422f80)
n_pl = p.read32(0x422fa4)
pl = [p.read32(gm + 0xa8 + 4 * k) for k in range(n_pl)]
mp = [p.read32(a + 0x54) for a in pl]
heads = [p.read32(m + 0x84) for m in mp]
print("players", n_pl, "head==node1", [p.read32(a + 0x17f4) == h for a, h in zip(pl, heads)], file=sys.stderr)
log = open(out, "w")
last, n = p.read32(VSYNC), 0
while n < want:
    v = p.read32(VSYNC)
    if v == last: continue
    last, n = v, n + 1
    g = p.read_block(gm + 0x50, 0x10)
    row = {"v": v, "phase": g[5], "shots": i(p.read_block(0x423060, 4), 0), "server": i(p.read_block(0x42304c, 4), 0), "p": []}
    mk = p.read32(pl[0] + 0x1400)
    mb = p.read_block(mk + 0x1f0, 0x10)
    row["mark"] = [mb[4], i(mb, 8)]
    for k in range(n_pl):
        a = p.read_block(pl[k] + 0x3d70, 0x10); b = p.read_block(pl[k] + 0x3ec0, 0x50); c = p.read_block(pl[k] + 0x4050, 4)
        m = p.read_block(mp[k], 0xc0)
        clip = u(m, 0x24); ln = f(p.read_block(clip + 0x2c, 4), 0) if clip else 0.0
        h = p.read_block(heads[k] + 0x80, 0x40)
        row["p"].append({"m": i(m, 0x20), "t": f(m, 0x38), "len": ln, "latch": c[0], "flags": list(m[0xa0:0xa3]),
                         "ang": [f(m, o) for o in range(0xa4, 0xbc, 4)], "cnt": i(m, 0xbc), "since": i(b, 0x40),
                         "branch": b[1], "z": f(a, 8), "head": [round(f(h, o), 6) for o in range(0, 0x30, 4)]})
    log.write(json.dumps(row) + "\n")
