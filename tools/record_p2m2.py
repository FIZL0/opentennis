#!/usr/bin/env python3
"""Capture game state every frame while PCSX2 plays an input recording (Tools → Input Recording → Play).
Usage: record_p2m2.py <state.p2s | slot> <out.bin> <frames>. With a .p2s: start it first, then start playback; it
arms on that state's load (vsync counter jumps). With a slot number it loads the state itself (bot games: slot 5).
Writes `frames` samples. It polls the running game (a PCSX2 copy runs at 1x; the user's own PCSX2
slowed: NominalScalar 0.5 or 0.25), taking a sample only when two reads in a row are identical, so it is never torn
mid-frame (a frame that ticks mid-read is skipped and logged).
Sample = u32 vsync counter + the regions in REGIONS order (fixed size; layout in README of the P0 journal), then
the live ball *(gm+0x88) and the rally block 0x3165f0 (not in round1.bin, which predates them)."""
import struct, sys, time, zipfile
from pine import Pine
from rec import match_regions

VSYNC, GM_PTR = 0x1d5780, 0x422f80
regions = match_regions  # the sample layout, shared with tools/play_p2m2.py

p, out, want = Pine(step=True), open(sys.argv[2], "wb"), int(sys.argv[3])
if sys.argv[1].isdigit():
    last = p.read32(VSYNC)
    p.load_state(int(sys.argv[1]))
    time.sleep(0.3)
    start = p.read32(VSYNC)
else:
    start = struct.unpack_from("<I", zipfile.ZipFile(sys.argv[1]).read("eeMemory.bin"), VSYNC)[0]
    last = p.read32(VSYNC)
print("waiting for vsync", start, flush=True)
n, armed = 0, False
while n < want:
    v = p.next_frame(last)
    if not armed:
        armed = abs(v - last) > 2 and start <= v <= start + 2
        if not armed:
            last = v
            continue
        print("armed at vsync", v, flush=True)
    elif v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    gm, r = regions(p)
    a = p.settle(r, v)
    if p.read32(GM_PTR) != gm: sys.exit(f"match object gone at vsync {v} (match over), {n} samples")
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 600 == 0: print(n, flush=True)
print("done", n)
