# Primary API migration — development checkpoint

## Scope and delivery

This branch prepares the 2.0.0 primary API contract. **It is not yet a complete
2.0.0 implementation or release candidate.** The legacy extension namespaces
exist and configured legacy free functions/native-only helpers have left the
root. Inherited Session settings and remaining typed-family work are pending. The contract test is an explicit structural bootstrap;
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
function if an extension export disappears. Stateful free functions and native
helpers are no longer root exports. Pure utilities still require their final
typed results. The final primary inventory must reject all undeclared exports.

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

## Differential-runner diagnosis

The baseline failure at `OCCULT_SELF_GEOMETRY_REJECTED` was an incorrect runner
expectation, not a missing search guard. Fixed-instant Moon geometry is admitted
by the pinned C API; unlike self-occultation searches, it terminates and can
return undefined geometry. The existing native-admission regression already
requires exact C words for both finite and non-finite outputs.

A separately compiled public-header witness at JD 2451545 with the Moon and
Moshier flags reports status 0, two non-finite geography words, and buffers of
10/20 words. No numerical reference vectors are retained. A trial general
rejection conflicted with that native-admission contract and was removed.
Neither the C engine nor valid-request admission is changed.

The corrected recipe compares native words. The comparator retains exact NaN
payload/sign bits for fixed-instant occultation geometry, while its existing
finite-only position/refraction checks remain unchanged. Negative controls
reject changed NaN payloads and matching non-finite words on finite-only routes.
No tolerance or numeric algorithm changes.

The retained-file-history mismatch in `SESSION_DEFAULT_CLEAR_CLOSE_RESET`
was another replay defect: the C script forced a repeated path setter/cache
reset while Session correctly reused its installed path. Input-only setter
tracking now applies the same guarded path semantics to C; every actual C
result, computation and metadata comparison remains intact. Free setters still
force reloads. This predicts no numerical output and fabricates no metadata.

The complete default `cargo test --no-fail-fast` run now passes: 184 ordinary
Rust tests, 88 doctests, seven-method and persistent-state Session controls,
and the 1,347-case native smoke profile. The smoke profile covers 104/105 public
operations and 98/101 native symbols; it is not the separate selected-data,
sanitizer or extracted-package qualification.

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
remains blocked as documented above. At that earlier checkpoint root aliases were still present. The subsequent
root-boundary slice removes configured free exports; complete primary schemas
and deterministic configuration remain separate gates.

## Root boundary and node/apse records

Configured legacy free operations and native-only helpers are removed from the
root. Their 111 extension signatures remain unchanged; the 23 root free
utilities are the deliberately pure subset, still awaiting final typed returns.
Examples, doctests and validation callers use explicit global/native paths.
The maintenance codemod has six data-independent regression tests and changes
only caller namespaces/documentation, not numerical arguments or assertions.

Primary Session node/apse methods use `(jd, body, method, flags)`, unlike the
frozen C-order global functions. The TT method uses the original native body
factored into a caller-locked helper. Both methods return four complete six-word
vectors plus optional actual status/diagnostic. Native legacy records remain
in native; the primary status is never labelled source flags.

The replicated contract now checks both methods and exhaustive record fields.
The existing direct-C UT test still executes all 15 method/flag/frame pairs and
its path-lease/error/reset checks, with no changed numerical assertion. The
swapped i32 values are discriminating: original method values 1/2 and masks
including SPEED/source/frame flags are not interchangeable.

## Work remaining

- Finish compiler-resolved mapping and the full closed primary inventory.
- Finish primary pure-function result signatures and the full vertical slice.
- Make Session configuration fully deterministic and complete its operations.
- Add final typed errors, full records and optional seven/ten event layouts.
- Preserve all native values/status/diagnostics and run same-engine C parity.
- Complete isolation, strict Rust checks, source-package checks and migration.
- Keep one cumulative PR, with phase-separated commits. No tag or publication.
