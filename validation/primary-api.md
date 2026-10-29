# Primary API migration — development checkpoint

## Scope and delivery

This branch prepares the 2.0.0 primary API contract. **It is not yet a complete
2.0.0 implementation or release candidate.** The legacy extension namespaces
exist, but transitional root exports and inherited Session settings have not
yet been removed. The contract test is an explicit structural bootstrap;
passing it does not qualify omitted operations.

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
and their extension paths. All 74 global and 37 native paths now exist as direct
re-exports of the original definitions, with no argument adapters. Fully
qualified paths in the frozen-signature test prevent fallback to a root
function if an extension export disappears. The root aliases remain transitional during this
incomplete migration; they must be removed before final acceptance. The final
primary inventory must reject all undeclared root exports.

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
mappings cannot pass the `check` command. The implemented extension nodes
permit inventory to exclude exactly `--extension global --extension native`,
never arbitrary members or root aliases. No remote CI is introduced.

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

## Executed advisory/schema slice

The new contract-local Advisory/AdvisoryDetail declarations match the approved
schema; Position carries owned advisory/message vectors. The native position
entries supply no structured channel, so those vectors remain empty while the
actual diagnostic is retained. No numerical or native source changed.

Checks on this slice:

- Six structural bootstrap tests, covering 218 constants and retained schemas.
- One legacy-signature test spelling all 111 original fn-pointer types. The
  localized expected type-complexity lint is intentional: aliases would obscure
  the exact legacy scalar argument signatures this test must preserve.
- One native position/advisory regression and two independent-C error-metadata
  regressions; two source-helper regressions and the component-fallback check.
- Strict all-target Clippy, formatting and strict private rustdoc pass.
- Fourteen local audit/negative-control tests pass, including field/method name
  collisions, hidden helpers, alias members and explicit Deref leakage.

This does not qualify the blocked default differential matrix or the still
unimplemented deterministic Session/primary-export migration. No version/tag or
package publication is performed in this checkpoint. Remote Actions was found
enabled and disabled before any branch push; no workflow was added or started.

## Executed extension-namespace slice

Starting from advisory revision `ec7f81b`, this slice adds direct re-exports in
`src/global.rs` (74 legacy functions) and `src/native.rs` (37), plus native buffer
and metadata types. It does not adapt arguments or change operation bodies.
All 111 frozen fn-pointer expressions retain exactly the same types after
normalizing only their destination paths. Fully qualified function paths make
an absent extension export a compile error, with no root-name fallback.

Twenty-six focused tests pass: six raw-angle tests, eleven time tests, six
structural bootstrap tests, one 111-signature test and two independent-C error
metadata tests. Those two tests retain eight exact negative C status/diagnostic
pairs, plus preflight and Session equivalence checks. Assertions and tolerances
are unchanged; only their Rust imports now target the extension namespaces.

Strict all-target Clippy, formatting, strict private rustdoc and the regenerated
compiler-resolved root inventory (excluding exactly global/native traversal)
pass. Fourteen local audit tests and the unchanged common-contract suffix pass.
Native pin remains `9083a12d59e98034fb2337061481ac8800c16e64` (2.10.03);
no C body, build setting or dataset changed. The default differential matrix
remains blocked as documented above. Legacy root aliases are still pending
removal; namespace availability alone does not close the primary API gate.

## Work remaining

- Finish compiler-resolved mapping and the full closed primary inventory.
- Finish the primary vertical slice and remove transitional root legacy exports.
- Make Session configuration fully deterministic and complete its operations.
- Add final typed errors, full records and optional seven/ten event layouts.
- Preserve all native values/status/diagnostics and run same-engine C parity.
- Complete isolation, strict Rust checks, source-package checks and migration.
- Keep one cumulative PR, with phase-separated commits. No tag or publication.
