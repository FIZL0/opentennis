#!/usr/bin/env python3
"""Per serve of a will_curve_rec.py recording: server, character, toss, ball kind (0 topspin, 2 flat), the server's
shot code (+0x3ee4) and timing offset, bend/turn/side, the CPU's target, the first bounce and how far it lands off the
straight hit→target line, the lateral offset at the apex, the direction change at the bounce.
Usage: will_curve_ana.py <rec.bin> [char] [--csv out.csv]"""
import math, struct, sys

SAMPLE = 4 + 0x90 + 0x180 + 0x100 + 0x290 + 4 * 0x600
SIZE = SAMPLE + 0x290 + 0x50
PL = 4 + 0x90 + 0x180 + 0x100 + 0x290

def f(b, o): return struct.unpack_from("<f", b, o)[0]
def i(b, o): return struct.unpack_from("<i", b, o)[0]
def v3(b, o): return struct.unpack_from("<3f", b, o)
def live(s): return s[SAMPLE:SAMPLE + 0x290]
def pf(s, k, o): return f(s, PL + k * 0x600 + 0x200 + o - 0x3c00)
def pi(s, k, o): return i(s, PL + k * 0x600 + 0x200 + o - 0x3c00)
def pu8(s, k, o): return s[PL + k * 0x600 + 0x200 + o - 0x3c00]
def launched(b): return b[0x58] == 0 and i(b, 0xac) == 0 and abs(f(b, 0x130)) + abs(f(b, 0x138)) > 0.05

def serves(data):
    fr = [data[k:k + SIZE] for k in range(0, len(data) - SIZE + 1, SIZE)]
    out = []
    for n in range(1, len(fr)):
        a, s = fr[n - 1], fr[n]
        if not launched(live(s)) or (i(a, 0) == i(s, 0) - 1 and launched(live(a))):
            continue
        k = next((k for k in range(2) if pu8(s, k, 0x3ec1) == 0 and pu8(s, k, 0x3fa6) == 3), -1)
        b = live(s)
        hit, tgt = v3(b, 0x70), v3(b, 0x80)
        path = [live(s)]
        m = n + 1
        while m < len(fr) and i(fr[m], 0) == i(fr[m - 1], 0) + 1 and i(live(fr[m]), 0xac) == i(live(fr[m - 1]), 0xac) + 1:
            path.append(live(fr[m])); m += 1
        bounce = next((j for j in range(1, len(path)) if i(path[j], 0x224) > i(path[0], 0x224)), None)
        # the CPU's/human's intended aim (+0x3e90) and the code a few frames on
        later = fr[min(n + 3, len(fr) - 1)]
        e = dict(vsync=i(s, 0), p=k, char=i(s, 4 + 0x90 + 0x28 + 4 * k), toss=pi(s, k, 0x3ea0), kind=i(b, 0x5c),
                 code=pi(s, k, 0x3ee4), code3=pi(later, k, 0x3ee4), off=pi(s, k, 0x3ecc), bend=f(b, 0x250),
                 turn=f(b, 0x1b0), side=v3(b, 0x90), curve=f(b, 0x254), frames=i(b, 0x260), wind=v3(b, 0x240),
                 hit=hit, tgt=tgt, aim=(pf(s, k, 0x3e90), pf(s, k, 0x3e94), pf(s, k, 0x3e98)), vel=v3(b, 0x130),
                 n=len(path), path=path, bounce=bounce)
        dx, dz = tgt[0] - hit[0], tgt[2] - hit[2]
        L = math.hypot(dx, dz)
        perp = lambda q: ((q[0] - hit[0]) * dz - (q[2] - hit[2]) * dx) / L  # signed distance off the line
        e["maxoff"] = max((perp(v3(q, 0xe0)) for q in path[:bounce or len(path)]), key=abs, default=0)
        if bounce:
            bp = v3(path[bounce], 0xe0)
            pb = v3(path[bounce - 1], 0xe0)
            e["bpos"], e["boff"] = bp, perp(bp)
            e["bmiss"] = math.hypot(bp[0] - tgt[0], bp[2] - tgt[2])
            vin, vout = v3(path[bounce - 1], 0x130), v3(path[bounce], 0x130)
            e["dturn"] = math.degrees(math.atan2(vout[0], vout[2]) - math.atan2(vin[0], vin[2]))
            if bounce + 10 < len(path):  # the line after the bounce vs the incoming one
                q = v3(path[bounce + 10], 0xe0)
                e["after"] = perp(q)
        out.append(e)
    return out

if __name__ == "__main__":
    data = open(sys.argv[1], "rb").read()
    want = int(sys.argv[2]) if len(sys.argv) > 2 and sys.argv[2].isdigit() else None
    rows = [e for e in serves(data) if want is None or e["char"] == want]
    for e in rows:
        print(f"v{e['vsync']} p{e['p']} c{e['char']} toss {e['toss']} kind {e['kind']} code {e['code']:#x}/{e['code3']:#x} "
              f"off {e['off']} bend {e['bend']:.3f} turn {math.degrees(e['turn']):.1f} side {e['side'][0]:.3f},{e['side'][2]:.3f} "
              f"hit {e['hit'][0]:.2f},{e['hit'][2]:.2f} tgt {e['tgt'][0]:.2f},{e['tgt'][2]:.2f} aim {e['aim'][0]:.2f},{e['aim'][2]:.2f} "
              f"frames {e['frames']} wind {e['wind'][0]:.3f},{e['wind'][2]:.3f} "
              + (f"bounce@{e['bounce']} {e['bpos'][0]:.2f},{e['bpos'][2]:.2f} off-line {e['boff']:+.3f} miss {e['bmiss']:.3f} "
                 f"maxoff {e['maxoff']:+.3f} dturn {e['dturn']:+.1f}" if e['bounce'] else f"no bounce ({e['n']} frames)"))
    if rows:
        kinds = {}
        for e in rows: kinds[e['kind']] = kinds.get(e['kind'], 0) + 1
        bent = [e for e in rows if e['bend'] != 0]
        print(f"{len(rows)} serves, kinds {kinds}, bent {len(bent)}")
        if bent:
            bo = [abs(e['boff']) for e in bent if e['bounce']]
            print(f"bend mean {sum(abs(e['bend']) for e in bent) / len(bent):.3f}; bounce off-line |mean| {sum(bo) / max(len(bo), 1):.3f} m, max {max(bo, default=0):.3f}")
