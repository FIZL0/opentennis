#!/usr/bin/env python3
"""research/split/summarize.py LOG… — R12: one line per HST_FPS=1 bench log, averaged over the 5 s reports from
FROM s on (default 15, past the intro): fps, worst 1 % low, CPU ms/frame, game ms, GPU ms of the main passes."""
import re, sys

FROM = 15
PASSES = ["main_opaque_pass_3d", "main_transparent_pass_3d", "msaa_writeback", "per_view_shadow_pass", "upscaling"]


def one(path):
    fps, low, cpu, game, passes = [], [], [], [], {p: [] for p in PASSES}
    at = None
    keep = False
    for line in open(path, errors="replace"):
        if m := re.match(r"fps ([\d.]+) .* at (\d+) s", line):
            at = int(m[2])
            keep = at >= FROM
            if keep:
                fps.append(float(m[1]))
        elif keep and (m := re.search(r"\| 1% low ([\d.]+) fps", line)):
            low.append(float(m[1]))
        elif keep and (m := re.match(r"cpu ([\d.]+) ms/frame", line)):
            cpu.append(float(m[1]))
        elif keep and (m := re.match(r"frame split ms: .* game ([\d.]+)", line)):
            game.append(float(m[1]))
        elif m := re.match(r"passes elapsed_gpu ms at (\d+) s: (.*)", line):
            if int(m[1]) >= FROM:
                for part in m[2].split(", "):
                    name, _, v = part.rpartition(" ")
                    if name in passes:
                        passes[name].append(float(v))
    avg = lambda v: sum(v) / len(v) if v else float("nan")
    p = " ".join(f"{n.replace('main_', '').replace('_pass_3d', '').replace('per_view_', '')} {avg(v):.2f}" for n, v in passes.items())
    return f"{path.rsplit('/', 1)[-1]:<18} fps {avg(fps):6.1f} (min {min(fps, default=0):5.1f}) 1%low {min(low, default=0):4.0f} cpu {avg(cpu):5.2f} game {avg(game):5.2f} | gpu ms: {p} | n={len(fps)}"


for f in sys.argv[1:]:
    print(one(f))
