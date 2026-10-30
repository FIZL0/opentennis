#!/usr/bin/env python3
"""B700a: which stick the singles CPU server's contact reads, live.
Usage (HST_PCSX2=N): b700a_serve_stick.py <slot> <seconds> <out.bin> [mash period s]
Loads the save state, mashes Cross for P1 and, for every CPU serve, prints one line per frame from the AI's press
(serve sub-state 5) to two frames after the launch: the AI's sub-state and countdown, the player's swing countdown
(+0x3ec4) and branch, the pad stick word the AI wrote this frame (+0x17d4), the aim target (+0x3e90 x, z) and the
ball kind. Each frame's player block +0x3e80..+0x4000 goes to out.bin (vsync u32 + 0x180 bytes) for diffing."""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
from pine import Pine, VSYNC

p = Pine()
p.load_state(int(sys.argv[1]))
time.sleep(0.5)
gm = p.read32(0x422f80)
pl = [p.read32(gm + 0xa8 + 4 * k) for k in range(2)]
ai = [p.read32(x + 0x80) for x in pl]
period = sys.argv[4] if len(sys.argv) > 4 else "0.5"
masher = subprocess.Popen([sys.executable, "-c", f"""
import subprocess, time
while True:
    subprocess.run(['tools/vpad.py', 'send', 'press cross 80'], capture_output=True); time.sleep({period})
"""], cwd=os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../.."))
out = open(sys.argv[3], "wb")
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
s32 = lambda a: struct.unpack("<i", struct.pack("<I", p.read32(a)))[0]
last, end = p.read32(VSYNC), time.time() + float(sys.argv[2])
follow = [None, None]  # frames left to log after the launch, or -1 while in the swing
try:
    while time.time() < end:
        last = p.next_frame(last)
        if p.read32(0x422f80) != gm: print("match over", flush=True); break
        b = p.read32(gm + 0x88)
        for k in range(2):
            if p.read32(pl[k] + 0x84) == 0: continue  # human
            st, sub = p.read8(ai[k] + 0x54), p.read8(ai[k] + 0x55)
            if follow[k] is None and st == 1 and sub == 5:
                follow[k] = -1
                print(f"-- p{k} press v{last} err {s32(ai[k] + 0xb8)} swing {p.read8(ai[k] + 0xb2)}", flush=True)
            if follow[k] is None: continue
            blk = p.read_block(pl[k] + 0x3e80, 0x180)
            out.write(struct.pack("<I", last) + blk)
            launched = p.read8(b + 0x58) == 0 and s32(b + 0xac) == 0 and abs(f(b + 0x130)) + abs(f(b + 0x138)) > 0.05
            print(f"v{last} st {st}/{sub} cd {s32(ai[k] + 0xb4)} swing_cd {s32(pl[k] + 0x3ec4)} branch {p.read8(pl[k] + 0x3ec1)} "
                  f"btn {p.read8(pl[k] + 0x3ee4)} stick {p.read16(pl[k] + 0x17d4):04x} aim ({f(pl[k] + 0x3e90):.3f}, {f(pl[k] + 0x3e98):.3f}) "
                  f"off {s32(pl[k] + 0x3fa0)} ball {p.read8(b + 0x58)}/{s32(b + 0xac)} kind {s32(b + 0x5c)}", flush=True)
            if launched and follow[k] == -1: follow[k] = 2
            elif follow[k] > 0:
                follow[k] -= 1
                if follow[k] == 0: follow[k] = None
            if st != 1 and follow[k] == -1: follow[k] = None
finally:
    masher.kill()
    out.close()
