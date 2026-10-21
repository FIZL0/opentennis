"""Parser for tools/record_flight.py captures: per frame vsync, fx, flight ribbon, its samples, ball glow, ball,
court marker 8."""
import struct
import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from rec import samples, vsync, cut

SIZE = 8 + 0x100 + 0xb0 + 0x320 + 0x70 + 0x290 + 0x40


def frames(path):
    for s in samples(path, SIZE):
        yield dict(v=vsync(s), **cut(s, 8, [("fx", 0x100), ("fl", 0xb0), ("pts", 0x320), ("glow", 0x70), ("ball", 0x290),
                                            ("mark", 0x40)]))
