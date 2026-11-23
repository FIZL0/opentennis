#!/usr/bin/env python3
"""B773: show record_anim.py's file per frame: vsync, the game manager's phase (gm+0x55) and tick (gm+0x58), and for
player P (index in the manager's player list) its anim object: motion, time sampled, time, speed, crossfade old
motion / old time / countdown / hold.
Usage: anim_show.py <file> <player> [first vsync] [last vsync]"""
import struct, sys
d = open(sys.argv[1], "rb").read()
P = int(sys.argv[2]); lo = int(sys.argv[3]) if len(sys.argv) > 3 else 0; hi = int(sys.argv[4]) if len(sys.argv) > 4 else 1 << 31
REC = 4 + 0x100 + 4 * (0x400 + 0x80 + 4)
for k in range(0, len(d) - REC + 1, REC):
    v = struct.unpack_from("<I", d, k)[0]
    if not lo <= v <= hi: continue
    gm = k + 4
    ph, tick = d[gm + 0x55], struct.unpack_from("<I", d, gm + 0x58)[0]
    a = gm + 0x100 + P * 0x484 + 0x400
    mot, = struct.unpack_from("<I", d, a + 0x20)
    samp, t, spd = struct.unpack_from("<3f", d, a + 0x38)
    om, = struct.unpack_from("<I", d, a + 0x44); ot, = struct.unpack_from("<f", d, a + 0x50)
    cd, = struct.unpack_from("<i", d, a + 0x60); hold = d[a + 0x6c]
    print(v, "ph", ph, "tick", tick, "mot", hex(mot), "samp %.4f t %.4f spd %.4f" % (samp, t, spd), "old", hex(om), "%.3f" % ot, "cd", cd, "hold", hold)
