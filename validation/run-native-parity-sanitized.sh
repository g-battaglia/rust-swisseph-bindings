#!/bin/sh
# Fresh nine-unit native sanitizer gate for the parity oracle.
#
# Compiles all nine pinned native translation units plus the owned oracle with
# ASan/UBSan (including float-cast-overflow) in a fresh owned directory, then
# runs a bounded safety-regression subset of the parity runner with the
# sanitized oracle. The runner itself selects this mode only through
# SWISSEPH_PARITY_SANITIZED_UNITS; ordinary runs are untouched. No production
# build input, dependency or pin changes; no native output vector is stored.
# Reports input/archive identities and pass/fail only.
#
# Usage:
#   SWISSEPH_EPHE_DIR="$PWD/swisseph/ephe" \
#     sh validation/run-native-parity-sanitized.sh [--profile smoke|data-pilot]
#
# Stable Rust itself is NOT ASan-instrumented by these C flags; the gate
# covers native memory/UB behavior observed through the sanitized oracle
# (timeouts are relaxed via SWISSEPH_PARITY_SANITIZED), not Rust-side
# instrumentation. Exit nonzero on any failure.
set -eu

PROFILE="smoke"
if [ "${1:-}" = "--profile" ]; then
  PROFILE="${2:-smoke}"
fi
case "$PROFILE" in
  smoke|data-pilot) ;;
  *) echo "usage: $0 [--profile smoke|data-pilot]" >&2; exit 2 ;;
esac

REPO="$(cd "$(dirname "$0")/.." && pwd)"
NATIVE="$REPO/swisseph"
SANDBOX="$(mktemp -d "${TMPDIR:-/tmp}/swisseph-sanitized.XXXXXX")"
trap 'rm -rf "$SANDBOX"' EXIT INT TERM

CC="${CC:-cc}"
# shellcheck disable=SC2086
SAN_FLAGS="-g -O1 -fsanitize=address,undefined -fno-sanitize-recover=all -fno-omit-frame-pointer"

echo "sanitizer build: compiler=$CC profile=$PROFILE"
echo "native pin: $(grep -m1 -o '[0-9a-f]\{40\}' "$REPO/validation/source-pins.json")"

# 1. Fresh sanitized objects for all nine pinned units.
mkdir "$SANDBOX/units"
for unit in swedate swehouse swejpl swemmoon swemplan sweph swephlib swecl swehel; do
  # shellcheck disable=SC2086
  $CC $SAN_FLAGS -I"$NATIVE" -c "$NATIVE/$unit.c" -o "$SANDBOX/units/$unit.o"
done

# Capacity sentinel for accepted UTC seconds and ordinary invalid clock fields.
# shellcheck disable=SC2086
$CC $SAN_FLAGS -I"$NATIVE" "$REPO/validation/probe-utc-diagnostics.c" \
  "$SANDBOX"/units/*.o -lm -o "$SANDBOX/utc-diagnostics"
env -u SE_EPHE_PATH "$SANDBOX/utc-diagnostics"

# 2. Bounded safety regression through the sanitized oracle: sessions
# (global-state writes, threaded clones), houses (wide buffers),
# crossings/eclipses (sentinels), rise/visibility, plus the stable
# nutation neighbour. UBSan float-cast-overflow is in scope via the
# same flags; sanitizer violations abort the run.
export SWISSEPH_PARITY_SANITIZED_UNITS="$SANDBOX/units"
export SWISSEPH_PARITY_SANITIZED=1
if [ "$PROFILE" = "data-pilot" ] && [ -z "${SWISSEPH_EPHE_DIR:-}" ]; then
  echo "data-pilot requires SWISSEPH_EPHE_DIR" >&2; exit 2
fi
cd "$REPO"
# Hazardous UTC seconds run through Rust-only rejection steps, never C.
# Accepted neighbours and ordinary invalid dates also exercise native diagnostics.
cargo test --test native_parity --locked --offline -- --family safety
cargo test --test native_parity --locked --offline -- --family time
if [ "$PROFILE" = "data-pilot" ]; then
  cargo test --test native_parity --locked --offline -- \
    --profile data-pilot --family sessions
  cargo test --test native_parity --locked --offline -- \
    --profile data-pilot --family houses
  # This Swiss-only regression must remain selectable without optional JPL data.
  env -u SWISSEPH_JPL_DIR cargo test --test native_parity --locked --offline -- \
    --profile data-pilot --case SOURCE_PCTR_NUTATION_HISTORY_DEPENDENCE
else
  cargo test --test native_parity --locked --offline -- --family sessions
  cargo test --test native_parity --locked --offline -- --family houses
fi

echo "PASS sanitized native gate ($PROFILE): nine units + oracle ASan/UBSan clean on the bounded safety subset"
echo "LIMIT: C-only instrumentation; stable Rust is not ASan-instrumented by these flags"
