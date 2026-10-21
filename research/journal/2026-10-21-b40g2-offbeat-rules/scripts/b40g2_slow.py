#!/usr/bin/env python3
"""B40g2: the Offbeat Rules' slow motion (rule 2) in the original.
  b40g2_slow.py <slot> <vsyncs> <out>  load <slot> (5: bot-only match), turn rule 2 on, then per vsync log the match
      phase gm+0x55, tick gm+0x36c, the rule's armed/slow/countdown bytes gm+0x377/0x378/0x37c and the slow-mo block
      0x2ef090 (on, frac, start, end, ramp ticks/left, in-between flag, period, count)."""
import struct, sys, time
sys.path.insert(0, 'tools')
from pine import Pine

GM, RULES = 0x422f80, 0x2ef7e0
p = Pine()
p.load_state(int(sys.argv[1])); time.sleep(0.05)
w = p._u32(RULES)
p.write32(RULES, (w & 0x00ffffff) | 0x01000000)  # rule 2 byte (the word's top byte)
out = open(sys.argv[3], 'w')
last = p.read32(0x1d5780)
for n in range(int(sys.argv[2])):
    v = last = p.next_frame(last)
    gm = p.read32(GM)
    if not gm: continue
    g = p.read_regions([(gm + 0x50, 0x10), (gm + 0x360, 0x20), (0x2ef090, 0x30)])
    m, r, sm = g[:0x10], g[0x10:0x30], g[0x30:0x60]
    i = lambda b, o: struct.unpack_from('<i', b, o)[0]
    f = lambda b, o: struct.unpack_from('<f', b, o)[0]
    print(v, 'ph %d tick %d' % (m[5], i(r, 0xc)), 'arm %d slow %d cd %d' % (r[0x17], r[0x18], i(r, 0x1c)),
          'blk %d frac %.4f a %.3f b %.3f n %d left %d btw %d per %d c %d' % (
              sm[0], f(sm, 4), f(sm, 8), f(sm, 12), i(sm, 0x10), i(sm, 0x1c), sm[0x20], i(sm, 0x24), i(sm, 0x28)),
          file=out, flush=True)
