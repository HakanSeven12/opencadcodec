#!/usr/bin/env bash
# bootstrap_oracle.sh — check out and build the gold oracle (LibreDWG) for
# the gold-vs-silver harness, on demand.
#
# Idempotent: exits immediately when a built dwgread + the test corpus are
# already in place. On success (or when already present) it prints the env
# exports the harness needs:
#   export GOLD_DWGREAD="$HOME/work/libredwg/programs/dwgread"
#   export GOLD_TESTDATA="$HOME/work/libredwg/test/test-data"
#
# The harness tests are oracle-optional (gold_roundtrip skips with a notice
# when the oracle is absent); use this script when you want the real
# gold-vs-silver fidelity checks to run on a machine without a checkout.
#
# Build dependencies (Ubuntu/WSL): build-essential autoconf automake libtool
# texinfo perl python3 — install with apt before the first build.

set -euo pipefail

LIBREDWG_DIR="${LIBREDWG_DIR:-$HOME/work/libredwg}"
GOLD_DWGREAD="${GOLD_DWGREAD:-$LIBREDWG_DIR/programs/dwgread}"
GOLD_TESTDATA="${GOLD_TESTDATA:-$LIBREDWG_DIR/test/test-data}"

if [ -x "$GOLD_DWGREAD" ] && [ -d "$GOLD_TESTDATA/2000" ]; then
    echo "gold oracle already present:"
    echo "  GOLD_DWGREAD=$GOLD_DWGREAD"
    echo "  GOLD_TESTDATA=$GOLD_TESTDATA"
    exit 0
fi

if [ ! -d "$LIBREDWG_DIR/.git" ]; then
    echo "cloning libredwg (online checkout) into $LIBREDWG_DIR ..."
    mkdir -p "$(dirname "$LIBREDWG_DIR")"
    git clone --depth 1 https://github.com/LibreDWG/libredwg.git "$LIBREDWG_DIR"
fi

cd "$LIBREDWG_DIR"
echo "building libredwg/dwgread (this takes a few minutes) ..."
if [ ! -f ./configure ]; then
    sh ./autogen.sh
fi
./configure --disable-bindings --disable-docs >/dev/null
make -j"$(nproc)" >/dev/null
# Smoke test: the harness relies on `-O JSON` support.
programs/dwgread -O JSON test/test-data/2000/Line.dwg >/dev/null
echo "smoke test passed (dwgread -O JSON)."

echo
echo "oracle ready. export before running the gold gates:"
echo "  export GOLD_DWGREAD=\"$GOLD_DWGREAD\""
echo "  export GOLD_TESTDATA=\"$GOLD_TESTDATA\""
