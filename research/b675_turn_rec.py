#!/usr/bin/env python3
"""B675 port: bit-exact head-look inputs and outputs, every frame (binary, settled reads).

usage: b675_turn_rec.py <slot> <frames> <out.bin>
Record: u32 vsync, gm+0x50 (0x10), globals 0x423040 (0x30), live ball +0xe0 (0x10), marker +0x1f0 (0x10), then per
player (4): motion player (0xc0), head node (0x120), the node before it (0x120), player +0x3d70 (0x10), +0x3ec0
(0x50), +0x4050 (8). A node: world matrix +0x00, local matrix +0x80, sampled quat/pos +0xe0, index +0x10c."""
import struct, sys, time
sys.path.insert(0, "tools")
from pine import Pine, VSYNC
p = Pine()
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(slot); time.sleep(3)
gm = p.read32(0x422f80)
pl = [p.read32(gm + 0xa8 + 4 * k) for k in range(4)]
mp = [p.read32(a + 0x54) for a in pl]
heads = [p.read32(m + 0x84) for m in mp]
ball = p.read32(gm + 0x88)
mk = p.read32(pl[0] + 0x1400)
print("head index", [p.read32(h + 0x10c) for h in heads], "neck index", [p.read32(h - 0x120 + 0x10c) for h in heads], file=sys.stderr)
regions = [(gm + 0x50, 0x10), (0x423040, 0x30), (ball + 0xe0, 0x10), (mk + 0x1f0, 0x10)]
for k in range(4):
    regions += [(mp[k], 0xc0), (heads[k], 0x120), (heads[k] - 0x120, 0x120), (pl[k] + 0x3d70, 0x10), (pl[k] + 0x3ec0, 0x50), (pl[k] + 0x4050, 8)]
last, got, miss = p.read32(VSYNC), 0, 0
while got < want:
    v = p.next_frame(last, 0.002)
    last = v
    a = p.settle(regions, v)
    if a is None:
        miss += 1
        continue
    out.write(struct.pack("<I", v) + a)
    got += 1
print("done", got, "missed", miss, file=sys.stderr)
