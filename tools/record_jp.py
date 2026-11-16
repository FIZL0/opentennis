#!/usr/bin/env python3
"""Record the Japanese release's (SCPS-15113) match frame by frame from a save-state load (M163a1a1), for the
Japanese build's serve grades. Boot it with `HST_ISO=fixtures/discs/SCPS-15113.iso tools/pcsx2-hst.sh` and copy a
state (fixtures/m163a1a/jp-bot-*.p2s) to the copy's sstates as `SCPS-15113 (A35F68F7).0N.p2s`.
Usage: record_jp.py <slot> <frames> <out.bin> [ai]. Writes <out.bin> (frames) and <out.bin>.locks (serve locks);
`ai` also records the CPU reaction draws' inputs (below).

The Japanese program keeps the US layout with shifts: its globals sit 0x1900 above the US ones (match object pointer
0x424880, player count +0x24, characters +0x28.., server +0xcc, shot count +0xe0), its frame clock is 0x1d7b80, and
its player record is smaller (one timing table, no serve tables: grades +0x14e0, length +0x151c, bias +0x13f0; the
swing state 0x290 below the US one up to +0x3c90, 0x2a0 below past it).

Header: u32 player count, + 0x100 with `ai`. Frame sample: u32 vsync, globals 0x424880 (0x180), the match object (0x100), then per
player its +0x12a0..+0x1530 (end, character, TParam values, ideal heights, timing table) and +0x3bc0..+0x3d40
(swing state: toss kind +0x3c10, branch +0x3c31, lock index +0x3c34, grade +0x3c58, bias +0x3cf8, offset +0x3d00).
Lock sample, when the server's lock index turns non-negative: u32 vsync, u32 server, the server's path record
header (0x60: entries +0x50, count +0x54, start +0x58) and its first 0x3c entries from the start (0x30 each).
With `ai` each frame sample goes on with the AI generator (MT19937 at 0x428a30, 0x9d0) and each player's AI record
(*(player+0x80), 0x260: incoming branch +0x124, reaction wait +0x238).
A frame that ticks mid-read is skipped (logged). Stops when the match object changes."""
import struct, sys, time
import pine
from pine import Pine

pine.VSYNC = 0x1d7b80                       # the Japanese program's frame clock (60/s; the US one is 0x1d5780)
GLOBALS, GM_PTR, COUNT, SERVER = 0x424880, 0x424880, 0x4248a4, 0x42494c
PLAYER = ((0x12a0, 0x290), (0x3bc0, 0x180))
LOCK, PATH, ENTRIES = 0x3c34, 0x13cc, 0x3c

slot, want, out = int(sys.argv[1]), int(sys.argv[2]), open(sys.argv[3], "wb")
ai = sys.argv[4:5] == ["ai"]
AI_MT, AI_SIZE = 0x428a30, 0x260
locks = open(sys.argv[3] + ".locks", "wb")
p = Pine(step=True)
p.load_state(slot)
time.sleep(0.5)
gm0, count = p.read32(GM_PTR), p.read32(COUNT)
players = [p.read32(gm0 + 0xa8 + 4 * i) for i in range(count)]
out.write(struct.pack("<I", count + (0x100 if ai else 0)))
regions = [(GLOBALS, 0x180), (gm0, 0x100)] + [(pl + o, n) for pl in players for o, n in PLAYER]
if ai: regions += [(AI_MT, 0x9d0)] + [(p.read32(pl + 0x80), AI_SIZE) for pl in players]
FRAME = sum(n for _, n in regions)
lock_at = 4 + 0x180 + 0x100 + 0x290 + LOCK - 0x3bc0   # player 0's lock index in a sample (past the vsync)

last, n, missed, prev, nlocks = p.read32(pine.VSYNC), 0, 0, None, 0
while n < want:
    v = p.next_frame(last)
    if v != last + 1 and n: print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    if p.read32(GM_PTR) != gm0: sys.exit(f"match object gone at vsync {v}, {n} samples")
    a = p.settle(regions, v)
    if a is None:
        missed += 1
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        if missed > 300: sys.exit(f"300 frames in a row missed at vsync {v}, {n} samples")
        continue
    missed = 0
    s = struct.pack("<I", v) + a
    out.write(s)
    server = struct.unpack_from("<I", a, SERVER - GLOBALS)[0]
    if server < count:
        idx = struct.unpack_from("<i", s, lock_at + server * (0x290 + 0x180))[0]
        if idx >= 0 and prev is not None and prev < 0:
            rec = p.read32(players[server] + PATH)
            head = p.read_block(rec, 0x60)
            arr, start = struct.unpack_from("<I", head, 0x50)[0], struct.unpack_from("<I", head, 0x58)[0]
            locks.write(struct.pack("<II", v, server) + head + p.read_block(arr + start * 0x30, ENTRIES * 0x30))
            locks.flush()
            nlocks += 1
            print(f"vsync {v}: server {server} locked index {idx}", flush=True)
        prev = idx
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n, "frames,", nlocks, "serve locks")
