# Rust/C API contract

Safe public Rust operations mapped to the pinned Swiss Ephemeris C ABI.
Raw C declarations are private. [source-pins.json](source-pins.json) identifies
the native dependency; the linked runtime version is `2.10.03`.
[Native validation](native-parity.md) defines independent public-header callers,
exact result projection and reproducible comparison commands.

## Common conventions

- One process-wide lock covers each complete native sequence. Separate free
  `set_*` and computation calls are not atomic. `Session` applies its owned
  configuration and computes under that same lock. Unrelated raw C callers
  are outside this synchronization.
- Unset session settings inherit globals at call time. Builders do not install
  configuration. `close()` is an explicit process-wide reset; dropping sessions
  does not reset native state. Same-thread reentry returns `InvalidInput`.
- Every argument is explicit. Time scales, units, frames and default native
  behavior are documented below and in Rustdoc. Ordinary positions contain
  degrees, AU and rates per day; flags select alternative frames/layouts.
- Results retain all public components, actual flags, names and diagnostics.
  Native buffers are initialized and owned. Valid zero, no event and error
  remain distinct states.
- `ErrorKind` distinguishes `InvalidInput`, `Native` and `LockPoisoned`.
  Dependent rejection prevents that native computation but does not promise
  rollback of global settings already applied by a session.
- Session file metadata is captured before unlocking completed attempts.
  Rejection preserves prior snapshots; native failure replaces them. Clones
  share completed history, which survives free calls and `close()`.
- External data is not included in the Cargo package. Paths reject NUL and
  excessive byte lengths. Source fallback stays visible; `require_source_flags`
  checks both primary flags and reported component sources.

## Input and arithmetic domains

These domains are representation limits, not ephemeris coverage or accuracy
claims. Out-of-domain inputs return `InvalidInput` before the
dependent native operation, with validation and computation under the same
native lock when configuration is involved.

- Date-based calculations, houses (including Gauquelin), stars, observer
  calculations, ayanamshas and events require input/effective dates whose years
  fit `i32` in both native calendars. Calendar bounds come from native `julday`
  at the year endpoints. UT preflights check configured Delta-T shifts for the
  requested source and possible Swiss/Moshier fallbacks, restoring the requested
  Delta-T source selection. Operations using inherited selection (`flags == -1`)
  check only that selection and preserve its automatic tidal value. Free
  functions and sessions share these checks.
  This does not alter the distinct UTC pre-1972 contract: UTC conversion bounds
  the final UT1 actually decomposed, allowing an out-of-calendar TT.
- `set_sid_mode` and `SessionBuilder::build` apply the same date bounds to the
  `SIDM_USER` reference epoch, including sidereal option bits. Native bit 1024 (`SE_SIDBIT_USER_UT`)
  also requires a representable effective ET reference under active Delta-T.
  Dependent sidereal operations recheck this after configuration changes;
  non-finite native ayanamshas are `Native` errors. Other mode reference
  arguments retain their native semantics. Rejected setters preserve the mode.
- `set_topo`, session observers and observer-taking calculations/searches reject
  absolute height above `i32::MAX * AUNIT_TO_KM * 1000` metres. This conservative
  arithmetic ceiling bounds observer-dependent light-time dates before native
  calendar casts; geographic and near-Earth space observers remain far inside
  it. Native operation-specific geographic limits/errors are retained.
- Global/local solar and lunar eclipse search starts are restricted to
  `[-63_412_861_279, 63_417_764_339]` JD, inclusive. These inward whole-day limits
  protect the native signed lunation counter. Accepted endpoints return native
  out-of-data errors; excluded adjacent days never enter native code. Local
  occultation searches share the local-search preflight. Crossing epochs must
  also distinguish the native `jd - 1` failure sentinel from the epoch.
- `d2l` requires finite magnitude below `2_147_483_647.5`, protecting its
  positive rounding intermediate for both signs. `cs2timestr` and
  `cs2lonlatstr` require `abs(cs) + 50 <= i32::MAX`; their remaining formatting
  behavior stays native. `utc_time_zone` checks finite clock intermediates
  whose truncated values fit `i32`, and the shifted Gregorian year.
- `fixstar2`, `fixstar2_ut` and `fixstar2_mag` first check catalog availability
  with the legacy native magnitude entry point while locked. Missing/empty
  catalogs return actionable `Native` errors containing the native diagnostic
  and path configuration guidance; native environment/default/legacy path
  selection and extended query semantics remain intact.
- `calc`, `calc_ut` and `calc_pctr` classify non-finite native position components
  as `Native` failure even if native status is non-negative. Native diagnostics
  are retained. Finite fallback results retain all flags and warnings.
- Session preflight rejection (including finite range and configured-shift
  rejection) preserves the previous owned file metadata snapshot. Native
  failures still capture the completed attempt's file state.

Input manifests are `tests/full_safety.rs`, angle/time/format
and session-history tests, and `tests/calendar_bounds.rs`. Environment
controlled tests run in child processes with `SE_EPHE_PATH` removed before
launch; no test mutates the shared process environment around native readers.

## Configuration and native restrictions

`library_path` owns a 257-byte native buffer (256 pathname bytes
plus the terminator), reports the executable/library rather than the ephemeris
search path, and retains native truncation. `solcross` and `solcross_ut` reject
`FLG_HELCTR` with `InvalidInput` before native access; the pinned solver does not
terminate for a stationary heliocentric Sun.

UT-based user sidereal references are checked under the same lock as settings
and dependent computations. Builder checks include known explicit/inherited
Delta-T overrides; computed references with builder model/data changes are
checked after the complete configuration is applied. Temporary preflight
Delta-T changes are restored before releasing native access. Binding metadata
tracks only write-only settings needed for validation and is reset on `close`;
it is not a public native getter. First tidal/Delta-T writes initialize native
defaults before installing the requested value, preventing lazy initialization
from discarding it.

Strict-source checks inspect primary flags and reported component fallbacks;
a Swiss Mars result using a Moshier Moon contribution fails a Swiss-only request.
Multiple requested sources permit those sources, and no requested source remains
unrestricted. Inherited-source date preflights preserve the held automatic tidal
selection without freezing it into a manual override. Input manifests and
in-memory direct-ABI comparisons are in `tests/configuration_edges.rs`;
no native vectors or residuals are retained.

The pinned engine has a native internal
stack-buffer overflow in `heliacal_ut` with `HELFLAG_AVKIND_VR` at J2000,
Greenwich, Venus rising and default atmosphere/observer. All `HELFLAG_AVKIND`
search bits are rejected with `InvalidInput` before native access or
observer mutation. The pinned native source stays unchanged and no alternative
model is substituted. Ordinary searches remain supported; the heliacal
regression verifies rejection for all four kind bits and their mask.
Instant-based phenomenon/visibility ABI probes with those five flag choices
pass combined sanitizers (10 cases); this search restriction does not apply
to those separate native operations.


## Core, positions and configuration

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| `version() -> Result<String, Error>` | `char *swe_version(char *)`, 256-byte caller buffer | none | version string, e.g. `"2.10.03"` | n/a | null return degrades to buffer read | owned `String`, copied under lock | `native_version_matches_pinned_build`; rustdoc + doctest |
| `library_path() -> Result<String, Error>` | `char *swe_get_library_path(char *)` | none | executable/library pathname (256-byte native truncation, lossy UTF-8) | n/a | diagnostics only | owned `String` | long-executable-path isolated sanitizer regression; independence from ephemeris path; rustdoc |
| `julday(y: i32, m: i32, d: i32, hour: f64, cal: Calendar) -> f64` | `double swe_julday(int,int,int,double,int)` | month/day flow through natively (no validation); `Calendar` makes the flag unrepresentable-wrong | days; JD 0 = noon 4713-01-01 BCE Julian proleptic | n/a (pure) | non-finite `hour` propagates, never clamped | infallible by construction | exact J2000 + BCE round trip; rustdoc + doctest |
| `revjul(jd: f64, cal: Calendar) -> Result<CalendarDate, Error>` | `void swe_revjul(double,int,int*,int*,int*,double*)` | finite JD from native `julday(i32::MIN,1,1,0,cal)` inclusive to `julday(i32::MAX,12,31,24,cal)` exclusive; the calendar year must fit `i32`, not the JD (both edges probed with UBSan); proleptic Gregorian, no JD-threshold auto-switch | `CalendarDate{year,month,day,hour}`, year 0 = 1 BCE | n/a (pure) | out-of-domain → `InvalidInput` pre-call; otherwise always succeeds natively | owned struct | round trips incl. BCE + rejection + full year bounds (`tests/calendar_bounds.rs`); rustdoc + doctest |
| `calc_ut(jd_ut: f64, body: i32, flags: i32) -> Result<Position, Error>` | `int32 swe_calc_ut(double,int32,int32,double[6],char[256])` | finite input/effective dates within the calendar arithmetic domain above; `body` = body numbers incl. `ECL_NUT`; unknown ids fail natively | UT input; default output deg/deg/AU + deg/day rates; per-flag frames | all six components + returned source bits + `serr` | invalid date/shift → `InvalidInput` pre-call; `ret < 0` or non-finite native output → `Native` error with diagnostic; fallback/warning stays `Ok` with flags+text | `Position{values, returned_flags, diagnostic}` owned | Moshier structural + SWIEPH flag provenance + unknown-body error + rejection; rustdoc + doctest |
| `calc(jd_et: f64, body: i32, flags: i32) -> Result<Position, Error>` | `int32 swe_calc(double,int,int32,double[6],char[256])` | same as `calc_ut` | Ephemeris Time input (TT-like scale of the data); output layout identical | same as `calc_ut` | same as `calc_ut` | same as `calc_ut` | UT/ET agreement at J2000; rustdoc |
| `get_planet_name(body: i32) -> Result<String, Error>` | `char *swe_get_planet_name(int,char *)`, 256-byte buffer | full native domain (classical, fictitious, `AST_OFFSET + n`, `PLMOON_OFFSET + n`); see the body-identifier section below | short native names (`"Sun"`, catalog names for asteroids) | n/a | out-of-range numbers keep the native verdict verbatim as `Ok` text; only a truly empty result becomes `Native` (no pinned-build input is known to produce one) | owned `String` | `"Sun"`/`"Moon"` (including: Cupido/Ceres/alias/`"not found"` verbatim); rustdoc + doctest |
| `set_ephe_path(path: Option<&str>) -> Result<(), Error>` | `void swe_set_ephe_path(const char *)` (copies; null = default) | NUL rejected; `> 242` bytes rejected (native would silently default); `None` = null | selects data directory for file-based precision | n/a | never fails natively; validation is pre-call | `CString` temporary is safe (native copies) | NUL + overlong rejection; data restore; rustdoc + doctest |
| `set_topo(lon: f64, lat: f64, alt: f64) -> Result<(), Error>` | `void swe_set_topo(double,double,double)` | lon east-positive deg, lat deg, alt meters; all three must be finite | observer for `FLG_TOPOCTR` and later observer families | n/a | non-finite → `InvalidInput` pre-call with the previous observer untouched (a NaN observer otherwise contaminates later topocentric results); finite altitude must also respect the observer arithmetic ceiling above; native geographic limits remain | values only | topocentric Moon succeeds + preservation; rustdoc |
| `set_sid_mode(mode: i32, t0: f64, ayan_t0: f64) -> Result<(), Error>` | `void swe_set_sid_mode(int32,double,double)` | `SIDM_*` modes; `t0`/`ayan_t0` parameterize `SIDM_USER` and must be finite | selects ayanamsha for `FLG_SIDEREAL` + `get_ayanamsa_ut` | n/a | non-finite → `InvalidInput` pre-call with the previous mode untouched; unknown modes are native-defined behavior | values only | Lahiri≠Fagan + Lahiri band + preservation; rustdoc |
| `get_ayanamsa_ut(jd_ut: f64) -> Result<f64, Error>` | `double swe_get_ayanamsa_ut(double)` | UT input | degrees of tropical-sidereal offset | n/a | date/reference domain → `InvalidInput`; non-finite native value → `Native` | value | band + mode-difference; rustdoc + doctest |
| `houses(jd_ut: f64, lat: f64, lon: f64, hsys: u8) -> Result<Houses, Error>` | `int swe_houses(double,double,double,int,double[13],double[10])` | `hsys` ASCII letter (`b'P'`); `'G'`/non-letters rejected pre-call (36-sector layout) | UT; cusps deg, angles deg except ARMC | 12 cusps (native 1-12, no index-0 padding) + 8 angles | native `ERR` (no `serr`) → `Native` error; no consumer fallback | `Houses{cusps:[f64;12], angles:[f64;8]}` | `houses == houses_ex(0)`; ranges; Gauquelin layout; rustdoc + doctest |
| `houses_ex(jd_ut: f64, flags: i32, lat: f64, lon: f64, hsys: u8) -> Result<Houses, Error>` | `int32 swe_houses_ex(double,int32,double,double,int,double[13],double[10])` | `flags` forwarded for systems needing a solar position | same as `houses` | same as `houses` | same as `houses` | same as `houses` | agreement with `houses`; rustdoc |
| `close() -> Result<(), Error>` | `void swe_close(void)` | process-wide | drops caches/open files; next use lazily re-inits | n/a | never fails natively | no `Drop` anywhere by design | close → re-point → calc succeeds; rustdoc |
| constants (`JUL_CAL`…`SIDM_USER`, bodies, `FLG_*`) | `SE_*` defines in `swephexp.h` | exact values verified against the pinned header | n/a | n/a | n/a | `pub const i32` | flag-value assertion (`JUL_CAL==0`, `GREG_CAL==1`) |

## Time conversions, Delta-T and sidereal time

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| `utc_to_jd(y: i32, m: i32, d: i32, h: i32, min: i32, sec: f64, cal: Calendar) -> Result<JulDays, Error>` | `int32 swe_utc_to_jd(int32,int32,int32,int32,int32,double,int32,double[2],char[256])` | finite seconds in `0.0..61.0` are required before native access (fixed-buffer diagnostic safety); other native date/time validation flows through; normalized calendar year and Gregorian `YYYYMMDD` leap-table key must fit `i32`; `60.x` s only at 23:59 on a leap-second UTC; pre-1972 input treated as UT1 | `JulDays{jd_et, jd_ut}` in days (TT and UT1) | n/a | non-finite/out-of-range seconds or unrepresentable calendar/key → `InvalidInput`; native `ERR` (including invalid leap-second labels) → `Native` with diagnostic; warnings stay `Ok` | owned struct | J2000 split, round trips, leap-second accept/reject, invalid inputs, pre-1972; rustdoc + doctest |
| `jdet_to_utc(jd_et: f64, cal: Calendar) -> Result<UtcDateTime, Error>` | `void swe_jdet_to_utc(double,int32,int32*,int32*,int32*,int32*,int32*,double*)` | finite TT input/intermediates; calendar casts actually reached after configured Delta-T shifts and Gregorian `YYYYMMDD` key must fit `i32` (checked under the computation lock); before the native 1972 TT cutoff only final UT1 is decomposed, so TT need not fit the calendar domain; leap-second instants report `60.x` s | `UtcDateTime{year,month,day,hour,minute,second}` | n/a | unrepresentable native integer conversions → `InvalidInput`; otherwise infallible natively except lock poisoning | owned struct | round trips + `calendar_bounds` out-of-calendar TT/representable UT1 regression; rustdoc + doctest |
| `jdut1_to_utc(jd_ut: f64, cal: Calendar) -> Result<UtcDateTime, Error>` | `void swe_jdut1_to_utc(...)` (same layout as above) | UT1 input | same as `jdet_to_utc` | n/a | same as `jdet_to_utc` | same as `jdet_to_utc` | clock-exact round trip incl. leap second; rustdoc |
| `utc_time_zone(y: i32, m: i32, d: i32, h: i32, min: i32, sec: f64, tz: f64) -> Result<UtcDateTime, Error>` | `void swe_utc_time_zone(int32 ×5,double,double,...)` | `tz` fractional hours east of Greenwich; no calendar flag natively | shifted `UtcDateTime` | n/a | clock intermediates or shifted calendar year outside `i32` → `InvalidInput`; lock poisoning → `LockPoisoned` | owned struct | +2 h shift; rustdoc |
| `date_conversion(y: i32, m: i32, d: i32, hour: f64, cal: Calendar) -> Result<f64, Error>` | `int swe_date_conversion(int,int,int,double,char,double*)` (`b'g'`/`b'j'`) | normalized JD must have an `i32` calendar year; impossible dates fail instead of rolling over (unlike `julday`) | Julian Day in days | n/a | unrepresentable calendar/key → `InvalidInput`; native `ERR` → `Native` | value | J2000 equality + Feb-30 rejection; rustdoc + doctest |
| `day_of_week(jd: f64) -> Result<i32, Error>` | `int swe_day_of_week(double)` | finite `floor(jd - 2433282.0 - 1.5)` must fit `i32`, preserving native subtraction order; this offset domain differs from calendar-year bounds  | Monday = 0 … Sunday = 6 | n/a | out-of-domain → `InvalidInput` pre-call | value | J2000 == Saturday (5) + rejection; rustdoc + doctest |
| `deltat(jd_ut: f64) -> Result<f64, Error>` | `double swe_deltat(double)` (native tidal-acc/ephemeris guess) | TT − UT in days | days (~0.00074 at J2000) | n/a | non-finite input → `InvalidInput`; lock poisoning possible | value | J2000 band; rustdoc + doctest |
| `deltat_ex(jd_ut: f64, flags: i32) -> Result<DeltaT, Error>` | `double swe_deltat_ex(double,int32,char[256])` | source bits select tidal acceleration for this call only | `DeltaT{value, diagnostic}` in days | n/a | always usable value; diagnostic is warning-only | owned struct | SWIEPH≈default, Moshier band; rustdoc |
| `time_equ(jd_ut: f64) -> Result<EquationOfTime, Error>` | `int32 swe_time_equ(double,double*,char[256])` | needs a solar position (data-dependent) | `EquationOfTime{value, diagnostic}` in days (±0.012 band) | n/a | `ERR` → `Native`; fallback warning preserved in `diagnostic` | owned struct | band + fallback-warning test; rustdoc |
| `lmt_to_lat(jd_lmt: f64, lon: f64) -> Result<LocalTime, Error>` | `int32 swe_lmt_to_lat(double,double,double*,char[256])` | lon east-positive deg | `LocalTime{jd, diagnostic}` | n/a | same as `time_equ` | owned struct | round trip with `lat_to_lmt`; rustdoc |
| `lat_to_lmt(jd_lat: f64, lon: f64) -> Result<LocalTime, Error>` | `int32 swe_lat_to_lmt(double,double,double*,char[256])` | same as `lmt_to_lat` (iterated internally) | same as `lmt_to_lat` | n/a | same as `time_equ` | owned struct | round trip; rustdoc |
| `sidtime(jd_ut: f64) -> Result<f64, Error>` | `double swe_sidtime(double)` | UT input | Greenwich mean sidereal hours 0-24 | n/a | date/shift domain violation → `InvalidInput`; lock poisoning possible | value | range + explicit-frame agreement; rustdoc + doctest |
| `sidtime0(jd_ut: f64, eps: f64, nut: f64) -> Result<f64, Error>` | `double swe_sidtime0(double,double,double)` | `eps` true obliquity deg, `nut` nutation-in-longitude deg | hours 0-24 | n/a | same as `sidtime` | value | agreement with `sidtime`; rustdoc |

## Planet-centric positions and model configuration

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| `calc_pctr(jd_et: f64, body: i32, center: i32, flags: i32) -> Result<Position, Error>` | `int32 swe_calc_pctr(double,int32,int32,int32,double[6],char[256])` | ET only — no UT variant exists natively; `FLG_TOPOCTR` has no effect on other bodies natively; Moshier planet-centric output is unsupported by the engine (explicit capability gap, preserved as `Native` error); `ECL_NUT` as target or centre has no history-independent native result — the engine recomputes nutation internally, so first-call positions/speeds differ from repeats (parity/contract gap, not a silent fallback: the mapping is documented, the matrix excludes such requests, and parity keeps only the stable no-speed-target neighbour) | same layout as `calc_ut`; distance is center-to-body AU | same as `calc_ut` | same as `calc_ut` | same as `calc_ut` | file-based Moon-from-Mars provenance + Moshier-gap error + unknown-body/center errors + nutation history-dependence note; rustdoc + data-dependent doctest |
| `set_jpl_file(fname: &str) -> Result<(), Error>` | `void swe_set_jpl_file(const char *)` (non-null; copies into native storage; truncates names ≥ 256 bytes; closes cached file data and tries to open the file) | NUL rejected; `> 255` bytes rejected (native would silently truncate); empty string closes any open JPL file; no `None` (null would crash the native setter) | selects the JPL file for `FLG_JPLEPH`; a directory part also feeds the native search path; honors `set_ephe_path`/`SE_EPHE_PATH` for lookup | n/a | never fails natively; an unopenable file surfaces later as a visible fallback in `FLG_JPLEPH` calculations, never here | `CString` temporary is safe (native copies) | NUL + overlong rejection; missing-file fallback provenance; rustdoc + `no_run` doctest (no JPL data ships in-repo) |
| `set_tid_acc(acc: f64) -> Result<(), Error>` | `void swe_set_tid_acc(double)` (`SE_TIDAL_AUTOMATIC` = 999999 restores automatic selection) | non-finite rejected; every finite value passes through | arcsec/cy²; rescales pre-1955 Delta T only; user Delta-T override takes precedence | n/a | never fails natively | value | round trip + historical-vs-modern Delta-T effect; rustdoc |
| `get_tid_acc() -> Result<f64, Error>` | `double swe_get_tid_acc(void)` | none | arcsec/cy² currently used by `deltat`/`deltat_ex`; in automatic mode the engine tracks the ephemeris in use, so opened data files can update the value (e.g. DE441 files report −25.936) | n/a | infallible natively; lock poisoning only | value | round trip incl. automatic restore; rustdoc + doctest |
| `set_delta_t_userdef(dt: Option<f64>) -> Result<(), Error>` | `void swe_set_delta_t_userdef(double)` (`SE_DELTAT_AUTOMATIC` = −1E-10 resumes computed values) | `None` sends the sentinel; non-finite `Some` rejected | days (TT − UT), pinned for both `deltat`/`deltat_ex` while active | n/a | never fails natively | values only | exact pin + clear-to-band restoration; rustdoc + doctest |
| center/flag matrix over `calc` | same `swe_calc` contract in the core section | heliocentric Sun, barycentric (file-based), equatorial, Cartesian, radians, true/geometric, J2000, no-nutation, astrometric alias, 3-point speeds | per-flag frames as in slice 1 | six components + returned flags throughout | Moshier barycentric is a native capability gap (`Native` error with diagnostic); heliocentric Sun is valid zero data (`Ok`) | same as `calc_ut` | helio-zero, xyz-norm, radian-scale, zero-speeds, astrometric bitwise alias, speed3 agreement; rustdoc |
| constants (`SE_TIDAL_*`, `SE_DELTAT_AUTOMATIC`, `MAX_JPL_FILE_LEN`) | `SE_TIDAL_*`/`SE_DELTAT_AUTOMATIC` defines in `swephexp.h` | exact values verified against the pinned header (`SE_` prefix canonical here) | n/a | n/a | n/a | `pub const f64` / `usize` | round-trip assertions (`SE_TIDAL_DE421 == -25.85`) |

## Houses, speeds, Gauquelin sectors and ayanamsha

The `ex2` calls compute speeds natively. ARMC calls have no Julian Day. Twelve-cusp shapes reject Gauquelin; `houses_gauquelin` returns its 36-sector layout.

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| `houses_ex2(jd_ut: f64, flags: i32, latitude: f64, longitude: f64, hsys: u8) -> Result<HousesWithSpeeds, Error>` | `int swe_houses_ex2(...)` (13/10/13/10 slots, `serr`) | `'G'`/`'g'` rejected pre-call (36 sectors do not fit; use `houses_gauquelin`); non-letters rejected | UT; cusps/angles deg, speeds deg/day | 12 cusps + 8 angles + 12 cusp speeds + 8 angle speeds | `ERR` → `Native` with diagnostic | `HousesWithSpeeds` owned | bitwise agreement with `houses` + diurnal-scale speeds + C cross-check; rustdoc + doctest |
| `houses_gauquelin(jd_ut: f64, flags: i32, latitude: f64, longitude: f64) -> Result<GauquelinSectors, Error>` | same `swe_houses_ex2` with `'G'` (37/10/37/10 slots) | sector layout only; twelve-cusp systems must use `houses_ex2` (no silent truncation) | same as `houses_ex2` | 36 sectors + 8 angles + speeds | same as `houses_ex2` | `GauquelinSectors` owned | 36-sector range/speed checks + C cross-check; rustdoc + doctest |
| `houses_armc(armc: f64, latitude: f64, eps: f64, hsys: u8) -> Result<Houses, Error>` | `int swe_houses_armc(...)` (13/10 slots, no `serr`) | ARMC normalized natively; Sunshine `'I'` reads 0° declination (use `houses_armc_ex2` for explicit input) | deg throughout; no time scale (no Julian Day) | 12 cusps + 8 angles | `ERR` → `Native` (no diagnostic exists) | `Houses` owned | ARMC echo + wrap + agreement with `armc_ex2`; rustdoc + doctest |
| `houses_armc_ex2(armc: f64, latitude: f64, eps: f64, hsys: u8, sun_declination: f64) -> Result<HousesWithSpeeds, Error>` | `int swe_houses_armc_ex2(...)` (`ascmc[9]` is the Sunshine declination input) | `sun_declination` ignored outside `'I'`/`'i'` (finiteness still required) | same as `houses_armc` | same as `houses_ex2` | `ERR` → `Native` with diagnostic | `HousesWithSpeeds` owned | Sunshine input + ignore-outside-`'I'` + C cross-check; rustdoc + doctest |
| `house_pos(armc: f64, latitude: f64, eps: f64, hsys: u8, longitude: f64, latitude_body: f64) -> Result<HousePosition, Error>` | `double swe_house_pos(...)` (`xpin[2]`, `serr`) | engine folds letter case (`'g'` = `'G'`); unknown systems use the documented simplified algorithm | fractional house 1-12 (sector 1-36 for `'G'`); Koch circumpolar failure is exactly `0.0` | value + diagnostic | no error return exists: `0.0` + diagnostic is the preserved failure state | `HousePosition` owned | cusp round trip + sector range + Koch sentinel + C cross-check; rustdoc + doctest |
| `house_name(hsys: u8) -> String` | `const char *swe_house_name(int)` (static storage, never null) | unknown selectors fall through to `"Placidus"` natively — preserved verbatim | n/a | n/a | infallible; poisoned lock degrades to the unlocked pure lookup | owned `String` | table assertions incl. fallthrough; rustdoc + doctest |
| `gauquelin_sector(jd_ut: f64, body: i32, star_name: Option<&str>, flags: i32, method: i32, longitude: f64, latitude: f64, altitude: f64, atpress: f64, attemp: f64) -> Result<GauquelinSector, Error>` | `int32 swe_gauquelin_sector(...)` (caller-owned 256-byte star buffer, `geopos[3]`, `dgsect`, `serr`) | methods 0-5 native (`ERR` + diagnostic outside); `0.0` pressure/temperature select native defaults; fixstar catalog semantics covered in fixed-star tests | fractional sector [1, 37); observer deg/deg/m | value + diagnostic | `ERR` (bad method, circumpolar rise/set search) → `Native` | `GauquelinSector` owned | Moshier geometric paths + empty-name equivalence + method error + C cross-check + fixed-star path check; rustdoc + doctest |
| `get_ayanamsa(jd_et: f64) -> Result<f64, Error>` | `double swe_get_ayanamsa(double)` | ET input (unlike `get_ayanamsa_ut`) | degrees | n/a | date/shift domain violation → `InvalidInput`; lock poisoning possible | value | UT/ET split + band; rustdoc + doctest |
| `get_ayanamsa_ex(jd_et: f64, flags: i32) -> Result<Ayanamsha, Error>` | `int32 swe_get_ayanamsa_ex(double,int32,double*,char*)` | source bits serve star-based modes | `Ayanamsha{value, returned_flags, diagnostic}` in degrees | returned source flags + warning text | `ERR` → `Native` | owned struct | band + provenance + C cross-check; rustdoc + doctest |
| `get_ayanamsa_ex_ut(jd_ut: f64, flags: i32) -> Result<Ayanamsha, Error>` | `int32 swe_get_ayanamsa_ex_ut(...)` (UT→ET internally) | same as `get_ayanamsa_ex` | same as `get_ayanamsa_ex` | same as `get_ayanamsa_ex` | same as `get_ayanamsa_ex` | same as `get_ayanamsa_ex` | nutation-scale agreement with plain path + C cross-check; rustdoc + doctest |
| `get_ayanamsa_name(mode: i32) -> Result<String, Error>` | `const char *swe_get_ayanamsa_name(int32)` (null = no predefined name) | negative modes rejected pre-call (native indexes its table after modulo 256: negative would read out of bounds); nameless modes → `Native` error | n/a | n/a | see domain | owned `String` | Lahiri/Fagan names + nameless/negative errors; rustdoc + doctest |

## Body identifiers and strict-source checks

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| full body domain over `calc_ut`/`calc`/`calc_pctr` (no new functions) | same `swe_calc_ut`/`swe_calc`/`swe_calc_pctr` contracts in the position sections | classical `SUN`..`INTP_PERG` plus `ECL_NUT`, fictitious `CUPIDO`..`WALDEMATH`, numbered asteroids `AST_OFFSET + n` (aliasing dedicated ids where both exist), `PLMOON_OFFSET + n`; no Rust-side range rejection | per-flag frames as in slice 1 | six components + returned flags throughout | numbers with no built-in or file elements (e.g. body 999) fail natively with the diagnostic kept; fictitious bodies compute from built-in elements under any source bit (the sub-arcsecond SWIEPH/MOSHIER split is native correction-chain behavior, confirmed per-flag against the C caller) | same as `calc_ut` | fictitious source-band + `AST_OFFSET + 1 == CERES` bitwise + missing-element error + `ECL_NUT` structural; rustdoc |
| `get_planet_name` verbatim verdicts | same `swe_get_planet_name` contract in the core section | same full domain as above | same in the core section | n/a | out-of-range numbers return the native descriptive text as `Ok` (`"name not found"`, `"989999: not found (asteroid)"`, `"10000: not found (planetary moon)"`); a consumer policy that errors on these lives in the caller, not in bindings | owned `String` | Cupido/Ceres/alias assertions + `"not found"` verbatim test; rustdoc + doctest |
| `require_source_flags(position: &Position, requested_flags: i32) -> Result<(), Error>` | none — pure-Rust inspection of `Position.returned_flags`/`diagnostic` | pass the same `flags` given to the position call; no source bit requested (default `0`) enforces nothing | n/a | source bits `FLG_SWIEPH`/`FLG_JPLEPH`/`FLG_MOSEPH` | primary source must match an allowed requested source and native `using ... eph` component diagnostics must name only allowed sources; mismatch → `Native` with flags and original diagnostic; valid data is never rewritten | no lock, no allocation beyond the error message | pure match/mismatch/default-chain tests + end-to-end file-based vs fallback gating; rustdoc + doctest |

## Fixed stars

Name buffers hold 512 bytes, twice `SE_MAX_STNAME`; resolved names are owned. Catalog-dependent calls need `sefstars.txt`.

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| `fixstar(star: &str, jd_et: f64, flags: i32) -> Result<StarPosition, Error>` | `int32 swe_fixstar(char*, double, int32, double[6], char[256])` | 512-byte in/out name buffer; NUL/overlong (`>= 512`)/non-finite rejected pre-call; empty query flows to the native verdict | ET input; layout per flags exactly as in `calc` | six components + resolved full name + returned source bits + `serr` | `ret < 0` → `Native` with diagnostic (unknown star, missing catalog); warnings stay `Ok` | `StarPosition{values, resolved_name, returned_flags, diagnostic}` owned | Sirius ET/UT band + bitwise repeat + C cross-check; rustdoc + doctest |
| `fixstar_ut(star: &str, jd_ut: f64, flags: i32) -> Result<StarPosition, Error>` | `int32 swe_fixstar_ut(...)` (same layout, UT→TT internally) | same as `fixstar` | UT input; otherwise identical | same as `fixstar` | same as `fixstar` | same as `fixstar` | same as `fixstar` |
| `fixstar2(star: &str, jd_et: f64, flags: i32) -> Result<StarPosition, Error>` | `int32 swe_fixstar2(...)` (same buffers as `swe_fixstar`) | traditional name, `",nomenclature"` (e.g. `",alCMa"`), catalog number, or trailing-`'%'` wildcard; the echo identifies the matched entry | same as `fixstar` | same as `fixstar` | same as `fixstar` | same as `fixstar` | nomenclature/number/wildcard equivalence + C cross-check; rustdoc |
| `fixstar2_ut(star: &str, jd_ut: f64, flags: i32) -> Result<StarPosition, Error>` | `int32 swe_fixstar2_ut(...)` | same query language as `fixstar2` | UT input; otherwise identical | same as `fixstar` | same as `fixstar` | same as `fixstar` | same as `fixstar2`; rustdoc + doctest |
| `fixstar_mag(star: &str) -> Result<StarMagnitude, Error>` | `int32 swe_fixstar_mag(char*, double*, char*)` (OK/ERR) | same 512-byte in/out name contract; `mag` is one writable slot | visual magnitude (e.g. `-1.46` Sirius) | magnitude + resolved name + `serr` | `ERR` → `Native` with diagnostic | `StarMagnitude{magnitude, resolved_name, diagnostic}` owned | Sirius/Regulus catalog bands + C cross-check; rustdoc + doctest |
| `fixstar2_mag(star: &str) -> Result<StarMagnitude, Error>` | `int32 swe_fixstar2_mag(...)` (same layout) | same query language as `fixstar2` | same as `fixstar_mag` | same as `fixstar_mag` | same as `fixstar_mag` | same as `fixstar_mag` | equivalence with `fixstar_mag` + unknown-star error; rustdoc |
| star catalog loading | `sefstars.txt` via the `set_ephe_path` search path | catalog follows the process-global path; repeat calls are exactly reproducible under the lock | n/a | n/a | missing catalog (empty dir) fails natively with the diagnostic kept, never invented data | path ownership as in `set_ephe_path` | empty-dir gap + restore + repeat-bitwise integration |

## Nodes, apsides and observer events

Nodes/apsides return native status rather than position source flags. Rise status `-2` is `Circumpolar`; no event time is fabricated.

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| `nod_aps(jd_et: f64, body: i32, flags: i32, method: i32) -> Result<NodesApsides, Error>` | `int32 swe_nod_aps(double,int32,int32,int32,double[6] ×4,char[256])` | any integer `method` passes through (0 = default mean-shape); no Rust-side range rejection | ET input; ecliptic longitude/latitude deg + AU + daily rates (rates zero without `FLG_SPEED`) | four six-component vectors (ascending, descending, perihelion, aphelion/focal point) + native OK status + `serr` | `ret < 0` → `Native` with diagnostic (unknown body, missing elements); warnings stay `Ok` | `NodesApsides` owned | Mars mean/oscu/focal-point + Moon + repeat-bitwise + C cross-check; rustdoc + doctest |
| `nod_aps_ut(jd_ut: f64, body: i32, flags: i32, method: i32) -> Result<NodesApsides, Error>` | `int32 swe_nod_aps_ut(...)` (same layout, UT→TT internally) | same as `nod_aps` | UT input; otherwise identical | same as `nod_aps` | same as `nod_aps` | same as `nod_aps` | UT/ET band + C cross-check; rustdoc + doctest |
| `rise_trans(jd_ut: f64, body: i32, star_name: Option<&str>, epheflag: i32, rsmi: i32, longitude: f64, latitude: f64, altitude: f64, atpress: f64, attemp: f64) -> Result<RiseTransitOutcome, Error>` | `int32 swe_rise_trans(double,int32,char*,int32,int32,double[3],double,double,double*,char*)` (`tret` is one slot: the event JD in UT) | `None`/empty star = body path (null passed); star names need the catalog; `atpress = 0` estimates pressure from altitude; `rsmi` = `CALC_*` selector + `BIT_*` modifiers | UT throughout; observer east-deg/north-deg/meters; event time as UT Julian Day | event time + `serr` | `0` → `Event`, `-2` → `Circumpolar` (both `Ok`); any other negative → `Native` with diagnostic | `RiseTransitOutcome` owned | sunrise/sunset/transit windows + polar no-event + star/empty/unknown-star paths + C cross-check; rustdoc + doctest |
| `rise_trans_true_hor(jd_ut: f64, body: i32, star_name: Option<&str>, epheflag: i32, rsmi: i32, longitude: f64, latitude: f64, altitude: f64, atpress: f64, attemp: f64, horhgt: f64) -> Result<RiseTransitOutcome, Error>` | `int32 swe_rise_trans_true_hor(...)` (same layout + horizon height deg) | same as `rise_trans` + finite `horhgt` | same as `rise_trans` | same as `rise_trans` | same as `rise_trans` | same as `rise_trans` | zero-horizon agreement + C cross-check; rustdoc + doctest |
| rise constants (`NODBIT_*`, `CALC_*`, `BIT_*`) | `SE_NODBIT_*`/`SE_CALC_*`/`SE_BIT_*` defines in `swephexp.h` | exact values verified against the pinned header (`BIT_HINDU_RISING == 896`) | n/a | n/a | n/a | `pub const i32` | value assertions (`NODBIT_MEAN==1`, `CALC_RISE==1`) |

## Coordinates, refraction and physical quantities

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| `azalt(jd_ut, calc_flag, longitude, latitude, altitude, atpress, attemp, lon_or_ra, lat_or_dec) -> Result<Horizontal, Error>` | `void swe_azalt(double,int32,double[3],double,double,double[3],double[3])` | `ECL2HOR` (0) ecliptic / `EQU2HOR` (1) equatorial input; other integers pass through engine-defined (observed nonzero → equatorial branch); distance plays no role, no distance argument | UT; observer east-deg/north-deg/m; input/output deg; azimuth from South westward | azimuth + true altitude + apparent altitude | no native failure mode; `Result` covers lock poisoning + pre-call finite/date/height checks | `Horizontal` owned | frame-difference + round-trip + C cross-check; rustdoc + doctest |
| `azalt_rev(jd_ut, calc_flag, longitude, latitude, altitude, azimuth, true_altitude) -> Result<Celestial, Error>` | `void swe_azalt_rev(double,int32,double[3],double[3],double[2])` (writes exactly 2 slots, established by probing) | `HOR2ECL` (0) / `HOR2EQU` (1) output; same pass-through note as `azalt` | same as `azalt` | celestial pair (RA/Dec or lon/lat) | same as `azalt` | `Celestial` owned | round-trip exact + C cross-check; rustdoc + doctest |
| `refrac(altitude, atpress, attemp, calc_flag) -> Result<f64, Error>` | `double swe_refrac(double,double,double,int32)` | `TRUE_TO_APP` (0) / `APP_TO_TRUE` (1); zero pressure returns the input unchanged; others pass through engine-defined | degrees throughout | converted altitude | non-finite input → `InvalidInput`; non-finite converted altitude → `Native` with a binding-generated diagnostic (no native error buffer); finite native values preserved | value | 10° band + zero-pressure identity + near-inverse + finite-input/non-finite-output regression + C cross-check; rustdoc + doctest |
| `refrac_extended(altitude, geoalt, atpress, attemp, lapse_rate, calc_flag) -> Result<Refraction, Error>` | `double swe_refrac_extended(double ×5,int32,double[4])` (return = converted altitude; details = true/apparent/refraction/dip, established by probing) | explicit lapse rate in K/m (`0.0065` conventional); below the dipped horizon the input returns unchanged with zero refraction | degrees + meters + K/m | converted + 4 details, all finite | same as `refrac`, checking both converted altitude and every detail; below-sea-level non-finite dip → `Native`, never a substituted sea-level result | `Refraction` owned | sea-level details + 1000 m dip band + lapse effect + below-sea-level/temperature/lapse/pressure non-finite-output regression + C cross-check; rustdoc + doctest |
| `set_lapse_rate(lapse_rate: f64) -> Result<(), Error>` | `void swe_set_lapse_rate(double)` (process-global, write-only) | non-finite rejected; every finite rate passes through | K/m | n/a | never fails natively | values only | acceptance + restore sequencing; rustdoc + doctest |
| `pheno(jd_et: f64, body: i32, flags: i32) -> Result<Phenomena, Error>` | `int32 swe_pheno(double,int32,int32,double[20],char[256])` | full body domain passes through; no star path exists natively | ET input; phase/elongation/diameter deg, fraction 0-1, magnitude mag | 5 components + retflags + `serr` (slots 5-19 engine-zeroed) | `ret < 0` → `Native` with diagnostic (e.g. body 999 missing elements); warnings stay `Ok` | `Phenomena` owned | Mars bands + Sun verbatim row + UT/ET split + repeat + C cross-check; rustdoc + doctest |
| `pheno_ut(jd_ut: f64, body: i32, flags: i32) -> Result<Phenomena, Error>` | `int32 swe_pheno_ut(...)` (same layout, UT→TT internally) | same as `pheno` | UT input; otherwise identical | same as `pheno` | same as `pheno` | same as `pheno` | same as `pheno` |
| `get_orbital_elements(jd_et: f64, body: i32, flags: i32) -> Result<OrbitalElements, Error>` | `int32 swe_get_orbital_elements(double,int32,int32,double[50],char[256])` (engine writes slots 0-16, established by probing Mars/Moon/Jupiter; wrapper zero-fills the 50-slot buffer) | Sun, lunar nodes/apsides and out-of-range numbers fail natively (`"object 0 not valid"`); `FLG_BARYCTR`/`FLG_ORBEL_AA` behaviors preserved verbatim | ET input; AU/deg/years/days/JD per slot | 17 elements + reserved-zero tail + `serr`; success always reports plain `OK` (0) | `ERR` → `Native` with diagnostic | `OrbitalElements` owned | Mars/Moon bands + Sun error + reserved tail + C cross-check; rustdoc + doctest |
| `orbit_max_min_true_distance(jd_et: f64, body: i32, flags: i32) -> Result<DistanceExtremes, Error>` | `int32 swe_orbit_max_min_true_distance(double,int32,int32,double*,double*,double*,char*)` (three single slots; order max/min/true preserved, not sorted) | same body domain as `get_orbital_elements` | ET input; AU throughout | max + min + current + `serr` | `ERR` → `Native` with diagnostic | `DistanceExtremes` owned | min < true < max + repeat + 999 error + C cross-check; rustdoc + doctest |
| coordinate/refraction constants (`ECL2HOR`, `EQU2HOR`, `HOR2ECL`, `HOR2EQU`, `TRUE_TO_APP`, `APP_TO_TRUE`) | `SE_*` defines in `swephexp.h` | exact values verified against the pinned header (`ECL2HOR == HOR2ECL == 0`, `EQU2HOR == HOR2EQU == 1`) | n/a | n/a | n/a | `pub const i32` | value assertions |

## Crossings and global eclipses

Solar/lunar crossings search forward and use the native `jd - 1` failure sentinel. Eclipse type zero is valid absence. Reserved slots are initialized.

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| `solcross(x2cross: f64, jd_et: f64, flags: i32) -> Result<LongitudeCrossing, Error>` | `double swe_solcross(double, double, int32, char[256])` | any finite target (engine normalizes); forward-only | ET input and result (Julian Day) | time + `serr` | below-epoch return → `Native` with diagnostic | `LongitudeCrossing` owned | Aries ingress + normalization + repeat + C cross-check; rustdoc + doctest |
| `solcross_ut(x2cross: f64, jd_ut: f64, flags: i32) -> Result<LongitudeCrossing, Error>` | `double swe_solcross_ut(...)` (same layout, UT) | same as `solcross` | UT input and result | same as `solcross` | same as `solcross` | same as `solcross` | UT/ET Delta-T agreement + C cross-check; rustdoc + doctest |
| `mooncross/mooncross_ut(x2cross, jd, flags) -> Result<LongitudeCrossing, Error>` | `double swe_mooncross(_ut)(...)` | same as `solcross` | ET/UT per variant | same as `solcross` | same as `solcross` | same as `solcross` | month window + longitude check + C cross-check; rustdoc + doctests |
| `mooncross_node(jd_et: f64, flags: i32) -> Result<NodeCrossing, Error>` | `double swe_mooncross_node(double, int32, double*, double*, char*)` (two single slots) | ascending or descending, whichever comes first; forward-only | ET input and result; lon/lat deg (lat ≈ 0) | time + position pair + `serr` | below-epoch return → `Native` | `NodeCrossing` owned | latitude-zero + repeat + C cross-check; rustdoc + doctest |
| `mooncross_node_ut(jd_ut: f64, flags: i32) -> Result<NodeCrossing, Error>` | `double swe_mooncross_node_ut(...)` (same layout, UT) | same as `mooncross_node` | UT input and result | same as `mooncross_node` | same as `mooncross_node` | same as `mooncross_node` | UT/ET agreement + C cross-check; rustdoc + doctest |
| `helio_cross(body: i32, x2cross: f64, jd_et: f64, flags: i32, dir: i32) -> Result<LongitudeCrossing, Error>` | `int32 swe_helio_cross(int32, double, double, int32, int32, double*, char*)` (single-slot `jd_cross`) | `dir >= 0` forward, `dir < 0` backward; Sun/Moon/nodes/out-of-range rejected natively | ET input and result; heliocentric longitude deg | time + `serr` | `ERR` → `Native` with diagnostic (e.g. `"not possible for object 0 = Sun"`) | `LongitudeCrossing` owned | forward/backward/repeat + helio-longitude check + Sun/Moon rejection + C cross-check; rustdoc + doctest |
| `helio_cross_ut(body: i32, x2cross: f64, jd_ut: f64, flags: i32, dir: i32) -> Result<LongitudeCrossing, Error>` | `int32 swe_helio_cross_ut(...)` (same layout, UT) | same as `helio_cross` | UT input and result | same as `helio_cross` | same as `helio_cross` | same as `helio_cross` | UT/ET agreement + C cross-check; rustdoc + doctest |
| `sol_eclipse_when_glob(jd_start: f64, flags: i32, eclipse_type: i32, backward: bool) -> Result<GlobalSolarEclipse, Error>` | `int32 swe_sol_eclipse_when_glob(double, int32, int32, double[10], int32, char[256])` | `eclipse_type` filter bits (0 = any); central+partial fails natively | UT throughout; ten `tret` times (maximum, noon-peak, begin/end, totality begin/end, centerline begin/end, two reserved hybrid slots) | type bitmask + 10 times + `serr` | `ERR` → `Native` (e.g. `"central partial eclipses do not exist"`) | `GlobalSolarEclipse` owned | type-bits/ordering/repeat/backward/impossible-combination/file-agreement + C cross-check; rustdoc + doctest |
| `lun_eclipse_when(jd_start: f64, flags: i32, eclipse_type: i32, backward: bool) -> Result<GlobalLunarEclipse, Error>` | `int32 swe_lun_eclipse_when(...)` (same layout) | annular-only fails natively; central/noncentral meaningless | UT throughout; ten `tret` times (maximum, reserved, partial/total/penumbral begin/end, two reserved) | same as solar | `ERR` → `Native` (e.g. `"annular lunar eclipses don't exist"`) | `GlobalLunarEclipse` owned | ordering/repeat/backward/annular-rejection + C cross-check; rustdoc + doctest |
| `sol_eclipse_where(jd_ut: f64, flags: i32) -> Result<SolarEclipseGeometry, Error>` | `int32 swe_sol_eclipse_where(double, int32, double[2], double[20], char[256])` | none | UT; geopos lon/lat deg of greatest eclipse; 20 attr (slots 0-10 meaningful, 11-19 reserved 0.0) | type bitmask (0 = no eclipse, kept as data) + geopos + attr + `serr` | `ERR` → `Native`; type 0 is `Ok` (e.g. `"no solar eclipse at tjd = ..."`) | `SolarEclipseGeometry` owned | type/magnitude/saros/reserved-tail + quiet-Ok + C cross-check; rustdoc + doctest |
| `sol_eclipse_how(jd_ut: f64, flags: i32, longitude: f64, latitude: f64, altitude: f64) -> Result<SolarEclipseCircumstances, Error>` | `int32 swe_sol_eclipse_how(double, int32, double[3], double[20], char[256])` | observer east-deg/north-deg/m; altitudes outside −500…25000 m fail natively | UT; same 20 attr as `where` | local phase bitmask (0 = none here, kept as data) + attr + `serr` | `ERR` → `Native` (e.g. altitude range) | `SolarEclipseCircumstances` owned | local totality + how/where equality + quiet + altitude error + C cross-check; rustdoc + doctest |
| `lun_eclipse_how(jd_ut: f64, flags: i32, observer: Option<(f64, f64, f64)>) -> Result<LunarEclipseCircumstances, Error>` | `int32 swe_lun_eclipse_how(double, int32, double[3] or NULL, double[20], char[256])` | `None` = null (geocentric; azimuth/altitude stay 0.0); same altitude domain otherwise | UT; 20 attr (umbral/penumbral magnitude, azimuth, altitudes, opposition distance, saros; slots 2-3 unused) | type bitmask (0 = none/not visible, kept as data) + attr + `serr` | `ERR` → `Native` | `LunarEclipseCircumstances` owned | geocentric/observer/quiet + C cross-check; rustdoc + doctest |
| eclipse constants (`ECL_CENTRAL`…`ECL_ONE_TRY`) | `SE_ECL_*` defines in `swephexp.h` | exact values verified against the pinned header (`ECL_ONE_TRY == 32768`) | n/a | n/a | n/a | `pub const i32` | value assertions |

## Local eclipses and lunar occultations

Observer units are east-positive longitude degrees, latitude degrees and altitude metres. Native local-event altitude limits are -500 to 25000 metres. `OccultSearchOptions` combines direction and `ECL_ONE_TRY`; a miss retains type zero and its continuation epoch. Moon self-occultation is rejected before native access.

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| `sol_eclipse_when_loc(jd_start, flags, longitude, latitude, altitude, backward) -> Result<LocalSolarEclipse, Error>` | `int32 swe_sol_eclipse_when_loc(double, int32, double[3], double[7], double[20], int32, char[256])` | observer + altitude domain as above; `backward` direction only (the native one-try mode is decoded only by the occultation searches, ) | UT throughout; seven `tret` times (maximum, four contacts, sunrise, sunset); twenty attr at maximum with the `sol_eclipse_how` layout | type bitmask (TOTAL/ANNULAR/PARTIAL + VISIBLE + contact bits) + times + attr + `serr` | `ERR` → `Native` (e.g. altitude range); inapplicable contacts read 0.0 (absent data) | `LocalSolarEclipse` owned | Dallas totality (type 8068, saros 139/30) + backward + repeat + altitude error + C cross-check; rustdoc + doctest |
| `lun_eclipse_when_loc(jd_start, flags, longitude, latitude, altitude, backward) -> Result<LocalLunarEclipse, Error>` | `int32 swe_lun_eclipse_when_loc(...)` (same layout; ten `tret` slots; observer must be non-null — it also feeds rise/set transits) | same as solar local | UT throughout; ten `tret` times (maximum, reserved, partial/total/penumbral begin/end, moonrise/moonset); twenty attr at maximum with the `lun_eclipse_how` layout | type bitmask (TOTAL/PARTIAL/PENUMBRAL + VISIBLE + phase-visibility bits) + times + attr + `serr` | `ERR` → `Native`; moon-clipped phases read 0.0 with moonrise/moonset carrying the end | `LocalLunarEclipse` owned | London penumbral (moon-clipped) + partial + backward + repeat + C cross-check; rustdoc + doctest |
| `lun_occult_when_glob(jd_start, body, star_name, flags, eclipse_type, backward) -> Result<GlobalOccultation, Error>` | `int32 swe_lun_occult_when_glob(double, int32, char*, int32, int32, double[10], int32, char*)` | `None`/empty star = body path (null passed); negative bodies fold to the Sun, `AST_OFFSET + 134340` to Pluto natively; `eclipse_type` filter bits (0 = any; central + partial fails, annular fails for non-Sun); Moon on the body path rejected pre-call (non-termination) | UT throughout; ten `tret` times with the global-eclipse layout | type bitmask (TOTAL/PARTIAL + CENTRAL/NONCENTRAL) + times + `serr` | `ERR` → `Native` (impossible types, unoccultable star, failed positions) | `GlobalOccultation` owned | Venus total-central + backward + repeat + empty-star equivalence + impossible-type/star errors + Aldebaran star path + C cross-check; rustdoc + doctest |
| `lun_occult_when_loc(jd_start, body, star_name, flags, longitude, latitude, altitude, backward) -> Result<LocalOccultation, Error>` | `int32 swe_lun_occult_when_loc(...)` (ten `tret` slots zeroed by the engine itself, first seven used; twenty attr slots) | body/star selection as in `when_glob`; observer + altitude domain as in solar local; Moon on the body path rejected pre-call | UT throughout; ten `tret` times (maximum, four contacts, body rise/set, three reserved zeros); twenty attr at maximum (slots 0-7 meaningful; solar NASA/saros slots 8-10 read 0.0) | type bitmask (TOTAL/PARTIAL + VISIBLE + contact + OCC_DAYLIGHT bits) + times + attr + `serr` | `ERR` → `Native` (incl. altitude range) | `LocalOccultation` owned | Venus Greenwich total-visible + backward + repeat + altitude error + Aldebaran star path + C cross-check; rustdoc + doctest |
| `OccultSearchOptions { backward: bool, one_try: bool }` (+ `forward()`/`backward()` constructors) | native `backward` integer: bit 0 = direction (both functions apply `backward &= 1` after extracting the bit), `SE_ECL_ONE_TRY` (32768) = single-conjunction mode | two booleans instead of one overloaded int; encoding is `bit0 \| (one_try ? 32768 : 0)`, no other bit ever set | n/a (control, not time) | n/a | invalid combinations impossible by construction (bools) | `Copy` struct, no native resources | encoding + miss/hit/reject regressions; rustdoc |
| `lun_occult_when_glob_with_options(jd_start, body, star_name, flags, eclipse_type, search) -> Result<GlobalOccultation, Error>` | same native symbol as `lun_occult_when_glob` | same selection/domain notes; Moon on the body path still rejected pre-call | same layout | same bitmask + times + `serr` | `ERR` → `Native` as above; one-try miss is `Ok` type 0 with `tret[0]` = examined conjunction epoch (continuation date) | `GlobalOccultation` owned | `one_try: false` bitwise-identical to the bool call (fwd + bwd); one-try Venus hit == full-search maximum bitwise; one-try Venus miss is type 0 with finite forward `tret[0]`; NaN/Moon/NUL inputs → `InvalidInput`; rustdoc + doctest |
| `lun_occult_when_loc_with_options(jd_start, body, star_name, flags, longitude, latitude, altitude, search) -> Result<LocalOccultation, Error>` | same native symbol as `lun_occult_when_loc` | same selection/observer/altitude notes; same Moon guard | same layout | same bitmask + times + attr + `serr` | `ERR` → `Native` as above; one-try miss is `Ok` type 0 with `tret[0]` = next-try date | `LocalOccultation` owned | `one_try: false` bitwise-identical (fwd + bwd, incl. attr); one-try Mercury miss is type 0 with finite forward `tret[0]`; default observer restored; rustdoc + doctest |
| `lun_occult_where(jd_ut, body, star_name, flags) -> Result<OccultationGeometry, Error>` | `int32 swe_lun_occult_where(double, int32, char*, int32, double[2], double[20], char*)` | body/star selection as in `when_glob`; Moon on the body path rejected pre-call | UT; geopos lon/lat deg of greatest occultation; twenty attr with the local-occultation layout | type bitmask (0 = no occultation in progress, kept as data) + geopos + attr + `serr` | `ERR` → `Native`; type 0 is `Ok` (e.g. `"no solar eclipse at tjd = ..."`, preserved verbatim) | `OccultationGeometry` owned | Venus/Aldebaran at own maxima + quiet type-0 + reserved-tail zeros + repeat + C cross-check; rustdoc + doctest |

## Heliacal events and visibility

Observer, atmosphere and vision blocks are owned arrays with native zero defaults. Search `HELFLAG_AVKIND` bits are rejected due to a pinned buffer defect; instant-based phenomena/visibility retain their separate flag behavior.

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| `heliacal_ut(jd_start_ut, longitude, latitude, altitude, atmosphere: [f64; 4], observer: [f64; 6], object: &str, event_type: i32, helflag: i32) -> Result<HeliacalEvent, Error>` | `int32 swe_heliacal_ut(double, double[3], double[4], double[6], char*, int32, int32, double[10], char[256])` | object name required (empty reaches the native lookup); `event_type` 1-4 valid, others pass through and fail natively (acronychal 5-6 "still not implemented"); `helflag` combines ephemeris source bits with `HELFLAG_*` search bits; forward search only | UT throughout; ten `dret` times (beginning, optimum, end + seven reserved 0.0) | `HeliacalEvent` owned | `ERR` → `Native` (Sun, unknown object, impossible type/object, altitude range); no "absent event" return; any `HELFLAG_AVKIND` bit → `InvalidInput` before FFI (native internal buffer defect) | `HeliacalEvent{times, diagnostic}` owned | Venus/Moon/Sirius ordering + AVKIND rejection + repeat + Sun/type-5/Mars-type-3/altitude/unknown/empty errors + C cross-check; rustdoc + doctest |
| `heliacal_pheno_ut(jd_ut, longitude, latitude, altitude, atmosphere, observer, object, event_type, helflag) -> Result<HeliacalPhenomena, Error>` | `int32 swe_heliacal_pheno_ut(double, double[3], double[4], double[6], char*, int32, int32, double[30], char[256])` | same selection as `heliacal_ut`; `helflag` carries display bits (no search runs) | UT; 30 components (28 phenomena + two reserved 0.0) | `HeliacalPhenomena` owned | `ERR` → `Native`; below-horizon instants yield negative altitudes as data, never a dedicated status | `HeliacalPhenomena{values, diagnostic}` owned | Venus bands + TJD_INVALID sentinel + Moon Yallop timing + below-horizon data + repeat + C cross-check; rustdoc + doctest |
| `vis_limit_mag(jd_ut, longitude, latitude, altitude, atmosphere, observer, object, helflag) -> Result<VisibilityOutcome, Error>` | `int32 swe_vis_limit_mag(double, double[3], double[4], double[6], char*, int32, double[8], char[256])` | same selection as `heliacal_ut` without an event type | UT; limiting magnitude + geometry details + object magnitude (8 slots) | vision status (0 photopic, 1 scotopic, +2 near-limit bit) + values + `serr` | `ERR` → `Native` (Sun, unknown object); −2 below horizon is `Ok` (`BelowHorizon` with the −100 marker, never an error) | `VisibilityOutcome::Visible/BelowHorizon` owned | photopic/scotopic statuses + brightness relation + geometry cross-agreement + below-horizon marker + Sun/unknown errors + C cross-check; rustdoc + doctest |
| heliacal constants (`HELIACAL_RISING`…`MORNING_LAST`, `HELFLAG_*`, `HELFLAG_BELOW_HORIZON`/`PHOTOPIC`/`SCOTOPIC`/`MIXED`, `PHOTOPIC_FLAG`/`SCOTOPIC_FLAG`/`MIXEDOPIC_FLAG`, `TJD_INVALID`) | `SE_HELIACAL_*`/`SE_HELFLAG_*`/`SE_PHOTOPIC_FLAG`/…/`TJD_INVALID` defines in `swephexp.h` | exact values verified against the pinned header (`HELFLAG_AVKIND == 983040`); acronychal 5-6 declared-but-unimplemented names have no constants | n/a | n/a | n/a | `pub const i32` / `f64` | value assertions |

## Sidereal and latitude behavior

Flag-scoped `_ex` ayanamshas use the corresponding sidereal calculation chain. Plain ayanamshas use a different native correction chain. Polar house failures retain native errors.

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| `set_sid_mode(SIDM_USER, t0, ayan_t0)` + `FLG_SIDEREAL` matrix over `calc_ut`/`houses_ex` (no new functions) | same `swe_set_sid_mode` / `swe_calc_ut` / `swe_houses_ex` contracts above | user epoch + offset parameterize the mode; scoped `_ex` reads carry the calculation flags | degrees throughout | six components / 12 cusps + 8 angles with the scoped shift | same as the underlying calls | same as the underlying calls | USER offset band + difference proof + bitwise repeats + scoped-shift agreement + latitude matrix in `tests/sidereals.rs` |

## Angles and coordinate utilities

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| `cotrans(longitude_or_ra, latitude_or_dec, distance, eps) -> Result<[f64; 3], Error>` | `void swe_cotrans(double[3], double[3], double)` | triple in degrees/AU; `eps` obliquity in degrees, sign selects direction | deg throughout; distance preserved | rotated triple | infallible natively; `Result` covers lock poisoning only | owned array | direction anchors + round trip + C cross-check; rustdoc + doctest |
| `cotrans_sp(..., lon_speed, lat_speed, dist_speed, eps) -> Result<([f64; 3], [f64; 3]), Error>` | `void swe_cotrans_sp(double[6], double[6], double)` | position triple + daily-rate triple; same direction contract | same as `cotrans`; rates in matching units/day | rotated pair | same as `cotrans` | owned pair | round trip + plain-path agreement + C cross-check; rustdoc + doctest |
| `degnorm(x) / radnorm(x) / difdegn(p1, p2) / difdeg2n(p1, p2) / difrad2n(p1, p2) / deg_midp(x1, x0) / rad_midp(x1, x0) -> f64` | `swe_degnorm` / `swe_radnorm` / `swe_difdegn` / `swe_difdeg2n` / `swe_difrad2n` / `swe_deg_midp` / `swe_rad_midp` | any finite or non-finite input propagates (plain arithmetic, no validation needed) | deg / rad per function; `difdegn` folds into [0, 360), `difdeg2n` to ±180, `difrad2n` to ±π; midpoints cross the seam the short way | value | infallible; poisoned lock degrades to the unlocked pure call | value | range/seam assertions + C cross-check; rustdoc + doctests |
| `d2l(x: f64) -> Result<i32, Error>` | `int32 swe_d2l(double)` | finite magnitude below `2_147_483_647.5`; halves away from zero; the positive rounding intermediate must fit `i32` for both signs | n/a | value | out-of-domain → `InvalidInput` pre-call | value | rounding + edge + rejection + C cross-check; rustdoc + doctest |

## Formatting, split degrees and file provenance

| Rust signature | Native symbol | Args / domain / sentinels | Units / frame / scale | Flags / components | Failure / warning / no-event | Ownership / errors | Tests / docs |
|---|---|---|---|---|---|---|---|
| `csnorm(p: i32) -> i32` | `centisec swe_csnorm(centisec)` | any `i32`; folds into [0, 129600000) | centiseconds (1° = 360000; full turn 129600000) | value | infallible; poisoned lock degrades to the unlocked pure call | value | range asserts + C cross-check; rustdoc + doctest |
| `difcsn(p1: i32, p2: i32) / difcs2n(p1: i32, p2: i32) -> Result<i32, Error>` | `swe_difcsn` / `swe_difcs2n` | pairs whose `p1 - p2` fits `i32` ; `difcsn` folds into [0, 360°), `difcs2n` to ±180° with an exact half-turn reporting −64800000 | centiseconds | value | overflow → `InvalidInput` pre-call | value | separation pair + half-turn + rejection + C cross-check; rustdoc + doctests |
| `csroundsec(x: i32) -> Result<i32, Error>` | `centisec swe_csroundsec(centisec)` | inputs must lie within `i32::MIN + 100..=i32::MAX - 100` ; result is a multiple of 100; values just below a 30° sector boundary round down instead of crossing it | centiseconds; 100 = 1 arcsecond | value | near-extreme → `InvalidInput` pre-call | value | rounding + boundary + rejection + C cross-check; rustdoc + doctest |
| `cs2timestr(cs: i32, sep: u8, suppress_zero: bool) -> Result<String, Error>` | `char *swe_cs2timestr(CSEC, int, AS_BOOL, char *)` | time in centiseconds (1 h = 360000); `sep` is the field-separator byte; `suppress_zero` drops a zero seconds field; a rounding carry out of the day wraps to midnight; negative inputs render engine-defined (normalize first, e.g. with `csnorm`) | `HH<sep>MM<sep>SS` hours | string | NUL separator or absolute input plus 50 overflowing `i32` → `InvalidInput` pre-call | owned `String`; bounded 256-byte caller buffer | formats + suppression + carry-wrap + NUL rejection + C cross-check; rustdoc + doctest |
| `cs2lonlatstr(cs: i32, plus: u8, minus: u8) -> Result<String, Error>` | `char *swe_cs2lonlatstr(CSEC, char, char, char *)` | absolute value plus 50 must fit `i32` (negation and rounding overflow guards); non-negative values (zero included) take the `plus` branch, negatives the `minus` branch with the absolute value; whole minutes render `{deg}{dir}{min:02d}`, nonzero rounded seconds append `'{sec:02d}` | degrees + direction character | string | NUL direction or unrepresentable rounding intermediate → `InvalidInput` pre-call | owned `String`; same buffer contract | direction selection + zero/tiny + NUL/MIN rejection + C cross-check; rustdoc + doctest |
| `cs2degstr(cs: i32) -> Result<String, Error>` | `char *swe_cs2degstr(CSEC, char *)` | `i32::MIN` rejected ; other `i32` values format; seconds truncated to whole arcseconds; UTF-8 degree sign (U+00B0) | `{deg}°{min:02d}'{sec:02d}` | string | MIN → `InvalidInput` pre-call | owned `String`; same buffer contract | truncation pair + negative + MIN rejection + C cross-check; rustdoc + doctest |
| `split_deg(degree: f64, roundflag: i32) -> Result<SplitDeg, Error>` with `SplitDeg { deg, min, sec, secfr, sign }` | `void swe_split_deg(double, int32, int32 *, int32 *, int32 *, double *, int32 *)` | finite `degree` within ±2147483647.0 ; `roundflag` combines `SPLIT_DEG_*`; unknown bits pass through engine-defined; negatives split the absolute value with `sign = −1` except the zodiacal form (segment index) and the nakshatra form (signed split); a full turn reports segment 0 | degrees / arcminutes / arcseconds; `secfr` is fractional arcseconds (rounded whole seconds when rounding); `sign` is +1/−1 or the zodiac/nakshatra segment index | five components | out-of-domain → `InvalidInput` pre-call (the native decomposition has no range check) | owned struct | plain/zodiacal/rounding/keep-guard/nakshatra + rejection + C cross-check (nakshatra `secfr` bitwise); rustdoc + doctest |
| `get_current_file_data(ifno: i32) -> Result<Option<CurrentFileData>, Error>` with `CurrentFileData { path, tfstart, tfend, denum }` | `const char *swe_get_current_file_data(int, double *, double *, int *)` | `0` = planet file, `1` = Moon file; any other integer passes through engine-defined; the report reflects the files the engine has actually opened | file path + covered Julian-Day range + JPL ephemeris number | `Some` = file backing the slot, `None` = no file | `None` is data (absence), never an error; out parameters untouched on null | owned struct; native static text copied before the lock is released | file-based open (planet + Moon identity, range, denum 441) + empty slots + repeat + restore in `tests/format.rs`; rustdoc + doctest |
| `SPLIT_DEG_ROUND_SEC/MIN/DEG/ZODIACAL/NAKSHATRA/KEEP_SIGN/KEEP_DEG` (1/2/4/8/1024/16/32), `ACRONYCHAL_RISING/SETTING` (5/6), `COSMICAL_SETTING` (6), `AUNIT_TO_KM` (149597870.700) / `AUNIT_TO_LIGHTYEAR` (1/63241.07708427) / `AUNIT_TO_PARSEC` (1/206264.8062471), `COMET_OFFSET` (1000), `FICT_MAX` (999), `FICT_OFFSET_1` (39), `DE_NUMBER` (431), `FLG_CENTER_BODY` (1048576), `ASC/MC/ARMC/VERTEX/EQUASC/COASC1/COASC2` (0-6), `ASTNAMFILE` (`seasnam.txt`) / `FICTFILE` (`seorbel.txt`) | pinned `swephexp.h`/`sweph.h` value block | exact native values, verified against the pinned headers | per constant | n/a | n/a | values | value asserts in `tests/format.rs` |
## Atomic configured operations

`SessionBuilder`/`Session` (`src/session.rs`) own a validated
configuration and every `Session` computation applies it and runs one
dependent computation in a single acquisition of the same
process-wide lock the free functions use. Canonical free functions are
preserved with the global-state semantics above; they remain
non-atomic across `set_*` + compute pairs by design. Configuration-independent
checks run before setters; dependent calendar/shift checks run after applying
settings under the lock. `InvalidInput` guarantees that the dependent native
computation was not invoked, not rollback of applied global settings. Rejection
preserves the previous owned file snapshot even when settings remain installed.

| Rust signature | Native coverage | Args / domain / sentinels | Configuration applied | Notes |
|---|---|---|---|---|
| `SessionBuilder::new()`, `ephe_path`, `default_ephe_path`, `jpl_file`, `close_jpl_file`, `topo`, `sid_mode`, `tid_acc`, `delta_t_override`, `clear_delta_t_override`, `lapse_rate`, `build() -> Result<Session, Error>` | all seven `swe_set_*` setters (path/JPL/topo/sidereal/tidal/Delta-T/lapse) | unset = inherit process-global state; `default_ephe_path` = null reset; `close_jpl_file` = empty-string close; `clear_delta_t_override` = automatic sentinel; NUL/overlong/non-finite rejected pre-build | build does not install session configuration; UT reference preflight holds native access and restores temporary Delta-T overrides | known explicit/inherited Delta-T shifts checked at build; computed model/data changes checked with complete configuration at dependent call time; no getters are invented |
| `Session::calc_ut/calc/houses/houses_ex/get_ayanamsa_ut/deltat/rise_trans` | the matching `swe_*` entry point per method, via already-locked helpers shared with the free functions | configuration-independent checks pre-lock, dependent calendar/shift checks after applying settings (`Session::` prefix in pre-lock diagnostics) | the full owned configuration, applied first under the same acquisition; a `rise_trans` search then runs with its own per-call observer, which becomes the global observer afterwards (same side effect as the free function) | owned file slots 0..=4 are captured after each native computation attempt before releasing the same lock; `close()` cannot interleave mid-session; same-thread nesting is rejected with `InvalidInput`; no `Drop` close, no user callbacks under the lock, no unsafe `Send`/`Sync` impls |
| `Session::get_current_file_data(ifno: i32) -> Result<Option<CurrentFileData>, Error>` | owned snapshot of `swe_get_current_file_data` slots 0..=4 | before first computation, empty/out-of-range slot → `None`; rejected inputs preserve the snapshot; a native failure replaces it | none: querying does not apply setters or read live native files | snapshots survive other sessions, free calls and `close()`; clones share the latest completed computation history; returned source flags remain the authority for fallback |

Tests: `tests/sessions.rs` — interleaved topo A/B and
sidereal A/B sessions reproduce their own in-memory serial baselines
bitwise (200 samples per configuration, barrier-synchronized start);
the session event search agrees bitwise with the serial free call and
its global-observer side effect lands deterministically; rejected
builders/pre-lock rejections preserve settings, while dependent rejection retains
applied settings and preserves history; session calls recover after `close()`;
mixed free + session work across 8 threads completes without
deadlocking and sequential session reuse repeats bitwise.


## Observer side effects

Rise/transit searches, heliacal calls, observer-taking local eclipse calls and
local occultation searches install their observer into shared native state.
Later topocentric calls read it until another observer is installed.
`lun_eclipse_how(None)` leaves it alone. Coordinate/refraction and house calls
have no corresponding observer-install side effect. Fully configure a session
when a computation must use its own settings under contention.

## Unbound capabilities and native limitations

- Advanced model configuration and nutation interpolation are not exposed:
  `swe_set_astro_models`, `swe_get_astro_models`, `swe_set_interpolate_nut`.
- `swe_heliacal_angle` and `swe_topo_arcus_visionis` are not bound.
  `swe_rise_transit` and `swe_set_timeout` are absent from the pinned public ABI.
- No native `get_orbital_elements_ut`, Delta-T-override getter or lapse-rate
  getter exists. No replacement solver or astronomical model is implemented.
- Acronychal heliacal types 5–6 are unsupported. Moshier barycentric and
  planet-centric failures remain native errors.
- Twelve-house results expose eight angles. Sunshine declination is supplied
  where required; the additional native output slot is not separately exposed.
  ARMC-based Gauquelin sectors have no dedicated constructor.
- Planet-centric `ECL_NUT` output depends on native calculation history.
  Exact matrices exclude those combinations and retain a stable no-speed
  neighbour. The standalone public-C repeatability probe documents this limit.

## Native build identity

The build selects nine upstream `SWEOBJ` units and eight required headers.
Objects, ABI checks, `libswisseph.a` and compiler/input metadata live in Cargo
`OUT_DIR`. Builds never update the submodule or download data. Packaging includes
native build inputs and notices, excluding upstream tests, tools, binaries and
ephemeris payloads.
