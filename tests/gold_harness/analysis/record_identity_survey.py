#!/usr/bin/env python3
"""H8h extension: record-wise conventional-emission identity for AC1021 files.

For each input DWG:
  1. DWG_NO_ECHO=1 DWG_RECORD_TRACE=1 dwgrewrite F -> conv.dwg + trace maps
  2. AC21_DIFF_RAW_DIR dumps her + our decompressed objects streams
  3. parse the trace maps, walk each record frame
     ([MS size][span: type..pad][crc16 2 bytes]), compare per handle
Prints per-file: identical / divergent / her-only / our-only counts plus
the first-divergence detail per divergent record.
"""
import os
import re
import subprocess
import sys

REPO = os.path.expanduser("~/work/cadcodec")
DWGREWRITE = os.path.join(REPO, "target/debug/dwgrewrite")
TOKENDIFF = os.path.join(REPO, "target/debug/ac21_token_diff")


def read_ms(buf, off):
    """DWG modular short: 15 data bits per 16-bit LE word; bit 15 of a
    word is the continuation flag. Returns (value, byte_len)."""
    val = 0
    i = 0
    while True:
        w = buf[off + i] | (buf[off + i + 1] << 8)
        val += (w & 0x7FFF) << (15 * (i // 2))
        i += 2
        if not (w & 0x8000):
            break
        if i > 8:
            raise ValueError("bad MS")
    return val, i


def record_extent(buf, off):
    """Return (ms_len, size, total_len incl. 2-byte CRC)."""
    size, ms_len = read_ms(buf, off)
    return ms_len, size, ms_len + size + 2


def parse_trace(path):
    her, ours = {}, {}
    rx = re.compile(r"\[record-trace (her|our)\] ([0-9A-Fa-f]+) (\d+)")
    with open(path) as f:
        for line in f:
            m = rx.search(line)
            if m:
                d = her if m.group(1) == "her" else ours
                d[int(m.group(2), 16)] = int(m.group(3))
    return her, ours


def measure(src):
    stem = os.path.splitext(os.path.basename(src))[0]
    work = f"/tmp/h8x/{stem}"
    os.makedirs(work, exist_ok=True)
    conv = f"{work}/conv.dwg"
    trace = f"{work}/trace.log"

    env = dict(os.environ)
    env["DWG_NO_ECHO"] = "1"
    env["DWG_RECORD_TRACE"] = "1"
    with open(trace, "w") as tf:
        subprocess.run([DWGREWRITE, src, conv], env=env,
                       stdout=tf, stderr=subprocess.STDOUT, check=True)
    her_map, our_map = parse_trace(trace)
    if not her_map:
        return None, "no [record-trace her] lines (not an AC1021 read?)"

    rawdir = f"{work}/raw"
    env2 = dict(os.environ)
    env2["AC21_DIFF_RAW_DIR"] = rawdir
    with open(f"{work}/ac21.log", "w") as af:
        subprocess.run([TOKENDIFF, src, "--section", "AcDb:AcDbObjects",
                        "--our-rt", conv], env=env2,
                       stdout=af, stderr=subprocess.STDOUT, check=True)
    her_bin = f"{rawdir}/AcDb_AcDbObjects.her.bin"
    our_bin = f"{rawdir}/AcDb_AcDbObjects.ours.bin"
    with open(her_bin, "rb") as f:
        her_stream = f.read()
    with open(our_bin, "rb") as f:
        our_stream = f.read()

    common = sorted(set(her_map) & set(our_map))
    her_only = sorted(set(her_map) - set(our_map))
    our_only = sorted(set(our_map) - set(her_map))

    identical, divergent, bad = [], [], []
    for h in common:
        ho, oo = her_map[h], our_map[h]
        try:
            hms, hsize, hlen = record_extent(her_stream, ho)
            oms, osize, olen = record_extent(our_stream, oo)
        except Exception as e:
            bad.append((h, str(e)))
            continue
        if hsize != osize:
            divergent.append((h, f"size {hsize} vs {osize}"))
            continue
        her_rec = her_stream[ho:ho + hlen]
        our_rec = our_stream[oo:oo + olen]
        if her_rec == our_rec:
            identical.append(h)
        else:
            first = next(i for i in range(min(len(her_rec), len(our_rec)))
                         if her_rec[i] != our_rec[i])
            divergent.append((h, f"first diff at +{first} "
                                 f"(her 0x{her_rec[first]:02x} "
                                 f"our 0x{our_rec[first]:02x}, "
                                 f"len {hlen})"))
    return {
        "stem": stem,
        "total": len(common),
        "identical": len(identical),
        "divergent": divergent,
        "bad": bad,
        "her_only": her_only,
        "our_only": our_only,
        "her_len": len(her_stream),
        "our_len": len(our_stream),
    }, None


def main():
    srcs = sys.argv[1:]
    total_div = 0
    for src in srcs:
        res, err = measure(src)
        if err:
            print(f"{os.path.basename(src)}: SKIPPED — {err}")
            continue
        d = res["divergent"]
        total_div += len(d)
        print(f"{res['stem']}: {res['identical']}/{res['total']} identical "
              f"({len(d)} divergent, {len(res['her_only'])} her-only, "
              f"{len(res['our_only'])} our-only; "
              f"streams her {res['her_len']} vs our {res['our_len']})")
        for h, why in d[:8]:
            print(f"    h={h:X}: {why}")
        if len(d) > 8:
            print(f"    ... {len(d) - 8} more")
    print(f"\nTOTAL divergent records across files: {total_div}")


if __name__ == "__main__":
    main()
