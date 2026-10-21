#!/usr/bin/env python3
"""P117 probe: per frame, the match object's per-player timing copies (the court pull's special gate reads them)
next to each player's own lock grade / offset / result-ready flag, and the last-hitter index. Prints a line
whenever any of them changes. Usage: p117_latch.py <slot> <frames>"""
import struct, sys, time
import os; sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
from pine import Pine

VSYNC, GM_PTR, HITTER, NPL = 0x1d5780, 0x422f80, 0x423058, 0x422fa4
i32 = lambda u: struct.unpack("<i", struct.pack("<I", u))[0]
p, want = Pine(step=True), int(sys.argv[2])
p.load_state(int(sys.argv[1]))
time.sleep(2)
gm = p.read32(GM_PTR)
mo = p.read32(gm + 0xb8)
players = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
print(f"gm {gm:x} match obj {mo:x} players {[hex(x) for x in players]} n {p.read32(NPL)}", flush=True)
last, prev = p.read32(VSYNC), None
for _ in range(want):
    v = p.next_frame(last); last = v
    lat = [(p.read32(mo + 0x10f24 + 4 * i), i32(p.read32(mo + 0x10f34 + 4 * i))) for i in range(4)]
    pl = [(p.read8(b + 0x3ee8), i32(p.read32(b + 0x3fa0)), p.read8(b + 0x3f97), i32(p.read32(b + 0x3ec4))) for b in players]
    st = (p.read32(HITTER), lat, [x[:3] for x in pl])
    if st != prev:
        print(v, "hit", st[0], "latch", lat, "pl(grade,off,ready,cd)", pl, flush=True)
        prev = st
