#!/usr/bin/env python3
"""B574: per-frame trace of P1's CPU partner while the other team's ball comes, P1 idle (taps ○ only to serve).
Each line: vsync, shots, hitter, partner substate (AI +0x57), follow +0xd4, fresh +0x98, defer +0x275, chase +0x99,
P1's shot record in the path object (+0x70 + 0x30·slot: stamp, n, kind, stand x/z), P1's human search (its AI
+0x40 found, +0x48 kind, +0x4c min, +0x50 volley), the partner's and P1's x/z, the ball's x/y/z; P1's facing, stamina, tick, run, velocity and move state (+0x3fa5).
Usage: b574_trace.py <slot> <frames> > log. Needs tools/vpad.py serve."""
import os, struct, sys, time
sys.path.insert(0, "tools")
from pine import Pine
from vpad import FIFO

VSYNC, GM_PTR, SHOTS, HITTER, SERVER, WINNER = 0x1d5780, 0x422f80, 0x423060, 0x423058, 0x42304c, 0x4230a8
slot, want = int(sys.argv[1]), int(sys.argv[2])
s32 = lambda u: struct.unpack("<i", struct.pack("<I", u))[0]
F = lambda b, o, n=3: struct.unpack_from(f"<{n}f", b, o)
I = lambda b, o: struct.unpack_from("<i", b, o)[0]
pad = os.open(FIFO, os.O_RDWR)
send = lambda c: os.write(pad, (c + "\n").encode())
p = Pine()
p.load_state(slot)
time.sleep(2)
gm = p.read32(GM_PTR)
players = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
teams = [p.read32(a + 0x12b8) & 1 for a in players]
mate = next(k for k in range(1, 4) if teams[k] == teams[0])
ai, hai = p.read32(players[mate] + 0x80), p.read32(players[0] + 0x80)
path = p.read32(gm + 0xa4)
ball = p.read32(gm + 0x88)
slot0 = p.read32(players[0] + 0x12b8)
print("teams", teams, "mate", mate, "slot0", slot0, flush=True)
last = p.read32(VSYNC)
start, ph_was, pressed = last, None, 0
while last - start < want:
    v = p.next_frame(last)
    last = v
    ph = p.read8(gm + 0x55)
    shots, hitter = s32(p.read32(SHOTS)), s32(p.read32(HITTER))
    if ph == 2 and s32(p.read32(SERVER)) == 0 and v - pressed > 40:
        send("press circle 50")
        pressed = v
    if ph_was == 3 and ph != 3:
        print(f"v{v} point over: winner {s32(p.read32(WINNER))} hitter {hitter}", flush=True)
    ph_was = ph
    if ph != 3 or hitter < 0 or hitter & 1 == teams[0]:
        continue
    a = p.read_block(ai, 0x280)
    h = p.read_block(hai, 0x60)
    r = p.read_block(path + 0x70 + 0x30 * slot0, 0x30)
    me, hp = F(p.read_block(players[mate] + 0x3d70, 0x10), 0), F(p.read_block(players[0] + 0x3d70, 0x10), 0)
    b = F(p.read_block(ball, 0x100), 0xe0)
    hb = p.read_block(players[0] + 0x3d60, 0x10) + p.read_block(players[0] + 0x3df0, 0x20) + p.read_block(players[0] + 0x3fa0, 0x8)
    print(f"v{v} s{shots} h{hitter} sub{a[0x57]} fol{a[0xd4]} fr{a[0x98]} def{a[0x275]} ch{a[0x99]} "
          f"rec[{I(r, 0)} {I(r, 4)} k{I(r, 8)} {F(r, 0x20)[0]:.2f},{F(r, 0x20)[2]:.2f}] "
          f"hs[{h[0x40]} k{h[0x48]} m{I(h, 0x4c)} v{h[0x50]}] me {me[0]:.2f},{me[2]:.2f} p1 {hp[0]:.2f},{hp[2]:.2f} "
          f"ball {b[0]:.2f},{b[1]:.2f},{b[2]:.2f} | face {F(hb, 0, 4)} st {I(hb, 0x14)} tick {I(hb, 0x18)} run {I(hb, 0x1c)} vel {F(hb, 0x20)[0]:.3f},{F(hb, 0x20)[2]:.3f} mov {hb[0x35]}", flush=True)
send("release")
