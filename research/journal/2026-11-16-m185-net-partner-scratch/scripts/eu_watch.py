#!/usr/bin/env python3
"""M185: find who writes one RAM quad (a poor man's write watch). Hooks the entry of each given routine; a hook logs
when the quad differs from the last logged value (so a change is bracketed between two hooks' entries), and the
"always" hooks log every call. European program addresses; copy fixtures/m169/eu-bot-start.p2s to the slot first and
run under tools/pcsx2.sh.
Usage: eu_watch.py <slot> <frames> <out.bin> <quad address> <always: a,b,..> <hooks: a,b,..>
File: records of u32 hook address, ra, vsync, a0, the quad's 4 words (on an always hook: a0 is the hooked
routine's a0)."""
import struct, sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
import pine
from pine import Pine
from rec import lw, sw, lui, addiu, sltu, j, li, br, assemble

VSYNC = 0x1ea480
pine.VSYNC = VSYNC
CODE, DATA, BUF, END = 0x1d00000, 0x1e07000, 0x1c40000, 0x1cf0000
D = DATA & 0xffff
PTR, LAST = D, D + 0x10


def stub(hook, always, watch, first, second):
    p = [lui("t4", DATA >> 16), *li("t5", watch)]
    if not always:
        for k in range(4):
            p += [lw("t6", 4 * k, "t5"), lw("t7", LAST + 4 * k, "t4"), br(5, "t6", "t7", "log"), 0]
        p += [br(4, "zero", "zero", "skip"), 0]
    p += ["log", lw("t9", PTR, "t4"), *li("t6", END - 0x20), sltu("t6", "t9", "t6"), br(4, "t6", "zero", "skip"), 0,
          *li("t6", hook), sw("t6", 0, "t9"), sw("ra", 4, "t9"), *li("t6", VSYNC), lw("t6", 0, "t6"), sw("t6", 8, "t9"),
          sw("a0", 12, "t9")]
    for k in range(4):
        p += [lw("t6", 4 * k, "t5"), sw("t6", 16 + 4 * k, "t9"), sw("t6", LAST + 4 * k, "t4")]
    p += [addiu("t9", "t9", 0x20), sw("t9", PTR, "t4"), "skip", first, second, j(hook + 8), 0]
    return p


def relocatable(w):
    op = w >> 26
    if op in (2, 3, 4, 5, 6, 7, 1, 0x14, 0x15, 0x16, 0x17): return False
    if op == 0 and (w & 0x3f) in (8, 9): return False
    return True


slot, want, out, watch = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3], int(sys.argv[4], 16)
always = [int(x, 16) for x in sys.argv[5].split(",") if x]
hooks = always + [int(x, 16) for x in sys.argv[6].split(",") if x and int(x, 16) not in always]
p = Pine()
assert p.game_id() == "SCES-54535", p.game_id()
p.load_state(slot)
time.sleep(1)  # the load lands late
v0 = p.read32(VSYNC)
while p.read32(VSYNC) - v0 < 5: time.sleep(0.05)
p.pause()
installed, skipped = [], []
p.write32(DATA + PTR - D, BUF)
at = CODE
for h in hooks:
    a, b = p.read32(h), p.read32(h + 4)
    if not (relocatable(a) and relocatable(b)):
        skipped.append(hex(h))
        continue
    words = assemble(stub(h, h in always, watch, a, b))
    for n, w in enumerate(words): p.write32(at + 4 * n, w)
    installed.append((h, a, b, at))
    at += 4 * len(words) + 0x10
for h, a, b, s in installed:
    p.write32(h + 4, 0)
    p.write32(h, j(s))
p.resume()
print("hooked", len(installed), "skipped (branch in the first two words):", skipped, flush=True)
v0 = p.read32(VSYNC)
try:
    while p.read32(VSYNC) - v0 < want and p.read32(DATA) < END - 0x20: time.sleep(0.5)
finally:
    p.pause()
    for h, a, b, s in reversed(installed):
        p.write32(h + 4, b)
        p.write32(h, a)
    p.resume()
end = p.read32(DATA)
data = b"".join(p.read_block(x, 0x10000) for x in range(BUF, end, 0x10000))[: end - BUF]
open(out, "wb").write(data)
print("done", (end - BUF) // 0x20, "records over", p.read32(VSYNC) - v0, "frames")
