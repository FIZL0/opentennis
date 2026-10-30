#!/usr/bin/env python3
"""B700a: does the CPU serve's flat outcome follow its serve timing error, the RNG left alone?
Usage (HST_PCSX2=N): b700a_err_poke.py <slot> <serves> <scratch slot> [errs, default -4,0,1,2,3,4] [mash period]
Loads <slot>, mashes Cross for P1 until a CPU serve's toss (singles AI serve sub-state 3), saves it to <scratch>,
then replays that serve once per error: load the scratch, keep the AI's serve error poked (+0x230, the copy the press
reads is made from it as the pick is taken; the toss draws it again, so it is re-poked every frame until then), run to the launch and print the press countdown, the swing's timing offset, the stick
the AI wrote on the contact frame (+0x17d4), the pressed button after contact (+0x3ee4: 1 ✕, 2 ○, 8 flat), the
aim target and the ball kind. The first replay is run twice with the logged error to show the replay is
deterministic. Then goes back to the scratch state's serve, plays it out with the masher and waits for the next."""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "../tools"))
from pine import Pine, VSYNC

p = Pine()
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
s32 = lambda a: struct.unpack("<i", struct.pack("<I", p.read32(a)))[0]
u32 = lambda v: v & 0xffffffff
slot, want, scratch = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
errs = [int(x) for x in (sys.argv[4] if len(sys.argv) > 4 else "-4,0,1,2,3,4").split(",")]
period = sys.argv[5] if len(sys.argv) > 5 else "0.5"
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")


def masher():
    return subprocess.Popen([sys.executable, "-c", f"""
import subprocess, time
while True:
    subprocess.run(['tools/vpad.py', 'send', 'press cross 80'], capture_output=True); time.sleep({period})
"""], cwd=ROOT)


def load(s):
    v0 = p.read32(VSYNC)
    p.load_state(s)
    t = time.time()
    while p.read32(VSYNC) >= v0: assert time.time() - t < 5, "load didn't land"  # the state's clock is earlier
    last = p.read32(VSYNC)
    while p.read32(VSYNC) < last + 3: pass
    gm = p.read32(0x422f80)
    pl = [p.read32(gm + 0xa8 + 4 * k) for k in range(2)]
    return gm, pl, [p.read32(x + 0x80) for x in pl]


def serve_out(gm, pl, ai, k, err):
    """Run the CPU k's serve from its toss to the launch; poke its error first (None: leave it)."""
    if err is not None:
        assert p.read8(ai + 0x54) == 1 and p.read8(ai + 0x55) == 3, "not in the toss any more"
    last, press, contact = p.read32(VSYNC), None, None
    for _ in range(400):
        last = p.next_frame(last)
        sub = p.read8(ai + 0x55)
        if err is not None and sub == 3 and s32(ai + 0x230) != err:
            p.write32(ai + 0x230, u32(err))  # the toss draws the timing errors again: keep it poked until the pick
        if press is None and sub == 5:
            press = (s32(ai + 0xb4), s32(ai + 0xb8), p.read8(ai + 0xb2))
        if press and p.read8(ai + 0x55) == 5 and s32(pl[k] + 0x3ec4) == -1 and contact is None:
            contact = (p.read16(pl[k] + 0x17d4), p.read8(pl[k] + 0x3ee4), s32(pl[k] + 0x3fa0),
                       f(pl[k] + 0x3e90), f(pl[k] + 0x3e98))
        b = p.read32(gm + 0x88)
        if contact and p.read8(b + 0x58) == 0 and s32(b + 0xac) == 0:
            return press, contact, s32(b + 0x5c)
    return press, contact, None


gm, pl, ai = load(slot)
m = masher()
done, last = 0, p.read32(VSYNC)
try:
    while done < want:
        last = p.next_frame(last)
        if p.read32(0x422f80) != gm: print("match over", flush=True); break
        k = next((k for k in range(2) if p.read32(pl[k] + 0x84) and p.read8(ai[k] + 0x54) == 1 and p.read8(ai[k] + 0x55) == 3), None)
        if k is None: continue
        m.kill()
        p.save_state(scratch)
        time.sleep(1.0)
        logged = s32(ai[k] + 0x230)
        print(f"-- serve {done + 1} p{k} v{last} err before the toss {logged}", flush=True)
        for e in [None, None] + errs:
            g2, pl2, ai2 = load(scratch)
            press, contact, kind = serve_out(g2, pl2, ai2[k], k, e)
            st = contact[0] if contact else 0
            sx, sz = ((st >> 8) - 0x80) / 127, ((st & 0xff) - 0x80) / 127
            print(f"  err {'logged' if e is None else e:>6}: press {press} stick {sx:+.2f},{sz:+.2f} "
                  f"button {contact[1] if contact else None} offset {contact[2] if contact else None} "
                  f"aim {tuple(round(x, 3) for x in contact[3:]) if contact else None} kind {kind}", flush=True)
        done += 1
        gm, pl, ai = load(scratch)
        m = masher()
        # play this serve out before looking for the next toss
        while p.read8(ai[k] + 0x54) == 1 and p.read32(0x422f80) == gm: last = p.next_frame(p.read32(VSYNC))
        last = p.read32(VSYNC)
finally:
    m.kill()
