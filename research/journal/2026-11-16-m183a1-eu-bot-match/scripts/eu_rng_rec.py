#!/usr/bin/env python3
"""M183a1: record the European disc's match generators every frame from a save-state load (copy
fixtures/m169/eu-bot-start.p2s into the copy's sstates as slot 8 first). Usage: eu_rng_rec.py <slot> <frames> <out.bin>.
Run under tools/pcsx2.sh. Sample = u32 vsync, then three MT19937 blocks of 0x9d0 (state at +4, index at +0x9c4): the
match's shared one (*(gm+0x80)), the AI's and the court's (*(*(*(gm+0x84)+0x154)+0x50)). The European addresses are the
US ones moved as research/README.md says (match globals +0x14e80; the AI generator found seeded 1 in
eu-bot-start_ee.bin). A frame that ticks mid-read is skipped."""
import struct, sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
import pine
from pine import Pine

VSYNC, GM_PTR, AI_MT = 0x1ea480, 0x437e00, 0x43bfb0
pine.VSYNC = VSYNC
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
p = Pine(step=True)
assert p.game_id() == "SCES-54535", p.game_id()
p.load_state(slot)
time.sleep(0.3)
last, n, missed, regions = p.read32(VSYNC), 0, 0, None
while n < want:
    v = last = p.next_frame(last)
    if regions is None:
        # the match object appears during the loading screen: wait for its generators
        gm = p.read32(GM_PTR)
        if not gm: continue
        try:
            mts = [p.read32(gm + 0x80), AI_MT, p.read32(p.read32(p.read32(gm + 0x84) + 0x154) + 0x50)]
        except Exception:
            continue
        if not all(0x100000 <= a < 0x2000000 for a in mts): continue
        print("generators", [hex(a) for a in mts], "from vsync", v, flush=True)
        regions = [(a, 0x9d0) for a in mts]
    a = p.settle(regions, v)
    if a is None:
        missed += 1
        if missed > 300: sys.exit(f"CAPTURE FAILED: 300 frames in a row missed at vsync {v}, {n} samples")
        continue
    missed = 0
    out.write(struct.pack("<I", v) + a)
    n += 1
print("done", n)
