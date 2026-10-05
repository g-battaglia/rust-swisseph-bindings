# Native parity coverage manifest

Declaration accounting: 101 native symbols, 105 free functions, 242 constants.
Full profile executes every row below; `full --require-jpl` adds the
JPL-success block. No row is scientific acceptance. See `native-parity.md` for
projectors, safety exclusions and input identities.

| Public operation | Native symbol | Family | Projector | Executed evidence | Coverage |
| --- | --- | --- | --- | --- | --- |
| `azalt` | `swe_azalt` | observer | Vector | 3 forward outputs, both frames | Full profile |
| `azalt_rev` | `swe_azalt_rev` | observer | Vector | 2 reverse outputs, both frames | Full profile |
| `calc` | `swe_calc` | positions | Position | 32k matrix v2 + 8k sweep; all bodies/flags/frames; ECL_NUT pctr gap documented | Full profile |
| `calc_pctr` | `swe_calc_pctr` | positions | Position | Swiss/JPL planet-centric success; nutation-target history gap excluded, no-speed neighbour kept | Full profile |
| `calc_ut` | `swe_calc_ut` | positions | Position | 32k matrix v2 + 8k sweep; Swiss/Moshier/JPL(default/JPL-file) sources | Full profile |
| `close` | `swe_close` | config | Unit | explicit reset; post-close reuse compares | Full profile |
| `cotrans` | `swe_cotrans` | angles | Vector | rotations incl. seams/antipodes/signed zero | Full profile |
| `cotrans_sp` | `swe_cotrans_sp` | angles | Vector | all 6 speed components | Full profile |
| `cs2degstr` | `swe_cs2degstr` | angles | Text | degree formatter | Full profile |
| `cs2lonlatstr` | `swe_cs2lonlatstr` | angles | Text | direction bytes + lossy encoding | Full profile |
| `cs2timestr` | `swe_cs2timestr` | angles | Text | formatters + separators | Full profile |
| `csnorm` | `swe_csnorm` | angles | Integer | centisecond normalization | Full profile |
| `csroundsec` | `swe_csroundsec` | angles | Integer | rounding ties/carries | Full profile |
| `d2l` | `swe_d2l` | angles | Integer | ties + i32 endpoints | Full profile |
| `date_conversion` | `swe_date_conversion` | time | Scalar | normalization vs native-error routes | Full profile |
| `day_of_week` | `swe_day_of_week` | time | Integer | J2000 band + historical epochs | Full profile |
| `deg_midp` | `swe_deg_midp` | angles | PureScalar | midpoints | Full profile |
| `degnorm` | `swe_degnorm` | angles | PureScalar | seams + non-finite propagation | Full profile |
| `deltat` | `swe_deltat` | time | Scalar | historical/modern; manual/automatic tide; cleared override | Full profile |
| `deltat_ex` | `swe_deltat_ex` | time | ScalarDiagnostic | Swiss/Moshier/JPL sources; fallback diagnostics | Full profile |
| `difcs2n` | `swe_difcs2n` | angles | Integer | centisecond subtraction variant | Full profile |
| `difcsn` | `swe_difcsn` | angles | Integer | centisecond subtraction | Full profile |
| `difdeg2n` | `swe_difdeg2n` | angles | PureScalar | pairs incl. antipodes | Full profile |
| `difdegn` | `swe_difdegn` | angles | PureScalar | pairs incl. antipodes | Full profile |
| `difrad2n` | `swe_difrad2n` | angles | PureScalar | pairs incl. antipodes | Full profile |
| `fixstar` | `swe_fixstar` | stars | Star | traditional/nomenclature/number/wildcard; UT/TT/flags | Full profile |
| `fixstar2` | `swe_fixstar2` | stars | Star | extended positions; legacy preflight when catalog missing | Full profile |
| `fixstar2_mag` | `swe_fixstar2_mag` | stars | Magnitude | extended magnitudes; legacy preflight routing | Full profile |
| `fixstar2_ut` | `swe_fixstar2_ut` | stars | Star | extended UT; legacy preflight when catalog missing | Full profile |
| `fixstar_mag` | `swe_fixstar_mag` | stars | Magnitude | magnitudes incl. rewritten names | Full profile |
| `fixstar_ut` | `swe_fixstar_ut` | stars | Star | same via UT entry point | Full profile |
| `gauquelin_sector` | `swe_gauquelin_sector` | houses | ScalarDiagnostic | body + star paths with catalog semantics | Full profile |
| `get_ayanamsa` | `swe_get_ayanamsa` | config | Scalar | 47 modes + USER paths vs native | Full profile |
| `get_ayanamsa_ex` | `swe_get_ayanamsa_ex` | config | Ayanamsha | explicit-source ayanamsha vs native | Full profile |
| `get_ayanamsa_ex_ut` | `swe_get_ayanamsa_ex_ut` | config | Ayanamsha | explicit-source UT ayanamsha vs native | Full profile |
| `get_ayanamsa_name` | `swe_get_ayanamsa_name` | config | Text | 47 predefined + USER absent-name Native error | Full profile |
| `get_ayanamsa_ut` | `swe_get_ayanamsa_ut` | config | Scalar | 47 modes + USER paths vs native | Full profile |
| `get_current_file_data` | `swe_get_current_file_data` | config | File | 5 slots pre/post success/failure/reset/switch; Swiss+JPL DE numbers | Full profile |
| `get_orbital_elements` | `swe_get_orbital_elements` | observer | Orbital | all 50 slots incl. reserved tail + 17 getters | Full profile |
| `get_planet_name` | `swe_get_planet_name` | positions | Text | classical/fictitious/asteroid-alias/missing-id native verdicts | Full profile |
| `get_tid_acc` | `swe_get_tid_acc` | config | Scalar | explicit/automatic reads incl. DE441 -25.936 note | Full profile |
| `heliacal_pheno_ut` | `swe_heliacal_pheno_ut` | events | HeliacalPhenomena | 30-slot phenomena + 28 getters | Full profile |
| `heliacal_ut` | `swe_heliacal_ut` | events | Heliacal | 10-slot searches incl. failures | Full profile |
| `helio_cross` | `swe_helio_cross` | events | Crossing | heliocentric directions vs native | Full profile |
| `helio_cross_ut` | `swe_helio_cross_ut` | events | Crossing | same via UT entry point | Full profile |
| `house_name` | `swe_house_name` | houses | Text | admitted letters incl. lowercase/unknown native fallback | Full profile |
| `house_pos` | `swe_house_pos` | houses | ScalarDiagnostic | systems + polar/error routes | Full profile |
| `houses` | `swe_houses` | houses | Houses | all ASCII letters; hemispheres/equator/polar; seams incl. ±0.0 | Full profile |
| `houses_armc` | `swe_houses_armc` | houses | Houses | ARMC systems vs native | Full profile |
| `houses_armc_ex2` | `swe_houses_armc_ex2` | houses | HouseSpeeds | ARMC speeds incl. Sunshine slot rewrite | Full profile |
| `houses_ex` | `swe_houses_ex` | houses | Houses | same + sidereal + Sunshine declination input | Full profile |
| `houses_ex2` | `swe_houses_ex2` | houses | HouseSpeeds | speeds + ascendant/MC getters; polar Native errors | Full profile |
| `houses_gauquelin` | `swe_houses_ex2` | houses | Sectors | 36 sectors + 8 angles + all speeds | Full profile |
| `jdet_to_utc` | `swe_jdet_to_utc` | time | Utc | representable final-UT1 regression under -1e12 override | Full profile |
| `jdut1_to_utc` | `swe_jdut1_to_utc` | time | Utc | representable final-UT1 regression under -1e12 override | Full profile |
| `julday` | `swe_julday` | time | PureScalar | both calendars; BCE/year-zero/reform/leap/midnight/fractional | Full profile |
| `lat_to_lmt` | `swe_lat_to_lmt` | time | ScalarDiagnostic | reverse route vs native | Full profile |
| `library_path` | `swe_get_library_path` | version | Text | smoke/data-pilot/full/sweep; per-worker path + long/multibyte boundary (Session state slice) | Full profile |
| `lmt_to_lat` | `swe_lmt_to_lat` | time | ScalarDiagnostic | forward route vs native | Full profile |
| `lun_eclipse_how` | `swe_lun_eclipse_how` | events | Circumstances | lunar circumstances | Full profile |
| `lun_eclipse_when` | `swe_lun_eclipse_when` | events | GlobalEvent | global lunar incl. backward search | Full profile |
| `lun_eclipse_when_loc` | `swe_lun_eclipse_when_loc` | events | LocalEvent | local 30-slot | Full profile |
| `lun_occult_when_glob` | `swe_lun_occult_when_glob` | events | GlobalEvent | classic spelling vs search-control int | Full profile |
| `lun_occult_when_glob_with_options` | `swe_lun_occult_when_glob` | events | GlobalEvent | options spelling vs search-control int | Full profile |
| `lun_occult_when_loc` | `swe_lun_occult_when_loc` | events | LocalEvent | local occultation | Full profile |
| `lun_occult_when_loc_with_options` | `swe_lun_occult_when_loc` | events | LocalEvent | options local occultation | Full profile |
| `lun_occult_where` | `swe_lun_occult_where` | events | Geometry | occultation geometry | Full profile |
| `mooncross` | `swe_mooncross` | events | Crossing | ET routes + sentinels | Full profile |
| `mooncross_node` | `swe_mooncross_node` | events | NodeCrossing | node time + both coordinates | Full profile |
| `mooncross_node_ut` | `swe_mooncross_node_ut` | events | NodeCrossing | same via UT entry point | Full profile |
| `mooncross_ut` | `swe_mooncross_ut` | events | Crossing | same via UT entry point | Full profile |
| `nod_aps` | `swe_nod_aps` | observer | Nodes | four 6-slot vectors, focal-point option | Full profile |
| `nod_aps_ut` | `swe_nod_aps_ut` | observer | Nodes | same via UT entry point | Full profile |
| `orbit_max_min_true_distance` | `swe_orbit_max_min_true_distance` | observer | Distances | all 3 distance outputs | Full profile |
| `pheno` | `swe_pheno` | observer | Phenomena | all 20 slots + 5 getters | Full profile |
| `pheno_ut` | `swe_pheno_ut` | observer | Phenomena | same via UT entry point | Full profile |
| `rad_midp` | `swe_rad_midp` | angles | PureScalar | midpoints | Full profile |
| `radnorm` | `swe_radnorm` | angles | PureScalar | seams + non-finite propagation | Full profile |
| `refrac` | `swe_refrac` | observer | Scalar | non-finite contract; finite altitude/details | Full profile |
| `refrac_extended` | `swe_refrac_extended` | observer | Vector | 5-slot incl. dip + valid zeros | Full profile |
| `require_source_flags` | `none (Rust-only)` | positions | Unit | strict Swiss/Moshier/JPL unions on C-checked positions | Full profile |
| `revjul` | `swe_revjul` | time | Calendar | both calendars; negative/J2000/fractional JD | Full profile |
| `rise_trans` | `swe_rise_trans` | observer | Rise | rise/set/transits both hemispheres; Circumpolar absence; lapse/atmosphere sensitivity | Full profile |
| `rise_trans_true_hor` | `swe_rise_trans_true_hor` | observer | Rise | true-horizon route vs native | Full profile |
| `set_delta_t_userdef` | `swe_set_delta_t_userdef` | config | Unit | pin/clear/restore incl. USER-UT preflight | Full profile |
| `set_ephe_path` | `swe_set_ephe_path` | config | Unit | complete/empty/planet-only/moon-only; NUL/overlong/multibyte rejections | Full profile |
| `set_jpl_file` | `swe_set_jpl_file` | config | Unit | missing-file fallback; real de440.eph success; NUL/overlong rejections | Full profile |
| `set_lapse_rate` | `swe_set_lapse_rate` | observer | Unit | 0/0.0065/0.01 sensitivity vs C rise geometry; non-finite rejections | Full profile |
| `set_sid_mode` | `swe_set_sid_mode` | config | Unit | 47 modes + USER ET/UT epochs/offsets; NaN rejections | Full profile |
| `set_tid_acc` | `swe_set_tid_acc` | config | Unit | explicit/automatic tides; lazy-init first-write scripts | Full profile |
| `set_topo` | `swe_set_topo` | config | Unit | finite observers incl. 2467 m; NaN/height-boundary rejections | Full profile |
| `sidtime` | `swe_sidtime` | time | Scalar | range + explicit-frame agreement | Full profile |
| `sidtime0` | `swe_sidtime0` | time | Scalar | agreement with sidtime path | Full profile |
| `sol_eclipse_how` | `swe_sol_eclipse_how` | events | Circumstances | 20 attributes + circumstances | Full profile |
| `sol_eclipse_when_glob` | `swe_sol_eclipse_when_glob` | events | GlobalEvent | global geometry incl. type-zero absence | Full profile |
| `sol_eclipse_when_loc` | `swe_sol_eclipse_when_loc` | events | LocalSolar | local 27-slot incl. contacts | Full profile |
| `sol_eclipse_where` | `swe_sol_eclipse_where` | events | Geometry | geography + getters | Full profile |
| `solcross` | `swe_solcross` | events | Crossing | ET longitude seam + forward/backward + sentinels | Full profile |
| `solcross_ut` | `swe_solcross_ut` | events | Crossing | same via UT entry point | Full profile |
| `split_deg` | `swe_split_deg` | angles | Split | 5 outputs under rounding/zodiacal/nakshatra/keep flags | Full profile |
| `time_equ` | `swe_time_equ` | time | ScalarDiagnostic | scalar + complete diagnostic vs native | Full profile |
| `utc_time_zone` | `swe_utc_time_zone` | time | Utc | fractional ±zones with date/year rollover | Full profile |
| `utc_to_jd` | `swe_utc_to_jd` | time | JulDays | leap instants incl. 2016-12-31 60.5s; invalid non-leap 60.x; extreme seconds Rust-only rejections + native capacity sentinel | Full profile |
| `version` | `swe_version` | version | Text | smoke/data-pilot/full/sweep; 256-byte own-executable ABI text | Full profile |
| `vis_limit_mag` | `swe_vis_limit_mag` | events | Visibility | 8 slots; -2 BelowHorizon with all slots | Full profile |

## Constants

All 242 entries are retained in `tests/native_parity/constants.rs` and
`constants.inc`; expressions are compiled through `swephexp.h`. Three
HELFLAG vision aliases map to native result-marker macros. The below-horizon
marker `-2` is its documented native status contract with real BelowHorizon
outcome coverage (all eight outputs). Path-length limits are binding-only
contracts with NUL/overlong/multibyte rejection plus exact boundary success.
The 243-field constants/marker gate passes in every profile.

## Session / helper accounting

| Public operation | Native reference | Executed evidence | Coverage |
| --- | --- | --- | --- |
| `SessionBuilder::new` | Explicit public-ABI configuration script | valid A/B construction; no install side effect | Full profile |
| `SessionBuilder::ephe_path` | Explicit public-ABI configuration script | full/empty/default paths; PARTIAL masked inheritance | Full profile |
| `SessionBuilder::default_ephe_path` | Explicit public-ABI configuration script | default reset individually + combined; Moshier probes | Full profile |
| `SessionBuilder::jpl_file` | Explicit public-ABI configuration script | missing file; real de440.eph via SWISSEPH_JPL_DIR | Full profile |
| `SessionBuilder::close_jpl_file` | Explicit public-ABI configuration script | close individually + combined | Full profile |
| `SessionBuilder::topo` | Explicit public-ABI configuration script | Munich/Sydney + partial TOPO inheritance | Full profile |
| `SessionBuilder::sid_mode` | Explicit public-ABI configuration script | Lahiri/USER-ET/USER-UT/reference/offset/pinned-DeltaT | Full profile |
| `SessionBuilder::tid_acc` | Explicit public-ABI configuration script | explicit/automatic/first-write; inherited preflight restore | Full profile |
| `SessionBuilder::delta_t_override` | Explicit public-ABI configuration script | explicit overrides; USER-UT preflight | Full profile |
| `SessionBuilder::clear_delta_t_override` | Explicit public-ABI configuration script | clear individually + combined + preflight restore | Full profile |
| `SessionBuilder::lapse_rate` | Explicit public-ABI configuration script | 0/0.0065/0.01 sensitivity + atmosphere triple vs C | Full profile |
| `SessionBuilder::build` | Explicit public-ABI configuration script | new/Default/cloned; 36 safe rejections preserve globals | Full profile |
| `Session::calc` | Matching C operation + owned configuration/history | A/B/4+8 threads/NaN/history; A/B/A serial | Full profile |
| `Session::calc_ut` | Matching C operation + owned configuration/history | A/B/4+8 threads/NaN/history; A/B/A serial | Full profile |
| `Session::houses` | Matching C operation + owned configuration/history | A/B/threads/getters/NaN; polar Native errors | Full profile |
| `Session::houses_ex` | Matching C operation + owned configuration/history | A/B/threads/getters/NaN | Full profile |
| `Session::get_ayanamsa_ut` | Matching C operation + owned configuration/history | A/B/threads/NaN | Full profile |
| `Session::deltat` | Matching C operation + owned configuration/history | A/B/threads/NaN | Full profile |
| `Session::rise_trans` | Matching C operation + owned configuration/history | A/B/threads/NaN; Circumpolar absence | Full profile |
| `Session::get_current_file_data` | Matching C operation + owned configuration/history | 5 slots/threads/post-close/NaN/invalid; clone histories | Full profile |
