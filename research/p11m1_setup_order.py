#!/usr/bin/env python3
"""P11m1: the match setup's draw order from context/p11m1/<name>.json (research/p11m1_setup_probe.py).
The poll log holds (time, AI generator (word 0, index), shared generator (word 0, index), its address, rand state)
at every change. The shared generator's seed is word 0 the first time it shows with index 624 (fresh); replaying it
gives which shared draw each AI reseed took. Prints the shared generator's draw count at each change (one line per
change: draws so far, AI seed/index) and checks the final AI and shared states against the replay."""
import json, os, struct, sys

def init(seed):
    mt = [seed & 0xffffffff]
    for i in range(1, 624):
        mt.append((1812433253 * (mt[-1] ^ (mt[-1] >> 30)) + i) & 0xffffffff)
    return mt

def twist(mt):
    mt = mt[:]
    for i in range(624):
        y = (mt[i] & 0x80000000) | (mt[(i + 1) % 624] & 0x7fffffff)
        mt[i] = mt[(i + 397) % 624] ^ (y >> 1) ^ (0x9908b0df if y & 1 else 0)
    return mt

def temper(y):
    y ^= y >> 11; y ^= (y << 7) & 0x9d2c5680; y ^= (y << 15) & 0xefc60000; return y ^ (y >> 18)

def outputs(seed, n):
    mt, out = init(seed), []
    while len(out) < n:
        mt = twist(mt); out += [temper(y) for y in mt]
    return out[:n]

def words(h):
    b = bytes.fromhex(h)
    return list(struct.unpack("<624I", b[4:4 + 2496])), struct.unpack("<I", b[0x9c4:0x9c8])[0]

d = json.load(open(os.path.join(os.path.dirname(__file__), "..", "context", "p11m1", f"{sys.argv[1]}.json")))
log = [l for l in d["log"] if l[1] != "serve-ready"]
seeds = []  # (log index, shared seed): every fresh shared generator
for k, l in enumerate(log):
    if l[2] and l[2][1] == 624 and (not seeds or l[2][0] != seeds[-1][1]) and not (seeds and l[2][0] == twist(init(seeds[-1][1]))[0]):
        seeds.append((k, l[2][0]))
print("shared seeds:", [s for _, s in seeds])
seed = seeds[0][1]
out = outputs(seed, 1248)
for k, l in enumerate(log):
    if seeds[1:] and k >= seeds[1][0]: break
    t, (aw, ai), sh = l[0], l[1], l[2]
    note = ""
    if ai == 624 and aw != 1:
        note = f"AI reseeded with shared draw #{out.index(aw)}" if aw in out else f"AI reseeded with {aw}: not a shared draw"
    print(f"{t:9.3f}  shared index {sh[1] if sh else '-':>4}  AI index {ai:>4}  {note}")
ai, ai_i = words(d["ai"])
print("final AI index", ai_i)
