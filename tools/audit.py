import argparse
import math
import mmap
import os
import re
import sys
from collections import Counter

HARD_EXT = {".nup", ".nsp", ".xci", ".nca", ".keys"}
SOFT_EXT = {".nso", ".pdb", ".dmp", ".bin", ".ips", ".pchtxt", ".rom", ".sav"}
KEY_LINE = re.compile(rb"[A-Za-z0-9_]{3,64}[ \t]{0,3}=[ \t]{0,3}[0-9A-Fa-f]{32,}")
HEX_RUN = re.compile(rb"(?<![0-9A-Fa-f])[0-9A-Fa-f]{32,}(?![0-9A-Fa-f])")
USER_PATH = re.compile(rb"[A-Za-z]:[\\/]+Users[\\/]+([^\\/\x00\"'<>|*?\r\n]{1,64})", re.I)
SKIP_USERS = {b"public", b"default", b"all users", b"default user"}
WIN = 65536
DEC_LUT = "".join("%02d" % i for i in range(100)).encode()


def views(data):
    yield data
    if b"\x00" in data:
        yield data.replace(b"\x00", b"")


def entropy(b):
    n = len(b)
    c = Counter(b)
    return -sum(v / n * math.log2(v / n) for v in c.values())


def masked(s):
    s = s.decode("latin-1")
    name, eq, val = s.partition("=")
    if eq:
        val = val.strip()
        return name.strip() + " = " + val[:4] + "..." + "(" + str(len(val)) + ")"
    return s[:4] + "..." + "(" + str(len(s)) + ")"


def build_ref(root, window, stride):
    seen = set()
    n = 0
    for dp, _, fs in os.walk(root):
        for fn in fs:
            p = os.path.join(dp, fn)
            try:
                if os.path.getsize(p) < window:
                    continue
                with open(p, "rb") as f, mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ) as m:
                    for off in range(0, len(m) - window + 1, stride):
                        blk = m[off:off + window]
                        if len(set(blk)) >= 8:
                            seen.add(hash(blk))
                n += 1
            except (OSError, ValueError):
                continue
    return seen, n


def overlap(data, seen, window):
    hits = []
    for i in range(len(data) - window + 1):
        if hash(data[i:i + window]) in seen:
            hits.append(i)
            if len(hits) >= 5:
                break
    return hits


def scan(path, rel, a, seen, ban, out):
    ext = os.path.splitext(path)[1].lower()
    if ext in HARD_EXT:
        out.append(("FAIL", rel, "forbidden file type " + ext))
    elif ext in SOFT_EXT:
        out.append(("WARN", rel, "suspicious file type " + ext))
    if os.path.getsize(path) > a.max_mb * 1048576:
        out.append(("WARN", rel, "too large for content scans"))
        return
    with open(path, "rb") as f:
        data = f.read()
    for v in views(data):
        for m in KEY_LINE.finditer(v):
            out.append(("FAIL", rel, "key-like line at " + str(m.start()) + ": " + masked(m.group(0))))
        for m in HEX_RUN.finditer(v):
            s = m.start()
            if v[max(0, s - 7):s] == b"/rustc/":
                continue
            if len(m.group(0).replace(DEC_LUT, b"")) < 32:
                continue
            if len(set(m.group(0))) <= 3:
                continue
            out.append(("WARN", rel, "long hex run at " + str(s) + ": " + masked(m.group(0))))
        for m in USER_PATH.finditer(v):
            if m.group(1).lower() in SKIP_USERS:
                continue
            out.append(("WARN", rel, "personal path at " + str(m.start()) + ": " + m.group(0).decode("latin-1")))
    host = os.environ.get("COMPUTERNAME", "")
    if len(host) >= 6:
        low = data.lower()
        if host.lower().encode() in low or host.lower().encode("utf-16le") in low:
            out.append(("WARN", rel, "machine name present"))
    for ln, term in ban:
        if term in data or term.decode("utf-8", "ignore").encode("utf-16le") in data:
            out.append(("FAIL", rel, "banned term from line " + str(ln)))
    hot = [o for o in range(0, len(data) - WIN + 1, WIN) if entropy(data[o:o + WIN]) > 7.5]
    if hot:
        out.append(("WARN", rel, str(len(hot)) + " high-entropy block(s), first at " + str(hot[0])))
    if seen:
        hits = overlap(data, seen, a.window)
        if hits:
            out.append(("FAIL", rel, "matches reference data at " + ", ".join(str(h) for h in hits)))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("target")
    ap.add_argument("--ref")
    ap.add_argument("--stride", type=int, default=8192)
    ap.add_argument("--window", type=int, default=32)
    ap.add_argument("--ban-file")
    ap.add_argument("--max-mb", type=int, default=256)
    a = ap.parse_args()

    ban = []
    if a.ban_file:
        with open(a.ban_file, "rb") as f:
            for ln, raw in enumerate(f.read().splitlines(), 1):
                if len(raw.strip()) >= 4:
                    ban.append((ln, raw.strip()))

    seen = set()
    if a.ref:
        seen, n = build_ref(a.ref, a.window, a.stride)
        print("reference: " + str(n) + " file(s), " + str(len(seen)) + " samples")

    files = []
    if os.path.isdir(a.target):
        for dp, _, fs in os.walk(a.target):
            for fn in fs:
                p = os.path.join(dp, fn)
                files.append((p, os.path.relpath(p, a.target)))
    else:
        files.append((a.target, os.path.basename(a.target)))

    out = []
    for p, rel in files:
        scan(p, rel, a, seen, ban, out)

    for lvl, rel, msg in sorted(out):
        print(lvl + "  " + rel + "  " + msg)
    fails = sum(1 for o in out if o[0] == "FAIL")
    warns = sum(1 for o in out if o[0] == "WARN")
    print("files scanned: " + str(len(files)) + "  FAIL: " + str(fails) + "  WARN: " + str(warns))
    for p, rel in files:
        print("  " + rel + "  " + str(os.path.getsize(p)) + " bytes")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
