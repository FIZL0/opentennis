#!/usr/bin/env python3
"""B700: the singles CPU server's press against the swing's own timing, live.
Usage (HST_PCSX2=N): b700_serve_press.py <slot> <seconds> [mash period s]
Loads the save state, mashes Cross for P1 and follows each CPU's serve state. On the frame the AI presses
(its serve state leaves the countdown stage) prints the countdown to its contact pick and the error it adds; on
the launch prints the swing's own timing offset and grade (what the special-shot test reads), its depth/side
errors, the toss, the ball kind and bend."""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
from pine import Pine, VSYNC

p = Pine()
p.load_state(int(sys.argv[1]))
time.sleep(0.5)
gm = p.read32(0x422f80)
pl = [p.read32(gm + 0xa8 + 4 * k) for k in range(2)]
ai = [p.read32(x + 0x80) for x in pl]
period = sys.argv[3] if len(sys.argv) > 3 else "0.5"
masher = subprocess.Popen([sys.executable, "-c", f"""
import subprocess, time
while True:
    subprocess.run(['tools/vpad.py', 'send', 'press cross 80'], capture_output=True); time.sleep({period})
"""], cwd=os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../.."))
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
s32 = lambda a: struct.unpack("<i", struct.pack("<I", p.read32(a)))[0]
last, was, end = p.read32(VSYNC), False, time.time() + float(sys.argv[2])
sub = [p.read8(a + 0x55) for a in ai]
first = [None, None]  # countdown when the pick was first taken
press = [None, None]
try:
    while time.time() < end:
        last = p.next_frame(last)
        if p.read32(0x422f80) != gm: print("match over", flush=True); break
        for k in range(2):
            if p.read8(ai[k] + 0x54) != 1:
                sub[k] = None
                continue
            s = p.read8(ai[k] + 0x55)
            if s == 4 and sub[k] == 3:
                first[k] = s32(ai[k] + 0xb4)
            if s == 5 and sub[k] in (3, 4):
                if sub[k] == 3: first[k] = s32(ai[k] + 0xb4)
                press[k] = (last, first[k], s32(ai[k] + 0xb4), s32(ai[k] + 0xb8), s32(ai[k] + 0x230), p.read8(ai[k] + 0xb2))
            sub[k] = s
        b = p.read32(gm + 0x88)
        now = p.read8(b + 0x58) == 0 and s32(b + 0xac) == 0 and abs(f(b + 0x130)) + abs(f(b + 0x138)) > 0.05
        if now and not was:
            k = next((k for k in range(2) if p.read8(pl[k] + 0x3ec1) == 0 and p.read8(pl[k] + 0x3fa6) == 3), -1)
            if k >= 0:
                print(f"v{last} p{k} press {press[k]} toss {s32(pl[k] + 0x3ea0)} offset {s32(pl[k] + 0x3fa0)} "
                      f"grade {p.read8(pl[k] + 0x3ee8)} depth {s32(pl[k] + 0x3ecc)} side {s32(pl[k] + 0x3ed0)} "
                      f"kind {s32(b + 0x5c)} bend {f(b + 0x250):.3f}", flush=True)
                press[k] = None
        was = now
finally:
    masher.kill()
