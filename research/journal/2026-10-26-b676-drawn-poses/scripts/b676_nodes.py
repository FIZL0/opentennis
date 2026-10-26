#!/usr/bin/env python3
"""B676: dump each player's posed nodes (the game's 0x120-byte node array) beside its motion player, every frame.

Usage: b676_nodes.py <slot> <frames> <out.bin> [stride]. Loads the save state, then every `stride`-th frame writes
per player: u32 vsync, then for players 0..3: u32 character, motion, sampled time (f32), time, fade fields (anim
object +0x00..+0x80 raw), node count, then per node: local quat (4 f32), local pos (4 f32), world matrix (16 f32).
Node 0 is found by walking back from the node at player+0x1808 to the one whose index is 0.
"""
import struct, sys, time
sys.path.insert(0, __import__("os").path.join(__import__("os").path.dirname(__file__), "..", "tools"))
from pine import Pine

VSYNC, GM_PTR, CHARS = 0x1d5780, 0x422f80, 0x422fa8
p = Pine()
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
stride = int(sys.argv[4]) if len(sys.argv) > 4 else 1
p.load_state(slot)
time.sleep(0.5)

def nodes(pl):
    n = p.read32(pl + 0x1808)
    for back in range(0, 8):
        if p.read32(n - back * 0x120 + 0x10c) == 0:
            base = n - back * 0x120
            break
    else:
        return 0, 0
    k = 0
    while k < 80 and p.read32(base + k * 0x120 + 0x10c) == k:
        k += 1
    return base, k

layout = None
last, got = p.read32(VSYNC), 0
while got < want:
    v = p.next_frame(last)
    last = v
    if v % stride:
        continue
    gm = p.read32(GM_PTR)
    if layout is None:
        layout = []
        for i in range(4):
            pl = p.read32(gm + 0xa8 + 4 * i)
            base, k = nodes(pl)
            layout.append((pl, p.read32(pl + 0x54), base, k, p.read32(CHARS + 4 * i)))
        print("layout", [(hex(a), c, k) for a, _, _, k, c in layout], flush=True)
    regions = []
    for pl, an, base, k, c in layout:
        regions += [(an, 0x80)] + [(base + j * 0x120, 0x100) for j in range(k)]
    a = p.settle(regions, v)
    if a is None:
        continue
    rec = struct.pack("<I", v)
    off = 0
    for pl, an, base, k, c in layout:
        rec += struct.pack("<II", c, k) + a[off:off + 0x80]
        off += 0x80
        for j in range(k):
            nd = a[off:off + 0x100]
            off += 0x100
            rec += nd[0xe0:0x100] + nd[0:0x40]
    out.write(rec)
    got += 1
print("done", got)
