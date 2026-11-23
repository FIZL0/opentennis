#!/usr/bin/env python3
"""B772: show eu_stack_tick_rec.py's file: per call of the computer-player dispatcher, the game tick counter, the
phase, the player's court slot and the dead-stack quads at sp - OFF (default 0x50 and 0x20, the two offsets M185's
notes name for the stand routine's never-initialised scratch quad).
Usage: eu_stack_tick_show.py <file> <words (hex, as recorded)> [slot] [first tick] [last tick] [phase]"""
import struct, sys
d = open(sys.argv[1], "rb").read()
words = int(sys.argv[2], 16)
rec = 0x20 + 4 * words
slot = int(sys.argv[3]) if len(sys.argv) > 3 and sys.argv[3] != "-" else None
lo = int(sys.argv[4]) if len(sys.argv) > 4 else 0
hi = int(sys.argv[5]) if len(sys.argv) > 5 else 1 << 31
phase = int(sys.argv[6]) if len(sys.argv) > 6 else None
for k in range(0, len(d) - rec + 1, rec):
    v, ai, ra, sp, s, tick, pw = struct.unpack_from("<7I", d, k)
    ph = (pw >> 8) & 0xff
    if slot is not None and s != slot or not lo <= tick <= hi or phase is not None and ph != phase:
        continue
    q = lambda off: struct.unpack_from("<4f", d, k + rec - off)
    print(v, "tick", tick, "phase", ph, "slot", s, "sp-0x50 (%.4f %.4f %.4f %.4f)" % q(0x50), "sp-0x20 (%.4f %.4f %.4f %.4f)" % q(0x20))
