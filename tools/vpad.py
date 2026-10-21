#!/usr/bin/env python3
"""Virtual gamepad for driving PCSX2 (or the port) unattended, via Linux uinput (no root needed when the user
has an ACL on /dev/uinput).

    tools/vpad.py serve &                  # creates the pad; keep it running (PCSX2 binds it as an SDL pad)
    tools/vpad.py send "press cross 120"   # hold Cross 120 ms
    tools/vpad.py send "stick l 0.0 -1.0" "sleep 300" "stick l 0 0"
    tools/vpad.py send < script.txt        # one command per line

Commands: press <btn> <ms> · down <btn> · up <btn> · stick <l|r> <x> <y> (−1..1, y down = +1) ·
          sleep <ms> · release (everything neutral)
Buttons (PS2 names → Xbox layout SDL sees): cross circle square triangle start l1 r1 l2 r2 l3 r3 up down left right.
`select` is refused while a shoulder, trigger or Start is held: PCSX2 hotkeys use Select + those combos (load/save
state, turbo); alone it reaches the game (the match's camera switch).
Timing is wall-clock, not frame-exact — use PCSX2 input recording for frame-exact replays.
The pad also takes rumble (FF_RUMBLE): every effect PCSX2 plays or stops on it is logged, one line each
(`<unix time> play|stop strong=<0..65535> weak=<0..65535> ms=<length>`), to $XDG_RUNTIME_DIR/hst-vpad<N>.rumble.
"""
import os, sys, time

# HST_PCSX2=N (parallel runs): pad N for PCSX2 copy N, its own fifo and USB id; tools/pcsx2-hst.sh makes copy N
# see only this pad (and the user's own PCSX2 none of them)
INST = os.environ.get("HST_PCSX2", "")
FIFO = os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), f"hst-vpad{INST}.fifo")
RUMBLE = os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), f"hst-vpad{INST}.rumble")

def serve():
    import fcntl
    global _lock
    _lock = open(FIFO + ".lock", "w")
    try: fcntl.flock(_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError: sys.exit(f"vpad: pad {INST or 0} is already being served")
    from evdev import UInput, AbsInfo, ecodes as e
    buttons = {
        # evdev's BTN_NORTH is BTN_X (the west face button) and BTN_WEST is BTN_Y (north): name them by Xbox letter
        "cross": e.BTN_A, "circle": e.BTN_B, "square": e.BTN_X, "triangle": e.BTN_Y,
        "start": e.BTN_START, "l1": e.BTN_TL, "r1": e.BTN_TR, "l3": e.BTN_THUMBL, "r3": e.BTN_THUMBR,
        "guide": e.BTN_MODE, "select": e.BTN_SELECT,
    }
    # (no swap for pad N: with it, "cross" reached PCSX2 copy 4 as ○ — pad word 0x4000, shot code 2, slice; N5)
    stick = AbsInfo(0, -32768, 32767, 16, 128, 0)
    trig = AbsInfo(0, 0, 255, 0, 0, 0)
    hat = AbsInfo(0, -1, 1, 0, 0, 0)
    caps = {
        e.EV_KEY: list(buttons.values()),
        e.EV_ABS: [(e.ABS_X, stick), (e.ABS_Y, stick), (e.ABS_RX, stick), (e.ABS_RY, stick),
                   (e.ABS_Z, trig), (e.ABS_RZ, trig), (e.ABS_HAT0X, hat), (e.ABS_HAT0Y, hat)],
        e.EV_FF: [e.FF_RUMBLE],
    }
    # identify as an Xbox 360 pad so SDL applies its standard mapping; pad N gets the pid.codes test id 1209:500N
    # (SDL guesses an Xbox layout from the BTN_SOUTH.. evdev codes)
    ui = UInput(caps, name=f"HST vpad {INST}" if INST else "Microsoft X-Box 360 pad", vendor=0x1209 if INST else 0x045E,
                product=0x5000 + int(INST) if INST else 0x028E, version=0x110, bustype=e.BUS_USB,
                max_effects=16)
    import threading
    threading.Thread(target=rumble_log, args=(ui, e), daemon=True).start()
    dpad = {"up": (e.ABS_HAT0Y, -1), "down": (e.ABS_HAT0Y, 1), "left": (e.ABS_HAT0X, -1), "right": (e.ABS_HAT0X, 1)}
    trigs = {"l2": e.ABS_Z, "r2": e.ABS_RZ}

    held = set()

    def key(name, down):
        if name == "select" and down and held & {"l1", "r1", "l2", "r2", "start"}:
            print("vpad: refusing select with a shoulder/trigger/Start held (PCSX2 hotkey combos)", flush=True)
            return
        if name != "select" and down and "select" in held and name in ("l1", "r1", "l2", "r2", "start"):
            print(f"vpad: refusing {name} while select is held (PCSX2 hotkey combos)", flush=True)
            return
        (held.add if down else held.discard)(name)
        if name in buttons:
            ui.write(e.EV_KEY, buttons[name], int(down))
        elif name in dpad:
            axis, v = dpad[name]
            ui.write(e.EV_ABS, axis, v if down else 0)
        elif name in trigs:
            ui.write(e.EV_ABS, trigs[name], 255 if down else 0)
        else:
            print(f"vpad: unknown button {name}", flush=True)
            return
        ui.syn()

    def run(line):
        p = line.split()
        if not p:
            return
        c = p[0].lower()
        if c == "press":
            key(p[1], True); time.sleep(int(p[2]) / 1000 if len(p) > 2 else 0.1); key(p[1], False)
        elif c in ("down", "up"):
            key(p[1], c == "down")
        elif c == "stick":
            ax = (e.ABS_X, e.ABS_Y) if p[1].lower() == "l" else (e.ABS_RX, e.ABS_RY)
            for a, v in zip(ax, p[2:4]):
                ui.write(e.EV_ABS, a, int(max(-1.0, min(1.0, float(v))) * 32767))
            ui.syn()
        elif c == "sleep":
            time.sleep(int(p[1]) / 1000)
        elif c == "release":
            for n in list(buttons) + list(dpad) + list(trigs):
                key(n, False)
            run("stick l 0 0"); run("stick r 0 0")
        else:
            print(f"vpad: unknown command {line!r}", flush=True)

    if not os.path.exists(FIFO):
        os.mkfifo(FIFO, 0o600)
    print(f"vpad: device {ui.device.path} ready; commands on {FIFO}", flush=True)
    try:
        while True:
            with open(FIFO) as f:  # blocks until a writer connects; one batch per open
                for line in f:
                    run(line.strip())
    finally:
        ui.close()

def rumble_log(ui, e):
    """Answer the kernel's force-feedback uploads (else the uploader blocks, up to 30 s each) and log each effect
    played/stopped. A bad event is logged and skipped, never ends the loop."""
    effects = {}
    log = open(RUMBLE, "a", buffering=1)
    for ev in ui.read_loop():
        try:
            rumble_event(ui, e, ev, effects, log)
        except Exception as x:
            print(f"vpad: rumble event {ev}: {x}", flush=True)

def rumble_event(ui, e, ev, effects, log):
    if ev.type == e.EV_UINPUT and ev.code == e.UI_FF_UPLOAD:
        up = ui.begin_upload(ev.value)
        up.retval = 0
        eff = up.effect
        effects[eff.id] = (eff.u.ff_rumble_effect.strong_magnitude, eff.u.ff_rumble_effect.weak_magnitude,
                           eff.ff_replay.length)
        ui.end_upload(up)
        if eff.id in playing:
            log.write("%.3f play strong=%d weak=%d ms=%d\n" % ((time.time(),) + effects[eff.id]))
    elif ev.type == e.EV_UINPUT and ev.code == e.UI_FF_ERASE:
        er = ui.begin_erase(ev.value)
        er.retval = 0
        effects.pop(er.effect_id, None)
        ui.end_erase(er)
    elif ev.type == e.EV_FF and ev.code in effects:
        if ev.value:
            playing.add(ev.code)
            log.write("%.3f play strong=%d weak=%d ms=%d\n" % ((time.time(),) + effects[ev.code]))
        else:
            playing.discard(ev.code)
            log.write("%.3f stop\n" % time.time())

playing = set()

def send(cmds):
    if not os.path.exists(FIFO):
        sys.exit("vpad: not running (start `tools/vpad.py serve`)")
    with open(FIFO, "w") as f:
        f.write("\n".join(cmds) + "\n")

if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "serve":
        serve()
    elif len(sys.argv) > 1 and sys.argv[1] == "send":
        send(sys.argv[2:] or sys.stdin.read().splitlines())
    else:
        sys.exit(__doc__)
