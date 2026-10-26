#!/usr/bin/env python3
"""B673: which face each player shows, per frame, from a save-state load. The anim object's first controller slot
holds the playing morph player and the table of the player's prebuilt ones (one per face index): the face index is
the playing one's place in that table.
Usage: face_rec.py <slot> <frames> [out.bin] [--every]. Without out.bin prints a line whenever the phase gm+0x55 or a
player's face changes (every point-over frame with --every): vsync, phase, per player face index @ morph time / speed
/ face length / loop byte. With out.bin writes per frame: u32 vsync, u32 phase byte, then per player (4) the anim
object (0x80), its morph player (0x30), u32 face index."""
import struct, sys, time
sys.path.insert(0, 'tools')
from pine import Pine

VS, GM = 0x1d5780, 0x422f80
args = [a for a in sys.argv[1:] if not a.startswith('--')]
out = open(args[2], 'wb') if len(args) > 2 else None
p = Pine(); p.load_state(int(args[0])); time.sleep(0.5)
last = p.read32(VS)
gm = p.read32(GM)
anims, tables = [], []
for i in range(4):
    a = p.read32(p.read32(gm + 0xa8 + 4 * i) + 0x54)
    anims.append(a)
    tables.append(struct.unpack('<%dI' % 0x600, p.read_block(p.read32(p.read32(a) + 0x18), 0x1800)))
prev, n = None, 0
while n < int(args[1]):
    v = p.next_frame(last)
    if v != last + 1: print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    mps = [p.read32(p.read32(a) + 0xc) for a in anims]
    b = p.settle([(gm + 0x50, 8)] + [r for a, mp in zip(anims, mps) for r in ((a, 0x80), (mp, 0x30))], v)
    if b is None or [p.read32(p.read32(a) + 0xc) for a in anims] != mps:
        print(f"torn {v}", flush=True); continue
    n += 1
    phase = b[5]
    rows = []
    for i, mp in enumerate(mps):
        o = 8 + i * 0xb0
        k = tables[i].index(mp) if mp in tables[i] else 0xffffffff
        rows.append((b[o:o + 0x80], b[o + 0x80:o + 0xb0], k))
    if out:
        out.write(struct.pack('<II', v, phase) + b''.join(a + m + struct.pack('<I', k) for a, m, k in rows))
        continue
    key = (phase, tuple(r[2] for r in rows))
    if key != prev or ('--every' in sys.argv and phase == 4):
        def fmt(m, k):
            end, base, lastt, t, speed, w = struct.unpack_from('<6f', m, 0x14)
            return '%#x@%.1f/%.2f/%.0f/%d' % (k, t, speed, end, m[0x2c])
        print(v, 'phase', phase, ' '.join(fmt(m, k) for _, m, k in rows), flush=True)
        prev = key
