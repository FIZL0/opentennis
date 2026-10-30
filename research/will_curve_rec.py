#!/usr/bin/env python3
"""Record every serve of a live match around its launch, in tools/record_p2m2.py's live layout (hst_sim::replay
frames_live: u32 vsync + match_regions, the live ball and rally block included).
Usage (HST_PCSX2=N, copy at 1x or slower): will_curve_rec.py <slot> <seconds> <out.bin> [mash[:period s]]
Loads the save state, optionally mashes Cross on the pad every period (0.4 s; vary it for a different match) (P1 serves and returns), keeps the last PRE frames in a
ring and, on each serve launch (live ball class 0, frame 0, moving), writes them plus the next POST frames.
Windows are written back to back; a window boundary is a vsync gap. Prints one line per serve: vsync, server,
character, toss, kind, timing offset, bend, turn."""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../tools"))
from pine import Pine, VSYNC
from rec import match_regions

PRE, POST = 150, 220
p = Pine()
slot, secs, out = int(sys.argv[1]), float(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(slot)
time.sleep(0.5)
gm0, r = match_regions(p)
sizes = [n for _, n in r]
off, o = [], 4
for n in sizes:
    off.append(o)
    o += n
GM, BALLP, PL, LIVE = off[2], off[3], off[4], off[12]
masher = None
if len(sys.argv) > 4 and sys.argv[4].startswith("mash"):
    period = sys.argv[4].split(":")[1] if ":" in sys.argv[4] else "0.4"
    masher = subprocess.Popen([sys.executable, "-c", f"""
import subprocess, time
while True:
    subprocess.run(['tools/vpad.py', 'send', 'press cross 80'], capture_output=True); time.sleep({period})
"""], cwd=os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

def f(b, o): return struct.unpack_from("<f", b, o)[0]
def i(b, o): return struct.unpack_from("<i", b, o)[0]
def launched(s):
    b = s[LIVE:]
    return b[0x58] == 0 and i(b, 0xac) == 0 and abs(f(b, 0x130)) + abs(f(b, 0x138)) > 0.05
def pl(s, k, o):  # player k's +0x3c00.. region
    return PL + k * 0x600 + 0x200 + o - 0x3c00

ring, post, n, serves, last, missed, was = [], 0, 0, 0, p.read32(VSYNC), 0, False
end = time.time() + secs
try:
    while time.time() < end:
        v = p.next_frame(last)
        if v != last + 1: missed += v - last - 1
        last = v
        gm, r = match_regions(p)
        if gm != gm0: print("match object gone at vsync", v, flush=True); break
        a = p.settle(r, v)
        if a is None:
            missed += 1
            continue
        s = struct.pack("<I", v) + a
        prev_was = was
        if post:
            out.write(s); n += 1; post -= 1
        else:
            ring.append(s)
            if len(ring) > PRE: ring.pop(0)
        now, was = launched(s), launched(s)
        if now and not prev_was:
            k = next((k for k in range(2) if s[pl(s, k, 0x3ec1)] == 0 and s[pl(s, k, 0x3fa6)] == 3), -1)
            ch = i(s, 4 + 0x90 + 0x28 + 4 * k) if k >= 0 else -1
            b = s[LIVE:]
            print(f"serve vsync {v} p{k} char {ch} toss {i(s, pl(s, k, 0x3ea0))} kind {i(b, 0x5c)} "
                  f"offset {i(s, pl(s, k, 0x3ecc))} bend {f(b, 0x250):.4f} turn {f(b, 0x1b0):.4f} "
                  f"side {f(b, 0x90):.3f},{f(b, 0x94):.3f},{f(b, 0x98):.3f} missed {missed}", flush=True)
            serves += 1
            if ring:
                for t in ring: out.write(t)
                n += len(ring)
                ring = []
            post = POST
finally:
    out.close()
    if masher: masher.kill()
    print(f"done: {serves} serves, {n} samples, {missed} frames missed, end vsync {last}", flush=True)
