#!/usr/bin/env python3
"""P11m2: what moves the AI generator between the match setup and the first point. Like p11m1_setup_probe.py (loads a
scratch save state on "Confirm Settings", presses ✕), but logs at every change: the AI generator's index, the match
object's phase byte and cut-scene flag, every player's state byte and its AI object's state bytes, until player 0 can
serve (+ `follow` s, serving with ✕ when following). Usage (under tools/pcsx2.sh): p11m2_intro_probe.py <name> <slot> [follow s]. Writes
context/p11m2/<name>.json."""
import json, os, sys, threading, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../tools"))
from pine import Pine
import pick

AI, GM, PLAYERS = 0x427130, 0x422f80, 0x422fa4
name, slot = sys.argv[1], int(sys.argv[2])
follow = float(sys.argv[3]) if len(sys.argv) > 3 else 0
p = Pine()
p.load_state(slot)
time.sleep(2)
log, last, t0, ready = [], None, time.time(), None
threading.Thread(target=lambda: pick.vpad("press cross 300"), daemon=True).start()
while time.time() - t0 < 170:
    gm = p.read32(GM)
    idx = p.read32(AI + 0x9c4)
    snap = [idx]
    if gm:
        n = p.read32(PLAYERS)
        snap += [p.read8(gm + 0x55), p.read8(gm + 0x56), p.read8(gm + 0x344), p.read8(0x423040)]
        for i in range(n if n in (2, 4) else 0):
            pl = p.read32(gm + 0xa8 + 4 * i)
            if not pl:
                snap.append(None); continue
            ai = p.read32(pl + 0x80)
            snap.append([p.read8(pl + 0x3fa4), p.read8(pl + 0x13f5)] + ([p.read8(ai + 0x54), p.read8(ai + 0x55),
                         p.read8(ai + 0x56), p.read8(ai + 0x57)] if ai else []))
    if snap != last:
        log.append([round(time.time() - t0, 4)] + snap); last = snap
    pl = gm and p.read32(gm + 0xa8)
    if pl and p.read8(gm + 0x55) == 2 and p.read8(pl + 0x3fa4) == 1 and not ready:
        ready = time.time()
        log.append([round(ready - t0, 4), "serve-ready", p.read32(0x42304c)])
        if follow:  # serve (✕ three times, then again in case) and follow into the next point
            threading.Thread(target=lambda: pick.vpad(*["sleep 3000", "press cross 300"] * 6), daemon=True).start()
    if ready and time.time() - ready > follow: break
d = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../context/p11m2")
os.makedirs(d, exist_ok=True)
json.dump(dict(log=log, server=p.read32(0x42304c)), open(os.path.join(d, f"{name}.json"), "w"))
print(len(log), "changes; ready" if ready else "changes; NOT ready")
