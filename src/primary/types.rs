//! Closed result records; no numerical computation is performed here.

/// Complete angle result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Angle {
    /// Angle in the producing operation's degrees or radians.
    pub value: f64,
}

/// Complete integer result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Integer {
    /// Native integer result, widened exactly; centisecond helpers retain centiseconds.
    pub value: i64,
}

/// Complete text result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    /// Owned native name or formatted text, without parsing.
    pub value: String,
}

/// Complete spherical result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Spherical {
    /// Longitude/RA degrees, latitude/declination degrees, and unchanged input radius.
    pub values: [f64; 3],
}

/// Complete spherical state result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct SphericalState {
    /// Rotated longitude/RA degrees, latitude/declination degrees, and radius.
    pub position: [f64; 3],
    /// Matching native angular/radial rates per day.
    pub velocity: [f64; 3],
}

/// Complete split deg result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct SplitDeg {
    /// Whole degrees, within the selected sign/segment when requested.
    pub degrees: i64,
    /// Whole arcminutes from the native splitter.
    pub minutes: i64,
    /// Whole arcseconds from the native splitter.
    pub seconds: i64,
    /// Fractional arcseconds, retained without rounding.
    pub fraction: f64,
    /// Native +/- sign or segment index, selected by the split flags.
    pub sign: i64,
}

/// Complete julian day result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct JulianDay {
    /// Julian Day in days; calendar conversion assigns no time scale.
    pub value: f64,
}

/// Complete calendar date result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct CalendarDate {
    /// Producer year; units and domain follow the operation.
    pub year: i32,
    /// Producer month; units and domain follow the operation.
    pub month: i32,
    /// Producer day; units and domain follow the operation.
    pub day: i32,
    /// Producer hour; units and domain follow the operation.
    pub hour: f64,
}

/// Complete date conversion result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct DateConversion {
    /// Actual acceptance of the original civil date label.
    pub valid: bool,
    /// Actual normalized Julian Day in days, without an assigned time scale.
    pub julian_day: f64,
    /// Producer date; units and domain follow the operation.
    pub date: CalendarDate,
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete weekday result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Weekday {
    /// Native weekday: Monday zero through Sunday six.
    pub index: i32,
}

/// Complete utc date time result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct UtcDateTime {
    /// Producer year; units and domain follow the operation.
    pub year: i32,
    /// Producer month; units and domain follow the operation.
    pub month: i32,
    /// Producer day; units and domain follow the operation.
    pub day: i32,
    /// Producer hour; units and domain follow the operation.
    pub hour: i32,
    /// Producer minute; units and domain follow the operation.
    pub minute: i32,
    /// Producer second; units and domain follow the operation.
    pub second: f64,
}

/// Complete jul days result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct JulDays {
    /// Terrestrial Time Julian Day in days.
    pub tt: f64,
    /// Universal Time Julian Day in days.
    pub ut1: f64,
}

/// Complete position result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    /// Longitude/latitude degrees, distance AU, then daily rates by default; flags select equatorial/XYZ/radian coordinates.
    pub values: [f64; 6],
    /// Actual flags returned by the native engine, without reinterpretation.
    pub returned_flags: i32,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: String,
    /// Structured producer advisories, empty when no channel exists.
    pub warnings: Vec<Advisory>,
    /// Separate producer messages, empty when no channel exists.
    pub messages: Vec<String>,
}

/// Complete houses result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Houses {
    /// Native cusp longitudes in degrees, in original house/sector order.
    pub cusps: [f64; 12],
    /// Eight native house angles in degrees, in original order.
    pub angles: [f64; 8],
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete houses with speeds result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct HousesWithSpeeds {
    /// Native cusp longitudes in degrees, in original house/sector order.
    pub cusps: [f64; 12],
    /// Eight native house angles in degrees, in original order.
    pub angles: [f64; 8],
    /// Native cusp rates in degrees per day.
    pub cusp_speeds: [f64; 12],
    /// Native angle rates in degrees per day.
    pub angle_speeds: [f64; 8],
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete gauquelin sectors result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct GauquelinSectors {
    /// All 36 native sector cusp longitudes in degrees.
    pub sectors: [f64; 36],
    /// Eight native house angles in degrees, in original order.
    pub angles: [f64; 8],
    /// Native sector cusp rates in degrees per day.
    pub sector_speeds: [f64; 36],
    /// Native angle rates in degrees per day.
    pub angle_speeds: [f64; 8],
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete ayanamsha result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Ayanamsha {
    /// Producer value; units and domain follow the operation.
    pub value: f64,
    /// Actual flags returned by the native engine, without reinterpretation.
    pub returned_flags: i32,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: String,
}

/// Complete house position result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct HousePosition {
    /// Fractional house/sector number; the native zero failure sentinel is preserved as data.
    pub value: f64,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete gauquelin sector result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct GauquelinSector {
    /// Native fractional Gauquelin sector number.
    pub value: f64,
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete star position result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct StarPosition {
    /// Producer values; units and domain follow the operation.
    pub values: [f64; 6],
    /// Canonical name returned by the producer, when provided.
    pub canonical_name: String,
    /// Actual flags returned by the native engine, without reinterpretation.
    pub returned_flags: i32,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete star magnitude result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct StarMagnitude {
    /// Static catalogue visual magnitude; this is not instantaneous brightness.
    pub magnitude: f64,
    /// Canonical name returned by the producer, when provided.
    pub canonical_name: Option<String>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete phenomena result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Phenomena {
    /// Producer values; units and domain follow the operation.
    pub values: [f64; 20],
    /// Actual flags returned by the native engine, without reinterpretation.
    pub returned_flags: i32,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: String,
}

/// Complete nodes apsides result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct NodesApsides {
    /// Producer ascending; units and domain follow the operation.
    pub ascending: [f64; 6],
    /// Producer descending; units and domain follow the operation.
    pub descending: [f64; 6],
    /// Producer perihelion; units and domain follow the operation.
    pub perihelion: [f64; 6],
    /// Producer aphelion; units and domain follow the operation.
    pub aphelion: [f64; 6],
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete orbital elements result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct OrbitalElements {
    /// Producer values; units and domain follow the operation.
    pub values: [f64; 50],
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete distance extremes result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct DistanceExtremes {
    /// Maximum true geocentric distance in AU.
    pub maximum: f64,
    /// Minimum true geocentric distance in AU.
    pub minimum: f64,
    /// Current true geocentric distance in AU.
    pub current: f64,
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete horizontal result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Horizontal {
    /// Native azimuth in degrees, south zero and west positive.
    pub azimuth: f64,
    /// Geometric altitude above the horizon in degrees.
    pub true_altitude: f64,
    /// Refracted apparent altitude above the horizon in degrees.
    pub apparent_altitude: f64,
}

/// Complete celestial result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Celestial {
    /// Native ecliptic longitude or right ascension in degrees.
    pub longitude_or_ra: f64,
    /// Native ecliptic latitude or declination in degrees.
    pub latitude_or_dec: f64,
}

/// Complete refraction result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Refraction {
    /// Native converted altitude in degrees.
    pub value: f64,
    /// True altitude, apparent altitude, refraction and horizon dip, all in degrees.
    pub details: [f64; 4],
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete delta t result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct DeltaT {
    /// Native TT-minus-UT time interval in days.
    pub days: f64,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete equation of time result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct EquationOfTime {
    /// Apparent-minus-mean solar time in days.
    pub days: f64,
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete local time result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalTime {
    /// Converted local apparent/mean Julian Day in days.
    pub julian_day: f64,
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete sidereal time result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct SiderealTime {
    /// Sidereal time in hours.
    pub hours: f64,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete longitude crossing result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct LongitudeCrossing {
    /// Native crossing Julian Day in the requested TT/UT time scale.
    pub time: f64,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: String,
}

/// Complete node crossing result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeCrossing {
    /// Native node-crossing Julian Day in the requested TT/UT scale.
    pub time: f64,
    /// Native ecliptic crossing longitude in degrees.
    pub longitude: f64,
    /// Native ecliptic crossing latitude in degrees.
    pub latitude: f64,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: String,
}

/// Complete global solar eclipse result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct GlobalSolarEclipse {
    /// Actual native integer status; None only when no status channel exists.
    pub status: i32,
    /// Complete producer time buffer in Julian days; scale follows the operation.
    pub times: [f64; 10],
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete global lunar eclipse result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct GlobalLunarEclipse {
    /// Actual native integer status; None only when no status channel exists.
    pub status: i32,
    /// Complete producer time buffer in Julian days; scale follows the operation.
    pub times: [f64; 10],
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete solar eclipse geometry result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct SolarEclipseGeometry {
    /// Actual native integer status; None only when no status channel exists.
    pub status: i32,
    /// All ten native geographic output slots, without truncation.
    pub geography: [f64; 10],
    /// Complete native circumstance buffer in its original slot order.
    pub attributes: [f64; 20],
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete solar eclipse circumstances result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct SolarEclipseCircumstances {
    /// Actual native integer status; None only when no status channel exists.
    pub status: i32,
    /// Complete native circumstance buffer in its original slot order.
    pub attributes: [f64; 20],
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete lunar eclipse circumstances result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct LunarEclipseCircumstances {
    /// Actual native integer status; None only when no status channel exists.
    pub status: i32,
    /// Complete native circumstance buffer in its original slot order.
    pub attributes: [f64; 20],
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete local solar eclipse result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalSolarEclipse {
    /// Actual native integer status; None only when no status channel exists.
    pub status: i32,
    /// Complete producer time buffer in Julian days; scale follows the operation.
    pub times: LocalSolarTimes,
    /// Complete native circumstance buffer in its original slot order.
    pub attributes: [f64; 20],
    /// Producer maximum; units and domain follow the operation.
    pub maximum: Option<f64>,
    /// Producer first contact; units and domain follow the operation.
    pub first_contact: Option<f64>,
    /// Producer second contact; units and domain follow the operation.
    pub second_contact: Option<f64>,
    /// Producer third contact; units and domain follow the operation.
    pub third_contact: Option<f64>,
    /// Producer fourth contact; units and domain follow the operation.
    pub fourth_contact: Option<f64>,
    /// Producer sunrise; units and domain follow the operation.
    pub sunrise: Option<f64>,
    /// Producer sunset; units and domain follow the operation.
    pub sunset: Option<f64>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete local lunar eclipse result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalLunarEclipse {
    /// Actual native integer status; None only when no status channel exists.
    pub status: i32,
    /// Complete producer time buffer in Julian days; scale follows the operation.
    pub times: [f64; 10],
    /// Complete native circumstance buffer in its original slot order.
    pub attributes: [f64; 20],
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete global occultation result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct GlobalOccultation {
    /// Actual native integer status; None only when no status channel exists.
    pub status: i32,
    /// Complete producer time buffer in Julian days; scale follows the operation.
    pub times: [f64; 10],
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete local occultation result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalOccultation {
    /// Actual native integer status; None only when no status channel exists.
    pub status: i32,
    /// Complete producer time buffer in Julian days; scale follows the operation.
    pub times: [f64; 10],
    /// Complete native circumstance buffer in its original slot order.
    pub attributes: [f64; 20],
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete occultation geometry result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct OccultationGeometry {
    /// Actual native integer status; None only when no status channel exists.
    pub status: i32,
    /// All ten native geographic output slots, without truncation.
    pub geography: [f64; 10],
    /// Complete native circumstance buffer in its original slot order.
    pub attributes: [f64; 20],
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete rise transit result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct RiseTransit {
    /// Actual native integer status; None only when no status channel exists.
    pub status: i32,
    /// Native UT event Julian Day when an event exists.
    pub time: Option<f64>,
    /// Complete producer time buffer in Julian days; scale follows the operation.
    pub times: RiseTransitTimes,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete heliacal event result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct HeliacalEvent {
    /// Actual producer event-time buffer in UT Julian days, retaining its native length.
    pub values: Vec<f64>,
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete heliacal phenomena result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct HeliacalPhenomena {
    /// Producer values; units and domain follow the operation.
    pub values: [f64; 50],
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete visibility result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Visibility {
    /// Actual scalar-return channel, absent for integer-return producers.
    pub scalar: Option<f64>,
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Producer values; units and domain follow the operation.
    pub values: [f64; 10],
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete armc houses result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct ArmcHouses {
    /// Native cusp longitudes in degrees, in original house/sector order.
    pub cusps: Vec<f64>,
    /// Eight native house angles in degrees, in original order.
    pub angles: [f64; 8],
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

/// Complete armc houses with speeds result from one native operation.
///
/// Native units, buffer order and undefined slots are preserved. The producing
/// operation documents the applicable coordinate frame and time scale.
#[derive(Clone, Debug, PartialEq)]
pub struct ArmcHousesWithSpeeds {
    /// Native cusp longitudes in degrees, in original house/sector order.
    pub cusps: Vec<f64>,
    /// Eight native house angles in degrees, in original order.
    pub angles: [f64; 8],
    /// Native cusp rates in degrees per day.
    pub cusp_speeds: Vec<f64>,
    /// Native angle rates in degrees per day.
    pub angle_speeds: [f64; 8],
    /// Actual native integer status; None only when no status channel exists.
    pub status: Option<i32>,
    /// Original native diagnostic; None means no diagnostic channel exists.
    pub diagnostic: Option<String>,
}

pub use crate::advisory::{Advisory, AdvisoryDetail};

/// Closed calendar selection/transport.
#[derive(Clone, Debug, PartialEq, Copy, Eq)]
pub enum Calendar {
    /// Julian native selection or transport.
    Julian,
    /// Gregorian native selection or transport.
    Gregorian,
}

/// Closed target selection/transport.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// Body native selection or transport.
    Body(i32),
    /// Star native selection or transport.
    Star(String),
}

/// Closed local solar times selection/transport.
#[derive(Clone, Debug, PartialEq)]
pub enum LocalSolarTimes {
    /// Seven native selection or transport.
    Seven([f64; 7]),
    /// Ten native selection or transport.
    Ten([f64; 10]),
}

/// Closed rise transit times selection/transport.
#[derive(Clone, Debug, PartialEq)]
pub enum RiseTransitTimes {
    /// One native selection or transport.
    One([f64; 1]),
    /// Ten native selection or transport.
    Ten([f64; 10]),
    /// Absent native selection or transport.
    Absent,
}

/// Closed error kind selection/transport.
#[derive(Clone, Debug, PartialEq, Copy, Eq)]
pub enum ErrorKind {
    /// InvalidInput native selection or transport.
    InvalidInput,
    /// MissingData native selection or transport.
    MissingData,
    /// OutOfRange native selection or transport.
    OutOfRange,
    /// Calculation native selection or transport.
    Calculation,
    /// Configuration native selection or transport.
    Configuration,
    /// Unsupported native selection or transport.
    Unsupported,
    /// Unavailable native selection or transport.
    Unavailable,
}

impl Calendar {
    /// Select Julian (0) or proleptic Gregorian (1); rejects other flags.
    pub fn from_flag(flag: i32) -> Result<Self, super::Error> {
        match flag {
            0 => Ok(Self::Julian),
            1 => Ok(Self::Gregorian),
            _ => Err(super::Error::invalid(
                "Calendar::from_flag",
                "calendar flag must be zero or one",
            )),
        }
    }
    /// Return the native integer calendar flag.
    pub fn flag(self) -> i32 {
        match self {
            Self::Julian => 0,
            Self::Gregorian => 1,
        }
    }
    pub(super) fn native(self) -> crate::time::Calendar {
        match self {
            Self::Julian => crate::time::Calendar::Julian,
            Self::Gregorian => crate::time::Calendar::Gregorian,
        }
    }
}

impl Position {
    /// First native component: longitude in default ecliptic degrees.
    pub fn longitude(&self) -> f64 {
        self.values[0]
    }
    /// Second native component: latitude in default ecliptic degrees.
    pub fn latitude(&self) -> f64 {
        self.values[1]
    }
    /// Third native component: distance in default spherical AU.
    pub fn distance(&self) -> f64 {
        self.values[2]
    }
}
impl Houses {
    /// Native Ascendant angle in degrees.
    pub fn ascendant(&self) -> f64 {
        self.angles[0]
    }
    /// Native Midheaven angle in degrees.
    pub fn mc(&self) -> f64 {
        self.angles[1]
    }
}
impl HousesWithSpeeds {
    /// Native Ascendant angle in degrees.
    pub fn ascendant(&self) -> f64 {
        self.angles[0]
    }
    /// Native Midheaven angle in degrees.
    pub fn mc(&self) -> f64 {
        self.angles[1]
    }
}
