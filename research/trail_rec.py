"""Parse fixtures/trails_s05.bin (tools/record_trails.py)."""
import struct
import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from rec import samples
P = 0xa8 + 48 * 0x30 + 0x90 + 0x40
SIZE = 0xc + 0x100 + 4 * P
def frames(path='fixtures/trails_s05.bin'):
    for s in samples(path, SIZE):
        v, clock = struct.unpack_from('<Ii', s, 0)
        pl = []
        for k in range(4):
            o = 0x10c + k * P
            pl.append(dict(t=s[o:o + 0xa8], pts=s[o + 0xa8:o + 0xa8 + 0x900], mot=s[o + 0x9a8:o + 0xa38], mat=s[o + 0xa38:o + P]))
        yield dict(v=v, clock=clock, fx=s[0xc:0x10c], pl=pl)
