#!/usr/bin/env python3
"""Record the live ball every frame from a save-state load (bot games: slot 5).
Usage: record_live.py <slot> <frames> <out.bin> [--vel vsync,vx,vy,vz] [--path]. A PCSX2 copy runs at 1x; the user's own PCSX2: run it slowed (NominalScalar 0.25).
Sample = u32 vsync + live ball *(gm+0x88) 0x290 + predictor *(gm+0x98) 0x290 + rally block 0x3165f0 0x40.
--vel: right after sampling that vsync, overwrite the live ball's velocity once (net cord, post and net tests;
pick it by stepping hst-sim from the same state); everything after is the game's own physics. That vsync goes to
<out.bin>.poke (replays skip the next frames); none is written if the sample was missed.
--path (P11n1): also write <out.bin>.path, the AI's ball-path record *(gm+0xa4) for every sample (same frames, same
order; the main file's format is unchanged). Path sample = u32 vsync; the record +0x50..+0x130 (0xe0: entry buffer,
end, start, active byte, mode byte, line gap, the four shot records); its +0x1f0..+0x200 (0x10, claim slot/frame);
the AI's shared copy 0x427108..0x427118 (stamp, -, count, -); i32 first, u32 n, then n entries of 0x30 = the record's
entries [first, end), the tip (at most PATH_TIP; first = max(0, end - PATH_TIP)). first = -1, n = 0: the tip read
ticked (the header is still this frame's)."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, VEL = 0x1d5780, 0x422f80, 0x130
COPY, PATH_TIP = 0x427108, 32
p, want, out = Pine(step=True), int(sys.argv[2]), open(sys.argv[3], "wb")
poke = sys.argv[sys.argv.index("--vel") + 1].split(",") if "--vel" in sys.argv else None
path_out = open(sys.argv[3] + ".path", "wb") if "--path" in sys.argv else None
p.load_state(int(sys.argv[1]))
time.sleep(0.3)
gm = p.read32(GM_PTR)
ball = p.read32(gm + 0x88)
r = [(ball, 0x290), (p.read32(gm + 0x98), 0x290), (0x3165f0, 0x40)]
main = 2 * 0x290 + 0x40
if path_out:
    path = p.read32(gm + 0xa4)
    r += [(path + 0x50, 0xe0), (path + 0x1f0, 0x10), (COPY, 0x10)]
last, n = p.read32(VSYNC), 0
while n < want:
    v = p.next_frame(last)
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    a = p.settle(r, v)
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    out.write(struct.pack("<I", v) + a[:main])
    if path_out:
        head = a[main:]
        buf, end = struct.unpack_from("<Ii", head, 0)
        first = max(0, end - PATH_TIP)
        tip = p.settle([(buf + first * 0x30, (end - first) * 0x30), (path + 0x50, 8)], v) if end > 0 else head[:8]
        if tip is None or tip[-8:] != head[:8]:
            print(f"missed frame {v}'s path tip", flush=True)
            first, tip = -1, b""
        path_out.write(struct.pack("<I", v) + head + struct.pack("<iI", first, len(tip[:-8]) // 0x30) + tip[:-8])
    n += 1
    if n % 1200 == 0: print(n, flush=True)
    if poke and v == int(poke[0]):
        for k, c in enumerate(poke[1:]):
            p.write32(ball + VEL + 4 * k, struct.unpack("<I", struct.pack("<f", float(c)))[0])
        open(sys.argv[3] + ".poke", "w").write(f"{v}\n")
        print(f"velocity written after vsync {v}", flush=True)
print("done", n)
