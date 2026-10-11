# Complete native output verification — 2026-10-11

Three additive public functions expose complete owned buffers:
sol_eclipse_where_full, lun_occult_where_full, heliacal_pheno_ut_full.
The retained convenience APIs project the same single native call to their
original longitude/latitude or thirty-word results and existing named getters.
No native astronomical algorithm, data identity or tolerance changed.

## Layout and units

Geography owns ten initialized words, followed by twenty native attributes.
The pinned C implementation writes longitude/latitude (east/north degrees) to
geography0/1. Geographic boundary slots2..9 are currently unimplemented native
reserved words and stay initialized zero; they are not inferred limits.
Heliacal transport owns fifty initialized words: native slots0..27 use the
existing circumstance units/order;28..49 are reserved initialized words.
The full heliacal result also retains actual status and diagnostic text.
Native failures never expose unspecified failure buffers as successful data.

## Before/after and executed proof

The new native API regression initially failed compilation because all three
complete functions were absent. The first projection test then reproduced the
owned two-word geography width (22combined words vs30expected); allocating the
complete initialized geography transport repaired that concrete shape gap.

An independent public-header C caller links the exact Cargo native archive.
Binary results stay in memory; failures report input/slot identifiers only.
Selected Swiss input files, the native archive and the approved DE440 archive
are fingerprinted before/after. Source selections SWIEPH/Moshier/JPL use the
same path and configuration as the Rust caller.

Final test:21recipes pass (18geography and3heliacal), complete30/50success
buffers, exact statuses/diagnostics/binary64 bits and finite output checks;
NaN/infinite clocks rejected safely without a direct hazardous C call.
Inputs:JD2451545.0,2457987.28,2460409.28; Sun occultation and Venus heliacal
circumstances, original selected atmosphere/observer parameters.
DE440 SHA25629915576d0a6555766b99485ac3056ee415e86df4fce282611c31afb329ad062.
Native C9083a12d59e98034fb2337061481ac8800c16e64 remains unchanged.

Existing convenience differential smoke also passes:1297cases,13811fields
plus243constants/status fields. Strict all-target Clippy, private Rustdoc and
formatting pass. The fixed matrix/sanitizer/extracted-package gates are separate
final-integration work; this proof is not scientific certification.
