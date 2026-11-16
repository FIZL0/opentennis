#!/usr/bin/env python3
"""Record each player's motion player (anim object *(player+0x54)) every frame from a save-state load (slot 5).
Usage: record_anim.py <slot> <frames> <out.bin>. A PCSX2 copy runs at 1x; the user's own PCSX2: run it slowed (NominalScalar 0.25).
Sample = u32 vsync + gm 0x100 + per player (4): player +0x3c00 0x400, anim object 0x80, its clip's length (f32, clip +0x2c).
Any release in rec.DISCS (US, European 50 Hz): same layout, from the running program's addresses."""
import struct, sys, time
from pine import Pine
from rec import disc

p, want, out = Pine(step=True), int(sys.argv[2]), open(sys.argv[3], "wb")
d = disc(p)
VSYNC, GM_PTR = d["vsync"], d["globals"]
p.load_state(int(sys.argv[1]))
time.sleep(0.3)

def regions():
    gm = p.read32(GM_PTR)
    r = [(gm, 0x100)]
    for i in range(4):
        pl = p.read32(gm + 0xa8 + 4 * i)
        a = p.read32(pl + 0x54)
        r += [(pl + 0x3c00, 0x400), (a, 0x80), (p.read32(a + 0x24) + 0x2c, 4)]
    return gm, r

last, n = p.read32(VSYNC), 0
while n < want:
    v = p.next_frame(last)
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    gm, r = regions()
    a = p.settle(r, v)
    if p.read32(GM_PTR) != gm: sys.exit(f"match object gone at vsync {v}, {n} samples")
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    # PINE reads whole 8-byte words: keep the clip length's own 4 (the sample layout above, as in anim_s05.bin)
    a = a[:0x100] + b"".join(a[o:o + 0x484] for o in range(0x100, len(a), 0x488))
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
