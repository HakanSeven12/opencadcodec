#!/usr/bin/env python3
"""§20 strict_load_probe — the mechanized strict-loader audit (the eighth-layer instrument).

The §20.4 recorded procedure for strict-loader verdicts is the user-run
loader audit. This tool mechanizes the verifiable part of that
procedure: it drives the loader `/b <script>` (BricsCAD V26 by default,
AutoCAD 2027 — the format's author — alongside it; the DEFAULT run
exercises BOTH), and the script's LISP records the evidence a verdict
needs — the 3DSOLID/REGION/BODY aggregate (the modeler-forced
bounding box: a healthy restore yields real extents; a failed restore
yields the null-extents sentinel — ±1e80 under BricsCAD, ±1e20 under
AutoCAD), the widened per-kind census (CENSUS_KINDS — every entity
kind the canonical carries walks the trapped bbox force), the
DBMOD/ERRNO pre/post-audit record, and the post-audit census. The
campaign's acceptance — "clean open + clean audit, the solids
model" — is decided from those.

Validation (2026-09-30, the campaign CLOSED on both loaders — the
2026-09-29 null-box record below is the historical mid-state):
every constructed fixture + the full gen_all canonical + both
chimeras + all controls read MODELED with real extents and clean
audits under BricsCAD V26 AND AutoCAD 2027; the canonical's audit
reads TOTAL ERRORS 0 with dbmod 0 pre+post on both loaders (the
MLeader style repair + the MText attachment-repeat repair closed
the two loader-side every-open repairs). The widened per-kind
census (CENSUS_KINDS, 2026-09-30) walks all 29 kinds the canonical
carries — nothing is untested-by-divergence under either loader
(the XLine/Ray pair resolved as a non-defect by the
authored-control precedent: ACAD's type-filtered ssget never
returns construction lines, her own authored pair absent the same
way). The historical 2026-09-29 validation, kept as the record of
the mid-state: authored specimens 2007/2013/2018 all yielded real
extents (Box 0,0,0..1,2,3 across eras); the constructed corpus
yielded the null box at BOTH the current rank and the pre-rank code
— the constructed-SAB restore gap was pre-existing and
ordering-independent, and every historical loader reading was the
error surface, not the restored-solid census (see the G-B/G-A
records in IMPLEMENTATION.md §20.6).

The AUDIT-REPORT channels (2026-09-30, the maintainer's directive —
"ensure that you are receiving AUDIT reports"): the loaders' own
command-line logs are now harvested VERBATIM, per fixture per loader.
(1) The GUI LOGFILEON session log, BOTH loaders: the script wraps its
whole session — `_.LOGFILEON` before `_.OPEN` (so the open-time
restore diagnostics land too), `_.LOGFILEOFF` before QUIT — and
LOGSECs `(getvar "LOGFILENAME")` into the result file; the probe
copies that file to `{run}_audit.log` and unlinks the source (the
per-document log name is stable across runs of the same staged
fixture — AutoCAD derives it from the DWG name, BricsCAD from the
name + session timestamp — so without the unlink the next run's
report would append to the last one's) — and both GUI launchers
pin `-WorkingDirectory` to the staging dir so the per-document
log lands there (an unpinned launch inherits the WSL UNC working
directory and the harvest would miss it). VALIDATED LIVE (the
2026-09-30 channel experiments): AutoCAD 2027's log carries the
open banner ("Non Autodesk DWG …") + the full audit report
("Auditing Header/Tables/Entities Pass 1/Pass 2/Blocks/AcDsRecords
… Total errors found N fixed M … Erased K objects"); BricsCAD
V26's log carries the modeler's own words verbatim ("Name:
AcDbRegion(31) / Value: Modeling operation error: / Data stream
is empty / Validation: Invalid / Replaced by: Removed / 107
objects audited / Total errors found during audit 1, fixed 1").
The 2026-09-29 "LOGFILENAME unset, LOGFILEON produces no file"
finding was the V18-era/wxWidgets build — V26 (Qt) writes the
log; the sysvar stays read-only but the file it names is written
and readable, which is all the probe needs.
(2) The AutoCAD CORE CONSOLE (`--loader core`, accoreconsole.exe):
the headless core prints its ENTIRE command transcript — open
diagnostics, LISP echo, the audit report — to stdout, which the
launcher redirects to `{run}_core.log` (UTF-16 — read_log_text
decodes it); no dialogs exist to stall the run (the GUI /b flow's
known hazard), and the same census LISP runs there (the drawing
arrives via /i, so the script omits the OPEN stanza). ONE
measured limit (2026-09-30): the ActiveX bridge is DEAD in this
build's core console — vlax-ename->vla-object returns nil even
for entities entget reads cleanly — so the bbox modeler force is
unavailable there; the census logs `bbox-UNAVAILABLE (nil
ActiveX bridge)` and the verdict reads `AUDIT-REPORT` (the
modeler force stays the GUI channels' evidence; the transcript is
the core channel's). VALIDATED LIVE the same day.
CLOSED channels (tested, stay closed): stdout/stderr redirection
on bricscad.exe (both empty — GUI-subsystem app); WM_GETTEXT
(empty — the UI text is painted, not in window-text slots); UI
Automation (no Text/Value patterns, no accessibility bridge).
The window-lifecycle scraper stays staged as the DIALOG-APPEARANCE
evidence (which dialogs appeared, when — the modeling-failure
prompt is a top-level window), and the DBMOD/ERRNO record stays
as the audit-EFFECT evidence; the audit REPORT itself now comes
from the log channels above. The maintainer's hand-run
transcripts remain the highest-fidelity console evidence; the
probe's logs are the mechanized daily driver.

One staged-copy note: AutoCAD's `_.QUIT _N` answers "Really want
to discard all changes? <N>" with No — which KEEPS the changes,
so AutoCAD SAVES the staged copy on the way out (the `__acad.bak`
files are our own staged bytes; the core console prints "Drawing
saved in AutoCAD 2018 format" for the same reason). BricsCAD's
QUIT does not save. The staged copies are scratch — re-copied
from source at every launch — and every census section precedes
the quit, so no verdict reads post-save state.

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

CLI: strict_load_probe.py [--probe-dir DIR] [--loader both|bcad|acad|core]
      [--bcad EXE] [--acad EXE] [--acore EXE] [--probe NAME:DWG]...
The DEFAULT run exercises BOTH GUI loaders (BricsCAD V26 via
DEFAULT_BCAD and AutoCAD 2027 via DEFAULT_ACAD): every fixture is
probed under each, with per-loader verdicts (the result files and
report rows carry a __bcad/__acad suffix). `--loader core` runs
the AutoCAD core console (accoreconsole.exe via DEFAULT_ACORE)
instead — the dialog-free transcript channel. Verdict vocabulary:
MODELED (real extents) / NULL-BOX (the null-extents sentinel —
±1e80 under BricsCAD, ±1e20 under AutoCAD; the B-rep did not
construct) / NO-SOLID / AMBIGUOUS. The targets default to the
constructed genus corpus (regenerate it first with genus_gates.py)
plus the authored specimen Box_2007 (the control).

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
# The author's oracle (the maintainer's alternative directive): the
# format's own tool. The default run exercises BOTH loaders — every
# fixture is probed under each, with per-loader verdicts — so a
# defect that only one modeler surfaces is still caught.
DEFAULT_ACAD = "C:\\Program Files\\Autodesk\\AutoCAD 2027\\acad.exe"
# The core-console channel (--loader core): accoreconsole.exe — the
# dialog-free core. Its entire command transcript (open diagnostics,
# LISP echo, the audit report) goes to stdout, which Start-Process
# redirects. Validated live 2026-09-30; the /s script path must be
# ABSOLUTE (a relative one reads "Can't find file").
DEFAULT_ACORE = "C:\\Program Files\\Autodesk\\AutoCAD 2027\\accoreconsole.exe"
CORE_LAUNCHER = ("$p = Start-Process -FilePath '{acore}' -ArgumentList "
                 "'/i','{dwg}','/s','{scr}' -WorkingDirectory '{cwd}' "
                 "-RedirectStandardOutput '{out}' -RedirectStandardError '{err}' "
                 "-PassThru; "
                 "if ($p.WaitForExit({timeout}000)) "
                 "{{ Write-Output ('exit: ' + $p.ExitCode) }} else "
                 "{{ Write-Output 'timeout: killing'; $p.Kill(); "
                 "$p.WaitForExit() }}")
WIN_LAUNCHER = ("$pc = Start-Process -FilePath '{bcad}' -ArgumentList "
                "'/b','{scr}' -WorkingDirectory '{cwd}' -PassThru; "
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
# audits, re-censuses, and quits. The LOGFILEON wrap (2026-09-30,
# the maintainer's audit-report directive) captures the loader's
# own command-line log — the open-time restore diagnostics AND the
# audit report verbatim — into the file LOGFILENAME names; the
# `logfilename:` LOGSEC line tells the launcher where to harvest
# (see harvest_audit_log). LOGFILEON precedes OPEN so the open
# banner lands in the log; LOGFILEOFF precedes QUIT.
# Flush discipline: every section closes its own handle (open →
# write → close) so a stalled run (the modeling-failure prompt at
# OPEN blocks the script engine mid-sequence) still leaves its
# partial evidence on disk — the buffered single-handle form died
# with the launcher's 150 s kill and read 0 bytes.
# The per-entity census kind list (2026-09-30): every entity kind
# the gen_all canonical carries (29 kinds; POLYLINE covers the
# 2D/3D/polyface variants, DIMENSION every dimension flavor). The
# modeler-family aggregate above keeps the NULL-BOX verdict
# semantics; this list drives the per-entity census loop - the 26
# non-modeler kinds gain first-class bbox evidence under both
# loaders (previously only 3DSOLID/REGION/BODY were walked).
CENSUS_KINDS = ("POINT,LINE,CIRCLE,ARC,ELLIPSE,XLINE,RAY,SOLID,"
                "SHAPE,TEXT,MTEXT,SPLINE,POLYLINE,LWPOLYLINE,MESH,"
                "MLINE,INSERT,VIEWPORT,TOLERANCE,DIMENSION,LEADER,"
                "MULTILEADER,HATCH,3DFACE,3DSOLID,REGION,BODY")

CENSUS_LISP = """(setq RESULT "{win_result}")
(defun LOGSEC (lines / f)
  (setq f (open RESULT "a"))
  (foreach l lines (write-line l f))
  (close f))
(LOGSEC (list "probe: {name}"))
(LOGSEC (list (strcat "logfilename: " (getvar "LOGFILENAME"))))
(setq solids (ssget "_X" (list (cons 0 "3DSOLID,REGION,BODY"))))
(LOGSEC (list (strcat "open-entity-count: " (if solids (itoa (sslength solids)) "0"))))
;; the widened per-entity census (2026-09-30): the gen_all carries
;; 29 kinds and the census walks EVERY kind now - the modeler-family
;; aggregate above keeps the verdict semantics (the NULL-BOX
;; sentinel arises on the modeler family), the per-entity loop
;; below carries the full-kind bbox evidence.
(setq allk (ssget "_X" (list (cons 0 "{kinds}"))))
(LOGSEC (list (strcat "census-entity-count: " (if allk (itoa (sslength allk)) "0"))))
(if (and solids (> (sslength solids) 0))
  (progn
    ;; the ActiveX bridge is dead in accoreconsole (2026-09-30,
    ;; measured: vlax-ename->vla-object returns nil even for
    ;; entities entget reads cleanly) - convert first, force only
    ;; when the object resolved, and log the unavailable force
    ;; honestly so the verdict classifier can name it.
    ;; ASCII-ONLY in this template: the .scr is read by the
    ;; loader under the ANSI codepage, where a UTF-8 em-dash's
    ;; 0x94 byte is a curly double-quote that AutoCAD's
    ;; smart-quote normalization turns into a bogus string-open -
    ;; the 2026-09-30 stall (the LISP reader left at depth 2
    ;; swallowed LOGFILEOFF/DELAY/QUIT and the window hung until
    ;; the maintainer killed it).
    (setq obj0 (vlax-ename->vla-object (ssname solids 0)))
    (if obj0
      (progn
        (setq r (vl-catch-all-apply '(lambda ()
          (vla-getboundingbox obj0 'mn 'mx)
          (strcat "bbox: " (vl-prin1-to-string (mapcar 'rtos (vlax-safearray->list mn)))
                  " .. " (vl-prin1-to-string (mapcar 'rtos (vlax-safearray->list mx)))))))
        (if (vl-catch-all-error-p r)
          (LOGSEC (list (strcat "bbox-FAIL: " (vl-catch-all-error-message r))))
          (LOGSEC (list r))))
      (LOGSEC (list "bbox-force: UNAVAILABLE (nil ActiveX bridge)")))))
;; the while test is the COUNT COMPARISON - a pickset-typed test
;; (while solids ...) never terminates (the trapped entget below
;; can no longer abort the loop, which is the point of the trap).
(setq eidx 0)
(while (< eidx (sslength allk))
  (setq pent (ssname allk eidx))
  ;; the whole per-entity body is trapped: a broken-model entity can
  ;; fail even ENTGET (the recorded entget-level failure - the
  ;; failure aborts the enclosing expression, so a bare entget
  ;; would kill the whole loop). The prefix uses the index; a
  ;; healthy entget upgrades it with the handle.
  (setq pr (vl-catch-all-apply '(lambda (/ lhnd pobj)
    (setq lhnd (cdr (assoc 5 (entget pent))))
    (setq pobj (vlax-ename->vla-object pent))
    (if pobj
      (progn
        (vla-getboundingbox pobj 'pmn 'pmx)
        (strcat "ent[" lhnd "]: bbox: " (vl-prin1-to-string (mapcar 'rtos (vlax-safearray->list pmn)))
                " .. " (vl-prin1-to-string (mapcar 'rtos (vlax-safearray->list pmx)))))
      (strcat "ent[" lhnd "]: bbox-UNAVAILABLE (nil ActiveX bridge)")))))
  (if (vl-catch-all-error-p pr)
    (LOGSEC (list (strcat "ent[" (itoa eidx) "]: bbox-FAIL: " (vl-catch-all-error-message pr))))
    (LOGSEC (list pr)))
  (setq eidx (1+ eidx)))
(LOGSEC (list (strcat "pre-audit-dbmod: " (itoa (getvar "DBMOD")))
        (strcat "pre-audit-errno: " (itoa (getvar "ERRNO")))))
(command "_.AUDIT" "_Y")
(LOGSEC (list (strcat "post-audit-dbmod: " (itoa (getvar "DBMOD")))
        (strcat "post-audit-errno: " (itoa (getvar "ERRNO")))))
(setq solids2 (ssget "_X" (list (cons 0 "3DSOLID,REGION,BODY"))))
(LOGSEC (list (strcat "post-audit-entity-count: " (if solids2 (itoa (sslength solids2)) "0"))))
(setq allk2 (ssget "_X" (list (cons 0 "{kinds}"))))
(LOGSEC (list (strcat "post-census-entity-count: " (if allk2 (itoa (sslength allk2)) "0"))))
(if (and solids2 (> (sslength solids2) 0))
  (progn
    (setq obj0b (vlax-ename->vla-object (ssname solids2 0)))
    (if obj0b
      (progn
        (setq r2 (vl-catch-all-apply '(lambda ()
          (vla-getboundingbox obj0b 'mn2 'mx2)
          (strcat "post-audit-bbox: " (vl-prin1-to-string (mapcar 'rtos (vlax-safearray->list mn2)))
                  " .. " (vl-prin1-to-string (mapcar 'rtos (vlax-safearray->list mx2)))))))
        (if (vl-catch-all-error-p r2)
          (LOGSEC (list (strcat "post-audit-bbox-FAIL: " (vl-catch-all-error-message r2))))
          (LOGSEC (list r2))))
      (LOGSEC (list "post-audit-bbox-force: UNAVAILABLE (nil ActiveX bridge)")))))
(setq qi 0)
(while (< qi (sslength allk2))
  (setq qent (ssname allk2 qi))
  (setq qr (vl-catch-all-apply '(lambda (/ lhnd qobj)
    (setq lhnd (cdr (assoc 5 (entget qent))))
    (setq qobj (vlax-ename->vla-object qent))
    (if qobj
      (progn
        (vla-getboundingbox qobj 'qmn 'qmx)
        (strcat "post-ent[" lhnd "]: bbox: " (vl-prin1-to-string (mapcar 'rtos (vlax-safearray->list qmn)))
                " .. " (vl-prin1-to-string (mapcar 'rtos (vlax-safearray->list qmx)))))
      (strcat "post-ent[" lhnd "]: bbox-UNAVAILABLE (nil ActiveX bridge)")))))
  (if (vl-catch-all-error-p qr)
    (LOGSEC (list (strcat "post-ent[" (itoa qi) "]: bbox-FAIL: " (vl-catch-all-error-message qr))))
    (LOGSEC (list qr)))
  (setq qi (1+ qi)))
(LOGSEC (list "probe-end"))
"""

# The GUI /b form. Visible hold (the maintainer's directive,
# 2026-09-29): the script DELAYs 10 s before QUIT so each probe
# window stays observable — the /b lifecycle is otherwise a
# sub-20-second flash. AutoCAD's QUIT _N answers "discard? <N>"
# with No, i.e. it SAVES the staged copy on the way out (the .bak
# files are our own staged bytes; scratch, re-copied every
# launch, every census section precedes the save); BricsCAD's
# QUIT does not save.
SCR_TEMPLATE = """_.LOGFILEON
_.OPEN
{win_file}
""" + CENSUS_LISP + """_.LOGFILEOFF
_.DELAY 10000
_.QUIT
_N
"""

# The core-console form (accoreconsole.exe /i DWG /s SCR): the
# drawing arrives via /i (no OPEN stanza, no document switch), the
# transcript IS stdout — the launcher redirects it to {run}_core.log
# — and no dialogs exist to stall the run. (vl-load-com) arms the
# vla-* census functions there; no log wrap and no visible hold.
CORE_SCR_TEMPLATE = "(vl-load-com)\n" + CENSUS_LISP + """_.QUIT
_N
"""


def assert_lisp_balanced(scr_text, label):
    """The STALL GUARD (2026-09-30): verify the generated .scr's LISP
    parenthesizes to depth 0 before it is ever handed to a loader.
    An unbalanced form does not fail loudly - the loader's LISP
    reader sits at the pending depth, swallows the script's tail
    (LOGFILEOFF/DELAY/QUIT) as input, and the window stalls until
    a human kills it (the maintainer caught ConstructedBox__acad
    exactly so: the reader at ((_> with two whiles unclosed - a
    one-paren-short else branch). Comments ( ;) and strings (with
    backslash escapes) are tracked; a negative depth or an
    unterminated string aborts the launch with the defect named."""
    depth = 0
    in_str = False
    esc = False
    for line_no, line in enumerate(scr_text.splitlines(), 1):
        in_comment = False
        for ch in line:
            if in_comment:
                continue
            if in_str:
                if esc:
                    esc = False
                elif ch == "\\":
                    esc = True
                elif ch == '"':
                    in_str = False
                continue
            if ch == ";":
                in_comment = True
            elif ch == '"':
                in_str = True
            elif ch == "(":
                depth += 1
            elif ch == ")":
                depth -= 1
                if depth < 0:
                    raise AssertionError(
                        f"{label}: unbalanced .scr - extra close paren "
                        f"at line {line_no} (depth {depth})")
    if depth != 0 or in_str:
        raise AssertionError(
            f"{label}: unbalanced .scr - final paren depth {depth}, "
            f"string open at EOF: {in_str}; the script would stall "
            f"the loader's LISP reader mid-run (the 2026-09-30 stall "
            f"class) - fix the template before launching")

# The modelers' null-extents sentinels (a failed restore yields the
# platform's infinite-extents marker): BricsCAD writes ±1e80, AutoCAD
# writes ±1e20 (measured 2026-09-29, the both-loader verification run).
NULL_BOXES = ("1.0000E+80", "1.0000E+20")


def probe_one(name, source, probe_dir, loader, timeout_s, mode="gui"):
    """Run one fixture through the loader; returns the result lines."""
    dwg = probe_dir / f"{name}.dwg"
    result = probe_dir / f"{name}_result.txt"
    scr = probe_dir / f"{name}.scr"
    console = probe_dir / f"{name}_console.log"
    core_log = probe_dir / f"{name}_core.log"
    audit_log = probe_dir / f"{name}_audit.log"
    scraper = probe_dir / "bricscad_console_scraper.ps1"
    shutil.copyfile(source, dwg)
    # Stage the console scraper next to the fixture (the launcher
    # spawns it with the loader PID; it exits when the loader does).
    # The core console has no windows to scrape — skip it there.
    if mode == "gui":
        shutil.copyfile(SCRIPT_DIR / "bricscad_console_scraper.ps1", scraper)
    for stale in (result, console, core_log, audit_log,
                  probe_dir / f"{name}_core.err"):
        if stale.exists():
            stale.unlink()
    win_probe = str(probe_dir).replace("/mnt/c/", "C:/")
    # The OPEN command line takes single backslashes; the LISP
    # (open ...) string needs them DOUBLED (LISP escape sequences
    # otherwise corrupt the path and every write dies at rf=nil —
    # the validated scripts all carried the doubled form).
    win_dir = win_probe.replace("/", "\\")
    win_result_lisp = (win_dir + f"\\{name}_result.txt").replace("\\", "\\\\")
    if mode == "core":
        # encoding="ascii" is the ENCODING half of the stall guard:
        # a non-ASCII byte in the template (a UTF-8 em-dash reads
        # as CP1252 0x94, a curly quote) throws HERE instead of
        # reaching the loader; assert_lisp_balanced is the PAREN
        # half (the 2026-09-30 stall was a one-paren-short else
        # branch - both halves are launch-time assertions now).
        scr_text = CORE_SCR_TEMPLATE.format(
            name=name,
            win_result=win_result_lisp,
            kinds=CENSUS_KINDS,
        )
        assert_lisp_balanced(scr_text, f"{name} (core)")
        scr.write_text(scr_text, encoding="ascii")
        launcher = CORE_LAUNCHER.format(
            acore=loader,
            dwg=dwg.as_posix().replace("/mnt/c/", "C:/").replace("/", "\\"),
            scr=scr.as_posix().replace("/mnt/c/", "C:/").replace("/", "\\"),
            cwd=win_dir,
            out=core_log.as_posix().replace("/mnt/c/", "C:/").replace("/", "\\"),
            err=(probe_dir / f"{name}_core.err").as_posix()
                .replace("/mnt/c/", "C:/").replace("/", "\\"),
            timeout=timeout_s,
        )
    else:
        scr_text = SCR_TEMPLATE.format(
            name=name,
            win_file=win_dir + f"\\{name}.dwg",
            win_result=win_result_lisp,
            kinds=CENSUS_KINDS,
        )
        assert_lisp_balanced(scr_text, f"{name} (gui)")
        scr.write_text(scr_text, encoding="ascii")
        launcher = WIN_LAUNCHER.format(
            bcad=loader,
            scr=scr.as_posix().replace("/mnt/c/", "C:/").replace("/", "\\"),
            cwd=win_dir,
            scraper=scraper.as_posix().replace("/mnt/c/", "C:/").replace("/", "\\"),
            console=console.as_posix().replace("/mnt/c/", "C:/").replace("/", "\\"),
            timeout=timeout_s,
        )
    subprocess.run(["powershell.exe", "-NoProfile", "-Command", launcher],
                   check=False, stdout=subprocess.DEVNULL,
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


def win_to_wsl(path):
    """A LISP-reported Windows path -> the WSL path the probe reads.
    C:\\a\\b -> /mnt/c/a/b; a \\wsl.<distro>\\ UNC path (what the
    loaders write when launched from a WSL working directory) maps
    to the same local tree."""
    p = path.strip().strip('"')
    if p.startswith("\\\\wsl."):
        # \\wsl.localhost\<distro>\home\... -> /home/...
        return "/" + "/".join(p.split("\\")[3:])
    if len(p) > 3 and p[1] == ":":
        return "/mnt/" + p[0].lower() + "/" + p[3:].replace("\\", "/")
    return p


def read_log_text(path):
    """Read a loader log/transcript. The BricsCAD/AutoCAD session
    logs are ANSI; accoreconsole's stdout transcript arrives as
    UTF-16 (the BOM-less NUL-heavy form when redirected). Detect
    and decode accordingly."""
    data = Path(path).read_bytes()
    if data[:2] in (b"\xff\xfe", b"\xfe\xff"):
        return data.decode("utf-16", errors="replace")
    head = data[:512]
    if head and head.count(b"\x00") > len(head) // 3:
        # NUL-heavy without a BOM: the UTF-16LE console stream.
        return data.decode("utf-16-le", errors="replace")
    return data.decode("utf-8", errors="replace")


def harvest_audit_log(run, probe_dir, mode):
    """Copy the loader's audit-report evidence to {run}_audit.log.

    GUI runs: the session log the script's LOGFILEON wrap wrote —
    its path arrives in the result file's `logfilename:` line. The
    source is then unlinked: AutoCAD derives the per-document log
    name from the DWG name (stable across runs of the same staged
    fixture), so without the unlink the next run's report would
    append to the last one's. The file is named after OUR staged
    copy — never the maintainer's own drawings.
    Core runs: the stdout transcript the launcher redirected to
    {run}_core.log (kept as-is — it is per-run already).
    Returns the harvested path, or None when no channel produced a log.
    """
    out = probe_dir / f"{run}_audit.log"
    if mode == "core":
        src = probe_dir / f"{run}_core.log"
        if src.exists() and src.stat().st_size > 0:
            shutil.copyfile(src, out)
            return out
        return None
    result = probe_dir / f"{run}_result.txt"
    if not result.exists():
        return None
    line = next((l for l in result.read_text(errors="replace").splitlines()
                 if l.startswith("logfilename: ")), None)
    if not line:
        return None
    src = Path(win_to_wsl(line.split("logfilename: ", 1)[1].strip()))
    if not src.exists():
        return None
    out.write_text(read_log_text(src))
    try:
        src.unlink()
    except OSError:
        pass
    return out


# The audit-report vocabulary: the lines that carry the loaders' own
# verdicts — the audit passes and totals, the modeler's failure text,
# the entity-level findings.
AUDIT_PATTERNS = (
    "auditing",
    "objects audited",
    "blocks audited",
    "total errors found",
    "erased",
    "name: acdb",
    "modeling operation error",
    "data stream is empty",
    "validation",
    "replaced by",
    "missing logical",
    "restore file",
    "non autodesk",
    "invalid",
)


def analyze_audit_log(path, max_lines=24):
    """The verbatim audit-report digest: the report lines from the
    loader's session log / core transcript, whitespace-collapsed,
    consecutive duplicates dropped. Returns [] when no log was
    harvested."""
    if not path or not Path(path).exists():
        return []
    out = []
    last = None
    for raw in read_log_text(path).splitlines():
        line = " ".join(raw.split())
        if not line:
            last = None
            continue
        low = line.lower()
        if any(p in low for p in AUDIT_PATTERNS):
            if line != last:
                out.append(line)
                last = line
    return out[:max_lines]


def verdict(lines):
    """Classify a probe result: modeled / null-box / ambiguous.

    The aggregate verdict reads entity[0]'s bbox line only (the
    `bbox:`/`bbox-FAIL:` aggregate lines stay first-class for the
    classifier's compatibility); the per-entity `ent[<handle>]:`
    lines carry the finer census for multi-entity files (a mixed
    file — some entities modeling, some aborting — reads by its
    entity[0] aggregate here and per-entity in the detail lines).
    """
    text = " ".join(lines)
    if "probe-end" not in text:
        return "AMBIGUOUS"
    if "UNAVAILABLE (nil ActiveX bridge)" in text:
        return ("AUDIT-REPORT (no modeler force in this loader — the "
                "census stays entity counts + the verbatim audit text)")
    bbox = next((l for l in lines if l.startswith("bbox:")), "")
    if any(s in bbox for s in NULL_BOXES):
        return "NULL-BOX (the ACIS body did not construct)"
    if bbox:
        return "MODELED"
    return "NO-SOLID"


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--probe-dir", type=Path, default=DEFAULT_PROBE_DIR)
    parser.add_argument("--bcad", default=DEFAULT_BCAD,
                        help="the BricsCAD executable (default: V26)")
    parser.add_argument("--acad", default=DEFAULT_ACAD,
                        help="the AutoCAD executable (default: 2027)")
    parser.add_argument("--acore", default=DEFAULT_ACORE,
                        help="the AutoCAD core console executable "
                             "(accoreconsole.exe, for --loader core)")
    parser.add_argument("--loader", choices=("both", "bcad", "acad", "core"),
                        default="both",
                        help="which loader(s) exercise the fixtures "
                             "(default: both GUI loaders — every fixture "
                             "under each, per-loader verdicts; 'core' = "
                             "the AutoCAD core console transcript channel)")
    parser.add_argument("--timeout", type=int, default=150, help="per-launch seconds")
    parser.add_argument("--probe", action="append", default=[], metavar="NAME=DWG",
                        help="extra fixture to probe (repeatable)")
    args = parser.parse_args()

    if args.loader == "core":
        loaders = {"core": (args.acore, "core")}
    else:
        # "both" = the two GUI loaders only — the default run stays
        # bounded; the core console is its own explicit pass.
        loaders = {}
        if args.loader in ("both", "bcad"):
            loaders["bcad"] = (args.bcad, "gui")
        if args.loader in ("both", "acad"):
            loaders["acad"] = (args.acad, "gui")

    args.probe_dir.mkdir(parents=True, exist_ok=True)
    targets = []
    if CONSTRUCTED.is_dir():
        targets += [(f"Constructed{p.stem}", p) for p in sorted(CONSTRUCTED.glob("*.dwg"))]
    control = SPECIMENS / "Box_2007.dwg"
    if control.exists():
        targets.append(("ControlBox2007", control))
    targets += [(name, Path(path)) for name, path in
                (item.split("=", 1) for item in args.probe)]

    print(f"probing {len(targets)} files through {len(loaders)} loader(s): "
          + ", ".join(f"{tag}={path}" for tag, (path, _mode) in loaders.items()))
    print(f"staging: {args.probe_dir}\n")
    for name, source in targets:
        for tag, (loader, mode) in loaders.items():
            run = f"{name}__{tag}"
            lines = probe_one(run, source, args.probe_dir, loader, args.timeout, mode)
            print(f"== {run}")
            for line in lines:
                print(f"   {line}")
            print(f"   VERDICT: {verdict(lines)}")
            digest = analyze_console(args.probe_dir / f"{run}_console.log")
            if digest:
                print("   console evidence (deduped, the strict loader's vocabulary):")
                for line in digest:
                    print(f"      | {line[:200]}")
            else:
                print("   console evidence: none captured (no matching text)")
            audit = analyze_audit_log(harvest_audit_log(run, args.probe_dir, mode))
            if audit:
                print("   audit report (verbatim, the loader's own log):")
                for line in audit:
                    print(f"      | {line[:160]}")
            else:
                print("   audit report: none captured (no log channel)")
            print()
    print("the authored control must read MODELED for the run to stand")
    return 0


if __name__ == "__main__":
    sys.exit(main())
