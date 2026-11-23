#!/usr/bin/env python3
"""M183a1 part 11: record the European disc's rand() state (newlib's 64-bit _rand_next, found in eu-bot-start_ee.bin)
and the match phase/tick every frame from a save-state load, to count the intro's rand() draws at 50 Hz (copy
fixtures/m169/eu-bot-start.p2s into the copy's sstates as slot 8 first). Usage: eu_rand_rec.py <slot> <frames> <out.txt>.
Run under tools/pcsx2.sh. Line = vsync, rand state (hex), phase, sub-phase, phase tick (0 0 0 before the match object)."""
import sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
import pine
from pine import Pine

VSYNC, GM_PTR, RAND = 0x1ea480, 0x437e00, 0x1b81a8
pine.VSYNC = VSYNC
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "w")
p = Pine(step=True)
assert p.game_id() == "SCES-54535", p.game_id()
for _ in range(20):
    p.load_state(slot)
    t = time.monotonic()
    while time.monotonic() - t < 3 and p.read32(VSYNC) < 30000: time.sleep(0.05)
    if p.read32(VSYNC) >= 30000: break
else: sys.exit("LOAD FAILED: vsync never reached the state's")
last = p.read32(VSYNC)
for _ in range(want):
    v = last = p.next_frame(last)
    r = p.read64(RAND)
    gm = p.read32(GM_PTR)
    ph = (p.read8(gm + 0x55), p.read8(gm + 0x56), p.read32(gm + 0x58)) if 0x100000 <= gm < 0x2000000 else (0, 0, 0)
    if p.read32(VSYNC) != v: r = "torn"
    else: r = f"{r:016x}"
    out.write(f"{v} {r} {ph[0]} {ph[1]} {ph[2]}\n")
out.close()
print("done")
