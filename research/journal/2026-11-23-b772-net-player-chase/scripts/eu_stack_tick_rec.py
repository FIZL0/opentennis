#!/usr/bin/env python3
"""B772 (from M185's eu_stack_rec.py, plus the game manager's tick counter and phase word): for every call of the European program's computer-player state dispatcher (the routine that runs a computer
player's per-frame state and, in a rally, the doubles NET/BASE stand routine), log the dead stack just below the
caller's stack pointer: what the stand routine's never-initialised scratch quad holds, and the saved return addresses
around it, which name the routines that last ran at that depth.
Usage: eu_stack_tick_rec.py <slot> <frames> <out.bin> [hook words slot-at]  (hook: default the dispatcher; any routine
entry or call site whose first two words are not branches; slot-at: "ai" (the player is *(AIREG+4)) or "player").
Env: AIREG (default a0; s5 at a call site whose delay slot sets a0), SRC/SRC_END (copy the words ending at SRC+SRC_END
instead of the dead stack below sp, e.g. SRC=a1 for the quad a1 points at). Slot 8 holds the EU bot-match start
(fixtures/m169/eu-bot-start.p2s); run under tools/pcsx2.sh with HST_PCSX2_BIN=pcsx2-qt (the state needs PCSX2 v2.9.179).
Addresses are the European program's: the dispatcher from M169's function matches, the rest found by eu_code_dump.py.
File: records of u32 vsync, ai, ra, sp, the player's court slot, the game manager's tick counter and its phase word (phase in byte 1), 1 word 0; then the WORDS words from sp - 4*WORDS
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
AIREG = os.environ.get("AIREG", "a0")  # the register holding the computer player (s5 at a call site whose delay slot sets a0)
SRC = os.environ.get("SRC", "sp")
SRC_END = int(os.environ.get("SRC_END", "0x10" if SRC != "sp" else "0"), 16)  # the copy ends at SRC + SRC_END  # SRC=a1: the record ends with the quad a1 points at (B772: the after test's in/out spot)
D = DATA & 0xffff


def stub(first, second):
    p = [lui("t4", DATA >> 16), lw("t5", D, "t4"), *li("t6", END - REC), sltu("t6", "t5", "t6"), br(4, "t6", "zero", "skip"), 0,
         *li("t6", VSYNC), lw("t6", 0, "t6"), sw("t6", 0, "t5"), sw(AIREG, 4, "t5"), sw("ra", 8, "t5"), sw("sp", 12, "t5"),
         *([lw("t6", 4, AIREG)] if AI else [addiu("t6", AIREG, 0)]), lw("t6", 0x12b8, "t6"), sw("t6", 16, "t5"), *li("t6", GM_PTR), lw("t6", 0, "t6"), lw("v1", 0x58, "t6"), sw("v1", 20, "t5"), lw("v1", 0x54, "t6"), sw("v1", 24, "t5"),
         sw("zero", 28, "t5"), addiu("t7", SRC, SRC_END - 4 * WORDS), addiu("t8", "t5", 0x20), addiu("t9", "zero", WORDS), "c",
         lw("v1", 0, "t7"), sw("v1", 0, "t8"), addiu("t7", "t7", 4), addiu("t8", "t8", 4), addiu("t9", "t9", -1),
         br(5, "t9", "zero", "c"), 0, addiu("t5", "t5", REC), sw("t5", D, "t4"),
         "skip", first, second, j(HOOK + 8), 0]
    return p


slot, want, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
p = Pine()
assert p.game_id() == "SCES-54535", p.game_id()
p.load_state(slot)
time.sleep(5)  # the load lands late (1 s was too short for PCSX2 v2.9.179: the load overwrote the hook)
v0 = p.read32(VSYNC)
while p.read32(VSYNC) - v0 < 5: time.sleep(0.05)
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
