# Primary API migration — development checkpoint

## Scope and delivery

This branch prepares the 2.0.0 primary API contract. **It is not yet a complete
2.0.0 implementation or release candidate.** The current root exports and
inherited Session settings have not yet been migrated. The contract test is an
explicit structural bootstrap; passing it does not qualify omitted operations.

The target is one primary API at the crate root: constants, pure utilities,
SessionBuilder/Session, complete result records and Error/ErrorKind. Stateful
calculations use Session. There is no separate `common` module, shared runtime
crate, backend trait, generic facade or Python package. Each implementation owns
its nominal types. Select a dependency with an import alias/Cargo feature.

## Compatibility boundaries

The migration will preserve every existing 1.x free-function signature while
moving it to `global` or `native`. In particular, `global::nod_aps` retains the
C-style `(jd_et, body, flags, method)` order and scalar observer arguments.
Only primary Session methods adopt the new parameter order. Do not reorder
legacy extension functions to make them resemble Session methods.

`global-signatures-v1.json` records all 111 original owned function declarations
and their planned extension paths. It is input to migration review, not a claim
that those modules already exist. No current root alias is removed in this
checkpoint. The final primary inventory must reject all undeclared root exports.

Explicit native-only destinations include `_full` operations,
`ensure_ephe_path`, `require_source_flags`, `set_jpl_file`, `library_path`,
version reporting, native catalogue enumeration and one-try convenience entry
points. Ordinary global setters/calculations go to `global`; the explicitly
named native destinations take precedence even when stateful.

All future contract enums are closed. **Adding a variant is a breaking change
requiring a coordinated new major in both implementations.** The new advisory
schema already follows this rule. Position advisories must not be created by
parsing native diagnostic text: the position ABI supplies no structured events,
so its advisory and separate-message vectors are empty. Original diagnostics
remain untouched.

## Contract checks

`tests/common_api_contract.rs` has a single initial engine binding and exactly
one `// COMMON_API_CONTRACT_BEGIN` marker. Everything after the marker is
synchronized byte-for-byte. Current coverage pins 218 constant types/values
(float bits), selected complete value records, Calendar variants, existing
builder/position signatures and ownership/thread-safety bounds. Remaining
functions, final ErrorKind and lossless event schemas are explicitly pending.

```sh
python3 -m unittest discover -s validation -p test_common_contract.py
python3 validation/check-common-contract.py diff \
  tests/common_api_contract.rs /explicit/path/to/peer/contract.rs
cargo test --locked --offline --test common_api_contract
```

The compiler-resolved inventory audit consumes rustdoc JSON, retaining hidden
and private items so re-exported private-module definitions and hidden public
methods are not missed. Generate it locally with the documented toolchain:

```sh
RUSTC_BOOTSTRAP=1 cargo +1.98.1 rustdoc --locked --offline --lib -- \
  -Z unstable-options --output-format json \
  --document-private-items --document-hidden-items
python3 validation/check-common-contract.py inventory \
  target/doc/swisseph_bindings.json --scope root
```

`RUSTC_BOOTSTRAP` is confined to this development-time rustdoc invocation. It is
not a package/toolchain requirement for consumers or an environment default.
The checker tests rejected extra exports, hidden Session methods, alias members,
missing private information and invalid contract markers. A manifest with pending
mappings cannot pass the `check` command. Once extension nodes exist, inventory
may exclude exactly `--extension global --extension native`, never arbitrary
members or root aliases. No remote CI is introduced.

## Existing differential-runner blocker

The unmodified starting revision reproduces the default runner failure at
`OCCULT_SELF_GEOMETRY_REJECTED` (JD 2451545, Moon target, Moshier flags): it
expects a safe-boundary rejection, while the current fixed-instant API admits
that query. The advisory-only candidate reproduces the same failure. This is
not reported as a passing native matrix.

A temporary recipe using the ordinary C-comparison path instead encounters the
runner's non-finite-output guard at float slot 0. That trial was reverted; no
native algorithm, safety rule, result word or comparison tolerance was changed.
Resolve the degenerate geometry contract explicitly before claiming a complete
native-parity pass. No numerical vectors are retained in this report.

Focused position/advisory and native error-metadata regressions are separate
checks. Their success does not erase the default-runner blocker.

## Work remaining

- Finish compiler-resolved mapping and the full closed primary inventory.
- Implement the primary vertical slice and relocate global/native exports.
- Make Session configuration fully deterministic and complete its operations.
- Add final typed errors, full records and optional seven/ten event layouts.
- Preserve all native values/status/diagnostics and run same-engine C parity.
- Complete isolation, strict Rust checks, source-package checks and migration.
- Keep one cumulative PR, with phase-separated commits. No tag or publication.
