# Agent instructions — Rust Swiss Ephemeris bindings

## Scope and engineering

This project provides independent Rust bindings to the original Swiss Ephemeris
C library. Direct inspection and engineering of owned APIs, FFI, native output
layouts, flags, source reporting, diagnostics, lifecycle, build code, tests and
documentation are authorized. Describe requirements through the native C
contract and this project's public API; keep application-specific adaptation
outside these bindings.

Preserve actual native values, units, time scales, complete output arrays,
returned flags, requested/actual data source and diagnostics. Do not fabricate
missing values, hide unsupported operations, fit constants or widen a tolerance
to pass a failed comparison. A native code correction needs a reproduced defect,
focused before/after evidence and recorded engine/source identity. Keep upstream
licenses and data provenance intact.

## Mandatory commit message rule

Every new commit message must be self-contained and **must NEVER name or refer
to any other repository**. This applies to the subject, body and all trailers,
including repository names, cross-repository paths/URLs and cross-repository
issue or pull-request references. Explain the binding's own problem, changed
native/API behavior and relevant validation without external project context.
Review the complete proposed message before committing. Keep code, docs and
commit messages in English. Do not rewrite existing history to apply this rule.

## Delivery and verification

Inspect Git state and live jobs first; preserve unrelated drafts and build
artifacts. Stage only explicitly reviewed paths, never `git add -A` or
`git add .`; never force-push. Ordinary focused fixes and commits are allowed.
Run one heavy build/comparison at a time with bounded parallelism. For code
changes run meaningful affected tests, formatting and strict scoped Clippy;
document public/nontrivial helper units, domains, outputs, errors and limits.
Compare binding changes to the actual C contract using matching data and
configuration. Record revision/dirty diff, native build/data identity and
input/status/count/scalar evidence; do not persist reference vectors or
reconstructible residual dictionaries.

No package publication, tag, merge, deployment, spending or data-identity change
is implied by engineering authorization. Numerical conformance is separate
from independent scientific certification and release approval.
