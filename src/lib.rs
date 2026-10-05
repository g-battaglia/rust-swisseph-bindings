//! Minimal Rust bindings to the original Swiss Ephemeris C library.
//!
//! All calculations are performed by the pinned native library (compiled
//! from the `swisseph/` submodule by `build.rs`); this crate contributes
//! ownership, error handling and serialized access, never astronomical
//! algorithms of its own.
//!
//! # Public API style
//!
//! The API consists of free functions with canonical operation names
//! (`calc_ut`, `julday`, `houses`, ...) and explicit arguments. Every
//! parameter — including the calendar system and the ephemeris flags — is
//! passed explicitly at each call site.
//!
//! Native configuration (data path, observer, sidereal mode) is
//! process-global inside the C library. Each function holds one private
//! process-wide lock across its complete native sequence, so any thread
//! may call any function without external synchronization. Free `set_*`
//! plus dependent computation pairs are two acquisitions and therefore not
//! atomic: [`Session`] applies an owned configuration and runs one
//! dependent computation in a single acquisition of the same lock.
//! A session also captures owned file metadata before releasing native
//! access, so later provenance queries survive other callers and resets.
//!
//! # Errors
//!
//! Fallible functions return `Result<_, Error>`. Native failures, invalid
//! Rust-side input and lock poisoning are distinct [`ErrorKind`] states,
//! and the native diagnostic text is preserved whenever the C call
//! provides one. A valid zero result is data, never an error.

mod angles;
mod config;
mod constants;
mod domain;
mod error;
mod events;
mod ffi;
mod houses;
mod observer;
mod positions;
mod session;
mod stars;
mod state;
mod time;
mod version;

pub use angles::{
    SplitDeg, cotrans, cotrans_sp, cs2degstr, cs2lonlatstr, cs2timestr, csnorm, csroundsec, d2l,
    deg_midp, degnorm, difcs2n, difcsn, difdeg2n, difdegn, difrad2n, rad_midp, radnorm, split_deg,
};
pub use config::{
    Ayanamsha, CurrentFileData, MAX_EPHE_PATH_LEN, MAX_JPL_FILE_LEN, close, get_ayanamsa,
    get_ayanamsa_ex, get_ayanamsa_ex_ut, get_ayanamsa_name, get_ayanamsa_ut, get_current_file_data,
    get_tid_acc, set_delta_t_userdef, set_ephe_path, set_jpl_file, set_sid_mode, set_tid_acc,
    set_topo,
};
pub use constants::{
    ACRONYCHAL_RISING, ACRONYCHAL_SETTING, ADMETOS, APOLLON, APP_TO_TRUE, ARMC, ASC, AST_OFFSET,
    ASTNAMFILE, AUNIT_TO_KM, AUNIT_TO_LIGHTYEAR, AUNIT_TO_PARSEC, BIT_ASTRO_TWILIGHT,
    BIT_CIVIL_TWILIGHT, BIT_DISC_BOTTOM, BIT_DISC_CENTER, BIT_FIXED_DISC_SIZE,
    BIT_FORCE_SLOW_METHOD, BIT_GEOCTR_NO_ECL_LAT, BIT_HINDU_RISING, BIT_NAUTIC_TWILIGHT,
    BIT_NO_REFRACTION, CALC_ITRANSIT, CALC_MTRANSIT, CALC_RISE, CALC_SET, CERES, CHIRON, COASC1,
    COASC2, COMET_OFFSET, COSMICAL_SETTING, CUPIDO, DE_NUMBER, ECL_1ST_VISIBLE, ECL_2ND_VISIBLE,
    ECL_3RD_VISIBLE, ECL_4TH_VISIBLE, ECL_ALLTYPES_LUNAR, ECL_ALLTYPES_SOLAR, ECL_ANNULAR,
    ECL_ANNULAR_TOTAL, ECL_CENTRAL, ECL_HYBRID, ECL_MAX_VISIBLE, ECL_NONCENTRAL, ECL_NUT,
    ECL_OCC_BEG_DAYLIGHT, ECL_OCC_END_DAYLIGHT, ECL_ONE_TRY, ECL_PARTBEG_VISIBLE,
    ECL_PARTEND_VISIBLE, ECL_PARTIAL, ECL_PENUMBBEG_VISIBLE, ECL_PENUMBEND_VISIBLE, ECL_PENUMBRAL,
    ECL_TOTAL, ECL_TOTBEG_VISIBLE, ECL_TOTEND_VISIBLE, ECL_VISIBLE, ECL2HOR, EQU2HOR, EQUASC,
    EVENING_FIRST, EVENING_LAST, FICT_MAX, FICT_OFFSET, FICT_OFFSET_1, FICTFILE, FIXSTAR,
    FLG_ASTROMETRIC, FLG_BARYCTR, FLG_CENTER_BODY, FLG_DEFAULTEPH, FLG_EQUATORIAL, FLG_HELCTR,
    FLG_ICRS, FLG_J2000, FLG_JPLEPH, FLG_MOSEPH, FLG_NOABERR, FLG_NOGDEFL, FLG_NONUT, FLG_RADIANS,
    FLG_SIDEREAL, FLG_SPEED, FLG_SPEED3, FLG_SWIEPH, FLG_TOPOCTR, FLG_TRUEPOS, FLG_XYZ, GREG_CAL,
    HADES, HARRINGTON, HELFLAG_AV, HELFLAG_AVKIND, HELFLAG_AVKIND_MIN7, HELFLAG_AVKIND_MIN9,
    HELFLAG_AVKIND_PTO, HELFLAG_AVKIND_VR, HELFLAG_BELOW_HORIZON, HELFLAG_HIGH_PRECISION,
    HELFLAG_LONG_SEARCH, HELFLAG_MIXED, HELFLAG_NO_DETAILS, HELFLAG_OPTICAL_PARAMS,
    HELFLAG_PHOTOPIC, HELFLAG_SCOTOPIC, HELFLAG_SEARCH_1_PERIOD, HELFLAG_VISLIM_DARK,
    HELFLAG_VISLIM_NOMOON, HELFLAG_VISLIM_PHOTOPIC, HELFLAG_VISLIM_SCOTOPIC, HELIACAL_RISING,
    HELIACAL_SETTING, HOR2ECL, HOR2EQU, INTP_APOG, INTP_PERG, ISIS, JUL_CAL, JUNO, JUPITER, KRONOS,
    MARS, MC, MEAN_APOG, MEAN_NODE, MERCURY, MIXEDOPIC_FLAG, MOON, MORNING_FIRST, MORNING_LAST,
    NEPTUNE, NEPTUNE_ADAMS, NEPTUNE_LEVERRIER, NIBIRU, NODBIT_FOPOINT, NODBIT_MEAN, NODBIT_OSCU,
    NODBIT_OSCU_BAR, NPLANETS, OSCU_APOG, PALLAS, PHOLUS, PHOTOPIC_FLAG, PLMOON_OFFSET, PLUTO,
    PLUTO_LOWELL, PLUTO_PICKERING, POSEIDON, PROSERPINA, SATURN, SCOTOPIC_FLAG,
    SE_DELTAT_AUTOMATIC, SE_TIDAL_26, SE_TIDAL_AUTOMATIC, SE_TIDAL_DE200, SE_TIDAL_DE403,
    SE_TIDAL_DE404, SE_TIDAL_DE405, SE_TIDAL_DE406, SE_TIDAL_DE421, SE_TIDAL_DE422, SE_TIDAL_DE430,
    SE_TIDAL_DE431, SE_TIDAL_DE441, SE_TIDAL_DEFAULT, SE_TIDAL_JPLEPH, SE_TIDAL_MOSEPH,
    SE_TIDAL_STEPHENSON_2016, SE_TIDAL_SWIEPH, SPLIT_DEG_KEEP_DEG, SPLIT_DEG_KEEP_SIGN,
    SPLIT_DEG_NAKSHATRA, SPLIT_DEG_ROUND_DEG, SPLIT_DEG_ROUND_MIN, SPLIT_DEG_ROUND_SEC,
    SPLIT_DEG_ZODIACAL, SUN, TJD_INVALID, TRUE_TO_APP, URANUS, VARUNA, VENUS, VERTEX, VESTA,
    VULCAN, VULKANUS, WALDEMATH, WHITE_MOON, ZEUS,
};
pub use constants::{
    EARTH, SIDM_ALDEBARAN_15TAU, SIDM_ARYABHATA, SIDM_ARYABHATA_522, SIDM_ARYABHATA_MSUN,
    SIDM_B1950, SIDM_BABYL_BRITTON, SIDM_BABYL_ETPSC, SIDM_BABYL_HUBER, SIDM_BABYL_KUGLER1,
    SIDM_BABYL_KUGLER2, SIDM_BABYL_KUGLER3, SIDM_DELUCE, SIDM_DJWHAL_KHUL, SIDM_FAGAN_BRADLEY,
    SIDM_GALALIGN_MARDYKS, SIDM_GALCENT_0SAG, SIDM_GALCENT_COCHRANE, SIDM_GALCENT_MULA_WILHELM,
    SIDM_GALCENT_RGILBRAND, SIDM_GALEQU_FIORENZA, SIDM_GALEQU_IAU1958, SIDM_GALEQU_MULA,
    SIDM_GALEQU_TRUE, SIDM_HIPPARCHOS, SIDM_J1900, SIDM_J2000, SIDM_JN_BHASIN, SIDM_KRISHNAMURTI,
    SIDM_KRISHNAMURTI_VP291, SIDM_LAHIRI, SIDM_LAHIRI_1940, SIDM_LAHIRI_ICRC, SIDM_LAHIRI_VP285,
    SIDM_RAMAN, SIDM_SASSANIAN, SIDM_SS_CITRA, SIDM_SS_REVATI, SIDM_SURYASIDDHANTA,
    SIDM_SURYASIDDHANTA_MSUN, SIDM_TRUE_CITRA, SIDM_TRUE_MULA, SIDM_TRUE_PUSHYA, SIDM_TRUE_REVATI,
    SIDM_TRUE_SHEORAN, SIDM_USER, SIDM_USHASHASHI, SIDM_VALENS_MOON, SIDM_YUKTESHWAR, TRUE_NODE,
};
pub use error::{Error, ErrorKind};
pub use events::{
    BelowHorizon, GlobalLunarEclipse, GlobalOccultation, GlobalSolarEclipse, HeliacalEvent,
    HeliacalPhenomena, LocalLunarEclipse, LocalOccultation, LocalSolarEclipse, LongitudeCrossing,
    LunarEclipseCircumstances, NodeCrossing, OccultSearchOptions, OccultationGeometry,
    SolarEclipseCircumstances, SolarEclipseGeometry, VisibilityLimit, VisibilityOutcome,
    heliacal_pheno_ut, heliacal_ut, helio_cross, helio_cross_ut, lun_eclipse_how, lun_eclipse_when,
    lun_eclipse_when_loc, lun_occult_when_glob, lun_occult_when_glob_with_options,
    lun_occult_when_loc, lun_occult_when_loc_with_options, lun_occult_where, mooncross,
    mooncross_node, mooncross_node_ut, mooncross_ut, sol_eclipse_how, sol_eclipse_when_glob,
    sol_eclipse_when_loc, sol_eclipse_where, solcross, solcross_ut, vis_limit_mag,
};
pub use houses::{
    GauquelinSector, GauquelinSectors, HousePosition, Houses, HousesWithSpeeds, gauquelin_sector,
    house_name, house_pos, houses, houses_armc, houses_armc_ex2, houses_ex, houses_ex2,
    houses_gauquelin,
};
pub use observer::{
    Celestial, DistanceExtremes, Horizontal, NodesApsides, OrbitalElements, Phenomena, Refraction,
    RiseTransitOutcome, azalt, azalt_rev, get_orbital_elements, nod_aps, nod_aps_ut,
    orbit_max_min_true_distance, pheno, pheno_ut, refrac, refrac_extended, rise_trans,
    rise_trans_true_hor, set_lapse_rate,
};
pub use positions::{Position, calc, calc_pctr, calc_ut, get_planet_name, require_source_flags};
pub use session::{Session, SessionBuilder};
pub use stars::{
    StarMagnitude, StarPosition, fixstar, fixstar_mag, fixstar_ut, fixstar2, fixstar2_mag,
    fixstar2_ut,
};
pub use time::{
    Calendar, CalendarDate, DeltaT, EquationOfTime, JulDays, LocalTime, UtcDateTime,
    date_conversion, day_of_week, deltat, deltat_ex, jdet_to_utc, jdut1_to_utc, julday, lat_to_lmt,
    lmt_to_lat, revjul, sidtime, sidtime0, time_equ, utc_time_zone, utc_to_jd,
};
pub use version::{library_path, version};
