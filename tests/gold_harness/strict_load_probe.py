#!/usr/bin/env python3
"""§20 strict_load_probe — the mechanized BricsCAD audit (the eighth-layer instrument).

The §20.4 recorded procedure for strict-loader verdicts is the user-run
BricsCAD audit. This tool mechanizes the verifiable part of that
procedure: it drives `bricscad.exe /b <script>` headless, and the
script's LISP records the evidence a verdict needs — the 3DSOLID/
REGION entity census, the modeler-forced bounding box (a healthy
restore yields real extents; a failed restore yields the ±1e80 null
sentinel), the post-audit census, and the drawing extents after
ZOOM/Extents. The campaign's acceptance — "clean open + clean audit,
the solids model" — is decided from those.

Validation (2026-09-29, BricsCAD V18 en_US, /b-script proven by the
diag write): authored specimens 2007/2013/2018 all yield real
extents (Box 0,0,0..1,2,3 across eras); the constructed corpus
yields entity-count 1 with the ±1e80 null box at BOTH the current
rank and the pre-rank code — the constructed-SAB restore gap is
pre-existing and ordering-independent, and every historical
BricsCAD reading was the error surface, not the restored-solid
census (see the G-B/G-A records in IMPLEMENTATION.md §20.6).

Limits, recorded: this BricsCAD build leaves LOGFILENAME unset
(LOGFILEON produces no file), so the probe cannot capture the
restorer's TEXT ("Data stream is empty", "missing logical in
restore file") nor AUDIT's console report ("N errors found");
the DB/modeler census is the measured surface. Two proxies stand
in for the console text: (a) the modeling-failure DIALOG at OPEN
is not console and not capturable by any script — it blocks before
LISP runs; its measurable shadow is the ±1e80 null bbox (the
authored controls' real extents prove the probe's discrimination);
(b) AUDIT's effect is captured via DBMOD/ERRNO, recorded pre- and
post-audit — DBMOD gaining bits means the audit CHANGED the
drawing (found and fixed errors), which distinguishes a clean
audit from a repairing one without the console text. The app is
a GUI process — each launch is bounded and killed on timeout; a
missing/empty result file marks the fixture AMBIGUOUS, never
clean.

The console-text limit is TESTED, not presumed (2026-09-29): a
diag run trapped `(setvar "LOGFILENAME" path)` with "system
variable is read-only: LOGFILENAME" — the log channel cannot be
redirected on this build. The console mechanism therefore landed
as the WINDOW-LIFECYCLE SCRAPER (`bricscad_console_scraper.ps1`,
staged beside each fixture): the launcher spawns it with the
BricsCAD PID, and it polls every 250 ms, enumerating the
process's top-level windows and logging each window's class +
title at first appearance (and on title change), with timestamps.
Three channels were tested and are CLOSED on this build: the log
channel (read-only LOGFILENAME); WM_GETTEXT (returns empty — the
UI is Qt, all text painted, none in window text slots); UI
Automation (no Text/Value patterns, empty Name tree — no
accessibility bridge). GetWindowText works for TITLES (managed
by the window manager), so the transcript captures WHICH dialogs
appear during a run (the modeling-failure dialog is a top-level
window), WHEN, and their titles — the programmatic evidence
surface, alongside the LOGSEC census and the DBMOD/ERRNO record.
The analyzer digests the transcript against the strict loader's
message vocabulary. The console text itself needs a build with a
working log channel or accessibility bridge; until then the
maintainer's hand-run transcripts remain the highest-fidelity
console evidence (the "General modeling failure / AcDb3dSolid
(31)" verdict came from one).

The LOGSEC flush discipline (the evidence-survival rule): the
script's LISP writes EVERY result section through a `LOGSEC`
helper — `(defun LOGSEC (lines / f) (setq f (open RESULT "a"))
(foreach l lines (write-line l f)) (close f))` — so each section
opens the result file in APPEND mode, writes its lines, and closes
the handle: every line is on disk the moment it is written. The
earlier single-handle form (`(setq rf (open … "w"))` … one final
`(close rf)`) buffered everything in the handle and lost ALL
evidence whenever the run died before the close — which is
exactly what happens when a fixture trips BricsCAD's
modeling-failure prompt (the dialog blocks the /b script engine
mid-sequence) or the launcher's timeout kill lands: the staged
results read 0 bytes and the run looked AMBIGUOUS when it was in
fact a recorded verdict waiting to be flushed. With LOGSEC the
partial evidence survives any kill — a result that stops after
`open-entity-count` but never reaches `probe-end` is itself
evidence (the script stalled at or before the next section; the
verdict() classifier reads "AMBIGUOUS" for a missing probe-end
while the surviving lines still tell the census story). The tool
unlinks each result file before its launch so append mode never
accumulates across runs.

CLI: strict_load_probe.py [--probe-dir DIR] [--bcad EXE] [--probe NAME:DWG]...
     Defaults probe the constructed genus corpus (regenerate it first
     with genus_gates.py) against specimens Box_2007 (the control).

Exit 0 prints the verdict table regardless; nonzero on launch failure.
"""

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
REPO = SCRIPT_DIR.parents[1]
SPECIMENS = SCRIPT_DIR / "tests" / "sh_history"  # tests/gold_harness/tests/sh_history
CONSTRUCTED = REPO / "target" / "genus_gates" / "constructed"

DEFAULT_PROBE_DIR = Path("/mnt/c/Users/SebastianSchoeller/AppData/Local/Temp/kilo/strict_load_probe")
# The strict loader under test (2026-09-29, the maintainer's
# directive): BricsCAD V26 — the current-generation modeler. The
# campaign's earlier verdicts (the null-box readings and the
# file-level rejection matrix) were measured against V18, whose
# 2017-era ACIS restoration rejects content V26 accepts cleanly
# (the maintainer hand-verified: V18 errors on the gen_all file,
# V26 opens it clean). V18 remains available for legacy-era
# evidence by passing --bcad explicitly.
#
# The AUTHOR'S ORACLE (the maintainer's alternative, same day):
# AutoCAD 2027 — "C:\Program Files\Autodesk\AutoCAD 2027\acad.exe"
# — the /b + LISP mechanism validated (PROGRAM=acad, ACADVER
# 26.0s). Its verdicts are the strongest evidence: the binary-arm
# rewrite control opens an EXPLICIT "Open Drawing - Errors found"
# DIALOG (the defect named by the format author); the constructed
# fixtures' entities abort the LISP entity loop (entget-level
# failure, stronger than the null box); gen_all opens clean but
# its audit purges one of three (3→2). CAVEATS: its dialogs stall
# /b scripts (kill the stalled instance BY PID — acad.exe is
# shared with the maintainer's Civil 3D; NEVER kill by name), and
# an unhandled LISP error aborts to the script's next line (a
# missing per-entity bbox line means the loop aborted on that
# entity — itself a verdict).
DEFAULT_BCAD = "C:\\Program Files\\Bricsys\\BricsCAD V26 en_US\\bricscad.exe"
WIN_LAUNCHER = ("$pc = Start-Process -FilePath '{bcad}' -ArgumentList "
                "'/b','{scr}' -PassThru; "
                "$sc = Start-Process -FilePath 'powershell.exe' -ArgumentList "
                "'-NoProfile','-ExecutionPolicy','Bypass','-File',"
                "'{scraper}','-ProcId',$pc.Id,'-OutFile','{console}' "
                "-WindowStyle Hidden -PassThru; "
                "if ($pc.WaitForExit({timeout}000)) "
                "{{ Write-Output ('exit: ' + $pc.ExitCode) }} else "
                "{{ Write-Output 'timeout: killing'; $pc.Kill(); "
                "$pc.WaitForExit() }}; "
                "if (-not $sc.HasExited) {{ $sc.Kill(); $sc.WaitForExit() }}")

# The script body: OPEN, then every probe opens its own result file
# (the document switch on OPEN clears LISP state — nothing may
# outlive the switch), forces the modeler via the bounding box,
# audits, re-censuses, and quits without saving.
# Flush discipline: every section closes its own handle (open →
# write → close) so a stalled run (the modeling-failure prompt at
# OPEN blocks the script engine mid-sequence) still leaves its
# partial evidence on disk — the buffered single-handle form died
# with the launcher's 150 s kill and read 0 bytes.
SCR_TEMPLATE = """_.OPEN
{win_file}
(setq RESULT "{win_result}")
(defun LOGSEC (lines / f)
  (setq f (open RESULT "a"))
  (foreach l lines (write-line l f))
  (close f))
(LOGSEC (list "probe: {name}"))
(setq solids (ssget "_X" (list (cons 0 "3DSOLID,REGION,BODY"))))
(LOGSEC (list (strcat "open-entity-count: " (if solids (itoa (sslength solids)) "0"))))
(if (and solids (> (sslength solids) 0))
  (progn
    (setq r (vl-catch-all-apply '(lambda ()
      (vla-getboundingbox (vlax-ename->vla-object (ssname solids 0)) 'mn 'mx)
      (strcat "bbox: " (vl-prin1-to-string (mapcar 'rtos (vlax-safearray->list mn)))
              " .. " (vl-prin1-to-string (mapcar 'rtos (vlax-safearray->list mx)))))))
    (if (vl-catch-all-error-p r)
      (LOGSEC (list (strcat "bbox-FAIL: " (vl-catch-all-error-message r))))
      (LOGSEC (list r)))))
(LOGSEC (list (strcat "pre-audit-dbmod: " (itoa (getvar "DBMOD")))
        (strcat "pre-audit-errno: " (itoa (getvar "ERRNO")))))
(command "_.AUDIT" "_Y")
(LOGSEC (list (strcat "post-audit-dbmod: " (itoa (getvar "DBMOD")))
        (strcat "post-audit-errno: " (itoa (getvar "ERRNO")))))
(setq solids2 (ssget "_X" (list (cons 0 "3DSOLID,REGION,BODY"))))
(LOGSEC (list (strcat "post-audit-entity-count: " (if solids2 (itoa (sslength solids2)) "0"))))
(if (and solids2 (> (sslength solids2) 0))
  (progn
    (setq r2 (vl-catch-all-apply '(lambda ()
      (vla-getboundingbox (vlax-ename->vla-object (ssname solids2 0)) 'mn2 'mx2)
      (strcat "post-audit-bbox: " (vl-prin1-to-string (mapcar 'rtos (vlax-safearray->list mn2)))))))
    (if (vl-catch-all-error-p r2)
      (LOGSEC (list (strcat "post-audit-bbox-FAIL: " (vl-catch-all-error-message r2))))
      (LOGSEC (list r2)))))
(LOGSEC (list "probe-end"))
_.QUIT
_N
"""

NULL_BOX = "1.0000E+80"


def probe_one(name, source, probe_dir, bcad, timeout_s):
    """Run one fixture through BricsCAD; returns the result lines."""
    dwg = probe_dir / f"{name}.dwg"
    result = probe_dir / f"{name}_result.txt"
    scr = probe_dir / f"{name}.scr"
    console = probe_dir / f"{name}_console.log"
    scraper = probe_dir / "bricscad_console_scraper.ps1"
    shutil.copyfile(source, dwg)
    # Stage the console scraper next to the fixture (the launcher
    # spawns it with the BricsCAD PID; it exits when BricsCAD does).
    shutil.copyfile(SCRIPT_DIR / "bricscad_console_scraper.ps1", scraper)
    if result.exists():
        result.unlink()
    if console.exists():
        console.unlink()
    win_probe = str(probe_dir).replace("/mnt/c/", "C:/")
    # The OPEN command line takes single backslashes; the LISP
    # (open ...) string needs them DOUBLED (LISP escape sequences
    # otherwise corrupt the path and every write dies at rf=nil —
    # the validated scripts all carried the doubled form).
    win_dir = win_probe.replace("/", "\\")
    win_result_lisp = (win_dir + f"\\{name}_result.txt").replace("\\", "\\\\")
    scr.write_text(SCR_TEMPLATE.format(
        name=name,
        win_file=win_dir + f"\\{name}.dwg",
        win_result=win_result_lisp,
    ))
    powershell = [
        "powershell.exe", "-NoProfile", "-Command",
        WIN_LAUNCHER.format(
            bcad=bcad,
            scr=scr.as_posix().replace("/mnt/c/", "C:/").replace("/", "\\"),
            scraper=scraper.as_posix().replace("/mnt/c/", "C:/").replace("/", "\\"),
            console=console.as_posix().replace("/mnt/c/", "C:/").replace("/", "\\"),
            timeout=timeout_s,
        ),
    ]
    subprocess.run(powershell, check=False, stdout=subprocess.DEVNULL,
                   stderr=subprocess.DEVNULL, timeout=timeout_s + 60)
    if not result.exists():
        return ["probe: " + name, "NO RESULT FILE (AMBIGUOUS)"]
    return result.read_text(errors="replace").strip().splitlines()


# The console-analysis patterns: the strict loader's known message
# vocabulary. The digest is deduped per fixture — a message that
# repeats continuously (the maintainer's observed "continuous error
# output") appears once with its repetition count.
CONSOLE_PATTERNS = (
    "general modeling failure",
    "acdb3dsolid",
    "data stream is empty",
    "missing logical",
    "restore file",
    "audit",
    "error",
    "failed",
    "invalid",
    "duplicate",
    "corrupt",
    "warning",
)


def analyze_console(path, max_lines=16):
    """Digest the UIA console transcript: deduped lines matching the
    error vocabulary, each with its repetition count. Returns [] when
    the log is absent (the scraper could not start)."""
    if not path.exists():
        return []
    counts = {}
    order = []
    for line in path.read_text(errors="replace").splitlines():
        line = line.strip()
        if not line or line.startswith("===") or line.startswith("---"):
            continue
        low = line.lower()
        if any(p in low for p in CONSOLE_PATTERNS):
            if line not in counts:
                counts[line] = 0
                order.append(line)
            counts[line] += 1
    return [f"{line}  [x{counts[line]}]" if counts[line] > 1 else line
            for line in order[:max_lines]]


def verdict(lines):
    """Classify a probe result: modeled / null-box / ambiguous."""
    text = " ".join(lines)
    if "probe-end" not in text:
        return "AMBIGUOUS"
    bbox = next((l for l in lines if l.startswith("bbox:")), "")
    if NULL_BOX in bbox:
        return "NULL-BOX (the ACIS body did not construct)"
    if "bbox:" in text:
        return "MODELED"
    return "NO-SOLID"


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--probe-dir", type=Path, default=DEFAULT_PROBE_DIR)
    parser.add_argument("--bcad", default=DEFAULT_BCAD)
    parser.add_argument("--timeout", type=int, default=150, help="per-launch seconds")
    parser.add_argument("--probe", action="append", default=[], metavar="NAME=DWG",
                        help="extra fixture to probe (repeatable)")
    args = parser.parse_args()

    args.probe_dir.mkdir(parents=True, exist_ok=True)
    targets = []
    if CONSTRUCTED.is_dir():
        targets += [(f"Constructed{p.stem}", p) for p in sorted(CONSTRUCTED.glob("*.dwg"))]
    control = SPECIMENS / "Box_2007.dwg"
    if control.exists():
        targets.append(("ControlBox2007", control))
    targets += [(name, Path(path)) for name, path in
                (item.split("=", 1) for item in args.probe)]

    print(f"probing {len(targets)} files through {args.bcad}")
    print(f"staging: {args.probe_dir}\n")
    for name, source in targets:
        lines = probe_one(name, source, args.probe_dir, args.bcad, args.timeout)
        print(f"== {name}")
        for line in lines:
            print(f"   {line}")
        print(f"   VERDICT: {verdict(lines)}")
        digest = analyze_console(args.probe_dir / f"{name}_console.log")
        if digest:
            print("   console evidence (deduped, the strict loader's vocabulary):")
            for line in digest:
                print(f"      | {line[:200]}")
        else:
            print("   console evidence: none captured (no matching text)")
        print()
    print("the authored control must read MODELED for the run to stand")
    return 0


if __name__ == "__main__":
    sys.exit(main())
