#!/usr/bin/env python3
"""R14: log every call of the AI generator's draw (EU 0x19f890, the US 0x19f5c0) on the AI's MT (0x43bfb0) from an
EU save-state load: per call the vsync, ra, sp, and 16 stack words (the callers' return addresses). Usage:
eu_draw_calls.py <slot> <vsyncs> <out.bin>. Run under tools/pcsx2.sh with eu-bot-start.p2s as the slot's state.
Record: 0x50 bytes = u32 vsync, ra, sp, a0, 16 stack words from sp. The buffer is drained while running."""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../tools"))
import pine
from pine import Pine
from rec import lw, sw, lui, addiu, sltu, j, li, br, assemble

VSYNC, HOOK, MT = 0x1ea480, 0x19f890, 0x43bfb0
CODE, DATA, BUF, END = 0x1e02000, 0x1e00000, 0x1e30000, 0x1f80000
D = DATA & 0xffff
pine.VSYNC = VSYNC
slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
p = Pine()
assert p.game_id() == "SCES-54535", p.game_id()
p.load_state(slot)
time.sleep(1)
v0 = p.read32(VSYNC)
while p.read32(VSYNC) - v0 < 5: time.sleep(0.05)
a, b = p.read32(HOOK), p.read32(HOOK + 4)
assert a != j(CODE), 'the hook is already in RAM: restart PCSX2 (tools/pcsx2-hst.sh stop) and reload'
prog = [lui("t4", DATA >> 16), *li("t5", MT), br(5, "a0", "t5", "skip"), 0,
        lw("t6", D, "t4"), *li("t7", END - 0x100), sltu("t7", "t6", "t7"), br(4, "t7", "zero", "skip"), 0,
        *li("t9", VSYNC), lw("t8", 0, "t9"), sw("t8", 0, "t6"), sw("ra", 4, "t6"), sw("sp", 8, "t6"), sw("a0", 12, "t6")]
for k in range(16): prog += [lw("t8", 4 * k, "sp"), sw("t8", 16 + 4 * k, "t6")]
prog += [addiu("t6", "t6", 0x50), sw("t6", D, "t4"), "skip", a, b, j(HOOK + 8), 0]
words = assemble(prog)
assert len(words) * 4 <= 0x1000
p.pause()
p.write32(DATA, BUF)
for n, w in enumerate(words): p.write32(CODE + 4 * n, w)
p.write32(HOOK, j(CODE)); p.write32(HOOK + 4, 0)
p.resume()
last, got = p.read32(VSYNC), 0
pos = BUF
def drain():
    global pos
    end = p.read32(DATA) // 0x50 * 0x50 + BUF - BUF % 0x50 if False else p.read32(DATA)
    data = b"".join(p.read_block(x, min(0x1000, end - x + 7 & ~7)) for x in range(pos, end, 0x1000))[: end - pos]
    pos = end
    return data
while not p.read32(0x437e00): time.sleep(0.02)  # the match object appears during the loading screen
last = start = p.read32(VSYNC)
# optional 4th argument: a second file with the match object's first 0x100 bytes at every vsync (u32 vsync + 0x100)
frames = open(sys.argv[4], "wb") if len(sys.argv) > 4 else None
while last - start < want:
    if frames:
        last = p.next_frame(last)
        gm = p.read32(0x437e00)
        frames.write(struct.pack("<I", last) + p.read_block(gm, 0x100))
    else:
        time.sleep(0.5)
        last = p.read32(VSYNC)
    d = drain()
    out.write(d); got += len(d) // 0x50
print("calls", got, "vsyncs", start, "-", last)
