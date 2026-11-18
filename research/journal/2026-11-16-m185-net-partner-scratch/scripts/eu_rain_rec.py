#!/usr/bin/env python3
"""M185 (variant, eu_rain_rec.py): as eu_stack_rec.py, but first sets the court's weather byte (HST_WEATHER, default 2 =
raining) so the gallery walkers' tick writes nothing; then logs.
Original notes: for every call of the European program's computer-player state dispatcher (the routine that runs a computer
player's per-frame state and, in a rally, the doubles NET/BASE stand routine), log the dead stack just below the
caller's stack pointer: what the stand routine's never-initialised scratch quad holds, and the saved return addresses
around it, which name the routines that last ran at that depth.
Usage: eu_stack_rec.py <slot> <frames> <out.bin> [hook words slot-at]  (hook: default the dispatcher; slot-at:
"ai" (the player is *(a0+4)) or "player" (a0); copy fixtures/m169/eu-bot-start.p2s to the slot first; run under
tools/pcsx2.sh). Addresses are the European program's (M169 function matches).
File: records of u32 vsync, ai, ra, sp, the player's court slot, 3 words 0; then the WORDS words from sp - 4*WORDS
up to sp."""
import struct, sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
import pine
from pine import Pine
from rec import lw, sw, lui, addiu, sltu, j, li, br, assemble

VSYNC, GM_PTR, HOOK = 0x1ea480, 0x437e00, 0x3e3ad0
if len(sys.argv) > 4: HOOK = int(sys.argv[4], 16)
AI = len(sys.argv) <= 6 or sys.argv[6] == "ai"
pine.VSYNC = VSYNC
CODE, DATA, BUF, END = 0x1e00000, 0x1e07000, 0x1c40000, 0x1df0000
WORDS = int(sys.argv[5], 16) if len(sys.argv) > 5 else 0x60  # 0x180 bytes below sp
REC = 0x20 + 4 * WORDS
D = DATA & 0xffff


def stub(first, second):
    p = [lui("t4", DATA >> 16), lw("t5", D, "t4"), *li("t6", END - REC), sltu("t6", "t5", "t6"), br(4, "t6", "zero", "skip"), 0,
         *li("t6", VSYNC), lw("t6", 0, "t6"), sw("t6", 0, "t5"), sw("a0", 4, "t5"), sw("ra", 8, "t5"), sw("sp", 12, "t5"),
         *([lw("t6", 4, "a0")] if AI else [addiu("t6", "a0", 0)]), lw("t6", 0x12b8, "t6"), sw("t6", 16, "t5"), sw("zero", 20, "t5"), sw("zero", 24, "t5"),
         sw("zero", 28, "t5"), addiu("t7", "sp", -4 * WORDS), addiu("t8", "t5", 0x20), addiu("t9", "zero", WORDS), "c",
         lw("v1", 0, "t7"), sw("v1", 0, "t8"), addiu("t7", "t7", 4), addiu("t8", "t8", 4), addiu("t9", "t9", -1),
         br(5, "t9", "zero", "c"), 0, addiu("t5", "t5", REC), sw("t5", D, "t4"),
         "skip", first, second, j(HOOK + 8), 0]
    return p


slot, want, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
p = Pine()
assert p.game_id() == "SCES-54535", p.game_id()
p.load_state(slot)
time.sleep(1)  # the load lands late
v0 = p.read32(VSYNC)
while p.read32(VSYNC) - v0 < 5: time.sleep(0.05)
gm = p.read32(GM_PTR); court = p.read32(gm + 0x84)
print("court", hex(court), "weather", p.read8(court + 0x135))
w = int(os.environ.get("HST_WEATHER", "2"))
al = (court + 0x135) & ~3; sh = 8 * ((court + 0x135) & 3)
p.pause(); p.write32(al, (p.read32(al) & ~(0xff << sh)) | (w << sh)); p.resume()
print("weather now", p.read8(court + 0x135))
a, b = p.read32(HOOK), p.read32(HOOK + 4)
assert HOOK != 0x3e3ad0 or (a, b) == (0x27bdffd0, 0x24090003), (hex(a), hex(b))
p.pause()
p.write32(DATA, BUF)
for n, w in enumerate(assemble(stub(a, b))): p.write32(CODE + 4 * n, w)
p.write32(HOOK + 4, 0)
p.write32(HOOK, j(CODE))
p.resume()
v0 = p.read32(VSYNC)
try:
    while p.read32(VSYNC) - v0 < want and p.read32(DATA) < END - REC: time.sleep(0.5)
finally:
    p.pause()
    p.write32(HOOK + 4, b)
    p.write32(HOOK, a)
    p.resume()
end = p.read32(DATA)
data = b"".join(p.read_block(x, 0x10000) for x in range(BUF, end, 0x10000))[: end - BUF]
open(out, "wb").write(data)
print("done", (end - BUF) // REC, "calls over", p.read32(VSYNC) - v0, "frames")
