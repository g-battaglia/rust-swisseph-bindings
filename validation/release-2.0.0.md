# 2.0.0 release review — 2026-11-05

Version 2.0.0 is the next publication: the live crates.io registry contains only
1.0.0 at this review. No patch-version bump is needed for an unpublished major
version. The package is AGPL-3.0-only, requires Rust 1.98.1 and a C compiler,
and carries the selected native sources and original license notices. Data
files remain external. No publication, tag or release announcement is performed
by preparation.

## Review of the owned-session change

PR #2 originally ended at `1268100ed90968dba37241926e2e9340c9caa5be`.
The review follows configuration from the public primary builder through the
native lock and the actual C setters, plus snapshot ownership and lock ordering.
The pinned engine is `9083a12d59e98034fb2337061481ac8800c16e64` (2.10.03).

- Empty JPL selection closes Swiss files too, so it precedes path preparation.
  Automatic tidal selection precedes file opening; nonempty JPL follows the
  path. Manual tidal and Delta-T overrides remain final. Cleared owned-model
  state triggers actual path preparation, including an unchanged path string.
- Configuration and calculation retain one native-lock scope. Legacy sessions
  retain their separate inherited-setting implementation. Lookup-context
  rejection still precedes native writes. Thread-dependent path/JPL identity
  and foreign model invalidation remain effective.
- Metadata queries forward to the owner's existing snapshot. No C pointer
  escapes; clones share the snapshot mutex. Complete metadata reads acquire
  native access then snapshot access. The reusable path query acquires only
  snapshot access. Invalid-slot behavior and unchanged output on error are
  documented, as are disabled capture and source-flag authority for fallback.
- Native kernels, FFI layouts, primary public signatures, arithmetic, data
  identities and tolerances are unchanged. The primary namespace is unchanged;
  the two snapshot functions are additive native SDK entries.

Two test defects were reproduced and repaired before merge in `cd44b49`:

1. **Parallel reference interference.** Three test functions used separate
   global setter/calculation calls without isolating the whole reference
   sequence. A run with eight test threads failed on the seventh bounded
   attempt. They now use the existing isolated-process helper; ten consecutive
   corrected parallel runs pass.
2. **Incomplete data admitted by fallback.** A selected directory containing
   JPL files but no Swiss files passed the original controls. The shared data
   admission helper now rejects this configuration; the negative control
   explicitly verifies that rejection. Snapshot tests require actual Swiss
   returned flags.

No additional production defect was established. This review changes tests and
documentation only beyond the original PR head. It does not optimize session
setup or assert a performance gain.

## Fresh qualification

Executed on macOS arm64 with Rust 1.98.1 and Apple Clang 21.0.0, from product/test
commit `cd44b49ecdf384120cf4ef5acd574f0d98e5d719` in a fresh Cargo target directory:

| Check | Result |
| --- | --- |
| Complete Cargo suite, eight test threads | 246 ordinary tests + 89 doctests; zero failed/ignored |
| Formatting, strict all-target Clippy, strict private Rustdoc | Pass |
| Native differential `full --require-jpl` | 35,376 cases; 105/105 public operations; 101/101 native symbols |
| Complete differential fields | 744,181 fields + 243 constant/status fields, exact comparisons |
| Differential outcome categories | 273,946 successes; 7,305 native errors; 15 absences; 45 rejections |
| Release-mode owned-source, metadata and primary runtime tests | 10 tests pass, including eight-thread A/B/A restoration |
| New position matrix | 384 full positions: all six binary64 words, flags and diagnostics |
| Original data-order matrix | 96 scalar comparisons, with extended flags/diagnostics now checked |
| Contract/migration tool controls | 20 tests pass |
| Corrected focused parallel repetitions | 10 runs of seven tests; no failures |
| Explicit incomplete Swiss directory | Correctly rejected; retained as an expected negative |

The full native differential compares with an independent C caller linked to
the same pinned native archive; the new source matrix compares owned primary
results with matched native API configuration. These are complementary binding
checks, not independent astronomical certification. The first review run with
the wrong data directory is retained and is not data qualification.

Selected read-only Swiss inputs are the four existing files under
`/Users/giacomo/.kerykeion/sweph`. JPL uses the existing
`/Users/giacomo/.libephemeris/SWISS/de440.eph`, SHA256
`ea9ffa66ab00ef336b3c80957b1e4e21fb791ff8f03d68639ef3a4f61323fa73`.
The bindings never read or write LEB assets in this workflow. No ephemeris
download, generation, replacement or pin change is part of the release.

Detailed logs, original negatives, data hashes and package receipts are local
under `/Users/giacomo/.pi/builds/swisseph-release-2.0.0-20261105`. Final package
identity is external, avoiding a self-referential checksum inside the archive.
Later release Markdown changes do not alter the qualified product/test inputs.
GitHub Actions has no run for the reviewed head. Only the named local platform
is qualified here; earlier sanitizer/platform receipts retain their own scope.

## Compatibility and publication

The 1.x crate-root free functions move to explicit `global` or `native`
namespaces. The 2.0 root exposes the typed owned API; see
[primary migration](primary-api.md), [function map](function-path-migration.md)
and [changelog](../CHANGELOG.md). Owned configuration may correct results that
previously inherited the wrong data-selected tidal acceleration; this restores
the selected engine's native configuration, without changing its formulas.

Before upload, from the exact reviewed clean `main` commit:

```sh
git status --short
git submodule status -- swisseph
cargo package --locked
cargo publish --locked --dry-run
```

Inspect the generated archive: it must contain required C units/headers,
Rust/build/test sources, migration documentation and licenses, with no ephemeris
payloads, Git metadata, targets, credentials or unrelated process files. Rebuild
and test the extracted package with explicit Swiss/JPL paths and a distinct
Cargo target directory; run its Moshier example with data variables unset.

After the owner authorizes actual publication, use `cargo publish --locked`
from that same commit. Registry authentication and upload are separate from the
dry run. Verify the registry version/checksum and versioned docs after upload;
only then attach the corresponding version tag/release if authorized. If the
release commit changes, regenerate the archive and its identity before upload.
