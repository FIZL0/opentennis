#!/usr/bin/env python3
"""B673: screenshots of the original's post-point reactions from a save-state load: at each of the first <points>
reactions (a face from 0x2b on, phase gm+0x55 == 4), shots <delays> frames in (tools/screenshot.sh), printing each
player's face index and morph time at the shot. Usage: face_shot.py <slot> <points> <outdir> [delay ...]."""
import os, struct, subprocess, sys, time
sys.path.insert(0, 'tools')
from pine import Pine

VS, GM = 0x1d5780, 0x422f80
slot, points, outdir = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
delays = [int(a) for a in sys.argv[4:]] or [30]
p = Pine(); p.load_state(slot); time.sleep(0.5)
gm = p.read32(GM)
anims = [p.read32(p.read32(gm + 0xa8 + 4 * i) + 0x54) for i in range(4)]
chars = [p.read32(p.read32(gm + 0xa8 + 4 * i) + 0x12bc) for i in range(4)]
print('characters', chars, flush=True)
def faces():
    out = []
    for a in anims:
        s0 = p.read32(a); mp = p.read32(s0 + 0xc)
        out.append((p.read32(a + 0x20), struct.unpack('<f', p.read_block(mp + 0x20, 8)[:4])[0]))
    return out
last, done, start = p.read32(VS), 0, None
while done < points:
    v = last = p.next_frame(last)
    if start is None:
        if p.read8(gm + 0x55) == 4 and all(m >= 0x2b for m, _ in faces()):
            start, taken = v, set()
        continue
    for d in [d for d in delays if d not in taken and v - start >= d][:1]:
            taken.add(d)
            if True:
                f = faces()
                png = os.path.join(outdir, f'p{done}_{d}.png')
                subprocess.run(['tools/screenshot.sh', png], check=True, stdout=subprocess.DEVNULL)
                print(png, 'vsync', v, ' '.join('%#x@%.0f' % x for x in f), flush=True)
    if v - start > max(delays):
        done, start = done + 1, None
        while p.read8(gm + 0x55) == 4: last = p.next_frame(last)
