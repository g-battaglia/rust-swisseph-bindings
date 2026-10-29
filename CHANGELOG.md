# Changelog

## 2.0.0 — prepared for publication

The crate root now exposes a typed primary API with owned `Session` and
`SessionBuilder`, 25 pure functions, 65 configured operations, 51 result records,
six closed enums and 218 constants. Primary sessions own complete numerical
settings, install them under one native lock and reject changed external lookup
context before writing native state.

This is a major Rust API change. The 111 original free-function signatures remain
available through `global` or `native`, with their original argument order and
record layouts. The configured native SDK retains its optional inherited
settings and lifecycle. See [the migration guide](validation/primary-api.md) and
[the original function map](validation/function-path-migration.md).

Primary records preserve complete native arrays, returned flags, actual status
and scalar channels, diagnostics and genuine error causes. ARMC houses retain
twelve or 36 cusps. Sunshine point positions are explicitly unsupported on the
primary path because the request cannot supply the solar declination retained by
the native routine; the original SDK entry remains available. Foreign model
changes invalidate owned model state before reuse, preventing stale native
precession/nutation configuration and cached positions.

The native dependency remains Swiss Ephemeris 2.10.03 at commit
`9083a12d59e98034fb2337061481ac8800c16e64`. This API change does not alter native
astronomical formulas, tolerances or data. Local qualification includes 239
ordinary tests and 89 doctests, full selected-data/JPL conformance, bounded C
sanitizer checks, strict Clippy and extracted Cargo package verification.
These checks establish binding preservation, not independent astronomical
certification. Detailed scope and limitations are in the migration guide.

The crate is AGPL-3.0-only. Required native sources and license notices ship in
the Cargo package; ephemeris data stays external. Publication is performed by
the owner.
