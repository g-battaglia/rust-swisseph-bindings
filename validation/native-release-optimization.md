# Native release archive optimization

Release builds at optimization level 3 use ordinary `-O3`. On Darwin, the
archive additionally contains compiler bitcode (`-flto`) so the native linker
can optimize across the original C translation units. Other targets and
development builds keep ordinary object files. No source, data, floating-point
precision, fast-math option or CPU-specific instruction flag changes.

The native input manifest records the actual flags and compiler. The engine
remains `9083a12d59e98034fb2337061481ac8800c16e64`; the separately attributed
versioned split utility remains an additional explicit translation unit.

On native macOS arm64, independent original-engine O2 and O3/LTO libraries
were compared in memory with identical data and configuration. All 90 position,
45 star and 18 house requests matched bit-for-bit, including output arrays,
returned status/flags, names and diagnostic text. The date inputs include JD
1700000.5, 2451545.0 and 2460310.5. No numerical vectors or residuals are saved.

The focused release library/input-domain/time/core/legacy-split/house/star
tests pass (54 tests). The existing filtered direct-C angle, position, house
and star parity controls also pass. Formatting and strict all-target Clippy
pass. These focused results do not close the separately recorded unrestricted
session reset/history smoke finding in `versioned-split.md`, or establish
independent scientific certification.

## Operation-local precession matrix preparation

The adjacent position and velocity projections in `app_pos_etc_plan` use one
stack-owned dimensionless 3×3 matrix. Both receive the same TT Julian Day,
flags and direction; their rectangular equatorial triples are disjoint from
each other and from the preparation. Linear units are preserved. The frame is
initialized for this operation and discarded on return. It has no global
owner, input lookup or lifetime beyond the two projections.

One native dispatcher retains the original model selection. Vondrak/Owen
prepare the matrix at its first use; scalar entry points and the other models
retain their original paths, including the J2000 identity case. Every projection
expression and copy-back stays unchanged. The speed prefix still precedes the
velocity rotation, and the complete coordinate conversions and rate correction
run in their original order. A private header documents the same-input lifetime
contract; it exposes no new Rust API.

The fail-closed generator composes after the existing filename overlay. Full
native/preceding/helper identities, replacement counts and whole-unit inverses
constrain both generated units. Earlier filename/precession fixtures remove
only this final stage, then retain all their original identities and inverses.
Three source tests cover composition, unknown inputs and the unique owning
caller with its complete speed body.

Independent vendor-C, prior-native and proposed-native libraries retain shipping
`-O3 -fPIC -flto`; a separate strict control unit selects the rounding mode.
All 27,136 isolated six-word inputs match bit for bit in four rounding modes,
including signed zero, scalar/framed paths, model/date boundaries and lifecycle
transitions. All 6,300 whole-engine RNE cases also match returned flags/status,
six outputs and all 256 diagnostic bytes. Observed sticky exception flags agree;
this adds no global traps or errno contract. Numerical payloads remain in RAM.
Cargo-native, smoke, strict and installed-artifact qualification remain separate
release gates. Source duplication and linked control flow alone establish no
performance ratio.


## Explicit independent-coordinate arithmetic

The private Moshier evaluator keeps longitude and latitude together through
Horner stages and ordered periodic accumulation. Complete coefficient records,
native units, radius arithmetic, harmonic preparation and scalar output layouts
remain unchanged. AArch64 little-endian uses architectural double NEON; the
fallback retains the original scalar expressions. Whole-object copies preserve
alignment and aliasing without deriving a vector pointer from a struct member.
There is no additional public operation, CPU flag or stored calculation result.

The source generator emits the reviewed helper calls directly. Full input and
generated-source guards bind the coefficient/control streams, while Cargo tracks
both private headers and records their generated identities. Tests retain the
original coefficient/radius/argument expectations through a guarded test-only
inverse; altered, truncated or reordered sources fail closed. Helper comments
describe native units, ownership, scalar normalization and floating-point limits.

Independent ordinary C controls pass 864 full evaluator cases over all nine
bodies and four rounding modes, 1,024 final-bit stage-group cases, and 756 public
TT/RNE cases. They compare coordinates, radius, all harmonic cells, native state,
diagnostics, error/exception state and output canaries; actual written positive
and negative zero signs are exercised. All workers exit cleanly. Individual
intermediate products and every Horner site are not separately observed, and
non-default synchronous trap ordering is not established. The new arithmetic is
not a new whole-engine directed-rounding promise. These controls prove neither
runtime speed nor the identities of a subsequently built Cargo archive; producer
tests, smoke checks and archive comparisons are separate required checks.
