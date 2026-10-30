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
