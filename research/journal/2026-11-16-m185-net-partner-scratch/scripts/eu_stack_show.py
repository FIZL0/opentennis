#!/usr/bin/env python3
"""M185: summarise eu_stack_rec.py's file: per call the scratch quad (sp - 0x50) and the code pointers in the dead
stack. Usage: eu_stack_show.py <file> [player] [first vsync] [count]"""
import struct, sys
WORDS = 0x60; REC = 0x20 + 4 * WORDS
d = open(sys.argv[1], "rb").read()
want = int(sys.argv[2]) if len(sys.argv) > 2 else None
v_from = int(sys.argv[3]) if len(sys.argv) > 3 else 0
count = int(sys.argv[4]) if len(sys.argv) > 4 else 40
shown = 0
for k in range(0, len(d) - REC + 1, REC):
    v, ai, ra, sp, slot = struct.unpack_from("<5I", d, k)
    if want is not None and slot != want or v < v_from: continue
    w = struct.unpack_from(f"<{WORDS}I", d, k + 0x20)
    q = struct.unpack_from("<4f", d, k + 0x20 + 4 * WORDS - 0x50)
    code = [(-(4 * WORDS) + 4 * n, x) for n, x in enumerate(w) if 0x100000 <= x < 0x450000 and x & 3 == 0]
    print(v, slot, hex(ra), hex(sp), "q=(%.4f %.4f %.4f %.4f)" % q,
          " ".join("%x@-%x" % (x, -o) for o, x in code if -o <= 0x100))
    shown += 1
    if shown >= count: break
