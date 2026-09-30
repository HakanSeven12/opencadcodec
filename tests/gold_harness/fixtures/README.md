# Authored gold fixtures (in-repo corpus extensions)

Golden test files authored outside libredwg live here. The authoritative
spec is [`../IMPLEMENTATION.md` §F2.1–F2.3](../IMPLEMENTATION.md); this
README is the quick card for landing files.

## Layout

```
<campaign>/<Entity>_<version>.dwg    one operation per file
<campaign>/<Entity>_<version>.txt   provenance companion, same stem
```

- One **campaign** directory per coverage effort (first resident:
  `sh_history/` — the ACS/SH solid-history family).
- **Version-suffixed stems** (`Polysolid_2018.dwg`), never per-version
  directories — stems must be globally unique (the corpus driver keys
  per-file workdirs by stem; the gold tree's colliding `Leader.dwg`
  stems are the known count-inflation cautionary tale).
- The root `.gitignore` blankets `*.dwg` — this tree is negated in via
  `!tests/gold_harness/fixtures/**/*.dwg`. If a fixture shows as ignored,
  the negation rule is missing; never `git add -f` past it silently.

## Landing a fixture — the checklist

1. Author in AutoCAD or BricsCAD: **fresh drawing, default template,
   layer 0, exactly ONE operation** (see the campaign table in
   IMPLEMENTATION.md §F2.3).
2. `SAVEAS` per target version — only versions where the feature
   natively persists (SH solid history is 2007+; a downsave that drops
   the ACSH records is not an SH fixture — verify by grepping the
   class in the JSON).
3. Qualify before landing:
   - `dwgread -O JSON <file>` — zero `Error` lines;
   - the target class/entity is present in the JSON;
   - object census minimal (tens of objects; a real-world working
     drawing is a calibration specimen, not a fixture).
4. Copy here as `<campaign>/<Entity>_<version>.dwg` and write the
   sibling `.txt`: author app + build, date, exact command sequence,
   save format, qualification result.
5. Re-run the corpus (after the one-time driver extension in
   `run_corpus.py::in_scope_files` — see §F2.2 step 7) and confirm the
   file appears with the expected per-file counts.

## Provenance note

Both AutoCAD and BricsCAD are native writers and both are acceptable
authors — record which in the `.txt`. Wire conventions are content- and
generation-class facts, not writer fingerprints (see the origin-quality
census in [`../README.md`](../README.md)); the provenance record is what
lets future sessions attribute bytes correctly.
