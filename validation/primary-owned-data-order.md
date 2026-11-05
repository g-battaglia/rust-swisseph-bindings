# Owned primary data preparation

Owned primary sessions previously installed the ephemeris path before the empty
JPL setter and restored automatic tidal acceleration after source preparation.
The native empty-name setter closes Swiss files as well as the JPL file. Restoring
automatic acceleration afterwards also discards the value selected from the opened
Moon file. Historical UT ayanamsha and sidereal fixed-star requests can therefore
use the compiled default instead of the selected data's intrinsic tidal value.

The owned path now closes an empty JPL selection first, restores automatic tidal
selection before data installation, and reapplies path preparation when native
files/model state were cleared. Nonempty JPL selection remains after the path.
Explicit manual tidal and delta-T overrides remain final. Legacy native sessions,
public signatures, C kernels and data identities are unchanged.

The regression fails before the correction and passes afterwards: 96 exact
binary64 checks cover four sidereal modes, historical/modern epochs, original
extended flags, repeated calls after native close, explicit observers, and manual
tidal/delta-T overrides. Fourteen focused primary/contract tests pass. The complete
333-test Rust/doctest suite, formatting, strict all-target Clippy and strict private
Rustdoc also pass; detailed local receipts retain every earlier failed attempt.

Native source: `9083a12d59e98034fb2337061481ac8800c16e64`, bindings 2.0.0,
existing Swiss data selected through `SWISSEPH_EPHE_DIR`. Numerical comparisons
call the matching native API with identical data and settings; no tolerance,
reference vector, astronomical constant or data file is added or changed.

## Pre-merge review, 2026-11-05

Review of the original head `1268100e` reproduced an intermittent failure when
the three data-order tests ran concurrently: their global setter/reference
sequences are not atomic across separate calls. Each now uses the existing
isolated-child helper. A second control showed that an explicitly incomplete
Swiss directory could pass through native fallback; these tests now use the
shared complete-directory admission helper. Neither correction changes library
implementation, native source or data.

The extended controls compare 384 complete position results exactly, including
all six binary64 words, flags and diagnostics. They cover implicit/empty/missing
JPL selection, real DE440 use, two sidereal modes, two tidal policies, automatic
and manual Delta-T, three epochs, two bodies, foreign clock writes and repeated
owned calls. The original 96 comparisons remain; extended ayanamsha checks also
compare flags and diagnostics. Metadata checks now require actual Swiss source,
cover selected-slot capture and retain shared snapshots after rejected input.

The initial insufficient-data run and the reproduced parallel failure are
retained under the local `swisseph-release-2.0.0-20261105` evidence directory.
Final review and release qualification are recorded separately in
`validation/release-2.0.0.md` when complete.
