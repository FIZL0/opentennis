#!/usr/bin/env python3
"""Record P1's presses around the edges of a run, for the dive's eligibility frame (P120). From slot 3/4 (P1 human,
driven by the virtual pad), each ball coming to P1's half gets a plan: run across toward where it crosses P1's line and
let go of the stick on the press (`stop`), keep running through the press (`run`), let go one write before the press
(`late`), or stand off to the side and push the stick with the press (`start`). Wall-clock timing scatters the
press around the run's edge; the pad word in each sample says what the game saw.
Needs tools/vpad.py serve (pcsx2-hst.sh starts it for HST_PCSX2 copies). Usage: record_dive_frame.py <slot> <out.bin>
<frames>. Sample (76 bytes) = u32 vsync, P1's pad (8 bytes from the pad manager's port 0 block), then from player 0:
0x3e58 (8), 0x3ec0 (8), 0x3f58 (8), 0x3fa0 (8), 0x3d70 (16), 0x3e00 (16)."""
import os, random, struct, sys, time
from pine import Pine
from rec import f32
from vpad import FIFO

VSYNC, GM_PTR, PAD = 0x1d5780, 0x422f80, 0x2efb00 + 0x30
PLAYER = ((0x3e58, 8), (0x3ec0, 8), (0x3f58, 8), (0x3fa0, 8), (0x3d70, 16), (0x3e00, 16))  # PINE reads 8-byte blocks

pad = os.open(FIFO, os.O_RDWR)
def send(*cmds): os.write(pad, ("\n".join(cmds) + "\n").encode())
releases = []
def press(btn, *with_cmds): send(*with_cmds, f"down {btn}"); releases.append((n + 3, btn))

rng = random.Random(int(os.environ.get("SEED", "120")))
p, out, want = Pine(step=True), open(sys.argv[2], "wb"), int(sys.argv[3])
send("release")
p.load_state(int(sys.argv[1]))
time.sleep(2)
gm = p.read32(GM_PTR)
me, ball = p.read32(gm + 0xa8), p.read32(gm + 0x88)
r = [(PAD, 8)] + [(me + o, k) for o, k in PLAYER]
last, n, idle, prev, held, plan, hit_seen, counts = p.read32(VSYNC), 0, 0, None, (0, 0), None, None, {}
while n < want:
    v = p.next_frame(last)
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    a = p.read_regions(r)
    while True:
        b = p.read_regions(r)
        if a == b and p.read32(VSYNC) == v: break
        a = b
    out.write(struct.pack("<I", v) + a)
    out.flush()
    n += 1
    while releases and releases[0][0] <= n: send(f"up {releases.pop(0)[1]}")
    pos, vel = [f32(p.read32(ball + 0x120 + 4 * j)) for j in range(3)], [f32(p.read32(ball + 0x130 + 4 * j)) for j in range(3)]
    acc = [x - y for x, y in zip(vel, prev)] if prev else [0, 0.01, 0]
    prev = vel
    mx, mz = f32(p.read32(me + 0x3d70)), f32(p.read32(me + 0x3d78))
    hit = tuple(p.read32(ball + o) for o in (0x70, 0x74, 0x78))
    idle += 1
    # when and where the ball next crosses P1's depth, stepping its current acceleration
    eta, qx = None, None
    if vel[2] * mz > 0 and abs(pos[2]) < abs(mz) + 1:
        q, u = list(pos), list(vel)
        for k in range(1, 120):
            u = [x + y for x, y in zip(u, acc)]
            q = [x + y for x, y in zip(q, u)]
            if abs(q[2]) >= abs(mz):
                eta, qx = k, q[0]
                break
    if eta is not None and hit != hit_seen:
        hit_seen, idle = hit, 0
        mode = rng.choice(("stop", "run", "late", "start"))
        side = rng.choice((-1, 1))
        plan = dict(mode=mode, side=side, at=rng.randint(6, 14), run=0 if mode == "start" else rng.randint(6, 20),
                    off=rng.uniform(1.6, 2.6) if mode == "start" else rng.uniform(3.0, 4.5), done=False)
    s = 1 if mz > 0 else -1  # the camera sits behind P1: on the near half stick right is −x
    stick = (0, 0)
    if plan and eta is not None and not plan["done"]:
        lateral = lambda dx: (round(-s * (1 if dx > 0 else -1), 1), 0)
        if eta <= plan["at"]:
            plan["done"] = True
            counts[plan["mode"]] = counts.get(plan["mode"], 0) + 1
            if plan["mode"] == "stop":
                held = (0, 0); press("cross", "stick l 0 0")
            elif plan["mode"] == "run":
                stick = held; press("cross")
            elif plan["mode"] == "late":
                send("stick l 0 0"); held = (0, 0); press("cross")
            else:
                held = lateral(qx - mx); press("cross", f"stick l {held[0]} {held[1]}")
            print(f"vsync {v}: {plan['mode']} press, ball {eta} frames off, {abs(qx - mx):.2f} m across", flush=True)
            continue
        if eta <= plan["at"] + plan["run"]:
            stick = lateral(qx - mx)
        else:
            dx = qx + plan["side"] * plan["off"] - mx
            stick = lateral(dx) if abs(dx) > 0.3 else (0, 0)
    elif plan and plan["done"] and eta is not None:
        stick = held
    if stick != held:
        held = stick
        send(f"stick l {stick[0]} {stick[1]}")
    if idle > (40 if p.read8(me + 0x3ec1) == 5 else 120):  # point over / serving: skip, toss, hit
        idle = 0
        press("cross")
    if n % 1200 == 0: print(n, flush=True)
send("release")
print("done", n, counts)
