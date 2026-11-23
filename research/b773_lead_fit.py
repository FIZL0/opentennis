#!/usr/bin/env python3
"""B773: per game tick T of one rally phase, which shift s makes the game's leftover z (slot 2, eu_stack_tick_rec.py
dispatcher capture, words 0x20, quad at sp-0x50) equal play's racket z of player 1 printed at play tick T+OFF+s (the
temporary HST_B773 print in scratch.rs's after_pose: the z the AI of that tick read, with AHEAD 2), to 4 digits.
Usage: lead_fit.py <eu_stack.bin> <play log> <phase vsync lo> <phase vsync hi> [OFF=1174]"""
import struct, sys, re
d = open(sys.argv[1], "rb").read(); REC = 0x20 + 0x80
lo, hi = int(sys.argv[3]), int(sys.argv[4]); OFF = int(sys.argv[5]) if len(sys.argv) > 5 else 1174
game = {}
for k in range(0, len(d) - REC + 1, REC):
    v, ai, ra, sp, s, tick, pw = struct.unpack_from("<7I", d, k)
    if s == 2 and lo <= v <= hi and (pw >> 8) & 0xff == 3:
        game[tick] = struct.unpack_from("<4f", d, k + REC - 0x50)[2]
play = {}
for line in open(sys.argv[2]):
    m = re.match(r"B773 (\d+) mot (\S+) samp (\S+) t \S+ spd \S+ rz Some\((\S+)\)", line)
    if m: play[int(m[1])] = (float(m[4]), m[2], float(m[3]))
for T in sorted(game):
    z = game[T]
    hits = [s for s in range(-6, 7) if T + OFF + s in play and abs(play[T + OFF + s][0] - z) < 6e-5]
    p = play.get(T + OFF)
    print(T, "%.4f" % z, "shifts", hits, "play mot", p and p[1], "samp", p and p[2])
