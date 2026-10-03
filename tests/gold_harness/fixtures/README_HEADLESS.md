# README_HEADLESS.md — unattended (headless) authoring of gold fixtures

The step-by-step procedures for authoring fixture DWGs **without any
interactive CAD session**, for both engines resident on this machine,
built from the measured `cylinder_2013` reference run (2026-10-02:
seed `acadiso.dwt` → one `CYLINDER` op → `SAVEAS 2013`, oracle-clean on
both engines) and hardened by four landed campaigns: the R13/R14
specimen wave (`r13_r14/`), the B6 routes (`b6_routes/`), the
golden-entity matrix (`golden_entities/` — 30 families × 8 versions,
221 fixtures), and the ACSH_BREP_CLASS mint (`brep/`). This is the
authoring arm of the fixtures work; the landing checklist stays
[`README.md`](README.md) and the campaign tables live in
[`../IMPLEMENTATION.md` §F2](../IMPLEMENTATION.md). The loader audit
channels these steps reuse are documented in
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
| boolean history | `SOLIDHIST` sysvar — **default 0 strips ALL boolean history**; set 1 to record operand chains (the BREP-mint discovery) | **no retention switch exists** (`SOLIDHIST` absent, `getvar` errors) — booleans record a bare `ACSH_HISTORY_CLASS` root only, operand nodes discarded, at every save format |
| `CONVTOSOLID` input | **MESH objects only** — a PFACE is refused ("Object cannot be converted") | **accepts POLYFACE meshes too** (the one measured raw-geometry door AutoCAD refuses) |
| SAT round trip | `ACISOUT`/`ACISIN` console-functional (file-prompt class); the import lands as a NAKED solid — no SH framework at any save format | same: imports are naked solids |

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

`AC1012` R13 · `AC1014` R14 · `AC1015` 2000 · `AC1018` 2004 ·
`AC1021` 2007 · `AC1024` 2010 · `AC1027` 2013 · `AC1032` 2018

## The validated entity cookbook (golden_entities, 2026-10-03)

Every family below is probe-validated headless and landed in
[`golden_entities/`](golden_entities/README.md) at its measured
versions — the `.scr` beside each `.dwg` is the authoritative dialect
record. Engine column: `A` = AutoCAD console, `B` = BricsCAD hidden
(all AC1012 files are BricsCAD — AutoCAD's floor is R14).

| family | versions | engine | the measured dialect |
|---|---|---|---|
| Point, Line, Circle, Arc, Ellipse, Ray, Xline, Solid (trace), 3DFACE, Text, MText, Tolerance | all 8 | A+B | **entmake** — the simplest and most robust route; one line, no prompts, no REGEN needed for the save |
| Polyline2D / Polygon | all 8 | A+B | entmake the POLYLINE + each VERTEX + SEQEND (the heavy form — R13/R14-native) |
| Polyline3D | all 8 | A+B | entmake with `(70 . 8)` flag, vertices `(70 . 32)` |
| LWPolyline | all 8 | A+B | `_.PLINE` points + blank; **R13 downsave converts to heavy `POLYLINE_2D`** (+`VERTEX_2D`s +`SEQEND`) — the era-native form, gate on either class |
| Spline | all 8 | A+B | `_.SPLINE` fit points + 3 blanks (end input, both tangents default) |
| Leader | all 8 | A+B | `_.LEADER` two points + 2 blanks + `_N` (no annotation) |
| Dimension | all 8 | A+B | `_.DIMLINEAR` p1 p2 dimline-pos |
| Shape | all 8 | A+B | `_.LOAD` **quoted** `.shx` path → `_.SHAPE` name → insert pos, height, rotation. Names in AutoCAD's `ltypeshp.shx`: `TRACK1 ZIG BOX CIRC1 BAT AMZIGZAG` (enumerate via `?` + blank). BricsCAD loads AutoCAD's own `.shx` |
| Viewport | all 8 | A+B | `(setvar "CTAB" "Layout1")` → `_.MVIEW` two corners → back to Model |
| HatchSolid / HatchLines | **R14+** | A+B | entmake boundary CIRCLE → `(setvar "HPNAME" "SOLID"/"ANSI31")` → `_.-HATCH` internal point + blank. **R13 explodes the hatch on downsave** (see gotchas) |
| MLine | all 8 | A+B | `_.MLINE` two points + blank |
| Polyface | all 8 | A+B | `_.PFACE` vertex coords, blank, then **face vertex numbers ONE PER LINE**, blank between faces, final blank terminates |
| Multileader | **AC1021+** | A+B | `_.MLEADER _O _CO _N _X` pos pos (options: content=none) |
| Mesh | **AC1024+** | A+B | `_.MESH _BO` first-corner, **other-corner as a POINT** (`10,10`), height |
| Region | all 8 | A: entmake circle → `_.REGEN` → `(command "_.REGION" ename "")` · B: typed window | REGEN before ename selection on the console (below); BricsCAD's typed windows work without it |
| Solid3d | all 8 | A+B | `_.CYLINDER`/`_.BOX` — **the ACIS solid SURVIVES the R13 downsave** (gold reads 3DSOLID + `ACSH_CYLINDER_CLASS` + `ACSH_HISTORY_CLASS` typed at AC1012) |
| Insert (block) | all 8 | A+B | entmake content → `_.-BLOCK` name origin `W` two corners + blank → `_.-INSERT` name pos + **3 blanks** (X, Y, rotation — both engines ask Y separately) |
| Body | all 8 | **REFUSED both** | INTERFERE is console-nonfunctional (silent return) and BricsCAD's INTERFERE completes but creates no BODY — recorded per version; re-opens only via an authentic specimen |

**The batch method that made 221 fixtures land clean**: probe the
unproven recipes ONCE on one version with full diagnostics (result file
+ console tail echo), fix the dialect, then run the version batches —
never batch on an unproven recipe.

## The ACSH_BREP_CLASS mint (the SOLIDHIST discovery, 2026-10-03)

The one class recorded as "external-only" (B4) is mintable from
scratch, headless, in one script — the confound was a sysvar default:

```
(setvar "SOLIDHIST" 1)          ; THE switch — default 0 strips boolean
                                ; history (with 0 even a plain two-box
                                ; union saves as a bare HISTORY root)
_.BOX 5,5,0 → 15,15 → 10        ; the parametric operand
_.MESH _BO 0,0,0 → 10,10 → 10  ; the raw-geometry source
_.CONVTOSOLID <mesh>            ; → a framework-less naked solid
_.UNION <box> <solid>           ; the framework records the chain; the
                                ; raw operand's node IS ACSH_BREP_CLASS
```

Saved file: one `3DSOLID` + the four-node chain `ACSH_BOX_CLASS` +
**`ACSH_BREP_CLASS`** + `ACSH_BOOLEAN_CLASS` + `ACSH_HISTORY_CLASS`.
Two raw-geometry doors measured: `MESH → CONVTOSOLID` (fixture route —
no intermediate file) and `ACISIN` of a self-authored SAT (the
`ACISOUT` export works in the console; the import lands naked, which
is exactly what the boolean needs). Landed in [`brep/`](brep/README.md)
at 6 versions (R14–2018 minus 2004); the derailed `major` field is
**container-coded**: 64 at R14/2000, 3528495168 at 2007–2013,
3545534528 at 2018. The 2004 downsave strips the whole SH family
wholesale; R13 is closed on both engines (AutoCAD floor + BricsCAD's
no-operand-history boolean).

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
- **Quote paths at file prompts** (2026-10-03, the SHAPE probe): script
  feeds are whitespace-tokenized at file-name prompts — a bare
  `C:\Program Files\...` splits at the space (`C:\Program` taken as the
  name, the rest re-parsed as commands). Quoted `"C:\Program Files\..."`
  passes whole. Applies to `_.LOAD`, `_.ACISIN`, and any path-bearing
  prompt.
- **`?` listing prompts consume the next script line** (2026-10-03, the
  shape-name enumeration): answering `?` at a list prompt reads the
  FOLLOWING line as the filter — feed a deliberate blank to accept the
  `<*>` default and list everything.
- **Typed text at a selection prompt is NOT an entity pick** (the
  `_b` lesson): a script line `_b` at `Select objects:` is literal
  text, not the LISP variable — it falls through to window-corner
  parsing. Entity picks go through `(command "_.OP" ename "")` (the
  LISP form passes the real ename) or typed `_L`/`_W`/`_C` selection
  keywords. Note the asymmetry: `(command)`-fed enames work at PLAIN
  selection prompts (ERASE, UNION, SWEEP, REGION) but NOT at the
  interactive-acquisition prompts (constraints, FILLET edges, SOLIDEDIT
  faces — see below).
- **The two selection-prompt classes** (the complete 2026-10-03 map):
  *classic prompts* take script input — typed points, windows,
  enames-via-`(command)`, `_L`/`_W`/`_C` keywords (ERASE, UNION,
  SUBTRACT, SWEEP, REGION, BricsCAD's constraint/`-HATCH` prompts);
  *acquisition prompts* resolve picks through the interactive pipeline
  and take NOTHING scriptable — AutoCAD's GC*/DC* constraints
  ("Invalid selection for Tangent" for a point ON the entity), FILLET
  edge picks ("Select first object" then silent repeat), SOLIDEDIT face
  picks ("No solids detected" post-REGEN). On the console the
  constraint family is absent entirely (the accore.dll/acad.exe module
  boundary). BricsCAD's FILLET edge pick DOES take typed points
  (point → radius → edge point → blank) — its acquisition resolves
  headless where AutoCAD's stalls.
- **`(getvar ...)` on an absent sysvar raises** and can halt a script —
  guard with `(vl-catch-all-apply (function getvar) (list "NAME"))`
  (BricsCAD's vlax bridge is alive; the AutoCAD console's is dead).
  Measured absent-in-BricsCAD: `SOLIDHIST`, `SURFACEASSOCIATIVITY`.
- **A generator must assemble `.scr` from a line LIST** (the
  char-per-line bug class, 2026-10-03): `lines += "string"` in Python
  iterates the string — one character per script line — and the LISP
  reader saw `(_> e`, `(_> n`, `(_> t`… (the 12-family AC1032 batch
  failure). Wrap bare-string recipes as `[recipe]` before extending.
- **Timeout budget discipline**: the console's cold start measured
  ~10 s, but a stalled prompt waits forever — always launch with a
  PID-scoped `WaitForExit(<ms>)` + `Kill()` (never by name; `acad.exe`
  is shared), and keep the launcher's own timeout UNDER the calling
  shell's window so the kill lands inside the observable span. When a
  run stalls, the transcript's last prompt names the exact exchange
  that wants input — read it before changing anything.
- **The engine A/B pattern for wire-level questions**: author the
  IDENTICAL geometry on both engines (same seed, same op, same save
  version) and diff the gold censuses — the `SweepSurfArc` /
  `SweepSurfArcB` pair resolved the ASSOC-network question (AutoCAD
  writes the network, BricsCAD writes none and consumes the profile),
  and the BREP mints double-sourced the `major`-field container
  coding. Record which engine authored every file (the `.txt`
  provenance rule).
- **acad.exe `/Automation /t /b` still SHOWS its window** (2026-10-03,
  the maintainer watched the run) — the GUI script route is visible;
  the hidden AutoCAD route is accoreconsole. Also: a manually closed
  GUI run resets LOGFILEPATH, so its session log lands in the
  `-WorkingDirectory` afterwards (harvest there when a run was closed
  by hand).
- **Headless-unauthorable operations** (the complete post-campaign
  list, 2026-10-03): `BODY` entities (INTERFERE non-functional in the
  console, creates no BODY in BricsCAD — the golden_entities refusal
  ×8), `LOFT` draft/settings variants (no prompt path in this release
  — B5), **the AutoCAD constraint family** (module-absent in console,
  acquisition-pipeline in GUI — BricsCAD authors them), **constraints
  on 3D curves** (the 2D manager's domain limit), and **sweeps along
  infinite paths** (geometrically refused). `SWEEP` and — the big
  falsification — **`ACSH_BREP_CLASS`** both moved OFF this list: the
  sweep via REGEN + ename, the BREP class via the SOLIDHIST=1 boolean
  recipe (above).
- **The console's dialogs cannot stall accoreconsole** (it auto-answers
  its internal MessageBox), but the **hidden BricsCAD run has no
  dialogs to stall** (measured clean); the *visible* `/b` GUI flow can
  still trip on first-run/trust dialogs — preflight interactively once
  per profile before relying on any unattended route.
- The **R13/R14 era** (B3 + golden_entities): BricsCAD is the R13
  author (AutoCAD starts at R14) — an era extension means
  BricsCAD-authored stems. The golden campaign's R13 downsave map:
  Region and Solid3d **survive typed** (the ACIS solid holds at
  AC1012), LWPolyline converts to the era-native heavy `POLYLINE_2D`
  (gate on either class), hatch explodes (R14+ only), Multileader and
  Mesh do not exist (AC1021+/AC1024+), and the whole SH object family
  survives — except that BricsCAD's booleans record no operand history
  there (or anywhere), so the BREP class's R13 door is closed on both
  engines.
- **Versioned COM ProgIDs**: the unversioned `AutoCAD.Application`
  ProgID on this machine resolves to BricsCAD (last-registered wins) —
  never rely on unversioned ProgIDs for automation.

**Scope note**: these procedures author fixtures the same way an
interactive session would; they do not change the oracle (LibreDWG
stays read-only, the differ and ignore-list stay frozen — the
AGENTS.md rules).
