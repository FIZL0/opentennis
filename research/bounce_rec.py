"""Parser for tools/record_bounce.py captures: header {court, per model (ballbound, chakudan) MOR/MTA entry counts};
per frame vsync, bounce object, fx, ball, 3 contact records, per model {model, mesh, MOR/MTA players, entries}."""
import struct
import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from rec import samples, vsync, cut, f32_at


def frames(path):
    court, *counts = struct.unpack_from("<5I", open(path, "rb").read(20))
    msz = [0x80 + 0x60 + 0x60 + counts[2 * k] * 0x28 + counts[2 * k + 1] * 0x20 for k in range(2)]
    size = 8 + 0xfc0 + 0x100 + 0x290 + 0xf0 + sum(msz)
    for s in samples(path, size, 20):
        c = cut(s, 8, [("bo", 0xfc0), ("fx", 0x100), ("ball", 0x290), ("rec", 0xf0), ("m0", msz[0]), ("m1", msz[1])])
        yield dict(v=vsync(s), court=court, counts=counts, bo=c["bo"], fx=c["fx"], ball=c["ball"], rec=c["rec"],
                   models=(c["m0"], c["m1"]))


f32 = f32_at  # the f32 at byte o of b
i32 = lambda b, o: struct.unpack_from("<i", b, o)[0]
