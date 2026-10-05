//! Native identifiers and flag bits for the implemented families.
//!
//! Values are taken verbatim from the pinned `swisseph/swephexp.h`
//! declaration block. Public names use bare aliases of the native `SE_*`
//! identifiers; their units and meanings are documented on each constant.

/// Julian calendar flag for [`crate::julday`] and [`crate::revjul`].
pub const JUL_CAL: i32 = 0;
/// Gregorian (proleptic) calendar flag for [`crate::julday`] and [`crate::revjul`].
pub const GREG_CAL: i32 = 1;

// ---------------------------------------------------------------------------
// Body numbers (`ipl` for [`crate::calc_ut`] / [`crate::calc`])
// ---------------------------------------------------------------------------

/// Nutation/obliquity pseudo-body for [`crate::calc_ut`] / [`crate::calc`].
pub const ECL_NUT: i32 = -1;
/// Fixed-star pseudo-body identifier; star operations select targets by name.
pub const FIXSTAR: i32 = -10;
/// Sun.
pub const SUN: i32 = 0;
/// Moon.
pub const MOON: i32 = 1;
/// Mercury.
pub const MERCURY: i32 = 2;
/// Venus.
pub const VENUS: i32 = 3;
/// Mars.
pub const MARS: i32 = 4;
/// Jupiter.
pub const JUPITER: i32 = 5;
/// Saturn.
pub const SATURN: i32 = 6;
/// Uranus.
pub const URANUS: i32 = 7;
/// Neptune.
pub const NEPTUNE: i32 = 8;
/// Pluto.
pub const PLUTO: i32 = 9;
/// Mean lunar node.
pub const MEAN_NODE: i32 = 10;
/// True (osculating) lunar node.
pub const TRUE_NODE: i32 = 11;
/// Mean lunar apogee (Mean Lilith).
pub const MEAN_APOG: i32 = 12;
/// Osculating lunar apogee (True Lilith).
pub const OSCU_APOG: i32 = 13;
/// Earth.
pub const EARTH: i32 = 14;
/// Chiron.
pub const CHIRON: i32 = 15;
/// Pholus.
pub const PHOLUS: i32 = 16;
/// Ceres.
pub const CERES: i32 = 17;
/// Pallas.
pub const PALLAS: i32 = 18;
/// Juno.
pub const JUNO: i32 = 19;
/// Vesta.
pub const VESTA: i32 = 20;
/// Interpolated lunar apogee.
pub const INTP_APOG: i32 = 21;
/// Interpolated lunar perigee.
pub const INTP_PERG: i32 = 22;
/// Count of classical numbered bodies (0-22).
pub const NPLANETS: i32 = 23;
/// Offset added for planetary-moon body numbers.
pub const PLMOON_OFFSET: i32 = 9000;
/// Offset added for asteroid body numbers.
pub const AST_OFFSET: i32 = 10000;
/// Varuna (`AST_OFFSET + 20000`).
pub const VARUNA: i32 = AST_OFFSET + 20000;
/// First fictitious (Hamburg/Uranian plus related) body number.
pub const FICT_OFFSET: i32 = 40;
/// Cupido.
pub const CUPIDO: i32 = 40;
/// Hades.
pub const HADES: i32 = 41;
/// Zeus.
pub const ZEUS: i32 = 42;
/// Kronos.
pub const KRONOS: i32 = 43;
/// Apollon.
pub const APOLLON: i32 = 44;
/// Admetos.
pub const ADMETOS: i32 = 45;
/// Vulkanus.
pub const VULKANUS: i32 = 46;
/// Poseidon.
pub const POSEIDON: i32 = 47;
/// Isis.
pub const ISIS: i32 = 48;
/// Nibiru.
pub const NIBIRU: i32 = 49;
/// Harrington.
pub const HARRINGTON: i32 = 50;
/// Neptune-Leverrier.
pub const NEPTUNE_LEVERRIER: i32 = 51;
/// Neptune-Adams.
pub const NEPTUNE_ADAMS: i32 = 52;
/// Pluto-Lowell.
pub const PLUTO_LOWELL: i32 = 53;
/// Pluto-Pickering.
pub const PLUTO_PICKERING: i32 = 54;
/// Vulcan.
pub const VULCAN: i32 = 55;
/// White Moon (Selena).
pub const WHITE_MOON: i32 = 56;
/// Proserpina.
pub const PROSERPINA: i32 = 57;
/// Waldemath.
pub const WALDEMATH: i32 = 58;

// ---------------------------------------------------------------------------
// Calculation flags (`iflag`)
// ---------------------------------------------------------------------------

/// Use JPL ephemeris data when available.
pub const FLG_JPLEPH: i32 = 1;
/// Use Swiss ephemeris data files when available (default source).
pub const FLG_SWIEPH: i32 = 2;
/// Use the built-in Moshier analytical ephemeris (no data files needed).
pub const FLG_MOSEPH: i32 = 4;
/// Heliocentric instead of geocentric positions.
pub const FLG_HELCTR: i32 = 8;
/// True/geometric positions instead of apparent positions.
pub const FLG_TRUEPOS: i32 = 16;
/// J2000 equinox, no precession to date.
pub const FLG_J2000: i32 = 32;
/// Mean equinox of date, no nutation.
pub const FLG_NONUT: i32 = 64;
/// Speeds from three positions (slower and less precise than [`FLG_SPEED`]).
pub const FLG_SPEED3: i32 = 128;
/// High-precision speeds (daily rates in the last three components).
pub const FLG_SPEED: i32 = 256;
/// Disable gravitational light deflection.
pub const FLG_NOGDEFL: i32 = 512;
/// Disable annual aberration of light.
pub const FLG_NOABERR: i32 = 1024;
/// Astrometric positions: light-time applied, aberration and deflection off.
pub const FLG_ASTROMETRIC: i32 = FLG_NOABERR | FLG_NOGDEFL;
/// Equatorial (right ascension/declination) instead of ecliptic output.
pub const FLG_EQUATORIAL: i32 = 2 * 1024;
/// Cartesian (x, y, z) instead of polar (longitude, latitude, distance).
pub const FLG_XYZ: i32 = 4 * 1024;
/// Output angles in radians instead of degrees.
pub const FLG_RADIANS: i32 = 8 * 1024;
/// Barycentric positions.
pub const FLG_BARYCTR: i32 = 16 * 1024;
/// Topocentric positions (requires [`crate::set_topo`]).
pub const FLG_TOPOCTR: i32 = 32 * 1024;
/// Sidereal positions (requires [`crate::set_sid_mode`]).
pub const FLG_SIDEREAL: i32 = 64 * 1024;
/// ICRS reference frame.
pub const FLG_ICRS: i32 = 128 * 1024;
/// Default ephemeris source flag used when no source bit is set.
pub const FLG_DEFAULTEPH: i32 = FLG_SWIEPH;

// ---------------------------------------------------------------------------
// Sidereal (ayanamsha) modes for [`crate::set_sid_mode`]
// ---------------------------------------------------------------------------

/// Fagan/Bradley ayanamsha.
pub const SIDM_FAGAN_BRADLEY: i32 = 0;
/// Lahiri ayanamsha.
pub const SIDM_LAHIRI: i32 = 1;
/// DeLuce ayanamsha.
pub const SIDM_DELUCE: i32 = 2;
/// Raman ayanamsha.
pub const SIDM_RAMAN: i32 = 3;
/// Ushashashi ayanamsha.
pub const SIDM_USHASHASHI: i32 = 4;
/// Krishnamurti ayanamsha.
pub const SIDM_KRISHNAMURTI: i32 = 5;
/// Djwhal-Khul ayanamsha.
pub const SIDM_DJWHAL_KHUL: i32 = 6;
/// Yukteshwar ayanamsha.
pub const SIDM_YUKTESHWAR: i32 = 7;
/// Jn-Bhasin ayanamsha.
pub const SIDM_JN_BHASIN: i32 = 8;
/// Babylonian/Kugler 1 ayanamsha.
pub const SIDM_BABYL_KUGLER1: i32 = 9;
/// Babylonian/Kugler 2 ayanamsha.
pub const SIDM_BABYL_KUGLER2: i32 = 10;
/// Babylonian/Kugler 3 ayanamsha.
pub const SIDM_BABYL_KUGLER3: i32 = 11;
/// Babylonian/Huber ayanamsha.
pub const SIDM_BABYL_HUBER: i32 = 12;
/// Babylonian/Etpsc ayanamsha.
pub const SIDM_BABYL_ETPSC: i32 = 13;
/// Aldebaran at 15 Taurus ayanamsha.
pub const SIDM_ALDEBARAN_15TAU: i32 = 14;
/// Hipparchos ayanamsha.
pub const SIDM_HIPPARCHOS: i32 = 15;
/// Sassanian ayanamsha.
pub const SIDM_SASSANIAN: i32 = 16;
/// Galactic center at 0 Sagittarius ayanamsha.
pub const SIDM_GALCENT_0SAG: i32 = 17;
/// J2000 fixed-epoch frame.
pub const SIDM_J2000: i32 = 18;
/// J1900 fixed-epoch frame.
pub const SIDM_J1900: i32 = 19;
/// B1950 fixed-epoch frame.
pub const SIDM_B1950: i32 = 20;
/// Suryasiddhanta ayanamsha.
pub const SIDM_SURYASIDDHANTA: i32 = 21;
/// Suryasiddhanta, mean Sun ayanamsha.
pub const SIDM_SURYASIDDHANTA_MSUN: i32 = 22;
/// Aryabhata ayanamsha.
pub const SIDM_ARYABHATA: i32 = 23;
/// Aryabhata, mean Sun ayanamsha.
pub const SIDM_ARYABHATA_MSUN: i32 = 24;
/// Suryasiddhanta/Revati ayanamsha.
pub const SIDM_SS_REVATI: i32 = 25;
/// Suryasiddhanta/Citra ayanamsha.
pub const SIDM_SS_CITRA: i32 = 26;
/// True Citra ayanamsha.
pub const SIDM_TRUE_CITRA: i32 = 27;
/// True Revati ayanamsha.
pub const SIDM_TRUE_REVATI: i32 = 28;
/// True Pushya ayanamsha.
pub const SIDM_TRUE_PUSHYA: i32 = 29;
/// Galactic center (Gilbrand) ayanamsha.
pub const SIDM_GALCENT_RGILBRAND: i32 = 30;
/// Galactic equator IAU 1958 ayanamsha.
pub const SIDM_GALEQU_IAU1958: i32 = 31;
/// Galactic equator (true) ayanamsha.
pub const SIDM_GALEQU_TRUE: i32 = 32;
/// Galactic equator (Mula) ayanamsha.
pub const SIDM_GALEQU_MULA: i32 = 33;
/// Galactic alignment (Mardyks) ayanamsha.
pub const SIDM_GALALIGN_MARDYKS: i32 = 34;
/// True Mula ayanamsha.
pub const SIDM_TRUE_MULA: i32 = 35;
/// Galactic center (Mula, Wilhelm) ayanamsha.
pub const SIDM_GALCENT_MULA_WILHELM: i32 = 36;
/// Aryabhata 522 ayanamsha.
pub const SIDM_ARYABHATA_522: i32 = 37;
/// Babylonian/Britton ayanamsha.
pub const SIDM_BABYL_BRITTON: i32 = 38;
/// True Sheoran ayanamsha.
pub const SIDM_TRUE_SHEORAN: i32 = 39;
/// Galactic center (Cochrane) ayanamsha.
pub const SIDM_GALCENT_COCHRANE: i32 = 40;
/// Galactic equator (Fiorenza) ayanamsha.
pub const SIDM_GALEQU_FIORENZA: i32 = 41;
/// Valens Moon ayanamsha.
pub const SIDM_VALENS_MOON: i32 = 42;
/// Lahiri 1940 ayanamsha.
pub const SIDM_LAHIRI_1940: i32 = 43;
/// Lahiri VP285 ayanamsha.
pub const SIDM_LAHIRI_VP285: i32 = 44;
/// Krishnamurti VP291 ayanamsha.
pub const SIDM_KRISHNAMURTI_VP291: i32 = 45;
/// Lahiri ICRC ayanamsha.
pub const SIDM_LAHIRI_ICRC: i32 = 46;
/// User-defined ayanamsha (`t0` is Terrestrial Time; see [`crate::set_sid_mode`]).
pub const SIDM_USER: i32 = 255;

// ---------------------------------------------------------------------------
// Tidal acceleration (`arcsec/cy^2`) for [`crate::set_tid_acc`] and the
// Delta-T override sentinel for [`crate::set_delta_t_userdef`]
// ---------------------------------------------------------------------------

/// Tidal acceleration of the DE200 ephemeris (-23.8946 arcsec/cy²).
pub const SE_TIDAL_DE200: f64 = -23.8946;
/// Tidal acceleration of the DE403 ephemeris (-25.580 arcsec/cy²).
pub const SE_TIDAL_DE403: f64 = -25.580;
/// Tidal acceleration of the DE404 ephemeris (-25.580 arcsec/cy²).
pub const SE_TIDAL_DE404: f64 = -25.580;
/// Tidal acceleration of the DE405 ephemeris (-25.826 arcsec/cy²).
pub const SE_TIDAL_DE405: f64 = -25.826;
/// Tidal acceleration of the DE406 ephemeris (-25.826 arcsec/cy²).
pub const SE_TIDAL_DE406: f64 = -25.826;
/// Tidal acceleration of the DE421 ephemeris (-25.85 arcsec/cy²).
pub const SE_TIDAL_DE421: f64 = -25.85;
/// Tidal acceleration of the DE422 ephemeris (-25.85 arcsec/cy²).
pub const SE_TIDAL_DE422: f64 = -25.85;
/// Tidal acceleration of the DE430 ephemeris (-25.82 arcsec/cy²).
pub const SE_TIDAL_DE430: f64 = -25.82;
/// Tidal acceleration of the DE431 ephemeris (-25.80 arcsec/cy²).
pub const SE_TIDAL_DE431: f64 = -25.80;
/// Tidal acceleration of the DE441 ephemeris (-25.936 arcsec/cy²).
pub const SE_TIDAL_DE441: f64 = -25.936;
/// Tidal acceleration -26.0 arcsec/cy² (Stephenson-style round value).
pub const SE_TIDAL_26: f64 = -26.0;
/// Tidal acceleration of Stephenson et al. 2016 (-25.85 arcsec/cy²).
pub const SE_TIDAL_STEPHENSON_2016: f64 = -25.85;
/// Default tidal acceleration (currently the DE431 value).
pub const SE_TIDAL_DEFAULT: f64 = SE_TIDAL_DE431;
/// Sentinel restoring automatic tidal acceleration in [`crate::set_tid_acc`].
pub const SE_TIDAL_AUTOMATIC: f64 = 999999.0;
/// Tidal acceleration assumed by the Moshier analytical model.
pub const SE_TIDAL_MOSEPH: f64 = SE_TIDAL_DE404;
/// Tidal acceleration assumed by the Swiss data files.
pub const SE_TIDAL_SWIEPH: f64 = SE_TIDAL_DEFAULT;
/// Tidal acceleration assumed by JPL data files.
pub const SE_TIDAL_JPLEPH: f64 = SE_TIDAL_DEFAULT;
/// Sentinel clearing the user Delta-T override in
/// [`crate::set_delta_t_userdef`] (`None` sends this value natively).
pub const SE_DELTAT_AUTOMATIC: f64 = -1e-10;

// ---------------------------------------------------------------------------
// Node/apse methods (`method` for [`crate::nod_aps`] / [`crate::nod_aps_ut`])
// ---------------------------------------------------------------------------

/// Mean nodes/apsides where defined (Moon, Mercury-Neptune); osculating
/// elsewhere. Also selected when `method` is 0.
pub const NODBIT_MEAN: i32 = 1;
/// Osculating nodes/apsides for every body.
pub const NODBIT_OSCU: i32 = 2;
/// Osculating nodes/apsides, barycentric ellipse beyond Jupiter.
pub const NODBIT_OSCU_BAR: i32 = 4;
/// Return the second focal point of the orbital ellipse in the aphelion
/// slot; combinable with any other method bit.
pub const NODBIT_FOPOINT: i32 = 256;

// ---------------------------------------------------------------------------
// Rise/set/transit selectors (`rsmi` for [`crate::rise_trans`] /
// [`crate::rise_trans_true_hor`])
// ---------------------------------------------------------------------------

/// Rising event selector.
pub const CALC_RISE: i32 = 1;
/// Setting event selector.
pub const CALC_SET: i32 = 2;
/// Upper meridian transit (culmination) selector.
pub const CALC_MTRANSIT: i32 = 4;
/// Lower meridian transit (anti-culmination) selector.
pub const CALC_ITRANSIT: i32 = 8;
/// Use the geocentric position projected on the ecliptic (latitude
/// ignored) instead of the topocentric place; part of Hindu rising.
pub const BIT_GEOCTR_NO_ECL_LAT: i32 = 128;
/// Rise/set of the disc center instead of the upper limb; part of Hindu rising.
pub const BIT_DISC_CENTER: i32 = 256;
/// Ignore atmospheric refraction for rise/set.
pub const BIT_NO_REFRACTION: i32 = 512;
/// Civil twilight (Sun at -6°) instead of rise/set.
pub const BIT_CIVIL_TWILIGHT: i32 = 1024;
/// Nautical twilight (Sun at -12°) instead of rise/set.
pub const BIT_NAUTIC_TWILIGHT: i32 = 2048;
/// Astronomical twilight (Sun at -18°) instead of rise/set.
pub const BIT_ASTRO_TWILIGHT: i32 = 4096;
/// Rise/set of the lower limb of the disc.
pub const BIT_DISC_BOTTOM: i32 = 8192;
/// Neglect the effect of distance on the apparent disc size.
pub const BIT_FIXED_DISC_SIZE: i32 = 16384;
/// Astrodienst in-house rise/set search variant.
pub const BIT_FORCE_SLOW_METHOD: i32 = 32768;
/// Hindu rising convention (disc center, no refraction, geocentric
/// ecliptic projection).
pub const BIT_HINDU_RISING: i32 = BIT_DISC_CENTER | BIT_NO_REFRACTION | BIT_GEOCTR_NO_ECL_LAT;

// ---------------------------------------------------------------------------
// Horizontal-coordinate and refraction selectors (`calc_flag` for
// [`crate::azalt`] / [`crate::azalt_rev`] and [`crate::refrac`] /
// [`crate::refrac_extended`])
// ---------------------------------------------------------------------------

/// `azalt` input frame: ecliptic longitude/latitude in degrees.
pub const ECL2HOR: i32 = 0;
/// `azalt` input frame: equatorial right ascension/declination in degrees.
pub const EQU2HOR: i32 = 1;
/// `azalt_rev` output frame: ecliptic longitude/latitude in degrees.
pub const HOR2ECL: i32 = 0;
/// `azalt_rev` output frame: equatorial right ascension/declination in degrees.
pub const HOR2EQU: i32 = 1;
/// Refraction direction: true (geometric) altitude to apparent altitude.
pub const TRUE_TO_APP: i32 = 0;
/// Refraction direction: apparent altitude to true (geometric) altitude.
pub const APP_TO_TRUE: i32 = 1;

// ---------------------------------------------------------------------------
// Eclipse types and visibility bits (`eclipse_type` / returned type for
// [`crate::sol_eclipse_when_glob`] / [`crate::lun_eclipse_when`] and the
// circumstance calls)
// ---------------------------------------------------------------------------

/// Eclipse whose shadow axis crosses the Earth (central line exists).
pub const ECL_CENTRAL: i32 = 1;
/// Eclipse whose shadow axis misses the Earth (no central line).
pub const ECL_NONCENTRAL: i32 = 2;
/// Total eclipse phase/type.
pub const ECL_TOTAL: i32 = 4;
/// Annular eclipse phase/type.
pub const ECL_ANNULAR: i32 = 8;
/// Partial eclipse phase/type.
pub const ECL_PARTIAL: i32 = 16;
/// Hybrid (annular-total) eclipse type.
pub const ECL_ANNULAR_TOTAL: i32 = 32;
/// Hybrid (annular-total) eclipse type; alias of [`ECL_ANNULAR_TOTAL`].
pub const ECL_HYBRID: i32 = 32;
/// Penumbral lunar eclipse type.
pub const ECL_PENUMBRAL: i32 = 64;
/// Acceptance mask for every solar eclipse type.
pub const ECL_ALLTYPES_SOLAR: i32 =
    ECL_CENTRAL | ECL_NONCENTRAL | ECL_TOTAL | ECL_ANNULAR | ECL_PARTIAL | ECL_ANNULAR_TOTAL;
/// Acceptance mask for every lunar eclipse type.
pub const ECL_ALLTYPES_LUNAR: i32 = ECL_TOTAL | ECL_PARTIAL | ECL_PENUMBRAL;
/// Part of the eclipse is visible above the local horizon.
pub const ECL_VISIBLE: i32 = 128;
/// Maximum eclipse phase visible above the local horizon.
pub const ECL_MAX_VISIBLE: i32 = 256;
/// Begin of the partial phase visible; alias [`ECL_PARTBEG_VISIBLE`].
pub const ECL_1ST_VISIBLE: i32 = 512;
/// Begin of the partial phase visible.
pub const ECL_PARTBEG_VISIBLE: i32 = 512;
/// Begin of the total phase visible; alias [`ECL_TOTBEG_VISIBLE`].
pub const ECL_2ND_VISIBLE: i32 = 1024;
/// Begin of the total phase visible.
pub const ECL_TOTBEG_VISIBLE: i32 = 1024;
/// End of the total phase visible; alias [`ECL_TOTEND_VISIBLE`].
pub const ECL_3RD_VISIBLE: i32 = 2048;
/// End of the total phase visible.
pub const ECL_TOTEND_VISIBLE: i32 = 2048;
/// End of the partial phase visible; alias [`ECL_PARTEND_VISIBLE`].
pub const ECL_4TH_VISIBLE: i32 = 4096;
/// End of the partial phase visible.
pub const ECL_PARTEND_VISIBLE: i32 = 4096;
/// Begin of the penumbral phase visible.
pub const ECL_PENUMBBEG_VISIBLE: i32 = 8192;
/// End of the penumbral phase visible.
pub const ECL_PENUMBEND_VISIBLE: i32 = 16384;
/// Occultation begins during daylight.
pub const ECL_OCC_BEG_DAYLIGHT: i32 = 8192;
/// Occultation ends during daylight.
pub const ECL_OCC_END_DAYLIGHT: i32 = 16384;
/// Search optimization hint for occultation scans: check only the next
/// conjunction instead of searching further.
pub const ECL_ONE_TRY: i32 = 32 * 1024;

// ---------------------------------------------------------------------------
// Heliacal event types (`event_type` for [`crate::heliacal_ut`] /
// [`crate::heliacal_pheno_ut`]) and visibility flags (`helflag`)
//
// The acronychal types 5 (`SE_ACRONYCHAL_RISING`) and 6
// (`SE_ACRONYCHAL_SETTING` / `SE_COSMICAL_SETTING`) are declared by the
// pinned header but marked "still not implemented" there: the engine
// rejects them natively, so they have no binding constants (an explicit
// capability gap, not a silent omission).
// ---------------------------------------------------------------------------

/// Heliacal rising: first morning visibility after invisibility.
pub const HELIACAL_RISING: i32 = 1;
/// Heliacal setting: last evening visibility before invisibility.
pub const HELIACAL_SETTING: i32 = 2;
/// First morning visibility; alias of [`HELIACAL_RISING`].
pub const MORNING_FIRST: i32 = 1;
/// Last evening visibility; alias of [`HELIACAL_SETTING`].
pub const EVENING_LAST: i32 = 2;
/// First evening visibility after superior conjunction (inner planets
/// Mercury/Venus and the Moon only; outer planets fail natively).
pub const EVENING_FIRST: i32 = 3;
/// Last morning visibility before superior conjunction (same domain as
/// [`EVENING_FIRST`]).
pub const MORNING_LAST: i32 = 4;
/// Extend the event search over a longer period.
pub const HELFLAG_LONG_SEARCH: i32 = 128;
/// Higher-precision (slower) heliacal search.
pub const HELFLAG_HIGH_PRECISION: i32 = 256;
/// Honor the optical-instrument entries of the observer block (magnification,
/// aperture, transmission) instead of naked-eye defaults.
pub const HELFLAG_OPTICAL_PARAMS: i32 = 512;
/// Skip detailed phenomenon computation in the search.
pub const HELFLAG_NO_DETAILS: i32 = 1024;
/// Restrict the search to one synodic period.
pub const HELFLAG_SEARCH_1_PERIOD: i32 = 2048;
/// Assume a dark sky (Sun at nadir) for the limiting magnitude.
pub const HELFLAG_VISLIM_DARK: i32 = 4096;
/// Exclude moonlight from the sky brightness.
pub const HELFLAG_VISLIM_NOMOON: i32 = 8192;
/// Force photopic (daylight) vision for the limiting magnitude.
pub const HELFLAG_VISLIM_PHOTOPIC: i32 = 16384;
/// Force scotopic (night) vision for the limiting magnitude.
pub const HELFLAG_VISLIM_SCOTOPIC: i32 = 32768;
/// Arcus-visionis search/phenomenon method (Victor Reijs VR criterion).
/// [`crate::heliacal_ut`] rejects all `AVKIND` search bits with
/// `InvalidInput` because of a pinned-native internal buffer defect.
/// Instant-based phenomenon calls retain the native flag semantics.
pub const HELFLAG_AV: i32 = 65536;
/// Arcus-visionis kind VR; alias of [`HELFLAG_AV`].
pub const HELFLAG_AVKIND_VR: i32 = 65536;
/// Arcus-visionis kind Ptolemy.
pub const HELFLAG_AVKIND_PTO: i32 = 131072;
/// Arcus-visionis kind Babylonian (minimum 7°).
pub const HELFLAG_AVKIND_MIN7: i32 = 262144;
/// Arcus-visionis kind Babylonian (minimum 9°).
pub const HELFLAG_AVKIND_MIN9: i32 = 524288;
/// Selection mask for every arcus-visionis kind bit.
/// These bits are unavailable in [`crate::heliacal_ut`]; see [`HELFLAG_AV`].
pub const HELFLAG_AVKIND: i32 =
    HELFLAG_AVKIND_VR | HELFLAG_AVKIND_PTO | HELFLAG_AVKIND_MIN7 | HELFLAG_AVKIND_MIN9;
/// [`crate::vis_limit_mag`] status: the object is below the local horizon
/// (a dedicated `Ok` outcome, never an error).
pub const HELFLAG_BELOW_HORIZON: i32 = -2;
/// [`crate::vis_limit_mag`] vision status: photopic (daylight) vision.
pub const HELFLAG_PHOTOPIC: i32 = 0;
/// [`crate::vis_limit_mag`] vision status: scotopic (night) vision.
pub const HELFLAG_SCOTOPIC: i32 = 1;
/// [`crate::vis_limit_mag`] vision status: near the photopic/scotopic
/// limit (bit 1 added to the photopic/scotopic bit).
pub const HELFLAG_MIXED: i32 = 2;
/// Photopic vision flag; equals [`HELFLAG_PHOTOPIC`].
pub const PHOTOPIC_FLAG: i32 = 0;
/// Scotopic vision flag; equals [`HELFLAG_SCOTOPIC`].
pub const SCOTOPIC_FLAG: i32 = 1;
/// Mixed photopic/scotopic vision flag; equals [`HELFLAG_MIXED`].
pub const MIXEDOPIC_FLAG: i32 = 2;
/// Invalid-time sentinel used by [`crate::heliacal_pheno_ut`] slot 15
/// (`TbYallop`) when the Yallop lunar-crescent timing does not apply
/// (non-lunar objects).
pub const TJD_INVALID: f64 = 99999999.0;

// ---------------------------------------------------------------------------
// Degree-splitting flags (`roundflag` for [`crate::split_deg`])
// ---------------------------------------------------------------------------

/// `split_deg` rounding: round to the nearest arcsecond.
pub const SPLIT_DEG_ROUND_SEC: i32 = 1;
/// `split_deg` rounding: round to the nearest arcminute.
pub const SPLIT_DEG_ROUND_MIN: i32 = 2;
/// `split_deg` rounding: round to the nearest degree.
pub const SPLIT_DEG_ROUND_DEG: i32 = 4;
/// `split_deg` layout: reduce into 30° zodiac segments (index 0-11).
pub const SPLIT_DEG_ZODIACAL: i32 = 8;
/// `split_deg` layout: reduce into nakshatra segments.
pub const SPLIT_DEG_NAKSHATRA: i32 = 1024;
/// `split_deg` guard: do not round across a sign/segment boundary.
pub const SPLIT_DEG_KEEP_SIGN: i32 = 16;
/// `split_deg` guard: do not round across a degree boundary.
pub const SPLIT_DEG_KEEP_DEG: i32 = 32;

// ---------------------------------------------------------------------------
// Heliacal event types declared but not implemented natively
// ---------------------------------------------------------------------------

/// Acronychal rising event type (5).
///
/// The pinned header marks this "still not implemented": the engine
/// rejects it natively (see [`crate::heliacal_ut`]). The constant is
/// bound so the declared-but-unavailable type is visible, not to enable
/// it.
pub const ACRONYCHAL_RISING: i32 = 5;
/// Acronychal setting event type (6).
///
/// Same unimplemented status as [`ACRONYCHAL_RISING`].
pub const ACRONYCHAL_SETTING: i32 = 6;
/// Cosmical setting event type (6); alias of [`ACRONYCHAL_SETTING`].
///
/// Same unimplemented status as [`ACRONYCHAL_RISING`].
pub const COSMICAL_SETTING: i32 = 6;

// ---------------------------------------------------------------------------
// Astronomical-unit conversions, body-number bounds, ephemeris number,
// center-of-body flag, house-angle indices and catalog file names
// ---------------------------------------------------------------------------

/// Astronomical unit in kilometers (exact IAU 2012 value).
pub const AUNIT_TO_KM: f64 = 149597870.700;
/// Astronomical units per light-year.
pub const AUNIT_TO_LIGHTYEAR: f64 = 1.5812507409819728e-05;
/// Astronomical units per parsec.
pub const AUNIT_TO_PARSEC: f64 = 4.848136811095274e-06;
/// Offset added for comet body numbers.
pub const COMET_OFFSET: i32 = 1000;
/// Highest fictitious body number.
pub const FICT_MAX: i32 = 999;
/// First fictitious body-number block (`FICT_OFFSET` is its alias).
pub const FICT_OFFSET_1: i32 = 39;
/// Default JPL Development Ephemeris number.
pub const DE_NUMBER: i32 = 431;
/// Calculate the position of the center of the body instead of the photo
/// center (flag bit for [`crate::calc`] / [`crate::calc_ut`]).
pub const FLG_CENTER_BODY: i32 = 1024 * 1024;
/// Ascendant index into the [`crate::Houses`] angles array.
pub const ASC: i32 = 0;
/// Medium Coeli index into the [`crate::Houses`] angles array.
pub const MC: i32 = 1;
/// ARMC (sidereal time of the MC) index into the angles array.
pub const ARMC: i32 = 2;
/// Vertex index into the [`crate::Houses`] angles array.
pub const VERTEX: i32 = 3;
/// Equatorial ascendant index into the [`crate::Houses`] angles array.
pub const EQUASC: i32 = 4;
/// Koch co-ascendant index into the [`crate::Houses`] angles array.
pub const COASC1: i32 = 5;
/// Munkasey co-ascendant index into the [`crate::Houses`] angles array.
pub const COASC2: i32 = 6;
/// Asteroid name catalog file.
pub const ASTNAMFILE: &str = "seasnam.txt";
/// Fictitious-body orbital-element catalog file.
pub const FICTFILE: &str = "seorbel.txt";
