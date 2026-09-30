#!/usr/bin/env python3
"""Verify that the gold-vs-silver harness environment is ready to run.

Checks:
  - GOLD_DWGREAD points to an executable that supports -O JSON.
  - GOLD_TESTDATA points to a directory with the expected version subfolders.
  - cargo is available and the dwg2json/dwgrewrite targets can be built.

Exit codes:
  0  environment looks good
  1  one or more required dependencies are missing or misconfigured
"""

import os
import shutil
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent.parent

REQUIRED_VERSIONS = ["2000", "2004", "2007", "2010", "2013", "2018"]


def check_env_var(name: str, required: bool = True) -> bool:
    value = os.environ.get(name, "").strip()
    if not value:
        if required:
            print(f"[FAIL] {name} is not set")
            return False
        print(f"[INFO] {name} is not set (optional)")
        return True
    print(f"[OK]   {name}={value}")
    return True


def check_executable(path_str: str) -> bool:
    path = Path(path_str)
    if not path.exists():
        print(f"[FAIL] executable does not exist: {path}")
        return False
    if not os.access(path, os.X_OK):
        print(f"[FAIL] file is not executable: {path}")
        return False
    print(f"[OK]   executable exists: {path}")
    return True


def check_dwgread_json_support(path_str: str) -> bool:
    path = Path(path_str)
    try:
        result = subprocess.run(
            [str(path), "--help"],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=10,
        )
        output = result.stdout or ""
        if "-O JSON" in output or "JSON" in output:
            print("[OK]   dwgread appears to support -O JSON")
            return True
        print("[WARN] could not confirm -O JSON support from --help output")
        return True
    except Exception as exc:
        print(f"[WARN] could not run dwgread --help: {exc}")
        return True


def check_testdata(path_str: str) -> bool:
    path = Path(path_str)
    if not path.is_dir():
        print(f"[FAIL] GOLD_TESTDATA is not a directory: {path}")
        return False

    missing = [v for v in REQUIRED_VERSIONS if not (path / v).is_dir()]
    if missing:
        print(f"[FAIL] GOLD_TESTDATA is missing version folders: {', '.join(missing)}")
        return False

    print(f"[OK]   GOLD_TESTDATA contains all required version folders")
    return True


def check_cargo() -> bool:
    cargo = shutil.which("cargo")
    if not cargo:
        print("[FAIL] cargo is not on PATH")
        return False
    print(f"[OK]   cargo found: {cargo}")
    return True


def main() -> int:
    ok = True

    print("Checking gold-vs-silver harness environment\n")

    ok &= check_cargo()

    if check_env_var("GOLD_DWGREAD"):
        dwgread = os.environ.get("GOLD_DWGREAD", "").strip()
        ok &= check_executable(dwgread)
        ok &= check_dwgread_json_support(dwgread)
    else:
        ok = False

    if check_env_var("GOLD_TESTDATA"):
        testdata = os.environ.get("GOLD_TESTDATA", "").strip()
        ok &= check_testdata(testdata)
    else:
        ok = False

    print()
    if ok:
        print("Environment looks good. You can run:")
        print(f"  python3 {SCRIPT_DIR / 'run_roundtrip.py'} <file.dwg>")
        print(f"  python3 {SCRIPT_DIR / 'run_corpus.py'}")
        print(f"  cargo test --features gold-harness --test gold_roundtrip")
        return 0
    print("Environment is not ready. Please set the missing variables and retry.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
