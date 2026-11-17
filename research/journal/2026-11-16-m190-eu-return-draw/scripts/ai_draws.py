#!/usr/bin/env python3
"""M190: log every draw from the European build's AI generator from a save-state load (slot holding
fixtures/m169/eu-bot-start.p2s). Usage: ai_draws.py <slot> <frames> <out.bin>. Run under tools/pcsx2.sh.
A stub at the generator's entry (EU address from context/m169/match.tsv) appends, when a0 is the AI generator, a record
of 0x230 bytes: u32 vsync, ra, sp, the generator's index, s0..s7, then 128 stack words from sp (the callers' frames,
for their saved return addresses). With a fourth argument (an AI's address, hex) each record also carries, at 0x230,
that AI's first 0x300 bytes, then its player's (*(ai+4)) +0x12b0, +0x13f0 and +0x3d70 (0x10 each): 0x560 bytes."""
import struct, sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
import pine
from pine import Pine
from rec import lw, sw, lui, addiu, sltu, j, li, br, assemble

VSYNC, GEN, MT = 0x1ea480, 0x19f890, 0x43bfb0
pine.VSYNC = VSYNC
CODE, DATA, BUF, END = 0x1d00000, 0x1d01000, 0x1d10000, 0x1fb0000
D, WORDS = DATA & 0xffff, 128
WATCH = int(sys.argv[4], 16) if len(sys.argv) > 4 else 0
SIZE = 0x560 if WATCH else 0x230
SREGS = ["s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7"]


def copy(src, n, at):
    """n bytes from src (an address, or a register holding one) to the record's +at; uses t7..t9, v1."""
    lbl = f"k{copy.n}"
    copy.n += 1
    head = li("t7", src) if isinstance(src, int) else [addiu("t7", src, 0)]
    return head + [addiu("t8", "t5", at), addiu("t9", "zero", n // 4), lbl, lw("v1", 0, "t7"), sw("v1", 0, "t8"),
                   addiu("t7", "t7", 4), addiu("t8", "t8", 4), addiu("t9", "t9", -1), br(5, "t9", "zero", lbl), 0]
copy.n = 0


def stub(first, second):
    p = [lui("t4", DATA >> 16), *li("t5", MT), br(5, "a0", "t5", "skip"), 0,
         lw("t5", D, "t4"), *li("t6", END - 0x1000), sltu("t6", "t5", "t6"), br(4, "t6", "zero", "skip"), 0,
         *li("t6", VSYNC), lw("t6", 0, "t6"), sw("t6", 0, "t5"), sw("ra", 4, "t5"), sw("sp", 8, "t5"),
         lw("t6", 0x9c4, "a0"), sw("t6", 12, "t5")]
    p += [sw(r, 16 + 4 * k, "t5") for k, r in enumerate(SREGS)]
    p += [addiu("t7", "sp", 0), addiu("t8", "t5", 48), addiu("t9", "zero", WORDS), "c",
          lw("v1", 0, "t7"), sw("v1", 0, "t8"), addiu("t7", "t7", 4), addiu("t8", "t8", 4), addiu("t9", "t9", -1),
          br(5, "t9", "zero", "c"), 0]
    if WATCH:
        p += copy(WATCH, 0x300, 0x230)
        for k, off in enumerate([0x12b0, 0x13f0, 0x3d70]):
            p += [*li("t6", WATCH), lw("t6", 4, "t6"), addiu("t6", "t6", off)] + copy("t6", 0x10, 0x530 + 0x10 * k)
    p += [addiu("t5", "t5", SIZE), sw("t5", D, "t4"), "skip", first, second, j(GEN + 8), 0]
    return p


slot, want, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
p = Pine()
assert p.game_id() == "SCES-54535", p.game_id()
p.load_state(slot)
time.sleep(1)
v0 = p.read32(VSYNC)
while p.read32(VSYNC) - v0 < 5: time.sleep(0.05)
p.pause()
a, b = p.read32(GEN), p.read32(GEN + 4)
assert (a, b) == (0x8c8209c4, 0x28420270), (hex(a), hex(b))
p.write32(DATA, BUF)
for n, w in enumerate(assemble(stub(a, b))): p.write32(CODE + 4 * n, w)
p.write32(GEN, j(CODE))
p.write32(GEN + 4, 0)
p.resume()
v0 = p.read32(VSYNC)
print("from vsync", v0, flush=True)
try:
    while p.read32(VSYNC) - v0 < want and p.read32(DATA) < END - 0x2000: time.sleep(0.5)
finally:
    p.pause()
    p.write32(GEN + 4, b)
    p.write32(GEN, a)
    p.resume()
end = p.read32(DATA)
data = b"".join(p.read_block(x, 0x10000) for x in range(BUF, end, 0x10000))[: end - BUF]
open(out, "wb").write(data)
print("done", (end - BUF) // SIZE, "draws over", p.read32(VSYNC) - v0, "frames")
