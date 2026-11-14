#!/usr/bin/env python3
"""Pull iPhoneOS.sdk and MacOSX.sdk out of an Xcode .xip on Linux (no Mac, no sudo), in one pass over it.

Usage: tools/ios-sdk.py ~/Downloads/Xcode_X.xip [OUT_DIR]   (default ~/.local/share/hst-ios → OUT_DIR/iPhoneOS.sdk,
OUT_DIR/MacOSX.sdk; one already there is kept, so a second run only adds the one missing). tools/ios-build.sh links
against the first, tools/macos-build.sh (M162) the second.

A .xip is a xar (bsdtar reads it) holding a pbzx stream (xz chunks) of an odc cpio. Hard-linked files keep their
bytes in one entry only; the other links hold a "NULLcanary" placeholder of the same size, so GNU cpio extracts
garbage for them. Pass 1 notes which hard-linked inodes the SDK needs, pass 2 takes their bytes from whichever
entry has them. The odc inode field is 18 bits and wraps, so links are matched on device + inode + size.

Linker stubs (.tbd): Xcode 27's list only arm64e and the new arm64e.x1, which no lld knows. An arm64 app links
against arm64e system libraries (Apple's ld allows it; the phone's libraries are arm64e), so arm64e.x1 is renamed
arm64 in every target list that doesn't already have arm64 (iOS and macOS lists alike).
"""
import lzma, os, re, stat, struct, subprocess, sys

TBD_LIST = re.compile(r"\[([^\]]*arm64e\.x1-(?:ios|macos)[^\]]*)\]")
SDKS = {name: re.compile(rf"/SDKs/{name}[\d.]*\.sdk/") for name in ("iPhoneOS", "MacOSX")}
CANARY = b"NULLcanary"


def entries(xip):
    p = subprocess.Popen(["bsdtar", "-xOf", xip, "Content"], stdout=subprocess.PIPE)
    r, buf = p.stdout, bytearray()

    def chunks():
        assert r.read(4) == b"pbzx", "not a pbzx stream"
        r.read(8)  # flags
        while h := r.read(16):
            size, csize = struct.unpack(">QQ", h)
            c = r.read(csize)
            yield c if csize == size else lzma.decompress(c)

    g = chunks()

    def read(n):
        while len(buf) < n:
            c = next(g, None)
            if c is None:
                break
            buf.extend(c)
        out = bytes(buf[:n])
        del buf[:n]
        return out

    while len(h := read(76)) == 76:
        assert h[:6] == b"070707", f"not an odc cpio entry: {h[:6]!r}"
        mode, nlink, size = int(h[18:24], 8), int(h[36:42], 8), int(h[65:76], 8)
        ino = (h[6:18], size)  # dev + 18-bit inode wraps on Xcode's file count; the size tells collisions apart
        name = read(int(h[59:65], 8))[:-1].decode()
        data = read(size)
        if name == "TRAILER!!!":
            break
        yield ino, mode, nlink, name, data
    p.stdout.close()
    p.wait()


SDK_PATTERNS = {}  # the SDKs this run extracts: name → pattern (main)


def sdk_path(name):
    """`(sdk, path inside it)` for an entry of an SDK this run extracts, else None."""
    for sdk, pattern in SDK_PATTERNS.items():
        if m := pattern.search(name):
            return sdk, name[m.end():]
    return None


def main():
    xip = sys.argv[1]
    base = sys.argv[2] if len(sys.argv) > 2 else os.path.expanduser("~/.local/share/hst-ios")
    outs = {sdk: os.path.join(base, f"{sdk}.sdk") for sdk in SDKS}
    for sdk, out in outs.items():
        if os.path.exists(out):
            print(f"{out} exists: kept (remove it to extract it again)")
        else:
            SDK_PATTERNS[sdk] = SDKS[sdk]
    if not SDK_PATTERNS:
        return
    need = {ino for ino, mode, nlink, name, _ in entries(xip) if nlink > 1 and stat.S_ISREG(mode) and sdk_path(name)}
    have, pending = {}, {}  # inode → bytes; inode → [(path, mode)] still waiting for bytes
    files = dict.fromkeys(SDK_PATTERNS, 0)
    for ino, mode, nlink, name, data in entries(xip):
        if ino in need and not data.startswith(CANARY):
            if have.setdefault(ino, data) != data:
                sys.exit(f"two different files share link key {ino}: {name}")
            for path, m in pending.pop(ino, []):
                write(path, m, data)
        hit = sdk_path(name)
        if hit is None:
            continue
        sdk, rel = hit
        path = os.path.join(outs[sdk], rel)
        if stat.S_ISDIR(mode):
            os.makedirs(path, exist_ok=True)
        elif stat.S_ISLNK(mode):
            os.makedirs(os.path.dirname(path), exist_ok=True)
            os.symlink(data.decode(), path)
        elif ino in need and data.startswith(CANARY):
            if ino in have:
                write(path, mode, have[ino])
            else:
                pending.setdefault(ino, []).append((path, mode))
            files[sdk] += 1
        else:
            write(path, mode, data)
            files[sdk] += 1
    if pending:
        sys.exit(f"{sum(map(len, pending.values()))} hard-linked files never got their bytes")
    for sdk, n in files.items():
        if not n:
            sys.exit(f"no {sdk} SDK in {xip}")
        for root, dirs, _ in os.walk(outs[sdk]):  # directories were made writable for extraction
            for d in dirs:
                os.chmod(os.path.join(root, d), 0o755)
        print(outs[sdk], f"({n} files)")


def arm64_tbd(text):
    def fix(m):
        ts = [t.strip() for t in m.group(1).split(",")]
        for os_ in ("ios", "macos"):
            new, arm64 = f"arm64e.x1-{os_}", f"arm64-{os_}"
            ts = [t for t in ts if t != new] if arm64 in ts else [arm64 if t == new else t for t in ts]
        return "[ " + ", ".join(ts) + " ]"
    return TBD_LIST.sub(fix, text)


def write(path, mode, data):
    if path.endswith(".tbd"):
        data = arm64_tbd(data.decode()).encode()
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)
    os.chmod(path, stat.S_IMODE(mode) | 0o200)


assert arm64_tbd("targets: [ arm64e-ios, arm64e.x1-ios ]") == "targets: [ arm64e-ios, arm64-ios ]"
assert arm64_tbd("[ arm64-ios, arm64e-ios, arm64e.x1-ios ]") == "[ arm64-ios, arm64e-ios ]"
assert arm64_tbd("targets: [ x86_64-macos, arm64e-macos, arm64e.x1-macos ]") == "targets: [ x86_64-macos, arm64e-macos, arm64-macos ]"
assert sdk_path.__doc__ and SDKS["MacOSX"].search("Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX26.0.sdk/usr/lib/libSystem.tbd")

if __name__ == "__main__":
    main()
