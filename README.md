# rust-swisseph-bindings

[![crates.io](https://img.shields.io/crates/v/rust-swisseph-bindings.svg)](https://crates.io/crates/rust-swisseph-bindings)
[![Documentation](https://img.shields.io/docsrs/rust-swisseph-bindings.svg)](https://docs.rs/rust-swisseph-bindings/1.0.0/swisseph_bindings/)
[![Rust 1.98.1](https://img.shields.io/badge/rust-1.98.1-orange.svg)](rust-toolchain.toml)
[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

**The original Swiss Ephemeris C engine, with a safe Rust interface.**

Calculate planetary positions, fixed stars, houses, eclipses and more through
owned Rust results and validated inputs. The crate compiles the pinned native
engine, **Swiss Ephemeris 2.10.03**, and serializes access to its shared state.

[Crate](https://crates.io/crates/rust-swisseph-bindings) ·
[API documentation](https://docs.rs/rust-swisseph-bindings/1.0.0/swisseph_bindings/) ·
[Example](examples/basic.rs) ·
[Releases](https://github.com/g-battaglia/rust-swisseph-bindings/releases)

- **Broad native coverage:** time conversions, positions and speeds, fixed
  stars, houses, ayanamshas, observer events, physical quantities, crossings,
  eclipses, occultations, heliacal visibility and angle/format utilities.
- **Explicit results:** classified errors, owned output values and native
  return flags and diagnostics where the operation provides them.
- **Configured sessions:** keep settings consistent across a complete
  computation with `SessionBuilder`.
- **Rust/C validation:** compare the binding with a separate C caller linked
  to the same pinned engine.

## Install

Available on [crates.io](https://crates.io/crates/rust-swisseph-bindings/1.0.0).
Add this to your `Cargo.toml`:

```toml
[dependencies]
swisseph_bindings = { package = "rust-swisseph-bindings", version = "1.0" }
```

Requires Rust **1.98.1** (edition 2024), a C compiler and `ar`. The published
crate includes the native build sources; no system Swiss installation is needed.
Set `CC` to select the C compiler executable.

## Quick start

Compute the Sun's position and speed using Moshier, without external data files:

```rust
use swisseph_bindings::{Calendar, FLG_MOSEPH, FLG_SPEED, SUN, SessionBuilder, julday};

fn main() -> Result<(), swisseph_bindings::Error> {
    let jd_ut = julday(2000, 1, 1, 12.0, Calendar::Gregorian);
    let session = SessionBuilder::new().default_ephe_path().build()?;
    let position = session.calc_ut(jd_ut, SUN, FLG_MOSEPH | FLG_SPEED)?;
    // Degrees, degrees, AU; then degrees/day, degrees/day, AU/day.
    println!("{:?}", position.values);
    println!("actual flags: {}", position.returned_flags);
    println!("diagnostic: {}", position.diagnostic);
    Ok(())
}
```

See the [full example](examples/basic.rs) for houses with speeds, errors,
explicit data selection, fallback and event searches. Browse the
[API documentation](https://docs.rs/rust-swisseph-bindings/1.0.0/swisseph_bindings/)
for individual functions and types.

## Configuration and ephemeris data

Free functions share process-global native configuration. Each call is
serialized, but a free setter followed by a calculation can interleave with
another caller. Use `SessionBuilder` and fully specify settings that must remain
consistent for a computation. Unset session settings inherit global state.
Configured paths check `SE_EPHE_PATH` on every computation by default. Select
`refresh_ephe_path_environment(false)` to sample it only when installing a path,
following the native setter lifecycle. Explicit setters always sample again;
path, close, thread, relative-directory and tidal invalidations still reinstall.
Independently, `reinstall_ephe_path_on_directory_change(false)` retains C's
installed relative path and open files when cwd changes; later native file opens
use the current directory. The default remains relative-directory tracking.

Swiss and JPL data files are external to the Cargo package. Configure the data
path through `set_ephe_path` or the session builder. `SWISSEPH_EPHE_DIR` selects
data for the example and tests; native `SE_EPHE_PATH` can override path selection.
Moshier supports data-independent calculations within its native coverage.
Fallback remains visible in returned flags and diagnostics; `require_source_flags`
can reject unexpected sources, including reported component fallback.

See the [Rust/C API contract](validation/api-contract.md) for domains, result
layouts, state behavior and native limitations.

## Build from source

Builds and native validation are exercised on macOS with Apple Clang.

```sh
git clone --recurse-submodules https://github.com/g-battaglia/rust-swisseph-bindings.git
cd rust-swisseph-bindings
cargo build --locked
```

For an existing clone, initialize the pinned dependency with
`git submodule update --init --checkout -- swisseph`.

The build compiles native sources into Cargo's output directory. It does not
download ephemeris data. The upstream pin and runtime version are recorded in
[source-pins.json](validation/source-pins.json).

The dependency checkout remains unmodified. A declared build overlay replaces
the v1 fixed-star reader's repeated ASCII-space deletion with a single in-place
pass on the already truncated name. The generated `OUT_DIR/sweph.c` includes
the owned helper; the build requires exactly one matching pinned fragment and
records the overlay in `native-inputs.txt`. File scanning, matching order,
locale handling, diagnostics, session lifecycle and every astronomical statement
remain unchanged. No catalog or calculated positions are cached by this change.

The read-only file overlay replaces only `swi_fopen`'s open call in the same
generated source. POSIX uses `open(O_RDONLY)` followed by `fdopen` with the
original `BFILE_R_ACCESS` mode, rejecting absent paths before allocating a
`FILE`. A successful `fdopen` owns the descriptor; failure closes it once.
Other platforms retain ordinary `fopen`. Filename construction, search order,
path limits, complete diagnostics and stream read/seek/close behavior remain
unchanged. Every call observes the current filesystem. Complete source,
consumer, preceding-overlay and helper fingerprints bind this substitution.
Errno, cancellation and resource-error precedence are outside this helper's
returned-stream contract.

The Moshier overlay generates `OUT_DIR/swemplan.c`, `OUT_DIR/swemptab.h` and
private Moshier coefficient/evaluator headers from the pinned tables. A
fail-closed decoder checks signed-byte structure, planet order, harmonic bounds,
sentinels and complete coefficient consumption. Original longitude/latitude
coefficient tokens are paired without parsing or rounding their values; radius retains its
native stream. Nine private evaluators use constant term, degree and harmonic
offsets; one shared entry retains harmonic preparation and final projection.
Each channel preserves its Horner recurrence, rounded-product/FMA stages and
term-ordered accumulation, without horizontal reductions or CPU-specific source
or flags. Larger compiled bodies and stack use replace runtime control decoding;
coefficient arrays are not duplicated. Complete consumer, emitter and table
fingerprints bind the ownership proof for disconnecting just eighteen
unused longitude/latitude pointers in the generated native table header. All
other header bytes remain intact, including coefficients and notices; the C
compiler can discard the redundant original arrays. New pointer reads or escapes
require review. Generated content identities are recorded in `native-inputs.txt`.
No time-dependent data or calculated results are cached.

The nutation table overlay changes only four static element declarations in
`OUT_DIR/swenut2000a.h` from integer to double. Its original consumer expressions
remain intact. Every original use already promotes those exactly representable
integers before arithmetic; a consumer fingerprint binds that assumption to the
reviewed expressions. Literal tokens, comments, term order and all model branches
remain intact. This trades 47,784 bytes of integer tables for 158,592 bytes of
read-only doubles to avoid repeated conversions; it changes no precision or
calculated result.

The angle normalizer overlay replaces only the remainder calls in
`swe_degnorm` and `swe_radnorm` in `OUT_DIR/swephlib.c`. For their positive
periods, 360 degrees and TWOPI radians, a truncated quotient and one fused
operation compute the exact binary64 remainder when the magnitude is at most
period times 2^50. Larger and nonfinite inputs retain libc `fmod`. Signed zero,
the native 1e-13 zero threshold and wrapping remain intact. Complete source and
helper fingerprints bind the transformation to the reviewed two consumers;
the native ABI probe requires binary64 precision, radix and exponent range.
The returned values retain their exact bits in all IEEE rounding directions.
Libc errno, floating-environment and trap side effects are outside this helper's
contract: quotient division may additionally raise `FE_INEXACT`.

Run the example or generate local documentation:

```sh
cargo run --example basic --locked
SWISSEPH_EPHE_DIR=/path/to/swiss-data cargo run --example basic --locked
cargo doc --no-deps --open
```

## Tests and native validation

```sh
cargo test --locked --offline
cargo fmt --all -- --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --test native_parity --locked --offline
```

The differential runner compares the safe Rust API with a separate public-header
C caller linked to the same native archive. It checks binding preservation;
independent astronomical accuracy requires a separate validation source.
[Validation commands and coverage](validation/native-parity.md) include external
Swiss/JPL prerequisites, full matrices, sanitizers and package checks.

## License

[AGPL-3.0-only](LICENSE). Native notices remain with the upstream submodule.

Negative `calc`/`calc_ut` results retain `native_status()`,
`native_operation()` and the original `native_diagnostic()` separately from the
existing contextual error message. Rust admission errors and other operations
without this transport return `None`; an absent status is never success. These
getters do not parse messages, change native state or infer physical data source.

Built-in extended position queries use the native C records even without a
catalog; other extended queries retain the missing-catalog safety preflight.
Fixed-instant lunar occultation geometry accepts the Moon, while self-target
event searches retain their nontermination guard. Failing longitude crossings
preserve `native_scalar_return()` (the actual TT/UT-day sentinel), entry name and
original diagnostic separately from contextual display detail. No integer
status, star record or astronomical result is synthesized by these changes.

A missing extended catalog retains the original negative status and diagnostic
of the safe legacy `fixstar_mag` preflight. `native_operation()` identifies the
actual preflight rather than implying that the withheld extended query ran.
Extended magnitude failures after admission retain their own actual C metadata.

### Versioned degree splitting

`split_deg` returns the pinned native engine's current output. Applications
that require the original Swiss Ephemeris 2.10.03 field layout can explicitly
use `split_deg_2_10_03`: rounding retains the lower minute/second fields and
returns rounded whole seconds in `secfr`. Its separately attributed upstream
C utility is included under `src/native/`; the engine submodule is unchanged.
Both entries use the same finite input capacity, native lock and sidereal
state. No ephemeris data or positional approximation is involved.

`Session::copy_captured_file_path_into` projects an already-owned file-path
snapshot without acquiring native C access. It preserves owned bytes and
reuses caller storage; calculations still refresh every native file slot.
