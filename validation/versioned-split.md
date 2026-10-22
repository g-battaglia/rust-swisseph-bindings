# Versioned degree-splitting contract

The current `split_deg` continues to expose the unmodified engine at
`9083a12d59e98034fb2337061481ac8800c16e64`. The explicit
`split_deg_2_10_03` entry compiles the original two degree-splitting functions
from the Swiss Ephemeris 2.10.03 source distributed with pyswisseph 2.10.3.2.
Their original license and source-file SHA256 are retained in
`src/native/split_deg_2_10_03.c`. Only C function identifiers are renamed;
the current engine source is unchanged.

The versioned API retains rounded minute/second fields and returns rounded
seconds in the fractional field, exactly as that original utility does.
Both APIs share finite-capacity validation, the native lock and active
sidereal state. This is an explicit native release contract, not a correction
applied to a computed current-engine result.

## Focused evidence

On native macOS arm64, an independent public-header C caller and the safe
Rust entry were compared in memory over 324 input combinations: nine finite
signed degree inputs, twelve rounding/segment masks and three sidereal modes,
including True Sheoran. All five result fields were compared exactly.

- Current utility versus original 2.10.03 C: 123 differing results.
- Versioned utility versus original 2.10.03 C: zero differences.
- Current utility versus the pinned current C: zero differences.
- Versioned/current contract controls: two tests pass, including invalid inputs.
- Owned snapshot storage controls: seven library tests pass.
- Complete geography/heliacal and visibility buffers: two direct-C tests pass.

No numerical reference vectors or residuals are retained. The C compiler uses
ordinary `-O3 -fPIC` for release optimization level 3 and `-O2 -fPIC` otherwise;
there is no fast-math, reassociation option or host-specific CPU target.

The new `Session::copy_captured_file_path_into` reads only owned snapshot bytes
under the metadata mutex. The existing native-access query retains its native
lock/reentry behavior. Native calculations still capture every file slot.

An unrestricted native-parity smoke run remains failed at
`SESSION_DEFAULT_CLEAR_CLOSE_RESET`, owner 6 / file slot 0, with a snapshot
result-layout mismatch. This is not relabeled as global conformance. The same
run passes the seven-operation Session A/B comparison, contention and clone
history group (630 results / 196 rejections / 8,106 fields). The existing
self-target geometry rejection control initially exposed undefined native
geographic components; the missing safe-boundary check is now applied while
keeping the original rejection control intact.
