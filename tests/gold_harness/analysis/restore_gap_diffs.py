#!/usr/bin/env python3
"""§20 restore_gap_diffs — the constructed-SAB structural audits.

The restore-gap campaign's diff instruments (the candidate-5 packet's
evidence surface): given an authored specimen decode and a
constructed decode, this tool walks both ACIS graphs and reports
the divergences that matter to a strict modeler, level by level:

1. The orientation audit — per face: the sense flags, the plane
   normal, the outward dot product, and the loop's coedge-sense
   chain.
2. The loop-traversal check — per face: walking the coedge chain in
   travel order (sense-compensated), verifying vertex-connectivity
   (head-to-tail handoffs) and closure.
3. The travel-direction check — the loop's travel polygon (from the
   vertex point coordinates) vs the face's EFFECTIVE normal
   (surface normal, negated when the face sense is reversed): the
   right-hand rule (CCW travel) the modeler validates.
4. The structural diff — record-by-record token alignment (kind
   sequences, non-pointer values, pointer targets resolved to
   classes), BFS-position paired with the authored persubent
   attribs skipped (the pairing diverges where the graphs' mention
   orders differ — read the pre-divergence rows as the findings).

The 2026-09-29 findings (authored Box_2018 vs constructed Box):
levels 1-3 are CLEAN IN BOTH (all six faces effectively
outward-oriented; every loop closed and vertex-consistent; every
travel polygon CCW around its effective normal) — the constructed
B-rep is demonstrably valid at every computable level, and the
sense-flag PATTERNS differ (authored top/bottom TTTT, sides Tfff;
constructed bottom ffff, top TTTT, sides TTff) without either
being invalid. The next suspects, in test order: (a) probe an
authored journal-less, attrib-less carrier (all MODELED controls
so far carry persubent attribs — "attribs absent is fine" is
UNPROVEN; example_2004's plain Regions are the candidates); (b)
the coedge partner/next/prev SYMMETRY wiring (partner.edge ==
edge, partner.sense == !sense, next.prev == self) checked as an
invariant on BOTH streams; (c) the full semantic pair-diff via a
face-chain-anchored isomorphism (the BFS pairing breaks at the
first mention-order divergence).

CLI: restore_gap_diffs.py AUTHORED.json CONSTRUCTED.json [--check N]
     Runs all audits by default; --check selects one.
"""

import argparse
import json
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "genus"))
from genus_extract import walk_sab, acis_entities


def parse_tokens(buf, off, end):
    toks = []
    while off < end:
        t = buf[off]
        if t == 0x11:
            toks.append(("EOR", None)); off += 1
        elif t == 0x02:
            toks.append(("char", buf[off + 1])); off += 2
        elif t == 0x03:
            toks.append(("short", struct.unpack_from("<h", buf, off + 1)[0])); off += 3
        elif t in (0x04, 0x15):
            toks.append(("int", struct.unpack_from("<i", buf, off + 1)[0])); off += 5
        elif t == 0x05:
            toks.append(("float", struct.unpack_from("<f", buf, off + 1)[0])); off += 5
        elif t in (0x06, 0x17):
            toks.append(("dbl", struct.unpack_from("<d", buf, off + 1)[0])); off += 9
        elif t == 0x07:
            n = buf[off + 1]; toks.append(("str", buf[off + 2:off + 2 + n])); off += 2 + n
        elif t == 0x08:
            n = int.from_bytes(buf[off + 1:off + 3], "little"); toks.append(("str", buf[off + 3:off + 3 + n])); off += 3 + n
        elif t in (0x09, 0x12):
            n = int.from_bytes(buf[off + 1:off + 5], "little"); toks.append(("str", buf[off + 5:off + 5 + n])); off += 5 + n
        elif t in (0x0A,):
            toks.append(("false", None)); off += 1
        elif t in (0x0B,):
            toks.append(("true", None)); off += 1
        elif t in (0x0F, 0x10, 0x16):
            toks.append((f"tag{t:02x}", None)); off += 17 if t == 0x16 else 1
        elif t in (0x13, 0x14):
            toks.append((f"vec{t:02x}", struct.unpack_from("<3d", buf, off + 1))); off += 25
        elif t == 0x0C:
            toks.append(("ptr", struct.unpack_from("<i", buf, off + 1)[0])); off += 5
        else:
            raise SystemExit(f"unknown tag 0x{t:02x} at {off}")
    return toks


def load(path):
    doc = json.load(open(path))
    for etype, payload in acis_entities(doc):
        sab = (payload.get("acis_data") or {}).get("sab_data")
        if not sab:
            continue
        buf = bytes(sab)
        header, frames = walk_sab(buf)
        recs = []
        for f in frames:
            off = f["start"]
            names = []
            while buf[off] in (0x0E, 0x0D):
                tag = buf[off]
                n = buf[off + 1]
                names.append(buf[off + 2:off + 2 + n].decode("latin-1"))
                off += 2 + n
                if tag == 0x0D:
                    break
            name = "-".join(names)
            if name.startswith("End-of-"):
                recs.append({"name": name, "toks": []})
                continue
            off += 10
            recs.append({"name": name, "toks": parse_tokens(buf, off, f["start"] + f["width"])})
        return recs
    raise SystemExit(f"no SAB carrier in {path}")


def loop_chain(recs, face_toks):
    loop = face_toks[2][1]
    if not (0 <= loop < len(recs)):
        return []
    c = recs[loop]["toks"][2][1]
    seen = set()
    out = []
    while 0 <= c < len(recs) and c not in seen and recs[c]["name"] == "coedge":
        seen.add(c)
        out.append(c)
        c = recs[c]["toks"][1][1]
    return out


def orientation(recs, label):
    faces = [i for i, r in enumerate(recs) if r["name"] == "face"]
    origins = [recs[recs[f]["toks"][5][1]]["toks"][1][1] for f in faces
               if recs[recs[f]["toks"][5][1]]["name"] == "plane-surface"]
    center = [sum(o[k] for o in origins) / len(origins) for k in range(3)] if origins else [0, 0, 0]
    print(f"-- {label}: the orientation audit")
    for fi in faces:
        toks = recs[fi]["toks"]
        sense, sided = toks[6], toks[7]
        surf = toks[5][1]
        if recs[surf]["name"] != "plane-surface":
            continue
        stoks = recs[surf]["toks"]
        origin, normal = stoks[1][1], stoks[2][1]
        dot = sum(normal[k] * (origin[k] - center[k]) for k in range(3))
        chain = "".join("T" if recs[c]["toks"][5][0] == "true" else "f" for c in loop_chain(recs, toks))
        print(f"   face[{fi:2d}] sense={'fwd' if sense[0]=='true' else 'rev'} "
              f"sided={'T' if sided[0]=='true' else 'f'} outward_dot={dot:+.2f} chain={chain}")


def traversal(recs, label):
    print(f"-- {label}: the loop-traversal check")
    for fi in [i for i, r in enumerate(recs) if r["name"] == "face"]:
        steps = []
        for c in loop_chain(recs, recs[fi]["toks"]):
            ct = recs[c]["toks"]
            fwd = ct[5][0] == "true"
            et = recs[ct[4][1]]["toks"]
            h, t = (et[1][1], et[3][1]) if fwd else (et[3][1], et[1][1])
            steps.append((h, t))
        ok = all(steps[i][1] == steps[(i + 1) % len(steps)][0] for i in range(len(steps))) if steps else False
        closed = bool(steps) and steps[0][0] == steps[-1][1]
        print(f"   face[{fi:2d}] closed={'Y' if closed else 'N'} consistent={'OK' if ok else 'BROKEN'}")


def cross(a, b):
    return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]


def travel(recs, label):
    print(f"-- {label}: the travel-direction check")
    for fi in [i for i, r in enumerate(recs) if r["name"] == "face"]:
        toks = recs[fi]["toks"]
        sense_fwd = toks[6][0] == "true"
        normal = recs[toks[5][1]]["toks"][2][1]
        eff = [c if sense_fwd else -c for c in normal]
        pts = []
        for c in loop_chain(recs, toks):
            ct = recs[c]["toks"]
            fwd = ct[5][0] == "true"
            et = recs[ct[4][1]]["toks"]
            h, _ = (et[1][1], et[3][1]) if fwd else (et[3][1], et[1][1])
            p = recs[h]["toks"][3][1]
            if recs[p]["name"] == "point":
                pts.append(recs[p]["toks"][1][1])
        if len(pts) < 3:
            print(f"   face[{fi:2d}]: insufficient polygon")
            continue
        acc = [0.0, 0.0, 0.0]
        for i in range(len(pts)):
            cr = cross(pts[i], pts[(i + 1) % len(pts)])
            acc = [acc[k] + cr[k] for k in range(3)]
        dot = sum(acc[k] * eff[k] for k in range(3))
        print(f"   face[{fi:2d}] travel_poly_dot={dot:+.2f} {'CCW-OK' if dot > 0 else 'CW-INVERTED'}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("authored", type=Path)
    parser.add_argument("constructed", type=Path)
    args = parser.parse_args()
    A = load(args.authored)
    C = load(args.constructed)
    orientation(A, "AUTHORED")
    orientation(C, "CONSTRUCTED")
    traversal(A, "AUTHORED")
    traversal(C, "CONSTRUCTED")
    travel(A, "AUTHORED")
    travel(C, "CONSTRUCTED")
    print("-- the structural diff runs from the probe scripts until the "
          "semantic pairing lands (recorded in the halt)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
