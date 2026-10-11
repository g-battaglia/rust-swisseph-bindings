# Native selected catalog and complete session operations

Native source remains9083a12d59e98034fb2337061481ac8800c16e64, runtime2.10.03.
Selected sefstars.txt SHA25618b0dcafbe5b7240773daba2c038a325f5b3fc4163f61e0a7f4e92abd4f517c6.
All numerical expected responses remain in RAM. No stored vectors or residuals.

Before: catalogue API/session missing(3compileerrors), extended-session regression
missing required methods. First catalogue implementation exposed a native cache
problem: after a valid catalogue, a missing path still returned its1141entries.
Scalar failure evidence is retained; accidental assertion dumps were removed.

fixed_star_catalog traverses actual native numeric queries in one acquisition,
keeps names/order/magnitudes/diagnostics and accepts only the exact native end
verdict. Independent public-header C traversal matches1141entries exactly in
all fields and magnitude bits. Repeated numeric queries retain duplicates.
Unavailable and empty selected catalogs remain errors, including after success.

The native path setter retains the extended catalog. On path/environment selection
change the bindings now use public close and reinstall the path, replaying the
binding-owned observer/full sidereal settings that the ordinary path setter
normally retains. Native tidal-reset semantics are preserved. No raw-C external
settings or clients are covered. Legacy preflight uses two equivalent numeric
forms because C's legacy last-query cache survives close; an unavailable catalog
must not enter C's unsafe extended lookup.

Session now also covers TT mean/extended and UT extended ayanamsa, house speeds
and Gauquelin sectors, center-body positions, equation of time, v1/v2 TT/UT star
positions and magnitudes, complete solar/occult geography and heliacal50. Held
helpers preserve free-function validation, native values, statuses and diagnostics.
Owned configuration/application/calculation/file snapshot remain one acquisition.
No callback, free-threaded Python or external raw-C isolation guarantee is added.

Regression:2owned configurations x24iterations x10operations contend on threads
and equal serial free-call results exactly; remaining6routes and invalid/native
errors have focused coverage. Catalog missing/empty and full C traversal pass.
Affected sessions/configuration/stars/provenance/input-domains/state/full-output
suite, strict all-target Clippy and private Rustdoc pass. Logs target/catalog-*,
target/extended-session-* retain inputs/status/counts; no astronomy fixtures.
