#!/usr/bin/env python3
"""A3 — the full era-corpus record-identity census.

For every in-scope DWG in a version's corpus directory, stage the
conventional-arm rewrite (DWG_NO_ECHO=1 dwgrewrite) and census it against
the authored bytes with record_size_census.py (handle-keyed size+CRC-16 —
the method that tolerates the rewrite's renumbering). Prints a per-file
tally and the per-era + grand totals.

Usage:
  era_corpus_census.py <version_dir> [<version_dir> ...]

Env: GOLD_DWGREAD must point at the oracle binary (dwgread -v9 supply).
"""
import os
import re
import subprocess
import sys

REPO = os.path.expanduser("~/work/cadcodec")
DWGREWRITE = os.path.join(REPO, "target/debug/dwgrewrite")
CENSUS = os.path.join(REPO, "tests/gold_harness/analysis/record_size_census.py")
WORK = "/tmp/a3_era_census"

SUMMARY = re.compile(r"paired: (\d+).*?divergent: (\d+); her-only: (\d+); our-only: (\d+)")


def census_one(src, tag):
    stem = os.path.splitext(os.path.basename(src))[0]
    # Workdir keyed by era tag + stem: the same stem (e.g. circle.dwg)
    # appears in several era dirs, and the census's log names derive from
    # the stem — a bare-stem key collides across eras.
    w = os.path.join(WORK, f"{tag}__{stem}")
    os.makedirs(w, exist_ok=True)
    conv = os.path.join(w, "conv.dwg")
    env = dict(os.environ)
    env["DWG_NO_ECHO"] = "1"
    r = subprocess.run([DWGREWRITE, src, conv], env=env,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if r.returncode != 0 or not os.path.exists(conv):
        return None
    c = subprocess.run(["python3", CENSUS, src, conv],
                       capture_output=True, text=True)
    m = SUMMARY.search(c.stdout)
    if not m:
        return None
    return tuple(int(x) for x in m.groups())  # paired, divergent, her-only, our-only


def main():
    dirs = sys.argv[1:]
    os.makedirs(WORK, exist_ok=True)
    grand = [0, 0, 0, 0]
    for d in dirs:
        era = os.path.basename(d.rstrip("/"))
        files = sorted(
            os.path.join(d, f) for f in os.listdir(d) if f.lower().endswith(".dwg")
        )
        era_tot = [0, 0, 0, 0]
        nonzero = []
        for src in files:
            res = census_one(src, era)
            if res is None:
                print(f"  {os.path.basename(src)}: SKIPPED")
                continue
            paired, div, her, ours = res
            era_tot[0] += paired; era_tot[1] += div
            era_tot[2] += her;   era_tot[3] += ours
            if div or her or ours:
                nonzero.append((os.path.basename(src), paired, div, her, ours))
        print(f"[{era}] {len(files)} files — paired {era_tot[0]}, divergent {era_tot[1]}, "
              f"her-only {era_tot[2]}, our-only {era_tot[3]}")
        for name, paired, div, her, ours in nonzero:
            print(f"    {name}: {div} divergent / {paired} paired "
                  f"(+{her} her-only, +{ours} our-only)")
        for i in range(4):
            grand[i] += era_tot[i]
    print(f"\nGRAND: paired {grand[0]}, divergent {grand[1]}, "
          f"her-only {grand[2]}, our-only {grand[3]}")


if __name__ == "__main__":
    main()
