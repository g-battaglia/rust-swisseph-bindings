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

Owned primary sessions preserve data-selected automatic tidal acceleration:
empty JPL selection closes files before path preparation, while explicit manual
tidal and Delta-T values remain final overrides. Native SDK queries
`primary_current_file_data` and `copy_primary_file_path_into` expose complete
per-owner file snapshots without another calculation or a global metadata read.
Snapshot clones share history; rejected input retains the previous observation.

The native dependency remains Swiss Ephemeris 2.10.03 at commit
`9083a12d59e98034fb2337061481ac8800c16e64`. This API change does not alter native
astronomical formulas, tolerances or data. Pre-release review passes 246
ordinary tests and 89 doctests, 35,376 selected-data/JPL differential cases,
strict Clippy/Rustdoc, release-mode session controls and 20 contract-tool tests.
Source-selection regressions retain 96 scalar comparisons and add 384 complete
position comparisons with exact binary64 values, flags and diagnostics. Global
reference sequences run in isolated processes and reject incomplete selected data.
Earlier bounded C sanitizer evidence retains its original scope; see the
[current release review](validation/release-2.0.0.md) for identities and limits.
These checks establish binding preservation, not independent astronomical
certification. Detailed scope and limitations are in the migration guide.

The crate is AGPL-3.0-only. Required native sources and license notices ship in
the Cargo package; ephemeris data stays external. Publication is performed by
the owner.
