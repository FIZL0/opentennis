#!/usr/bin/env python3
"""B784: log every rand() call (newlib's rand, at the same address on the European and US discs) over a vsync window
from a European (or US) save-state load: per call the vsync, ra, sp and 16 stack words from the caller's sp (its frame, so
the callers' return addresses). Usage (the window after the state's vsync): eu_rand_calls.py <slot> <from vsync> <to vsync> <out.bin>. Run under
tools/pcsx2.sh. The hook goes in once the load passes <from>; record = 0x50 bytes: u32 vsync, ra, sp, 0, 16 words."""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
import pine
from pine import Pine
from rec import lw, sw, lui, addiu, sltu, j, li, br, assemble

HOOK = 0x11f840
# per disc: the vsync counter and the lowest vsync its state could load at (above a fresh boot's)
DISCS = {"SCES-54535": (0x1ea480, 30000), "SCUS-97610": (0x1d5780, 7000)}  # the European and the US disc
CODE, DATA, BUF, END = 0x1e02000, 0x1e00000, 0x1e30000, 0x1f80000
D = DATA & 0xffff
slot, lo, hi, out = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]), open(sys.argv[4], "wb")
p = Pine()
VSYNC, low = DISCS[p.game_id()]
landed = lambda: low <= p.read32(VSYNC) < lo  # the state's vsync, before the window (a late-landing load)
pine.VSYNC = VSYNC
for _ in range(20):
    p.load_state(slot)
    t = time.monotonic()
    while time.monotonic() - t < 3 and not landed(): time.sleep(0.05)
    if landed(): break
else: sys.exit("LOAD FAILED")
a, b = p.read32(HOOK), p.read32(HOOK + 4)
assert a != j(CODE), 'the hook is already in RAM: restart PCSX2 and reload'
prog = [lui("t4", DATA >> 16),
        lw("t6", D, "t4"), *li("t7", END - 0x100), sltu("t7", "t6", "t7"), br(4, "t7", "zero", "skip"), 0,
        *li("t9", VSYNC), lw("t8", 0, "t9"), sw("t8", 0, "t6"), sw("ra", 4, "t6"), sw("sp", 8, "t6"), sw("zero", 12, "t6")]
for k in range(16): prog += [lw("t8", 4 * k, "sp"), sw("t8", 16 + 4 * k, "t6")]
prog += [addiu("t6", "t6", 0x50), sw("t6", D, "t4"), "skip", a, b, j(HOOK + 8), 0]
words = assemble(prog)
while p.read32(VSYNC) < lo - 100: time.sleep(0.2)
while p.read32(VSYNC) < lo: time.sleep(0.01)
p.pause()
p.write32(DATA, BUF)
for n, w in enumerate(words): p.write32(CODE + 4 * n, w)
p.write32(HOOK, j(CODE)); p.write32(HOOK + 4, 0)
p.resume()
pos = BUF
def drain():
    global pos
    end = p.read32(DATA)
    data = b"".join(p.read_block(x, min(0x1000, end - x + 7 & ~7)) for x in range(pos, end, 0x1000))[: end - pos]
    pos = end
    return data
got = 0
while p.read32(VSYNC) < hi:
    time.sleep(0.3)
    d = drain(); out.write(d); got += len(d) // 0x50
p.pause(); p.write32(HOOK, a); p.write32(HOOK + 4, b); p.resume()
d = drain(); out.write(d); got += len(d) // 0x50
print("calls", got)
