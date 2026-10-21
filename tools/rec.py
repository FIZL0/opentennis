"""Shared helpers for the recorders and research scripts (T6): the MIPS stub assembler the hooking recorders patch
into the game, the fixed-size fixture-sample reader, and f32/u32 bit casts.

    from rec import R, lw, sw, lui, ori, li, j, jr, br, assemble   # stubs
    for s in samples(path, SIZE): v, fields = vsync(s), cut(s, 4, [("gm", 0x100), ("ball", 0x290)])
    f32(p.read32(a)), u32(1.5), f32_at(buf, 0xe0, 3)

Stubs: an instruction is an int; `br(op, rs, rt, label)` is a branch to a label (beq 4 / bne 5) and a string in the
list is a label. `assemble(prog)` resolves the labels and returns the words. Add every delay slot yourself."""
import struct

# MIPS o32 register numbers (the EE's GPRs)
R = dict(zero=0, at=1, v0=2, v1=3, a0=4, a1=5, a2=6, a3=7, t0=8, t1=9, t2=10, t3=11, t4=12, t5=13, t6=14, t7=15,
         s0=16, s1=17, s2=18, s3=19, s4=20, s5=21, s6=22, s7=23, t8=24, t9=25, k0=26, k1=27, gp=28, sp=29, fp=30, ra=31)


def i(op, rs, rt, imm): return op << 26 | R[rs] << 21 | R[rt] << 16 | imm & 0xffff
def lw(rt, off, rs): return i(0x23, rs, rt, off)
def lbu(rt, off, rs): return i(0x24, rs, rt, off)
def sw(rt, off, rs): return i(0x2b, rs, rt, off)
def swc1(ft, off, rs): return 0x39 << 26 | R[rs] << 21 | ft << 16 | off & 0xffff
def lui(rt, imm): return i(0x0f, "zero", rt, imm)
def addiu(rt, rs, imm): return i(0x09, rs, rt, imm)
def ori(rt, rs, imm): return i(0x0d, rs, rt, imm)
def bne(rs, rt, words): return i(0x05, rs, rt, words)
def sll(rd, rt, sa): return R[rt] << 16 | R[rd] << 11 | sa << 6
def srl(rd, rt, sa): return R[rt] << 16 | R[rd] << 11 | sa << 6 | 0x02
def addu(rd, rs, rt): return R[rs] << 21 | R[rt] << 16 | R[rd] << 11 | 0x21
def subu(rd, rs, rt): return R[rs] << 21 | R[rt] << 16 | R[rd] << 11 | 0x23
def sltu(rd, rs, rt): return R[rs] << 21 | R[rt] << 16 | R[rd] << 11 | 0x2b
def slt(rd, rs, rt): return R[rs] << 21 | R[rt] << 16 | R[rd] << 11 | 0x2a
def j(a): return 0x02 << 26 | (a >> 2) & 0x3ffffff
def jr(rs): return R[rs] << 21 | 0x08
def li(rt, v): return [lui(rt, v >> 16), ori(rt, rt, v & 0xffff)]
def br(op, rs, rt, label): return ("br", op, rs, rt, label)


def assemble(prog):
    """Words from a list of ints, `br` tuples and label strings."""
    at, labels = 0, {}
    for x in prog:
        if isinstance(x, str): labels[x] = at
        else: at += 1
    out = []
    for x in prog:
        if isinstance(x, str): continue
        if isinstance(x, tuple):
            _, op, rs, rt, label = x
            x = i(op, rs, rt, labels[label] - len(out) - 1)
        out.append(x)
    return out


def samples(path, size, start=0):
    """The whole `size`-byte samples of a fixture file, from byte `start` on (a cut-off last one is dropped)."""
    d = open(path, "rb").read()
    for o in range(start, len(d) - size + 1, size):
        yield d[o:o + size]


def vsync(s):
    """A sample's leading u32: the vsync it was taken on."""
    return struct.unpack_from("<I", s)[0]


def cut(s, at, layout):
    """Consecutive named slices of `s` from byte `at`: layout [(name, size), …] → {name: bytes}."""
    out = {}
    for name, n in layout:
        out[name], at = s[at:at + n], at + n
    return out


def f32(u): return struct.unpack("<f", struct.pack("<I", u & 0xffffffff))[0]
def u32(f): return struct.unpack("<I", struct.pack("<f", f))[0]
def f32_at(b, o, n=None):
    """The f32 at byte `o` of `b`, or a tuple of `n` of them."""
    v = struct.unpack_from("<%df" % (n or 1), b, o)
    return v if n else v[0]


# The match fixture's sample (tools/record_p2m2.py writes it, tools/play_p2m2.py replays against it): u32 vsync, then
# these regions in order
MATCH_PAD = (0x2efb00, 0x90)            # pad manager: decoded state port0 at +0x30, port1 at +0x78 (18 bytes each)
MATCH_GLOBALS = (0x422f80, 0x180)       # gm pointer, player count 0x422fa4, ...
MATCH_GM, MATCH_BALL = 0x100, 0x290
MATCH_PLAYER = ((0x1380, 0x200), (0x3c00, 0x400))  # reach/params, positions/contact state
MATCH_RALLY = (0x3165f0, 0x50)          # judge inputs, faults, let, tiebreak, deuce/advantage


def match_regions(p):
    """(gm, [(address, size), …]) for one match sample, read through Pine `p`."""
    gm = p.read32(MATCH_GLOBALS[0])
    r = [MATCH_PAD, MATCH_GLOBALS, (gm, MATCH_GM), (p.read32(gm + 0x98), MATCH_BALL)]
    for k in range(4):
        pl = p.read32(gm + 0xa8 + 4 * k)
        r += [(pl + o, n) for o, n in MATCH_PLAYER]
    return gm, r + [(p.read32(gm + 0x88), MATCH_BALL), MATCH_RALLY]
