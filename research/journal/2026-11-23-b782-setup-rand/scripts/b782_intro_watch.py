#!/usr/bin/env python3
"""B782: watch the European match setup and intro from a save-state load, one line per frame where anything changes.

Logs the match tick and phase, the intro's two presentation gates (camera shot over, banner over), the 64-bit rand()
state, and the court object's weather fields: the first game's weather, the setup's spawn-clouds flag and the weather
holder's last-set weather byte (which its constructor never writes). Usage (under tools/pcsx2.sh): b782_intro_watch.py <slot> <frames> <out.txt> [min vsync]. The US program (SCUS-97610) works too, without the rand() state.
"""
import sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
import pine
from pine import Pine

slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "w")
p = Pine(step=True)
EU = p.game_id() == "SCES-54535"
assert EU or p.game_id() == "SCUS-97610", p.game_id()
# rand() state isn't located in the US program here: the US line logs 0
VSYNC, GM_PTR, RAND, PRES = (0x1ea480, 0x437e00, 0x1b81a8, 0x450058) if EU else (0x1d5780, 0x422f80, None, 0x43b1d8)
pine.VSYNC = VSYNC
ok = lambda a: 0x100000 <= a < 0x2000000
MIN = int(sys.argv[4]) if len(sys.argv) > 4 else 30000  # the state's vsync is at least this (load landed)
for _ in range(20):
    p.load_state(slot)
    t = time.monotonic()
    while time.monotonic() - t < 3 and p.read32(VSYNC) < MIN: time.sleep(0.05)
    if p.read32(VSYNC) >= MIN: break
else: sys.exit("LOAD FAILED")
last, prev = p.read32(VSYNC), None
for _ in range(want):
    v = last = p.next_frame(last)
    r = p.read64(RAND) if RAND else 0
    gm = p.read32(GM_PTR)
    row = [0] * 9
    if ok(gm):
        row[0:3] = p.read8(gm + 0x55), p.read8(gm + 0x56), p.read32(gm + 0x58)
        court = p.read32(gm + 0x84)
        if ok(court):
            row[3:6] = p.read8(court + 0x134), p.read8(court + 0x135), p.read8(court + 0x136)
            w = p.read32(court + 0x158)
            row[6] = p.read8(w + 0x50) if ok(w) else -1
    pr = p.read32(PRES)
    if ok(pr): row[7:9] = p.read8(pr + 0xd94), p.read8(pr + 0xd95)
    key = tuple(row[:2] + row[3:])
    if key != prev or row[2] in (0, 1, 2, 3):
        out.write(f"{v} {r:016x} ph {row[0]} {row[1]} tick {row[2]} court134 {row[3]} weather {row[4]} spawn {row[5]} "
                  f"held {row[6]} d94 {row[7]} d95 {row[8]}\n")
    prev = key
    if row[0] == 2: break
out.close()
print("done")
