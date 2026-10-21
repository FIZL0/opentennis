#!/usr/bin/env python3
"""P128: every frame of the live-ball recordings (context/fixtures/*.bin, frames_live layout) where the ball touched
the ghost material, with the frame's ghost-touch count, the latest contact's material and normal. Prints a histogram
of the per-frame count (the game resets it every step), so a count of 2 or more would be a repeated ghost touch.

  python3 -I research/ghost_touches.py [context/fixtures]
"""
import collections
import glob
import struct
import sys

PAD = 4
GLOBALS = PAD + 0x90
GM = GLOBALS + 0x180
BALL = GM + 0x100
PLAYERS = BALL + 0x290
SAMPLE = PLAYERS + 4 * 0x600
LIVE = SAMPLE + 0x290 + 0x50
# live ball object fields
GHOSTS, MATERIAL, NORMAL, STEPS, STATE = 0x264, 0x220, 0x210, 0xAC, 0xA4

root = sys.argv[1] if len(sys.argv) > 1 else "context/fixtures"
counts = collections.Counter()
for path in sorted(glob.glob(f"{root}/*.bin")):
    data = open(path, "rb").read()
    if len(data) % LIVE or not len(data) % SAMPLE:
        continue  # not a frames_live recording
    name = path.rsplit("/", 1)[-1]
    for k in range(len(data) // LIVE):
        b = data[k * LIVE + SAMPLE : k * LIVE + SAMPLE + 0x290]
        g = struct.unpack_from("<i", b, GHOSTS)[0]
        counts[g] += 1
        if g:
            n = struct.unpack_from("<3f", b, NORMAL)
            print(f"{name} frame {k}: ghosts {g} material {b[MATERIAL]} normal "
                  f"({n[0]:.3f}, {n[1]:.3f}, {n[2]:.3f}) steps {struct.unpack_from('<i', b, STEPS)[0]} state {b[STATE]}")
print("ghost touches per frame:", dict(sorted(counts.items())))
