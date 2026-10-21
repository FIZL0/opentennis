#!/usr/bin/env python3
"""P133: record court 5's ball-boy cart wheels frame by frame. First run: loads save slot 2 (doubles character
select), takes the default picks, opens Select Court and saves that to scratch slot 8 (check it opened on the
carousel's court 2). Second run (P133_FROM_8=1): picks court 5 from there, waits for P1's first serve, finds the four
carts' motion clocks, then for <frames> vsyncs writes per frame: vsync, the match phase, each cart's clock (sampled,
time) and its two wheels' world matrices. P1 pauses twice (Start, Start) partway. Output: <out.bin> of fixed-size
records (`crates/hst-sim/tests/world.rs` reads it); prints the court and frame count.
Usage (under tools/pcsx2.sh): [P133_FROM_8=1] p133_wheels.py <out.bin> [frames, default 2400]"""
import os, struct, sys, threading, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
from pine import Pine, VSYNC
from pick import vpad

GAME, COURT, LIST = 0x422f80, 0x422f90, 0x423928
out, frames = sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 2400
p = Pine()
# Select Court opens on the confirm screen's (random) court; a run that opened it on the carousel's court 2 is saved to
# scratch slot 8 once, and from there two left is the carousel's court 11, disc court 5 (carousel 1, 2, 3, 4, 5, 7, 11
# are disc courts 1, 4, 6, 2, 8, 11, 5)
if not os.environ.get("P133_FROM_8"):
    p.load_state(2)
    time.sleep(1.5)
    vpad("sleep 2500", *["press cross 150", "sleep 1200"] * 4, "press start 150", "sleep 3000", "press down 150",
         "sleep 800", "press cross 150", "sleep 2500")
    p.save_state(8)
    time.sleep(2)
    sys.exit("Select Court saved to slot 8: check (tools/screenshot.sh) that it's on the carousel's court 2, then run "
             "again with P133_FROM_8=1")
p.load_state(8)
time.sleep(1.5)
vpad("sleep 1000", *["press left 150", "sleep 1200"] * 2, "press cross 150", "sleep 1500", "press up 150", "sleep 800",
     "press cross 150")
t = time.time()
while True:
    gm = p.read32(GAME)
    pl = gm and p.read32(gm + 0xa8)
    if pl and p.read8(gm + 0x55) == 2 and p.read8(pl + 0x3fa4) == 1:
        break
    if time.time() - t > 150: sys.exit(f"match didn't reach the serve (court {p.read32(COURT)})")
    time.sleep(0.1)
court = p.read32(COURT)
if court != 5: sys.exit(f"landed on court {court}, not 5")

# colliding props' node arrays in list order (tests/world.rs walks the same chain)
nodes, node = [], p.read32(LIST)
while node:
    inst = p.read32(p.read32(p.read32(p.read32(node))) + 0xc)
    if p.read8(p.read32(inst + 0x54) + 0x48): nodes.append(p.read32(inst + 0x64))
    node = p.read32(node + 0x10)
carts = nodes[3:7]
# the carts' clocks: words (1, _, 1.0, sampled, time, 1.0) with a whole sampled time below 40 and time = sampled + 1
blk = b"".join(p.read_block(0x1800000 + a, 0x8000) for a in range(0, 0x80000, 0x8000))
one = struct.pack("<f", 1.0)
clocks = []
for i in range(4, len(blk) - 8, 4):
    if blk[i - 4:i] == one and blk[i + 8:i + 12] == one:
        s, tm = struct.unpack_from("<2f", blk, i)
        if s == int(s) and 0 <= s < 40 and tm == s + 1 and blk[i - 12:i - 8] == b"\1\0\0\0":
            clocks.append(0x1800000 + i)
if len(clocks) != 4: sys.exit(f"found {len(clocks)} cart clocks: {[hex(c) for c in clocks]}")
# in list order: each cart's clock sits 0xb8 bytes before its node array (as in P125's dump)
clocks.sort(key=lambda c: [n - c for n in carts].index(0xb8) if 0xb8 in [n - c for n in carts] else sys.exit("clock/cart pairing"))
print("carts", [hex(c) for c in carts], "clocks", [hex(c) for c in clocks], flush=True)

regions = [(VSYNC & ~7, 8), ((gm + 0x55) & ~7, 8)] + [(c & ~7, 16) for c in clocks] + [(n + 0x120, 0x40) for n in carts] + [(n + 0x240, 0x40) for n in carts]
def presses():
    time.sleep(12); vpad("press start 150", "sleep 2000", "press start 150")
    time.sleep(30); vpad("press start 150", "sleep 1000", "press start 150")
threading.Thread(target=presses, daemon=True).start()
recs, v = [], p.read32(VSYNC)
while len(recs) < frames:
    v = p.next_frame(v, 0.002)
    b = p.settle(regions, v)
    if b is None: continue
    rv = struct.unpack_from("<I", b, VSYNC - (VSYNC & ~7))[0]
    phase = b[8 + (gm + 0x55) - ((gm + 0x55) & ~7)]
    o, cl = 16, []
    for c in clocks:
        k = c - (c & ~7)
        cl += struct.unpack_from("<2f", b, o + k); o += 16
    recs.append(struct.pack("<II8f", rv, phase, *cl) + b[o:])
open(out, "wb").write(b"".join(recs))
print("court", court, "frames", len(recs), "vsync", struct.unpack_from("<I", recs[0])[0], "→", v)
