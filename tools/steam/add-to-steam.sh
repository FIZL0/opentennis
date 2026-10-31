#!/bin/sh
# add-to-steam.sh — adds this game to Steam as a non-Steam game, with its artwork (capsule, wide capsule, hero, logo,
# icon from steam/ beside it). Run it once from the unpacked folder (Steam Deck: in Desktop Mode, double-click it).
# Steam is closed while it writes (it rewrites shortcuts.vdf on exit) and started again after. Running it again
# updates the same shortcut, e.g. after moving the folder.
NAME=OpenTennis
EXE=opentennis
here=$(cd "$(dirname "$0")" && pwd)
command -v python3 >/dev/null || { echo "add-to-steam needs python3" >&2; exit 1; }
exec python3 - "$here" "$NAME" "$EXE" <<'EOF'
import os, sys, struct, zlib, shutil, subprocess, time, glob

here, name, exe_name = sys.argv[1:4]
exe = os.path.join(here, exe_name)

# --- binary VDF (shortcuts.vdf): 0x00 map, 0x01 string, 0x02 int32, 0x08 end of map
def parse(b, i=0):
    d = {}
    while True:
        t = b[i]; i += 1
        if t == 8: return d, i
        j = b.index(0, i); k = b[i:j].decode('utf-8', 'replace'); i = j + 1
        if t == 0: d[k], i = parse(b, i)
        elif t == 1: j = b.index(0, i); d[k] = b[i:j].decode('utf-8', 'replace'); i = j + 1
        elif t == 2: d[k] = struct.unpack_from('<i', b, i)[0]; i += 4
        else: raise ValueError(f'shortcuts.vdf: unknown type {t}')

def dump(d):
    out = b''
    for k, v in d.items():
        kb = k.encode() + b'\0'
        if isinstance(v, dict): out += b'\0' + kb + dump(v)
        elif isinstance(v, int): out += b'\2' + kb + struct.pack('<i', v)
        else: out += b'\1' + kb + str(v).encode() + b'\0'
    return out + b'\x08'

def add(vdf_bytes, appid_hint):
    """Return (new vdf bytes, unsigned appid). Reuses the entry for the same game if there is one."""
    root = parse(vdf_bytes)[0] if vdf_bytes else {'shortcuts': {}}
    sc = root.setdefault('shortcuts', {})
    entry = next((e for e in sc.values() if e.get('AppName') == name or os.path.basename(e.get('Exe', '').strip('"')) == exe_name), None)
    if entry is None:
        entry = sc[str(len(sc))] = {'appid': appid_hint - (1 << 32)}
    entry.update({
        'AppName': name, 'Exe': f'"{exe}"', 'StartDir': f'"{here}"', 'icon': os.path.join(here, 'steam', 'icon.png'),
        'ShortcutPath': '', 'LaunchOptions': entry.get('LaunchOptions', ''), 'IsHidden': 0, 'AllowDesktopConfig': 1,
        'AllowOverlay': 1, 'OpenVR': 0, 'Devkit': 0, 'DevkitGameID': '', 'DevkitOverrideAppID': 0,
        'LastPlayTime': entry.get('LastPlayTime', 0), 'FlatpakAppID': '', 'tags': entry.get('tags', {}),
    })
    entry.setdefault('appid', appid_hint - (1 << 32))
    return dump(root), entry['appid'] & 0xffffffff

def install(userdir):
    cfg = os.path.join(userdir, 'config'); os.makedirs(os.path.join(cfg, 'grid'), exist_ok=True)
    path = os.path.join(cfg, 'shortcuts.vdf')
    old = open(path, 'rb').read() if os.path.exists(path) else b''
    if old: shutil.copy(path, path + '.bak')
    hint = zlib.crc32(f'"{exe}"{name}'.encode()) | 0x80000000  # what Steam itself picks for a new shortcut
    new, appid = add(old, hint)
    open(path, 'wb').write(new)
    art = {'grid.png': f'{appid}p.png', 'wide.png': f'{appid}.png', 'hero.png': f'{appid}_hero.png', 'logo.png': f'{appid}_logo.png'}
    for src, dst in art.items():
        shutil.copy(os.path.join(here, 'steam', src), os.path.join(cfg, 'grid', dst))
    return appid

def demo():
    # round trip + add + re-add keeps one entry and one appid
    global here, exe
    here, exe = '/x', '/x/' + exe_name
    b, a = add(b'', 0x80001234)
    b2, a2 = add(b, 0x8000ffff)
    sc = parse(b2)[0]['shortcuts']
    assert a == a2 == 0x80001234 and len(sc) == 1 and sc['0']['Exe'] == f'"/x/{exe_name}"', sc
    assert dump(parse(b2)[0]) == b2
    print('ok')

if os.environ.get('ADD_TO_STEAM_DEMO'): demo(); sys.exit()

home = os.path.expanduser('~')
roots = [os.path.join(home, p) for p in ('.steam/steam', '.local/share/Steam', '.var/app/com.valvesoftware.Steam/.local/share/Steam')]
users = sorted({os.path.realpath(u) for r in roots for u in glob.glob(os.path.join(r, 'userdata', '[1-9]*')) if os.path.isdir(u)})
if not users: sys.exit('no Steam user found: start Steam and log in once, then run this again')
if not os.path.exists(exe): sys.exit(f'{exe} not found: run this from the unpacked game folder')

running = subprocess.run(['pgrep', '-x', 'steam'], capture_output=True).returncode == 0
if running:
    print('closing Steam…')
    subprocess.run(['steam', '-shutdown'], capture_output=True)
    for _ in range(60):
        if subprocess.run(['pgrep', '-x', 'steam'], capture_output=True).returncode: break
        time.sleep(1)
    else: sys.exit('Steam did not close: quit it yourself and run this again')
    time.sleep(2)
for u in users:
    print(f'{name} added for Steam user {os.path.basename(u)} (appid {install(u)})')
if running:
    subprocess.Popen(['steam'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
    print('Steam started again')
EOF
