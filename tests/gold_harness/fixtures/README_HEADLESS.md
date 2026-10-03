# README_HEADLESS.md — unattended (headless) authoring of gold fixtures

The step-by-step procedures for authoring fixture DWGs **without any
interactive CAD session**, for both engines resident on this machine,
built from the measured `cylinder_2013` reference run (2026-10-02:
seed `acadiso.dwt` → one `CYLINDER` op → `SAVEAS 2013`, oracle-clean on
both engines). This is the authoring arm of the fixtures work; the
landing checklist stays [`README.md`](README.md) and the campaign tables
live in [`../IMPLEMENTATION.md` §F2](../IMPLEMENTATION.md). The loader
audit channels these steps reuse are documented in
[`../loaders/strict_load_probe.py`](../loaders/strict_load_probe.py).

## Engine matrix (measured 2026-10-02)

| | **AutoCAD 2027** | **BricsCAD V26** |
|---|---|---|
| headless binary | `accoreconsole.exe` — a TRUE console (PE subsystem 3) | **none** — the full GUI app, launched hidden |
| launch | `accoreconsole.exe /i <seedfile> /s <script>` | `bricscad.exe /Automation /b <script>` |
| window | never any | hidden; main-window title measured EMPTY for the whole run (~10 s, exit 0) |
| transcript | its own stdout — the full command transcript, arrives UTF-16 BOM-less-NUL-heavy when redirected | stdout EMPTY (GUI-subsystem app) — evidence = LISP-written result file + the `LOGFILEON` session log |
| script dialect | `_.CYLINDER 0,0,0 1 2`, `_.SAVEAS 2013 <path>` | **identical** — same tokens, same order |
| LISP | entget/ssget/princ alive; **vlax bridge DEAD** (`vlax-ename->vla-object` → nil — bbox force unavailable) | full Visual LISP; **vlax bridge ALIVE** |
| quit saves? | with `_N` it quit-saves in place (`.bak` noise — answer `_Y` after a SAVEAS: discard nothing) | **same hazard** (2026-10-02 correction of the probe-era record): "Disregard changes?" answered `_N` KEEPS and **saves the dirty doc in place** — a mid-script mis-feed that leaves the seed doc dirty poisoned the staged seed exactly like AutoCAD would; answer `_Y` |
| era save tokens | `R14` (the console's list: R14(LT98&LT97)/2000/.../2018/DXF/Template) | `R13` verified; the full measured list is **R11/R12/R13/LT2/R14/LT95/2000/2004/2007/2010/2013/2018/DXF** — one era wider than the TODO B3 "R13+" record |
| interactive session | not required | required (hidden window, but a desktop session hosts the process) |
| authorable versions | R14+ (the TODO B3 maintainer measurement) | **R13+** — BricsCAD is the R13 author for the B3 wave |

## The reference recipe (both engines)

Per the [`README.md`](README.md) checklist, one fixture = fresh drawing from
a seed template, layer 0, **exactly ONE operation**, then `SAVEAS` to the
target version. Headless just replaces the human at the keyboard:

1. **Stage the seed.** Copy the template to a scratch dir and pass the
   COPY to the engine — never the Program Files original (a quit-time
   save-back or an accidental in-place save must not be able to reach it;
   the cylinder runs verified both originals stayed byte-identical).
   - AutoCAD ships `acadiso.dwt` (ISO, `MEASUREMENT=1`, `INSUNITS=4`) at
     `C:\Program Files\Autodesk\AutoCAD 2027\UserDataCache\en-us\Template\`.
   - **BricsCAD ships no `acadiso.dwt`** — its metric line is
     `...\BricsCAD V26 en_US\UserDataCache\Templates\en_US\Default-mm.dwt`
     (also `3D-modeling-mm.dwt`). For strict engine A/B parity BricsCAD
     opens the staged AutoCAD `acadiso.dwt` (measured clean — the seed's
     metric header carried into the save). Record the seed in the `.txt`.
2. **Write the `.scr`.** Shared dialect rules (measured):
   - ASCII only — /b scripts are read under the ANSI codepage; a UTF-8
     em-dash byte becomes a smart-quote that stalls the LISP reader
     (the 2026-09-30 stall class).
   - Every LISP line must be paren-balanced (the stall guard — an open
     form swallows the script's tail; see `assert_lisp_balanced`).
   - LISP string literals carry DOUBLED backslashes; command-prompt
     responses carry single ones.
   - No tabs, no trailing blanks (a trailing space acts as an extra Enter).
   - The engine cwd is pinned to the scratch dir (`Start-Process
     -WorkingDirectory`) — otherwise a WSL-launched run inherits the
     UNC cwd and per-document artifacts land unpredictably.
3. **The operation lines.** The cylinder example, verbatim:

   ```
   _.CYLINDER
   0,0,0
   1
   2
   _.SAVEAS
   2013
   C:\Users\<user>\Downloads\cylinder_2013.dwg
   ```

   `SAVEAS` takes the version token then the path — identical prompt
   order on both engines (`2013` measured; for other versions the token
   must be measured before batch-relying on it — see the getschas list).
4. **The evidence stanza.** Read-only LISP after the save — an entity
   census plus the template-seed proof, written with the LOGSEC flush
   discipline (open/append/close per line, so a killed run still leaves
   its partial verdict):

   ```
   (setq RESULT "C:\\<scratch>\\<stem>_result.txt")
   (defun LOGSEC (lines / f) (setq f (open RESULT "a")) (foreach l lines (write-line l f)) (close f))
   (LOGSEC (list (strcat "census: last=" (cdr (assoc 0 (entget (entlast)))))))
   (LOGSEC (list (strcat "census: measurement=" (itoa (getvar "MEASUREMENT"))
                         " insunits=" (itoa (getvar "INSUNITS")))))
   (LOGSEC (list "probe-end"))
   ```

   AutoCAD's console also echoes everything to stdout, so there the
   `(princ …)` forms are enough; BricsCAD has no stdout — the result
   file IS the channel, plus `_.LOGFILEON` … `_.LOGFILEOFF` around the
   session for the command log (harvest the path from
   `(getvar "LOGFILENAME")`, copy it out, then **unlink the source** —
   the next same-named run appends).
5. **The exit.** End the script with `_.QUIT` and the discard answer:
   `_Y` on BOTH engines (post-SAVEAS there is nothing to discard; the
   alternative `_N` keeps-and-saves the still-dirty document in place —
   the 2026-10-02 seed-contamination measurement corrected the probe-era
   "BricsCAD's QUIT does not save" record: a mis-fed mid-script op that
   skips its SAVEAS leaves the seed doc dirty and `_N` writes it back).
   The console exits when its script ends anyway; the GUI app needs the
   QUIT. **Re-stage the pristine seed before every launch** — never rely
   on quit answers to protect a shared seed copy.
6. **Launch, wait, kill-by-PID on timeout** (both engines, the
   strict_load_probe launcher pattern):

   ```powershell
   $p = Start-Process -FilePath '<engine>' -ArgumentList '<switches>' `
        -WorkingDirectory '<scratch>' -PassThru   # + redirects for accoreconsole
   if ($p.WaitForExit(<timeout-ms>)) { Write-Output ('exit: ' + $p.ExitCode) }
   else { Write-Output 'timeout: killing'; $p.Kill(); $p.WaitForExit() }
   ```

   Never kill by name — `acad.exe` is shared with the maintainer's
   Civil 3D sessions (the probe's standing rule). accoreconsole's `/s`
   path must be ABSOLUTE (relative reads "Can't find file" — measured).
   Fresh-product cold runs measured ~10 s on this machine; 240 s is a
   generous default.
7. **Sweep before you run.** Unlink the stale target DWG (avoids any
   SAVEAS-overwrite exchange) and the stale result/log files (append
   channels) before each launch — the launchers do this.
8. **Qualify before landing** ([`README.md`](README.md) steps 3–5):
   - `dwgread -O JSON <file>` — zero `Error` lines, exit 0. Note the
     known JSON quirk: bare `-nan`/`nan` tokens are invalid JSON and
     need the `normalize_gold.py` shim when post-checking by script.
   - target class present and census minimal (the cylinder reference:
     `3DSOLID: 1` + `ACSH_CYLINDER_CLASS`/`ACSH_HISTORY_CLASS: 1` —
     BricsCAD V26 also writes the ACSH cylinder genus, so SH fixtures
     authored on either engine carry the modeler records).
   - magic bytes match the target version (see table below).
   - Then land as `<campaign>/<Entity>_<version>.dwg` + sibling `.txt`
     provenance (author app, build, seed template, exact command
     sequence, save token, qualification result) and extend
     `run_corpus.py::in_scope_files` per §F2.2 step 7.

### Magic bytes (file-header version markers)

`AC1014` R14 · `AC1015` 2000 · `AC1018` 2004 · `AC1021` 2007 ·
`AC1024` 2010 · `AC1027` 2013 · `AC1032` 2018

## The reference run, fully scripted (2026-10-02 record)

The complete worked pair lives outside the repo as long as the machine
stays standing — the launchers are self-contained (they embed and
(re)write their `.scr`, stage the seed, sweep, launch, harvest, verify
and print PASS/FAIL):

- `C:\Users\<user>\Downloads\gen_cylinder_2013.py` (+ `cylinder_2013.scr`)
  — AutoCAD 2027 core console; transcript = redirected stdout (decode:
  BOM-less UTF-16).
- `C:\Users\<user>\Downloads\gen_cylinder_2013_bcad.py`
  (+ `cylinder_2013_bcad.scr`) — BricsCAD V26 hidden `/Automation /b`;
  transcript = result file + harvested session log.

Measured outcomes (both oracle-clean, `dwgread` exit 0 / zero Error
lines / one `3DSOLID` / `INSUNITS=4` from the ISO seed):

| artifact | engine | bytes | magic | objects |
|---|---|---|---|---|
| `cylinder_2013.dwg` | accoreconsole /i acadiso.dwt | 34 995 | AC1027 | 152 |
| `cylinder_2013_bcad.dwg` | bricscad /Automation /b | 29 875 | AC1027 | 138 |

(The baseline census differs — 24 vs 25 VISUALSTYLE, 16 vs 6 XRECORD,
+`SUN` under BricsCAD — expected engine-baseline noise, both "minimal"
by the fixture definition; the entity payload is identical.)

## Measured limits and gotchas

- **No structural blank lines in a `.scr`** (the 2026-10-02 stall class,
  B3 batch 1): a blank line at the idle `Command:` prompt is an Enter
  that re-triggers the last command and swallows the following script
  lines — it stalls the run invisibly. Only DELIBERATE in-command blanks
  (command terminators, prompt defaults) belong in a script, and every
  generator must assemble scripts from a line list so no accidental
  blanks appear between sections.
- **`-INSERT` prompts X scale, Y scale AND rotation — on BOTH engines**
  (2026-10-02 measurements: AutoCAD 2027's console shows `Enter Y scale
  factor <use X scale factor>` and BricsCAD asks `Y scale factor: <Equal
  to X scale>`), so the blank pattern is point + `_blank_ + _blank_ +
  _blank_`. A short-by-one feed misaligns the rest of the script into
  the rotation prompt.
- **BricsCAD `-HATCH` needs a view aperture for the internal-point
  boundary** ("Nothing was found on screen" in the hidden run). The
  headless recipe: `_.-HATCH` → `_S` (Select entities) → `_L` (Last) →
  blank (end selection) → blank (place). NOTE the R13 downsave EXPLODES
  the hatch (anonymous block of pattern lines + INSERT) — hatch does not
  persist pre-R14, which is the era-correct outcome recorded as refusal
  data, not a fixture (the 2026-10-02/03 Hatch_r13 refusal record).
- **accoreconsole `/i` accepts a `.dwt`** directly (the run opened the
  staged `acadiso.dwt`; its SAVEAS banner even read "Current file
  format: AutoCAD Drawing Template (*.dwt)"). BricsCAD's `_.OPEN` takes
  the `.dwt` path equally.
- **`(vla-SaveAs … 60)`** — if a version token ever fails, the LISP
  fallback exists under BricsCAD's live bridge (`60` = `ac2013_dwg`,
  from the shipped COM enum); the AutoCAD console has no bridge — use
  the script tokens there.
- **REGEN before selection — the console's universal lever** (the
  2026-10-03 root-cause discovery, B6): entmade entities are INVISIBLE
  to AutoCAD's selection machinery until a regen displays them, and
  the regen is DEFERRED while a LISP `(command)` sequence is active.
  That one mechanism produced every earlier "aperture wall" reading
  ("0 found" windows, ignored enames, "No valid constraint point
  found") — and the visible-GUI stall where the arc appeared only AFTER
  the failed sweep (the engine idled, then regen'd). The measured
  console selection model: **entmake → `_.REGEN` → select via ENAMES
  works** (ERASE "1 found"; SWEEP profile+path both picked — the
  SweepSurfArc family landed 4/4 this way); **typed windows NEVER work
  in the console** (still "0 found" post-REGEN — no screen, ever).
- **The constraint command family is ABSENT from accoreconsole**
  (2026-10-03): Core Console carries only accore.dll commands; the
  constraint manager (GC*/DC*) lives in acad.exe modules — the same
  boundary that keeps AutoCAD LT from creating constraints (the official
  docs name the module split). GCPARALLEL/GCTANGENT self-cancel after
  the first pick prompt with the same signature on infinite AND bounded
  geometry.
- **The GUI constraint commands are NOT script-drivable** (2026-10-03,
  both dialects measured): `(command)`-wrapped enames are not consumed
  by the acquisition prompts (the second ename dumps at the idle prompt;
  the next `(command)` reads "LISP command is not available"), and
  plain typed points at `Select first object:` read "Invalid selection
  for Tangent..." even when the point is ON the entity — the constraint
  commands use the interactive acquisition pipeline (mouse). This
  closes AutoCAD's constraint routes headless in every dialect.
- **BricsCAD's constraint commands ARE script-drivable** (the B6
  constraint families' engine): classic keyword prompts
  (`Select first entity [selection options (?)]`, `Select first
  constraint point or [Entity] <Entity>:`) that take typed POINT PICKS
  on entities. Measured dialect details: the XLINE is NOT
  point-pickable at constraint prompts (the point falls through to an
  implicit window → "Invalid input. Please select a line/straight
  polyline segment.") — pick it via `_L` (Last); the dimensional
  constraints (DCLINEAR/DCRADIUS) ask `Dimension text <measured>:`
  after the location — answer with a deliberate blank (accept the
  default); and the 2D constraint manager REFUSES 3D curves (its own
  error text names its domain: line/straight polyline segment/arc/
  curved polyline segment — the ConstrHelix refusal datum).
- **An infinite path is not sweepable — on either engine** (2026-10-03,
  verbatim): `This entity cannot be a sweeping path.` for both XLINE
  and RAY paths (profile selection succeeded). Geometric truth, not a
  tooling gap: a sweep along an unbounded path has no defined extent.
  The well-posed bounded-path replacement (ARC) completes the
  surface-sweep path-kind map; the infinite-line kind-19 route rides
  the constraint network instead (an XLINE IS constrainable on
  BricsCAD even though it is not sweepable).
- **BricsCAD has NO associative-surface machinery** (2026-10-03, the
  ASSOCPATHACTIONPARAM authoring attempt): its surface-mode sweep
  completes but writes NO ASSOC modeling network — `SURFACEASSOCIATIVITY`
  (AutoCAD's control sysvar) is absent (`getvar` → nil), the NOD carries
  no `ACAD_ASSOCNETWORK` after the sweep, and the sweep CONSUMES the
  profile circle (AutoCAD's associative sweep keeps profile+path because
  the network references them). `ASSOCPATHACTIONPARAM` is therefore
  AutoCAD-modeler-specific on this toolchain; BricsCAD-authored
  sweep-surface files read `ARC` + the surface as `UNKNOWN_ENT` with
  zero ASSOC classes (the SweepSurfArcB family is the recorded A/B).
  `ASSOCGEOMDEPENDENCY`, by contrast, IS BricsCAD-authored — via the
  constraint commands (the landed GConstrNet/GConstrXline/ConstrSmooth/
  DimConstr fixtures carry it).
- **acad.exe `/Automation /t /b` still SHOWS its window** (2026-10-03,
  the maintainer watched the run) — the GUI script route is visible;
  the hidden AutoCAD route is accoreconsole. Also: a manually closed
  GUI run resets LOGFILEPATH, so its session log lands in the
  `-WorkingDirectory` afterwards (harvest there when a run was closed
  by hand).
- **Headless-unauthorable operations** (updated after the B6
  campaign): `ACSH_BREP_CLASS` fixtures (no user-facing lever — B4),
  `LOFT` draft/settings variants (no prompt path in this release — B5),
  **the AutoCAD constraint family** (module-absent in console,
  acquisition-pipeline in GUI — see above; BricsCAD authors them), and
  **constraints on 3D curves** (the 2D manager's domain limit, both
  measured refusals recorded). `SWEEP` in accoreconsole moved OFF this
  list — it is authorable via REGEN + ename (the falsified B6-era
  "view aperture" record; the SweepSurfArc family is the proof).
- **The console's dialogs cannot stall accoreconsole** (it auto-answers
  its internal MessageBox), but the **hidden BricsCAD run has no
  dialogs to stall** (measured clean); the *visible* `/b` GUI flow can
  still trip on first-run/trust dialogs — preflight interactively once
  per profile before relying on any unattended route.
- **Versioned COM ProgIDs**: the unversioned `AutoCAD.Application`
  ProgID on this machine resolves to BricsCAD (last-registered wins) —
  never rely on unversioned ProgIDs for automation.
- The **R13/R14 era** (B3): BricsCAD is the R13 author (AutoCAD starts
  at R14) — an era extension means BricsCAD-authored stems.

**Scope note**: these procedures author fixtures the same way an
interactive session would; they do not change the oracle (LibreDWG
stays read-only, the differ and ignore-list stay frozen — the
AGENTS.md rules).
