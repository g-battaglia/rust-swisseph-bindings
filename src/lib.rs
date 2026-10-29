//! Minimal Rust bindings to the original Swiss Ephemeris C library.
//!
//! All calculations are performed by the pinned native library (compiled
//! from the `swisseph/` submodule by `build.rs`); this crate contributes
//! ownership, error handling and serialized access, never astronomical
//! algorithms of its own.
//!
//! # Public API style
//!
//! The primary API uses [`SessionBuilder`]/[`Session`] for configured
//! calculations. [`global`] preserves legacy process-global operations;
//! [`native`] preserves native-only extensions and raw 1.x pure signatures.
//! Legacy extension arguments retain C ordering and scalar observer values.
//!
//! Configured free functions are available only through the declared extension
//! modules. The root contains Session operations, pure utilities and records.
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

mod advisory;
mod angles;
mod config;
mod constants;
mod domain;
mod error;
mod events;
mod ffi;
mod houses;
mod node_result;
mod observer;
mod positions;
mod pure_calendar;
mod session;
mod stars;
mod state;
mod time;
mod version;

pub mod global;
pub mod native;

pub use advisory::{Advisory, AdvisoryDetail};
pub use angles::{
    SplitDeg, cotrans, cotrans_sp, cs2degstr, cs2lonlatstr, cs2timestr, csnorm, csroundsec, d2l,
    deg_midp, degnorm, difcs2n, difcsn, difdeg2n, difdegn, difrad2n, rad_midp, radnorm, split_deg,
};
pub use config::Ayanamsha;
// Internal metadata storage uses the same owned native record; it is not a
// primary export. External metadata access is explicitly native-specific.
pub(crate) use config::CurrentFileData;
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
    LunarEclipseCircumstances, NodeCrossing, OccultationGeometry, SolarEclipseCircumstances,
    SolarEclipseGeometry, VisibilityLimit, VisibilityOutcome,
};
pub use houses::{GauquelinSector, GauquelinSectors, HousePosition, Houses, HousesWithSpeeds};
pub use node_result::NodesApsides;
pub use observer::{
    Celestial, DistanceExtremes, Horizontal, OrbitalElements, Phenomena, Refraction,
    RiseTransitOutcome,
};
pub use positions::Position;
pub use pure_calendar::{DateConversion, JulianDay, Weekday, date_conversion, day_of_week, julday};
pub use session::{Session, SessionBuilder};
pub use stars::{StarMagnitude, StarPosition};
pub use time::{
    Calendar, CalendarDate, DeltaT, EquationOfTime, JulDays, LocalTime, UtcDateTime, revjul,
    utc_time_zone,
};

// Private aliases keep the locked Session implementation independent of the
// extension re-export spelling. Complete buffers are public only in native.
pub(crate) use events::{
    HeliacalPhenomenaFull, OccultationGeometryFull, SolarEclipseGeometryFull, VisibilityFull,
};
