#!/usr/bin/env python3
"""B782: where the European match's weather holder gets its never-written last-weather byte. Loads the EU save state,
runs until the court object and its weather holder exist and notes the holder's address, then reloads the state and
dumps that memory (the heap block before the holder is made) frame by frame until the byte appears. Usage (under
tools/pcsx2.sh, EU disc, current PCSX2 build): b782_held_source.py <slot> <out.txt>."""
import sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
import pine
from pine import Pine

VSYNC, GM_PTR = 0x1ea480, 0x437e00
pine.VSYNC = VSYNC
slot, out = int(sys.argv[1]), open(sys.argv[2], "w")
p = Pine(step=True)
assert p.game_id() == "SCES-54535", p.game_id()
ok = lambda a: 0x100000 <= a < 0x2000000

def load():
    for _ in range(20):
        p.load_state(slot)
        t = time.monotonic()
        while time.monotonic() - t < 3 and p.read32(VSYNC) < 30000: time.sleep(0.05)
        if p.read32(VSYNC) >= 30000: return p.read32(VSYNC)
    sys.exit("LOAD FAILED")

def holder():
    gm = p.read32(GM_PTR)
    c = p.read32(gm + 0x84) if ok(gm) else 0
    w = p.read32(c + 0x158) if ok(c) else 0
    return gm, c, w if ok(w) else 0

last = load()
for _ in range(400):
    last = p.next_frame(last)
    gm, c, w = holder()
    if w and p.read8(w + 0x50) not in (0,): break
out.write(f"found at vsync {last}: gm {gm:x} court {c:x} holder {w:x} byte {p.read8(w + 0x50)}\n")
last = load()
for _ in range(10):
    blk = p.read_block(w - 0x20, 0x90)
    out.write(f"{last} holder-now {holder()[2]:x} mem[-0x20..0x70]: {blk.hex()}\n")
    last = p.next_frame(last)
out.close()
print("done")
