#!/usr/bin/env python3
"""M183a1: slot 5's generators every frame, as P3b's p3b_rng_rec.py records them (same sample layout: u32 vsync, u64
rand() state, then the shared, AI, court and sound MT19937 blocks), but waiting for the state load to land (vsync past
`min_vsync`, the match object's generators valid) before it reads their addresses, so it can run long.
Usage: us_rng_rec.py <slot> <frames> <out.bin> [min_vsync=7000]. Run under tools/pcsx2.sh."""
import struct, sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
from pine import Pine

VSYNC, GM_PTR, AI_MT, REENT, SND = 0x1d5780, 0x422f80, 0x427130, 0x1b80f0, 0x43b1d0
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
min_vsync = int(sys.argv[4]) if len(sys.argv) > 4 else 7000
p = Pine(step=True)
p.load_state(slot)
last, regions = p.read32(VSYNC), None
for _ in range(600):
    last = p.next_frame(last)
    if last < min_vsync: continue
    try:
        gm = p.read32(GM_PTR)
        mts = [p.read32(gm + 0x80), AI_MT, p.read32(p.read32(p.read32(gm + 0x84) + 0x154) + 0x50), p.read32(p.read32(SND) + 0x740)]
    except Exception:
        continue
    if all(0x100000 <= a < 0x2000000 for a in mts):
        regions = [(p.read32(REENT) + 0xa8, 8)] + [(a, 0x9d0) for a in mts]
        break
if regions is None: sys.exit("CAPTURE FAILED: the state load never landed")
print("generators", [hex(a) for _, a in [(0, x) for x in mts]], "from vsync", last, flush=True)
n, missed = 0, 0
while n < want:
    v = last = p.next_frame(last)
    a = p.settle(regions, v)
    if a is None:
        missed += 1
        if missed > 300: sys.exit(f"CAPTURE FAILED: 300 frames in a row missed at vsync {v}, {n} samples")
        continue
    missed = 0
    out.write(struct.pack("<I", v) + a)
    n += 1
print("done", n)
