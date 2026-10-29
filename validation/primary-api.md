# Primary API 2.0 migration and qualification

The crate root is the primary API: 218 explicit constants, 25 pure functions,
65 configured Session operations, 51 complete result records, six closed enums,
Session/SessionBuilder and Error. The reviewed signatures and fields are in
`primary-interface.json`; `common_api_contract.rs` pins every public signature,
record, variant, helper and constant. `common-api.json` is a deliberate closed
inventory, including explicit trait implementations. Qualification below is
scope-specific; passing an interface check is not scientific certification.

## Configured operations and data

Every configured primary calculation borrows `&Session`. Pure angle/calendar
utilities are free functions with typed results. A session installs its owned
settings and performs its calculation under one native access acquisition.
It does not compose public setters and a later public calculation.

`SessionBuilder` exposes only the agreed numerical settings. Defaults are
observer `(0,0,0)` degrees/degrees/metres, Fagan/Bradley sidereal mode, automatic
tidal acceleration, computed Delta-T and lapse rate `0.0065 K/m`. Construction
performs no native write or inherited Delta-T read. Omitted settings never
inherit another caller's configuration on the primary path.

Data paths and JPL file selection are engine-specific:

```rust
use swisseph_bindings as eph;
let mut sdk = eph::native::SessionBuilder::new();
sdk.ephe_path("/absolute/ephemeris/path").jpl_file("de440.eph");
let session = eph::native::primary_session(&sdk)?;
# Ok::<(), eph::Error>(())
```

Native `SE_EPHE_PATH` precedence remains real. Primary construction captures
external lookup inputs; observed environment or relevant working-directory
changes reject before native writes. Rebuild explicitly to adopt them. SDK
lifecycle relaxation options cannot weaken this primary policy. Concurrent
external environment/filesystem mutation is outside the native lock contract.
Unchanged owned configuration retains native caches. A foreign model selection
invalidates model ownership, so reselecting a sidereal label cannot leave stale
precession/nutation models or positions from a previous caller.

Session clones share their owned file snapshots. Dropping a session does not
reset global native resources or another caller's session.

## Results, units and errors

Primary nodes use `(jd, body, method, flags)`. Observer methods take explicit
site arrays `[longitude_east_deg, latitude_north_deg, height_m]`. `Target` is an
owned body number or star name; borrowed targets never inspect arbitrary objects.
Julian-day time scales, frame flags and units follow the producing operation.

All result words come from the actual native producer, without extra calculation.
Positions retain six words, phenomena 20, nodes/apsides four six-word arrays,
orbital elements 50, eclipse geography ten plus twenty attributes, heliacal
phenomena 50 and visibility ten. ARMC cusps retain twelve or 36 elements.
Heliacal event vectors retain the actual ten-word initialized C transport.
`LocalSolarTimes` and `RiseTransitTimes` preserve their actual producer layouts,
including native no-event states. Named local solar contacts retain the native
entries, including zero sentinels. They are not inferred missing events.

Optional status/diagnostic/scalar fields describe real channels. A supplied empty
diagnostic is `Some("")`; no diagnostic channel is `None`. Integer status is not
converted into a scalar or advertised as source flags. Native position calls
supply no separate structured advisories/messages, so those vectors are empty;
original diagnostic text remains unchanged and is never parsed into events.

Errors retain the genuine native SDK cause through `std::error::Error::source`.
Actual integer status and scalar-return bits are distinct channels. Structured
classification uses existing variants and known admission rules, never diagnostic
parsing. Primary validation checks integer/character narrowing before C calls.
No tolerance, numerical model, native formula, provider fallback or data asset
was changed by this API transport work.

Sunshine point positions (`house_pos` with `I`/`i`) are explicitly unsupported
on the primary path: the C call reads solar declination retained by a previous
houses calculation, but this request shape cannot supply it. An independent
public-header witness reproduces history dependence after two successful native
houses calls. The primary guard returns no invented C status/diagnostic or
fallback value. The original SDK call remains available with its existing
sequencing contract; ARMC cusp/rate operations accept explicit declination.

## Original SDK and migration

All 111 original free-function signatures remain in `global` or `native`.
`global-signatures-v1.json` lists each exact destination. Legacy nodes retain
`(jd, body, flags, method)`, observer methods retain scalar arguments, and legacy
records remain their original shapes. Fully qualified fn-pointer tests pin all
111 signatures. Additional current-main extensions also remain native-specific.

`native::SessionBuilder` and `native::Session` retain the SDK's optional inherited
settings, preflight and path/environment lifecycle. They are distinct from the
closed primary session. Ordinary free setter/calculation pairs remain separate
lock acquisitions. The primary API has no Deref, extension trait or hidden method
that exposes the SDK's extra configuration, callback or metadata interface.

Nominal result types belong to this crate. Selecting another implementation by
an import alias provides matching source signatures, not shared Rust type identity.
Closed enums deliberately omit `non_exhaustive`: adding a variant requires a
coordinated major contract revision.

## Renewed local checks

Current source incorporates main `d30b95ca`. Historical API-branch results are
not reused as acceptance of this source. The native pin remains
`9083a12d59e98034fb2337061481ac8800c16e64`.

Final source checks:

- Full default suite: **239 ordinary tests + 89 doctests**, zero failures. This
  includes all 65 primary operations, eight-thread A/B/A restoration, the native
  original-precession witness, strict lookup context and the Sunshine admission
  witness. Library poison and multi-call metadata controls run in isolated
  children, preserving the actual native state each comparison requires.
- All 111 original free-function signatures compile unchanged at their explicit
  SDK paths. The common consumer suffix is byte-identical to its agreed copy.
- Closed compiler inventory: **844 primary paths + two declared SDK containers**,
  zero extra/missing/changed entries, including private/hidden rustdoc items.
- Full selected-data profile with genuine JPL success: **35,376 cases**,
  **105/105 public operations**, **101/101 native symbols**, **744,181 fields +
  243 constant/status fields**. Counts: 273,946 successes, 7,305 native errors,
  15 absences and 45 safety rejections. No provider-success block was skipped.
- Seven-operation C/session contention: 630 results, 196 rejections, 8,178
  fields. Persistent SDK state: 48 calculations, 17 builders, 36 rejections,
  three native errors, one absence and 4,102 fields. Four cold/reset scripts
  compare 332 fields.
- Independent C output tests preserve all geography/heliacal/visibility slots
  through the new primary route, and pure calendars retain 5,184 C cases.
- Native sanitizer data-pilot gate: all nine pinned C units + oracle with
  ASan/UBSan, clean on the bounded safety/time/session/house/history subset.
  This instruments C; stable Rust itself was not ASan-instrumented.
- Strict all-target Clippy and private+hidden rustdoc passed. Local Cargo package
  creation and extracted-package verification passed offline (206 files).

Selected JPL data is the unchanged existing
`/Users/giacomo/.libephemeris/SWISS/de440.eph`, SHA-256
`ea9ffa66ab00ef336b3c80957b1e4e21fb791ff8f03d68639ef3a4f61323fa73`.
Its historical admission MD5 remains `8a1c6e63ce2b0ab4716e8c07e45f7d5a`.
Planet/Moon/asteroid/catalogue input identities are printed by the full native
runner before/after comparison. The unsanitized archive used by this local source
build has SHA-256 `84bffbd64d78186c2d77bd59d9853057440b34b6e8880e9ba12af0caf2e78548`. Sanitizer identity is separate because its
instrumented pinned-source build is a different archive.

Logs are under `target/primary-api/complete-*`. Numerical responses remained in
memory; reports contain identities/counts, not reference vectors. Conformance
against the same native engine does not independently certify astronomy.
GitHub Actions stays disabled. No merge, tag or publication is part of this
update. Source packages are prepared locally for owner review.
