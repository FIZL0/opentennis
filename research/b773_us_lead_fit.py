#!/usr/bin/env python3
"""B773: the US counterpart of lead_fit.py. Per NET/BASE call of slot 2's AI object (0x1074470) in
fixtures/ai_net_s05.bin (entry record: tag, length, vsync at +8, AI at +0x10, the stand scratch quad at +0x730), the
shifts s that make its scratch z equal play's racket z of player 1 printed at play tick (vsync + 1 - 7566) + s (the
temporary HST_B773 print, US bot match). Usage: us_lead_fit.py <ai_net_s05.bin> <play log> <max vsync>"""
import struct, sys, re
d = open(sys.argv[1], "rb").read(); hi = int(sys.argv[3])
play = {}
for line in open(sys.argv[2]):
    m = re.match(r"B773 (\d+) mot (\S+) samp (\S+) t \S+ spd \S+ rz Some\((\S+)\)", line)
    if m: play[int(m[1])] = (float(m[4]), m[2], float(m[3]))
o = 8 + 0x9c8; seen = set()
while o < len(d):
    e = d[o:o + struct.unpack_from("<I", d, o + 4)[0]]; o += len(e)
    x = d[o:o + struct.unpack_from("<I", d, o + 4)[0]]; o += len(x)
    tag, _, v = struct.unpack_from("<3I", e, 0); ai = struct.unpack_from("<I", e, 0x10)[0]
    if tag not in (2, 3) or ai != 0x1074470 or v > hi or v in seen: continue
    seen.add(v)
    z = struct.unpack_from("<f", e, 0x738)[0]; k = v + 1 - 7566  # the call runs in the frame whose sample is taken at v + 1
    hits = [s for s in range(-6, 7) if k + s in play and abs(play[k + s][0] - z) < 6e-5]
    p = play.get(k)
    print(k, "%.4f" % z, "shifts", hits, "play mot", p and p[1], "samp", p and p[2])
