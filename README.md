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
