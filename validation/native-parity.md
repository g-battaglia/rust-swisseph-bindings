# Native C differential validation

## Purpose

The `native_parity` integration target compares the safe public Rust API with a
separate C caller using the pinned `swephexp.h`. Both use the same Cargo-selected
native archive. This validates ABI conversion, configuration, ownership and
complete public results. It does not establish independent astronomical accuracy.

The [coverage manifest](native-parity-coverage.md) maps 105 free functions,
101 bound native symbols, 242 constants and session/helper checks to their
independent projections. Profiles and filters report actual execution counts;
declaration inventory alone is not executed coverage.

## Commands and data

```sh
# File-independent smoke, also run by ordinary cargo test.
cargo test --test native_parity --locked --offline

# Inspect recipe keys or select a family/case.
cargo test --test native_parity --locked --offline -- --list
cargo test --test native_parity --locked --offline -- --family sessions
cargo test --test native_parity --locked --offline -- --family crossings

# Explicitly selected Swiss data.
SWISSEPH_EPHE_DIR=/path/to/swiss-data \
  cargo test --test native_parity --locked --offline -- --profile data-pilot

# Complete fixed matrices with real JPL success required.
SWISSEPH_EPHE_DIR=/path/to/swiss-data SWISSEPH_JPL_DIR=/path/to/jpl-data \
  cargo test --test native_parity --locked --offline -- --profile full --require-jpl
SWISSEPH_EPHE_DIR=/path/to/swiss-data SWISSEPH_JPL_DIR=/path/to/jpl-data \
  cargo test --test native_parity --locked --offline -- --profile sweep --require-jpl
```

| Profile | Inputs |
| --- | --- |
| `smoke` | Data-independent bounded cases |
| `data-pilot` (`catalog-pilot` alias) | Swiss/catalog/source/state cases |
| `matrix-pilot` | Data pilot plus 32,000 positions, 741 sidereal and 1,027 house recipes |
| `full` | Complete matrix plus optional JPL success block |
| `sweep` | Data cases plus 8,000 additional deterministic positions |

`--family` selects a recipe family; `--case` selects one exact key. `--samples`
selects an explicit prefix for matrix/sweep profiles. Unknown or empty selections
fail. A filtered run is labelled and never presented as complete coverage.

Swiss profiles require nonempty `sepl_18.se1`, `semo_18.se1`, `seas_18.se1` and
`sefstars.txt` in the explicitly selected directory. Optional ancillary files are
fingerprinted when present. Planet-only and Moon-only source recipes use owned
copies of the selected inputs. No data is downloaded or modified.

Real JPL cases require `SWISSEPH_JPL_DIR` with `de440.eph`, MD5
`8a1c6e63ce2b0ab4716e8c07e45f7d5a`, within the declared JD window
2415020.0–2500000.0. Without it, the optional JPL-success block is reported
`BLOCKED`; `--require-jpl` makes absence a failure. Swiss regressions and
missing-JPL fallback still execute without JPL. Ordinary integration tests skip
implicit missing data explicitly; an explicitly selected incomplete directory
fails with an actionable message.

## Identity and equality

The C oracle uses the compiler identified in `OUT_DIR/native-inputs.txt`, the
public header and `OUT_DIR/libswisseph.a`. SHA-256 fingerprints cover native build
inputs, archive/build metadata, Rust/C workers, executable, pins and selected
data. Inputs are rechecked after comparison; concurrent changes invalidate the run.

Finite outputs require identical bits, including signed zero. Native return
statuses/flags, integers, resolved names and complete public diagnostics compare
exactly. Every public array slot, reserved tail and named accessor is projected
independently. Pure floating helpers allow only documented NaN-class equality;
infinities retain their sign. Unexpected scientific non-finite results fail.

Native errors, Rust input rejection, event absence, circumpolar status and
below-horizon results remain distinct. Undefined native failure buffers are never
promoted into successful public results. Hazardous inputs are Rust-only rejection
recipes and are never sent directly to C.

Planet-centric `ECL_NUT` target/centre output depends on native history. Matrix
version 2 excludes these combinations and retains a stable no-speed neighbour.
The matrix seed is `73776973735f7631`; sweep seed is `0x00737765_705f7331`,
recipe version 1. See the [API contract](api-contract.md) for guarded native
search paths and other capability limits.

## Workers and state

Sources under `tests/native_parity/` separate protocol/schema, operation and
constant inventory, independent C/Rust workers, result projection, input recipes,
session drivers and process supervision. `generate-native-parity.py` regenerates
only declaration inventories from owned Rust names and public native macros.
Cargo does not run this developer tool.

Frames use bounded lengths and explicit integer/binary64 transport. Independent
shape checks reject malformed, truncated or incompatible requests. Synthetic
mutation checks detect changed flags, fields, ordering, tails and diagnostics.
Values, bit patterns, protocol frames and numerical residuals remain in memory;
reports contain only input identities, field categories, counts and pass/fail.

Workers run in owned empty directories with child `SE_EPHE_PATH` removed and
`LC_ALL=C`. Pipes are drained continuously, stderr retention is bounded, deadlines
are enforced and every child/reader is reaped. Supervision timing probes may retry
up to three times; scientific comparisons are never retried.

Session tests compare all seven computation methods with independent serial C
baselines, including A/B configuration under four/eight-thread contention and
captured five-slot file histories. Cold workers cover defaults, first writes and
reset/automatic transitions. Builder/inheritance/clone/drop/error scripts check
actual public sessions. Fully explicit configuration provides atomic per-call
isolation; unset settings inherit globals and no rollback is promised.

## Native sanitizers and diagnostic probes

```sh
sh validation/run-native-parity-sanitized.sh --profile smoke
SWISSEPH_EPHE_DIR=/path/to/swiss-data \
  sh validation/run-native-parity-sanitized.sh --profile data-pilot
```

The script compiles all nine native units and the C oracle into a fresh temporary
directory with ASan/UBSan, including float-cast-overflow, then runs bounded safety,
time, session and house cases. The Swiss nutation regression runs explicitly
without JPL. A public-ABI UTC capacity sentinel checks 28 bounded calls.
Instrumentation covers native C through the oracle; stable Rust itself is not
ASan-instrumented by these flags.

`probe-utc-diagnostics.c` is part of that required sanitizer command.
`probe-pctr-pseudobody.c` is an opt-in repeatability diagnostic for the documented
native history dependence, rather than a required passing comparison. Compile it
with the public include directory, the exact Cargo archive and `-lm`, then pass
an explicitly selected Swiss directory. It reports counts only and exits 1 when
repeats differ or become non-finite; that outcome reproduces the limitation.

## Package and documentation checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --locked --offline
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --locked --offline
cargo package --locked --offline
```

Inspect and extract the generated `.crate` outside the checkout. Run the complete
suite and example with data variables unset, then selected data cases using an
explicit external path. The package must include native build sources/headers,
owned tests and validation tools, while excluding ephemerides and upstream test
or tool trees. Private-item integration Rustdoc also uses the exact Cargo JSON
`.rlib` path as its `swisseph_bindings` extern.

`publish = false` remains set. Package verification does not publish the crate.

## Verified results

Fresh checkout on macOS, Rust 1.98.1 and Apple Clang 21. The native pin is
recorded in `source-pins.json`. Counts describe executed comparisons with the
same engine and declared inputs, not exhaustive input safety or precision.

| Check | Result |
| --- | --- |
| Rust suite | 146 ordinary tests and 88 doctests pass |
| Differential smoke | 1,297 cases; 104/105 free functions, 98/101 native symbols |
| Full with required JPL | 35,326 cases; 743,881 compared/rejection fields; 105/105 functions, 101/101 symbols |
| Sweep with required JPL | 9,558 cases; 147,051 compared/rejection fields; 105/105 functions, 101/101 symbols |
| Native sanitizer smoke/data profiles | Both pass; UTC sentinel covers 28 bounded calls; no runtime diagnostics |
| Formatting, Clippy and private Rustdoc | Pass with warnings denied where applicable |
| README and basic examples | Execute successfully |
| Package rehearsal | 114 files before VCS metadata; 2.7 MiB unpacked, approximately 677 KiB compressed |
| Extracted package without data | Complete suite, 88 doctests, 1,297-case smoke and basic example pass with explicit data skips |
| Extracted package with selected Swiss data | 28 safety/state tests and the no-JPL nutation regression pass |

The constants/status comparison contributes another 243 fields per invocation.
Session and cold-state counts are separate from ordinary recipe counts. The
package excludes ephemeris data and upstream tests/tools; native notices and all
required build inputs are retained. Compiler deprecation warnings from the
unmodified native sources are distinct from sanitizer runtime diagnostics.

Selected Swiss input identities:

| File | SHA-256 |
| --- | --- |
| `sepl_18.se1` | `ca1393ceab3a44fbc895887cf789c68819ae6a1cbc9b22225872dbe4ccd99a66` |
| `semo_18.se1` | `1ca07bd67c24374d77226180c20a4f9996cba013697894810518e7eb582ca4f7` |
| `seas_18.se1` | `a2cd8fc33807c78ca9a700c91c2e042258b12fc4796519e00781440b5ad8b2e2` |
| `sefstars.txt` | `18b0dcafbe5b7240773daba2c038a325f5b3fc4163f61e0a7f4e92abd4f517c6` |

JPL identity and input windows are specified above. Reference numerical results
and residuals are never stored.
