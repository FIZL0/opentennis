#!/usr/bin/env python3
"""The CPU server's drawn serve error against the contact's timing offset, live.
Usage (HST_PCSX2=N): will_curve_err.py <slot> <seconds> [mash period s] [dump.bin]
Loads the save state, prints each player's menu character/outfit, AIParam row and level, then mashes Cross for P1 and
prints, on each serve launch, the server, its AI object's serve error (the timing draw, the frame before; the singles AI keeps it at +0x230), the toss, its timing offset
and the ball kind/bend; with dump.bin, appends (vsync, offset, the server's AI object 0x400 bytes) per serve."""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../tools"))
from pine import Pine, VSYNC

TABLE, RECORD, SLOTS = 0x3174c0, 0x118, 0x2ef7f4
p = Pine()
p.load_state(int(sys.argv[1]))
time.sleep(0.5)
gm = p.read32(0x422f80)
pl = [p.read32(gm + 0xa8 + 4 * k) for k in range(2)]
ai = [p.read32(x + 0x80) for x in pl]
for k in range(2):
    print(f"p{k} char {p.read8(SLOTS + 12 * k)} outfit {p.read8(SLOTS + 12 * k + 1)} setup {p.read32(SLOTS + 12 * k + 8):#x} "
          f"row {(p.read32(ai[k] + 12) - TABLE) // RECORD} level {p.read8(ai[k] + 16)}", flush=True)
period = sys.argv[3] if len(sys.argv) > 3 else "0.4"
masher = subprocess.Popen([sys.executable, "-c", f"""
import subprocess, time
while True:
    subprocess.run(['tools/vpad.py', 'send', 'press cross 80'], capture_output=True); time.sleep({period})
"""], cwd=os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
s32 = lambda a: struct.unpack("<i", struct.pack("<I", p.read32(a)))[0]
last, was, err, end = p.read32(VSYNC), False, [0, 0], time.time() + float(sys.argv[2])
try:
    while time.time() < end:
        last = p.next_frame(last)
        if p.read32(0x422f80) != gm: print("match over", flush=True); break
        b = p.read32(gm + 0x88)
        now = p.read8(b + 0x58) == 0 and s32(b + 0xac) == 0 and abs(f(b + 0x130)) + abs(f(b + 0x138)) > 0.05
        if now and not was:
            k = next((k for k in range(2) if p.read8(pl[k] + 0x3ec1) == 0 and p.read8(pl[k] + 0x3fa6) == 3), -1)
            print(f"v{last} p{k} err {err[k]} now {s32(ai[k] + 0x230)} toss {s32(pl[k] + 0x3ea0)} offset {s32(pl[k] + 0x3ecc)} "
                  f"kind {s32(b + 0x5c)} bend {f(b + 0x250):.3f}", flush=True)
            if len(sys.argv) > 4 and k >= 0:
                with open(sys.argv[4], "ab") as d: d.write(struct.pack("<ii", last, s32(pl[k] + 0x3ecc)) + p.read_block(ai[k], 0x400))
        else:
            err = [s32(a + 0x230) for a in ai]
        was = now
finally:
    masher.kill()
