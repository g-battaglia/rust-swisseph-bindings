# rust-swisseph-bindings

Rust bindings to the original Swiss Ephemeris C library. Calculations run in the
pinned native engine; the Rust API provides owned results, input validation,
classified errors and serialized access to native state.

The library covers time conversions, positions, fixed stars, houses and speeds,
ayanamshas, observer events, physical quantities, crossings, eclipses,
occultations, heliacal visibility and angle/format utilities.

## Build

Requires Rust **1.98.1** (edition 2024), a C compiler and `ar`. Builds and native
validation are exercised on macOS with Apple Clang. Set `CC` to select the C
compiler executable.

```sh
git clone --recurse-submodules https://github.com/g-battaglia/rust-swisseph-bindings.git
cd rust-swisseph-bindings
cargo build --locked
```

For an existing clone, initialize the pinned dependency with:

```sh
git submodule update --init --checkout -- swisseph
```

The build compiles the native sources into Cargo's output directory. It does not
use a system Swiss installation or download ephemeris data. The upstream pin and
runtime version are recorded in [source-pins.json](validation/source-pins.json).

## Use

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
swisseph_bindings = { package = "rust-swisseph-bindings", version = "1.0" }
```

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

The runnable example also demonstrates houses with speeds, errors, explicit data
selection, fallback and event searches:

```sh
cargo run --example basic --locked
SWISSEPH_EPHE_DIR=/path/to/swiss-data cargo run --example basic --locked
cargo doc --no-deps --open
```

See the [Rust/C API contract](validation/api-contract.md) for domains, result
layouts, state behavior and native limitations.

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
