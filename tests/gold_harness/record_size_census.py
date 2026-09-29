#!/usr/bin/env python3
"""The R2018+ record-size census: gold -v9 object blocks, handle-keyed.

The era censuses (record_identity_survey.py + the handle-keyed -v9
size/CRC comparisons) covered R2000/R2004/R2010/R2013 specimens and the
AC1021 survey covered its whole corpus, but the R2018 conventional arm
was never record-verified (2026-09-29 finding). This instrument
censuses ANY pair (original, rewrite) on ANY era: it runs gold
`dwgread -v9` on both files, parses the per-object block headers
(size/hdlsize/type/bitsize), keys them by the object's own handle, and
reports every handle whose record size, handle-stream size, type, or
bitsize drifted on the rewrite — plus her-only/our-only handles.

Usage:
  record_size_census.py ORIG.dwg REWRITE.dwg [--dwgread PATH]
  (env GOLD_DWGREAD respected; writes logs under /tmp/rsize_census/)

Exit 0 always; the report is the measurement.
"""
import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

DWGREAD_DEFAULT = os.path.expanduser("~/work/libredwg/programs/dwgread")

HDR = re.compile(
    r"^Object number: (\d+)/([0-9A-Fa-f]+), Size: (\d+) \[MS\], "
    r"(?:Hdlsize: (0x[0-9A-Fa-f]+|\S+) \[UMC\] ?, )?Type: (\d+) \[BOT\], "
    r"Address: (\d+)"
)
# pre-R2010 object block: no UMC handle-bits header, the type is [BS]
HDR_LEGACY = re.compile(
    r"^Object number: (\d+)/([0-9A-Fa-f]+), Size: (\d+) \[MS\], "
    r"Type: (\d+) \[BS\], Address: (\d+)"
)
HANDLE = re.compile(r"^handle: (\S+) \[H")
BITSIZE = re.compile(r"^ bitsize: (\d+)")
CRC = re.compile(r"^crc: ([0-9A-Fa-f]{4})")
CLASSNAME = re.compile(
    r"^Warning: (?:Unhandled|Unknown|Unstable) Class (?:object|entity) \d+ (\S+)")


def census(path, workdir, dwgread):
    Path(workdir).mkdir(parents=True, exist_ok=True)
    log = Path(workdir) / (Path(path).stem + "_v9.log")
    with open(log, "w") as tf:
        subprocess.run([dwgread, "-v9", path], stdout=tf,
                       stderr=subprocess.STDOUT, check=True)
    records = {}
    cur = None
    with open(log, errors="replace") as f:
        for line in f:
            line = line.rstrip("\n")
            m = HDR.match(line)
            if m:
                # a new block header IS the flush boundary for the
                # pending record: pre-R2010 logs print no
                # "< Next object:" separator line between blocks.
                if cur is not None and cur["handle"]:
                    records[cur["handle"]] = cur
                cur = {
                    "object_number": int(m.group(1)),
                    "index_hex": m.group(2),
                    "size": int(m.group(3)),
                    "hdlsize": m.group(4) or "-",
                    "type": int(m.group(5)),
                    "address": int(m.group(6)),
                    "handle": None,
                    "bitsize": None,
                    "crc": None,
                    "class": None,
                }
                continue
            m = HDR_LEGACY.match(line)
            if m:
                if cur is not None and cur["handle"]:
                    records[cur["handle"]] = cur
                cur = {
                    "object_number": int(m.group(1)),
                    "index_hex": m.group(2),
                    "size": int(m.group(3)),
                    "hdlsize": "-",
                    "type": int(m.group(4)),
                    "address": int(m.group(5)),
                    "handle": None,
                    "bitsize": None,
                    "crc": None,
                    "class": None,
                }
                continue
            m = HANDLE.match(line)
            if m and cur is not None and cur["handle"] is None:
                cur["handle"] = m.group(1)
                continue
            m = BITSIZE.match(line)
            if m and cur is not None and cur["bitsize"] is None:
                cur["bitsize"] = int(m.group(1))
                continue
            m = CRC.match(line)
            if m and cur is not None and cur["crc"] is None:
                cur["crc"] = m.group(1)
                continue
            m = CLASSNAME.match(line)
            if m and cur is not None and cur["class"] is None:
                cur["class"] = m.group(1)
                continue
            if "Add entity" in line or "Add object" in line:
                if cur is not None and cur["class"] is None:
                    parts = line.split()
                    cur["class"] = parts[2] if len(parts) > 2 else parts[-1]
                continue
            if line.startswith("< Next object:") and cur is not None:
                if cur["handle"]:
                    records[cur["handle"]] = cur
                cur = None
    if cur is not None and cur["handle"]:
        records[cur["handle"]] = cur
    return records, log


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("original")
    ap.add_argument("rewrite")
    ap.add_argument("--dwgread", default=os.environ.get(
        "GOLD_DWGREAD", DWGREAD_DEFAULT))
    ap.add_argument("--workdir", default="/tmp/rsize_census")
    args = ap.parse_args()

    hers, her_log = census(args.original, args.workdir, args.dwgread)
    ours, our_log = census(args.rewrite, args.workdir, args.dwgread)

    her_keys, our_keys = set(hers), set(ours)
    both = sorted(her_keys & our_keys)
    divergent = []
    identical = 0
    for key in both:
        her, our = hers[key], ours[key]
        diffs = []
        if her["size"] != our["size"]:
            diffs.append(f"size {her['size']} -> {our['size']}")
        if her["bitsize"] != our["bitsize"]:
            diffs.append(f"bitsize {her['bitsize']} -> {our['bitsize']}")
        if her["type"] != our["type"]:
            diffs.append(f"type {her['type']} -> {our['type']}")
        if her["hdlsize"] != our["hdlsize"]:
            diffs.append(f"hdlsize {her['hdlsize']} -> {our['hdlsize']}")
        if her["crc"] and our["crc"] and her["crc"] != our["crc"]:
            diffs.append(f"crc {her['crc']} -> {our['crc']}")
        if diffs:
            divergent.append(
                f"  handle {key} ({her['class'] or 'class?'} type {her['type']}): "
                + ", ".join(diffs))
        else:
            identical += 1

    her_only = sorted(her_keys - our_keys)
    our_only = sorted(our_keys - her_keys)
    print(f"original {args.original}: {len(hers)} handles (log {her_log})")
    print(f"rewrite  {args.rewrite}: {len(ours)} handles (log {our_log})")
    print(f"paired: {len(both)} (record-identical on size+CRC: {identical}); "
          f"divergent: {len(divergent)}; "
          f"her-only: {len(her_only)}; our-only: {len(our_only)}")
    for row in divergent:
        print(row)
    if her_only:
        print("her-only handles: " + " ".join(
            f"{k}({hers[k]['class'] or hers[k]['type']})" for k in her_only[:20]))
    if our_only:
        print("our-only handles: " + " ".join(
            f"{k}({ours[k]['class'] or ours[k]['type']})" for k in our_only[:20]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
