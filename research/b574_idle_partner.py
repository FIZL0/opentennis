#!/usr/bin/env python3
"""B574: how often does a CPU partner give way to an idle human? Loads slot <slot> (P1 human beside a CPU partner),
P1 stands still (it only taps ○ while it is the server in the serve phase), and logs per point: whether team 0 lost
it on a ball the other team hit last (unreturned) and whether P1's partner (player 2) had its follow flag (AI +0xd4:
give way to the partner's shot record) set on that ball. Same counts as the port's `idle_human_partner_probe`.
Usage: b574_idle_partner.py <slot> <frames> > log. Needs tools/vpad.py serve."""
import os, struct, sys, time
sys.path.insert(0, "tools")
from pine import Pine
from vpad import FIFO

VSYNC, GM_PTR, SHOTS, HITTER, SERVER, WINNER = 0x1d5780, 0x422f80, 0x423060, 0x423058, 0x42304c, 0x4230a8
slot, want = int(sys.argv[1]), int(sys.argv[2])
s32 = lambda u: struct.unpack("<i", struct.pack("<I", u))[0]
F = lambda b, o, n=3: struct.unpack_from(f"<{n}f", b, o)
pad = os.open(FIFO, os.O_RDWR)
send = lambda c: os.write(pad, (c + "\n").encode())
p = Pine()
p.load_state(slot)
time.sleep(2)
gm = p.read32(GM_PTR)
players = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
teams = [p.read32(a + 0x12b8) & 1 for a in players]
mate = next(k for k in range(1, 4) if teams[k] == teams[0])
ai = p.read32(players[mate] + 0x80)
print("teams", teams, "mate", mate, flush=True)
last = p.read32(VSYNC)
start, ph_was, shots_was, following = last, None, None, False
points = opp = follows = lost_f = lost_n = 0
pressed = 0
while last - start < want:
    v = p.next_frame(last)
    last = v
    ph = p.read8(gm + 0x55)
    shots, hitter = s32(p.read32(SHOTS)), s32(p.read32(HITTER))
    if ph == 2 and s32(p.read32(SERVER)) == 0 and v - pressed > 40:
        send("press circle 50")
        pressed = v
    if shots != shots_was:
        if shots_was is not None and shots > shots_was and hitter >= 0 and hitter & 1 != teams[0]:
            opp += 1
        shots_was, following = shots, False
    if ph == 3 and hitter >= 0 and hitter & 1 != teams[0] and p.read8(ai + 0xd4) and not following:
        following = True
        follows += 1
        pos = [F(p.read_block(a + 0x3d70, 0x10), 0) for a in players]
        print(f"v{v} follow: p0 ({pos[0][0]:.1f},{pos[0][2]:.1f}) mate ({pos[mate][0]:.1f},{pos[mate][2]:.1f})", flush=True)
    if ph_was == 3 and ph != 3:
        points += 1
        if s32(p.read32(WINNER)) != teams[0] and hitter >= 0 and hitter & 1 != teams[0]:
            if following: lost_f += 1
            else: lost_n += 1
            print(f"v{v} team 0 lost, unreturned; partner following: {following}", flush=True)
    ph_was = ph
send("release")
print(f"points {points} opp shots {opp} follows {follows} lost (follow) {lost_f} lost (no follow) {lost_n}", flush=True)
