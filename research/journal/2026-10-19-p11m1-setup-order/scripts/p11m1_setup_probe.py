#!/usr/bin/env python3
"""P11m1: the order of the match setup's draws. Loads a scratch save state sitting on the menus' "Confirm Settings"
screen with the cursor on "Start the match!" (made by hand: Fun Time Tennis → singles/doubles → controllers/COM →
characters → Start), presses ✕ (this copy's confirm button) and polls the AI generator and the match object's
generator (seed word and index) as fast as PINE allows until player 0 can serve, then dumps both generators and
`rand()`'s state. Usage (under tools/pcsx2.sh): p11m1_setup_probe.py <name> <slot> [s to follow on after: serves ✕ and logs the next point]. Writes context/p11m1/<name>.json."""
import json, os, sys, threading, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
from pine import Pine
import pick

AI, GM = 0x427130, 0x422f80
name, slot = sys.argv[1], int(sys.argv[2])
p = Pine()
p.load_state(slot)
time.sleep(2)
def mt(base): return p.read32(base + 4), p.read32(base + 0x9c4)
def rand(): return p.read64(p.read32(0x1b80f0) + 0xa8)
ai0, rand0 = p.read_block(AI, 0x9c8).hex(), rand()
log = []
threading.Thread(target=lambda: pick.vpad("press cross 300"), daemon=True).start()
last, t0, ready = None, time.time(), None
follow = float(sys.argv[3]) if len(sys.argv) > 3 else 0
while time.time() - t0 < 150:
    gm = p.read32(GM)
    sh = gm and p.read32(gm + 0x80)  # the match object's generator (a pointer)
    cur = (mt(AI), sh and mt(sh), sh)
    if cur != last:
        log.append([round(time.time() - t0, 4), cur[0], cur[1], sh, rand()]); last = cur
    pl = gm and p.read32(gm + 0xa8)
    if pl and p.read8(gm + 0x55) == 2 and p.read8(pl + 0x3fa4) == 1 and not ready:
        ready = time.time()  # player 0 can serve: serve (✕ three times) and follow into the next point
        log.append([round(ready - t0, 4), "serve-ready", p.read32(0x42303c)])
        threading.Thread(target=lambda: pick.vpad(*["sleep 3000", "press cross 300"] * 6), daemon=True).start()
    if ready and time.time() - ready > follow: break
gm = p.read32(GM)
out = dict(log=log, seed=p.read32(0x42303c), players=p.read32(0x422fa4), ai_before=ai0, rand_before=rand0,
           rand_after=rand(), ai=p.read_block(AI, 0x9c8).hex(), main=gm and p.read_block(p.read32(gm + 0x80), 0x9c8).hex())
d = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../context/p11m1")
os.makedirs(d, exist_ok=True)
json.dump(out, open(os.path.join(d, f"{name}.json"), "w"))
print(len(log), "changes; seed", out["seed"], "gm", hex(gm))
