#!/usr/bin/env python3
"""B772: load an EU save state and save EE RAM 0x100000-0x480000 (the game overlay) to a file, for finding where a US
routine sits in the European build. Usage: eu_code_dump.py <slot> <out.bin>"""
import sys, time, os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.realpath(__file__)), "../../../../tools"))
from pine import Pine
p = Pine()
assert p.game_id() == "SCES-54535", p.game_id()
p.load_state(int(sys.argv[1])); time.sleep(5); p.pause()
open(sys.argv[2], "wb").write(b"".join(p.read_block(a, 0x10000) for a in range(0x100000, 0x480000, 0x10000)))
p.resume(); print("saved")
