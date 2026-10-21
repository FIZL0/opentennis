#!/usr/bin/env python3
"""B40g2: the Offbeat Rules' irregular bounce (rule 1) in the original.
  b40g2_irregular.py <slot> <frames> <out.bin> <rule byte>  load <slot> (5: bot-only match), set the rules word's
      byte <rule byte> (0..3, little-endian byte of the word) to 1, then record the live ball every frame in
      record_live.py's format (u32 vsync + live ball 0x290 + predictor 0x290 + rally block 0x40)."""
import struct, sys, time
sys.path.insert(0, 'tools')
from pine import Pine

VSYNC, GM_PTR, RULES = 0x1d5780, 0x422f80, 0x2ef7e0
p, want, out = Pine(step=True), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(int(sys.argv[1]))
time.sleep(0.3)
byte = int(sys.argv[4])
w = p.read32(RULES)
p.write32(RULES, (w & ~(0xff << 8 * byte)) | (1 << 8 * byte))
print(f"rules word {w:#010x} -> {p.read32(RULES):#010x}", flush=True)
gm = p.read32(GM_PTR)
ball = p.read32(gm + 0x88)
r = [(ball, 0x290), (p.read32(gm + 0x98), 0x290), (0x3165f0, 0x40)]
last, n = p.read32(VSYNC), 0
while n < want:
    v = p.next_frame(last)
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    a = p.settle(r, v)
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
