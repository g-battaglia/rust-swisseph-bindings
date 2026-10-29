//! Data-independent primary API structural bootstrap; incomplete until all mapping entries are active.
use swisseph_bindings as eph;
// COMMON_API_CONTRACT_BEGIN
// P0 structural bootstrap, NOT the complete 2.0/4.0 contract. The mapping lists
// unresolved signatures/schemas explicitly; final acceptance forbids pending
// entries. These checks do not call either engine, load data, or infer sources.

use std::fmt::Debug;

fn owned<T: Clone + Send + Sync + Debug>() {}

#[test]
fn retained_value_and_session_traits() {
    owned::<eph::Calendar>();
    owned::<eph::CalendarDate>();
    owned::<eph::Houses>();
    owned::<eph::HousesWithSpeeds>();
    owned::<eph::GauquelinSectors>();
    owned::<eph::Ayanamsha>();
    owned::<eph::LongitudeCrossing>();
    owned::<eph::NodeCrossing>();
    owned::<eph::Horizontal>();
    owned::<eph::Celestial>();
    owned::<eph::Position>();
    owned::<eph::Error>();
    owned::<eph::Session>();
    owned::<eph::SessionBuilder>();
}

#[test]
fn retained_builder_signatures() {
    let _: fn() -> eph::SessionBuilder = eph::SessionBuilder::new;
    let _: fn(&eph::SessionBuilder) -> Result<eph::Session, eph::Error> =
        eph::SessionBuilder::build;
    let _: fn(&mut eph::SessionBuilder, f64, f64, f64) -> &mut eph::SessionBuilder =
        eph::SessionBuilder::topo;
    let _: fn(&mut eph::SessionBuilder, i32, f64, f64) -> &mut eph::SessionBuilder =
        eph::SessionBuilder::sid_mode;
    let _: fn(&mut eph::SessionBuilder, f64) -> &mut eph::SessionBuilder =
        eph::SessionBuilder::tid_acc;
    let _: fn(&mut eph::SessionBuilder, f64) -> &mut eph::SessionBuilder =
        eph::SessionBuilder::delta_t_override;
    let _: fn(&mut eph::SessionBuilder) -> &mut eph::SessionBuilder =
        eph::SessionBuilder::clear_delta_t_override;
    let _: fn(&mut eph::SessionBuilder, f64) -> &mut eph::SessionBuilder =
        eph::SessionBuilder::lapse_rate;
}

#[test]
fn retained_position_entry_signatures() {
    let _: fn(&eph::Session, f64, i32, i32) -> Result<eph::Position, eph::Error> =
        eph::Session::calc;
    let _: fn(&eph::Session, f64, i32, i32) -> Result<eph::Position, eph::Error> =
        eph::Session::calc_ut;
    let _: fn(&eph::Session, f64, i32, i32, i32) -> Result<eph::Position, eph::Error> =
        eph::Session::calc_pctr;
}

#[test]
fn retained_helper_signatures() {
    let _: fn(&eph::Position) -> f64 = eph::Position::longitude;
    let _: fn(&eph::Position) -> f64 = eph::Position::latitude;
    let _: fn(&eph::Position) -> f64 = eph::Position::distance;
    let _: fn(&eph::Houses) -> f64 = eph::Houses::ascendant;
    let _: fn(&eph::Houses) -> f64 = eph::Houses::mc;
    let _: fn(&eph::HousesWithSpeeds) -> f64 = eph::HousesWithSpeeds::ascendant;
    let _: fn(&eph::HousesWithSpeeds) -> f64 = eph::HousesWithSpeeds::mc;
}

// Helpers are compiled even though no physical record is fabricated merely to
// execute a destructuring test. A field addition or type change must fail.
#[allow(dead_code, clippy::too_many_arguments)]
fn retained_record_schemas(
    date: eph::CalendarDate,
    houses: eph::Houses,
    speeds: eph::HousesWithSpeeds,
    sectors: eph::GauquelinSectors,
    ayanamsha: eph::Ayanamsha,
    crossing: eph::LongitudeCrossing,
    node: eph::NodeCrossing,
    horizontal: eph::Horizontal,
    celestial: eph::Celestial,
) {
    let eph::CalendarDate {
        year,
        month,
        day,
        hour,
    } = date;
    let _: (i32, i32, i32, f64) = (year, month, day, hour);
    let eph::Houses { cusps, angles } = houses;
    let _: ([f64; 12], [f64; 8]) = (cusps, angles);
    let eph::HousesWithSpeeds {
        cusps,
        angles,
        cusp_speeds,
        angle_speeds,
    } = speeds;
    let _: ([f64; 12], [f64; 8], [f64; 12], [f64; 8]) = (cusps, angles, cusp_speeds, angle_speeds);
    let eph::GauquelinSectors {
        sectors,
        angles,
        sector_speeds,
        angle_speeds,
    } = sectors;
    let _: ([f64; 36], [f64; 8], [f64; 36], [f64; 8]) =
        (sectors, angles, sector_speeds, angle_speeds);
    let eph::Ayanamsha {
        value,
        returned_flags,
        diagnostic,
    } = ayanamsha;
    let _: (f64, i32, String) = (value, returned_flags, diagnostic);
    let eph::LongitudeCrossing { time, diagnostic } = crossing;
    let _: (f64, String) = (time, diagnostic);
    let eph::NodeCrossing {
        time,
        longitude,
        latitude,
        diagnostic,
    } = node;
    let _: (f64, f64, f64, String) = (time, longitude, latitude, diagnostic);
    let eph::Horizontal {
        azimuth,
        true_altitude,
        apparent_altitude,
    } = horizontal;
    let _: (f64, f64, f64) = (azimuth, true_altitude, apparent_altitude);
    let eph::Celestial {
        longitude_or_ra,
        latitude_or_dec,
    } = celestial;
    let _: (f64, f64) = (longitude_or_ra, latitude_or_dec);
}

#[allow(dead_code)]
fn calendar_variants(calendar: eph::Calendar) -> bool {
    match calendar {
        eph::Calendar::Julian => false,
        eph::Calendar::Gregorian => true,
    }
}

#[test]
fn calendar_constants_have_exact_type_and_value() {
    let _: i32 = eph::JUL_CAL;
    let _: i32 = eph::GREG_CAL;
    assert_eq!(eph::JUL_CAL, 0);
    assert_eq!(eph::GREG_CAL, 1);
}

#[test]
fn agreed_constant_values_and_types() {
    let _: i32 = eph::ACRONYCHAL_RISING;
    assert_eq!(eph::ACRONYCHAL_RISING, 5);
    let _: i32 = eph::ACRONYCHAL_SETTING;
    assert_eq!(eph::ACRONYCHAL_SETTING, 6);
    let _: i32 = eph::ADMETOS;
    assert_eq!(eph::ADMETOS, 45);
    let _: i32 = eph::APOLLON;
    assert_eq!(eph::APOLLON, 44);
    let _: i32 = eph::ARMC;
    assert_eq!(eph::ARMC, 2);
    let _: i32 = eph::ASC;
    assert_eq!(eph::ASC, 0);
    let _: &str = eph::ASTNAMFILE;
    assert_eq!(eph::ASTNAMFILE, "seasnam.txt");
    let _: i32 = eph::AST_OFFSET;
    assert_eq!(eph::AST_OFFSET, 10000);
    let _: f64 = eph::AUNIT_TO_KM;
    assert_eq!(eph::AUNIT_TO_KM.to_bits(), 0x41a1d55d5d666666);
    let _: f64 = eph::AUNIT_TO_LIGHTYEAR;
    assert_eq!(eph::AUNIT_TO_LIGHTYEAR.to_bits(), 0x3ef094a33c2ea847);
    let _: f64 = eph::AUNIT_TO_PARSEC;
    assert_eq!(eph::AUNIT_TO_PARSEC.to_bits(), 0x3ed455a5b2ff8f38);
    let _: i32 = eph::BIT_ASTRO_TWILIGHT;
    assert_eq!(eph::BIT_ASTRO_TWILIGHT, 4096);
    let _: i32 = eph::BIT_CIVIL_TWILIGHT;
    assert_eq!(eph::BIT_CIVIL_TWILIGHT, 1024);
    let _: i32 = eph::BIT_DISC_BOTTOM;
    assert_eq!(eph::BIT_DISC_BOTTOM, 8192);
    let _: i32 = eph::BIT_DISC_CENTER;
    assert_eq!(eph::BIT_DISC_CENTER, 256);
    let _: i32 = eph::BIT_FIXED_DISC_SIZE;
    assert_eq!(eph::BIT_FIXED_DISC_SIZE, 16384);
    let _: i32 = eph::BIT_FORCE_SLOW_METHOD;
    assert_eq!(eph::BIT_FORCE_SLOW_METHOD, 32768);
    let _: i32 = eph::BIT_GEOCTR_NO_ECL_LAT;
    assert_eq!(eph::BIT_GEOCTR_NO_ECL_LAT, 128);
    let _: i32 = eph::BIT_HINDU_RISING;
    assert_eq!(eph::BIT_HINDU_RISING, 896);
    let _: i32 = eph::BIT_NAUTIC_TWILIGHT;
    assert_eq!(eph::BIT_NAUTIC_TWILIGHT, 2048);
    let _: i32 = eph::BIT_NO_REFRACTION;
    assert_eq!(eph::BIT_NO_REFRACTION, 512);
    let _: i32 = eph::CALC_ITRANSIT;
    assert_eq!(eph::CALC_ITRANSIT, 8);
    let _: i32 = eph::CALC_MTRANSIT;
    assert_eq!(eph::CALC_MTRANSIT, 4);
    let _: i32 = eph::CALC_RISE;
    assert_eq!(eph::CALC_RISE, 1);
    let _: i32 = eph::CALC_SET;
    assert_eq!(eph::CALC_SET, 2);
    let _: i32 = eph::CERES;
    assert_eq!(eph::CERES, 17);
    let _: i32 = eph::CHIRON;
    assert_eq!(eph::CHIRON, 15);
    let _: i32 = eph::COASC1;
    assert_eq!(eph::COASC1, 5);
    let _: i32 = eph::COASC2;
    assert_eq!(eph::COASC2, 6);
    let _: i32 = eph::COMET_OFFSET;
    assert_eq!(eph::COMET_OFFSET, 1000);
    let _: i32 = eph::COSMICAL_SETTING;
    assert_eq!(eph::COSMICAL_SETTING, 6);
    let _: i32 = eph::CUPIDO;
    assert_eq!(eph::CUPIDO, 40);
    let _: i32 = eph::DE_NUMBER;
    assert_eq!(eph::DE_NUMBER, 431);
    let _: i32 = eph::EARTH;
    assert_eq!(eph::EARTH, 14);
    let _: i32 = eph::ECL_1ST_VISIBLE;
    assert_eq!(eph::ECL_1ST_VISIBLE, 512);
    let _: i32 = eph::ECL_2ND_VISIBLE;
    assert_eq!(eph::ECL_2ND_VISIBLE, 1024);
    let _: i32 = eph::ECL_3RD_VISIBLE;
    assert_eq!(eph::ECL_3RD_VISIBLE, 2048);
    let _: i32 = eph::ECL_4TH_VISIBLE;
    assert_eq!(eph::ECL_4TH_VISIBLE, 4096);
    let _: i32 = eph::ECL_ALLTYPES_LUNAR;
    assert_eq!(eph::ECL_ALLTYPES_LUNAR, 84);
    let _: i32 = eph::ECL_ALLTYPES_SOLAR;
    assert_eq!(eph::ECL_ALLTYPES_SOLAR, 63);
    let _: i32 = eph::ECL_ANNULAR;
    assert_eq!(eph::ECL_ANNULAR, 8);
    let _: i32 = eph::ECL_ANNULAR_TOTAL;
    assert_eq!(eph::ECL_ANNULAR_TOTAL, 32);
    let _: i32 = eph::ECL_CENTRAL;
    assert_eq!(eph::ECL_CENTRAL, 1);
    let _: i32 = eph::ECL_HYBRID;
    assert_eq!(eph::ECL_HYBRID, 32);
    let _: i32 = eph::ECL_MAX_VISIBLE;
    assert_eq!(eph::ECL_MAX_VISIBLE, 256);
    let _: i32 = eph::ECL_NONCENTRAL;
    assert_eq!(eph::ECL_NONCENTRAL, 2);
    let _: i32 = eph::ECL_NUT;
    assert_eq!(eph::ECL_NUT, -1);
    let _: i32 = eph::ECL_OCC_BEG_DAYLIGHT;
    assert_eq!(eph::ECL_OCC_BEG_DAYLIGHT, 8192);
    let _: i32 = eph::ECL_OCC_END_DAYLIGHT;
    assert_eq!(eph::ECL_OCC_END_DAYLIGHT, 16384);
    let _: i32 = eph::ECL_ONE_TRY;
    assert_eq!(eph::ECL_ONE_TRY, 32768);
    let _: i32 = eph::ECL_PARTBEG_VISIBLE;
    assert_eq!(eph::ECL_PARTBEG_VISIBLE, 512);
    let _: i32 = eph::ECL_PARTEND_VISIBLE;
    assert_eq!(eph::ECL_PARTEND_VISIBLE, 4096);
    let _: i32 = eph::ECL_PARTIAL;
    assert_eq!(eph::ECL_PARTIAL, 16);
    let _: i32 = eph::ECL_PENUMBBEG_VISIBLE;
    assert_eq!(eph::ECL_PENUMBBEG_VISIBLE, 8192);
    let _: i32 = eph::ECL_PENUMBEND_VISIBLE;
    assert_eq!(eph::ECL_PENUMBEND_VISIBLE, 16384);
    let _: i32 = eph::ECL_PENUMBRAL;
    assert_eq!(eph::ECL_PENUMBRAL, 64);
    let _: i32 = eph::ECL_TOTAL;
    assert_eq!(eph::ECL_TOTAL, 4);
    let _: i32 = eph::ECL_TOTBEG_VISIBLE;
    assert_eq!(eph::ECL_TOTBEG_VISIBLE, 1024);
    let _: i32 = eph::ECL_TOTEND_VISIBLE;
    assert_eq!(eph::ECL_TOTEND_VISIBLE, 2048);
    let _: i32 = eph::ECL_VISIBLE;
    assert_eq!(eph::ECL_VISIBLE, 128);
    let _: i32 = eph::EQUASC;
    assert_eq!(eph::EQUASC, 4);
    let _: i32 = eph::EVENING_FIRST;
    assert_eq!(eph::EVENING_FIRST, 3);
    let _: i32 = eph::EVENING_LAST;
    assert_eq!(eph::EVENING_LAST, 2);
    let _: &str = eph::FICTFILE;
    assert_eq!(eph::FICTFILE, "seorbel.txt");
    let _: i32 = eph::FICT_MAX;
    assert_eq!(eph::FICT_MAX, 999);
    let _: i32 = eph::FICT_OFFSET;
    assert_eq!(eph::FICT_OFFSET, 40);
    let _: i32 = eph::FICT_OFFSET_1;
    assert_eq!(eph::FICT_OFFSET_1, 39);
    let _: i32 = eph::FIXSTAR;
    assert_eq!(eph::FIXSTAR, -10);
    let _: i32 = eph::FLG_ASTROMETRIC;
    assert_eq!(eph::FLG_ASTROMETRIC, 1536);
    let _: i32 = eph::FLG_BARYCTR;
    assert_eq!(eph::FLG_BARYCTR, 16384);
    let _: i32 = eph::FLG_CENTER_BODY;
    assert_eq!(eph::FLG_CENTER_BODY, 1048576);
    let _: i32 = eph::FLG_DEFAULTEPH;
    assert_eq!(eph::FLG_DEFAULTEPH, 2);
    let _: i32 = eph::FLG_EQUATORIAL;
    assert_eq!(eph::FLG_EQUATORIAL, 2048);
    let _: i32 = eph::FLG_HELCTR;
    assert_eq!(eph::FLG_HELCTR, 8);
    let _: i32 = eph::FLG_ICRS;
    assert_eq!(eph::FLG_ICRS, 131072);
    let _: i32 = eph::FLG_J2000;
    assert_eq!(eph::FLG_J2000, 32);
    let _: i32 = eph::FLG_JPLEPH;
    assert_eq!(eph::FLG_JPLEPH, 1);
    let _: i32 = eph::FLG_MOSEPH;
    assert_eq!(eph::FLG_MOSEPH, 4);
    let _: i32 = eph::FLG_NOABERR;
    assert_eq!(eph::FLG_NOABERR, 1024);
    let _: i32 = eph::FLG_NOGDEFL;
    assert_eq!(eph::FLG_NOGDEFL, 512);
    let _: i32 = eph::FLG_NONUT;
    assert_eq!(eph::FLG_NONUT, 64);
    let _: i32 = eph::FLG_RADIANS;
    assert_eq!(eph::FLG_RADIANS, 8192);
    let _: i32 = eph::FLG_SIDEREAL;
    assert_eq!(eph::FLG_SIDEREAL, 65536);
    let _: i32 = eph::FLG_SPEED;
    assert_eq!(eph::FLG_SPEED, 256);
    let _: i32 = eph::FLG_SPEED3;
    assert_eq!(eph::FLG_SPEED3, 128);
    let _: i32 = eph::FLG_SWIEPH;
    assert_eq!(eph::FLG_SWIEPH, 2);
    let _: i32 = eph::FLG_TOPOCTR;
    assert_eq!(eph::FLG_TOPOCTR, 32768);
    let _: i32 = eph::FLG_TRUEPOS;
    assert_eq!(eph::FLG_TRUEPOS, 16);
    let _: i32 = eph::FLG_XYZ;
    assert_eq!(eph::FLG_XYZ, 4096);
    let _: i32 = eph::GREG_CAL;
    assert_eq!(eph::GREG_CAL, 1);
    let _: i32 = eph::HADES;
    assert_eq!(eph::HADES, 41);
    let _: i32 = eph::HARRINGTON;
    assert_eq!(eph::HARRINGTON, 50);
    let _: i32 = eph::HELFLAG_AV;
    assert_eq!(eph::HELFLAG_AV, 65536);
    let _: i32 = eph::HELFLAG_AVKIND;
    assert_eq!(eph::HELFLAG_AVKIND, 983040);
    let _: i32 = eph::HELFLAG_AVKIND_MIN7;
    assert_eq!(eph::HELFLAG_AVKIND_MIN7, 262144);
    let _: i32 = eph::HELFLAG_AVKIND_MIN9;
    assert_eq!(eph::HELFLAG_AVKIND_MIN9, 524288);
    let _: i32 = eph::HELFLAG_AVKIND_PTO;
    assert_eq!(eph::HELFLAG_AVKIND_PTO, 131072);
    let _: i32 = eph::HELFLAG_AVKIND_VR;
    assert_eq!(eph::HELFLAG_AVKIND_VR, 65536);
    let _: i32 = eph::HELFLAG_BELOW_HORIZON;
    assert_eq!(eph::HELFLAG_BELOW_HORIZON, -2);
    let _: i32 = eph::HELFLAG_HIGH_PRECISION;
    assert_eq!(eph::HELFLAG_HIGH_PRECISION, 256);
    let _: i32 = eph::HELFLAG_LONG_SEARCH;
    assert_eq!(eph::HELFLAG_LONG_SEARCH, 128);
    let _: i32 = eph::HELFLAG_MIXED;
    assert_eq!(eph::HELFLAG_MIXED, 2);
    let _: i32 = eph::HELFLAG_NO_DETAILS;
    assert_eq!(eph::HELFLAG_NO_DETAILS, 1024);
    let _: i32 = eph::HELFLAG_OPTICAL_PARAMS;
    assert_eq!(eph::HELFLAG_OPTICAL_PARAMS, 512);
    let _: i32 = eph::HELFLAG_PHOTOPIC;
    assert_eq!(eph::HELFLAG_PHOTOPIC, 0);
    let _: i32 = eph::HELFLAG_SCOTOPIC;
    assert_eq!(eph::HELFLAG_SCOTOPIC, 1);
    let _: i32 = eph::HELFLAG_SEARCH_1_PERIOD;
    assert_eq!(eph::HELFLAG_SEARCH_1_PERIOD, 2048);
    let _: i32 = eph::HELFLAG_VISLIM_DARK;
    assert_eq!(eph::HELFLAG_VISLIM_DARK, 4096);
    let _: i32 = eph::HELFLAG_VISLIM_NOMOON;
    assert_eq!(eph::HELFLAG_VISLIM_NOMOON, 8192);
    let _: i32 = eph::HELFLAG_VISLIM_PHOTOPIC;
    assert_eq!(eph::HELFLAG_VISLIM_PHOTOPIC, 16384);
    let _: i32 = eph::HELFLAG_VISLIM_SCOTOPIC;
    assert_eq!(eph::HELFLAG_VISLIM_SCOTOPIC, 32768);
    let _: i32 = eph::HELIACAL_RISING;
    assert_eq!(eph::HELIACAL_RISING, 1);
    let _: i32 = eph::HELIACAL_SETTING;
    assert_eq!(eph::HELIACAL_SETTING, 2);
    let _: i32 = eph::INTP_APOG;
    assert_eq!(eph::INTP_APOG, 21);
    let _: i32 = eph::INTP_PERG;
    assert_eq!(eph::INTP_PERG, 22);
    let _: i32 = eph::ISIS;
    assert_eq!(eph::ISIS, 48);
    let _: i32 = eph::JUL_CAL;
    assert_eq!(eph::JUL_CAL, 0);
    let _: i32 = eph::JUNO;
    assert_eq!(eph::JUNO, 19);
    let _: i32 = eph::JUPITER;
    assert_eq!(eph::JUPITER, 5);
    let _: i32 = eph::KRONOS;
    assert_eq!(eph::KRONOS, 43);
    let _: i32 = eph::MARS;
    assert_eq!(eph::MARS, 4);
    let _: i32 = eph::MC;
    assert_eq!(eph::MC, 1);
    let _: i32 = eph::MEAN_APOG;
    assert_eq!(eph::MEAN_APOG, 12);
    let _: i32 = eph::MEAN_NODE;
    assert_eq!(eph::MEAN_NODE, 10);
    let _: i32 = eph::MERCURY;
    assert_eq!(eph::MERCURY, 2);
    let _: i32 = eph::MIXEDOPIC_FLAG;
    assert_eq!(eph::MIXEDOPIC_FLAG, 2);
    let _: i32 = eph::MOON;
    assert_eq!(eph::MOON, 1);
    let _: i32 = eph::MORNING_FIRST;
    assert_eq!(eph::MORNING_FIRST, 1);
    let _: i32 = eph::MORNING_LAST;
    assert_eq!(eph::MORNING_LAST, 4);
    let _: i32 = eph::NEPTUNE;
    assert_eq!(eph::NEPTUNE, 8);
    let _: i32 = eph::NEPTUNE_ADAMS;
    assert_eq!(eph::NEPTUNE_ADAMS, 52);
    let _: i32 = eph::NEPTUNE_LEVERRIER;
    assert_eq!(eph::NEPTUNE_LEVERRIER, 51);
    let _: i32 = eph::NIBIRU;
    assert_eq!(eph::NIBIRU, 49);
    let _: i32 = eph::NODBIT_FOPOINT;
    assert_eq!(eph::NODBIT_FOPOINT, 256);
    let _: i32 = eph::NODBIT_MEAN;
    assert_eq!(eph::NODBIT_MEAN, 1);
    let _: i32 = eph::NODBIT_OSCU;
    assert_eq!(eph::NODBIT_OSCU, 2);
    let _: i32 = eph::NODBIT_OSCU_BAR;
    assert_eq!(eph::NODBIT_OSCU_BAR, 4);
    let _: i32 = eph::NPLANETS;
    assert_eq!(eph::NPLANETS, 23);
    let _: i32 = eph::OSCU_APOG;
    assert_eq!(eph::OSCU_APOG, 13);
    let _: i32 = eph::PALLAS;
    assert_eq!(eph::PALLAS, 18);
    let _: i32 = eph::PHOLUS;
    assert_eq!(eph::PHOLUS, 16);
    let _: i32 = eph::PHOTOPIC_FLAG;
    assert_eq!(eph::PHOTOPIC_FLAG, 0);
    let _: i32 = eph::PLMOON_OFFSET;
    assert_eq!(eph::PLMOON_OFFSET, 9000);
    let _: i32 = eph::PLUTO;
    assert_eq!(eph::PLUTO, 9);
    let _: i32 = eph::PLUTO_LOWELL;
    assert_eq!(eph::PLUTO_LOWELL, 53);
    let _: i32 = eph::PLUTO_PICKERING;
    assert_eq!(eph::PLUTO_PICKERING, 54);
    let _: i32 = eph::POSEIDON;
    assert_eq!(eph::POSEIDON, 47);
    let _: i32 = eph::PROSERPINA;
    assert_eq!(eph::PROSERPINA, 57);
    let _: i32 = eph::SATURN;
    assert_eq!(eph::SATURN, 6);
    let _: i32 = eph::SCOTOPIC_FLAG;
    assert_eq!(eph::SCOTOPIC_FLAG, 1);
    let _: i32 = eph::SIDM_ALDEBARAN_15TAU;
    assert_eq!(eph::SIDM_ALDEBARAN_15TAU, 14);
    let _: i32 = eph::SIDM_ARYABHATA;
    assert_eq!(eph::SIDM_ARYABHATA, 23);
    let _: i32 = eph::SIDM_ARYABHATA_522;
    assert_eq!(eph::SIDM_ARYABHATA_522, 37);
    let _: i32 = eph::SIDM_ARYABHATA_MSUN;
    assert_eq!(eph::SIDM_ARYABHATA_MSUN, 24);
    let _: i32 = eph::SIDM_B1950;
    assert_eq!(eph::SIDM_B1950, 20);
    let _: i32 = eph::SIDM_BABYL_BRITTON;
    assert_eq!(eph::SIDM_BABYL_BRITTON, 38);
    let _: i32 = eph::SIDM_BABYL_ETPSC;
    assert_eq!(eph::SIDM_BABYL_ETPSC, 13);
    let _: i32 = eph::SIDM_BABYL_HUBER;
    assert_eq!(eph::SIDM_BABYL_HUBER, 12);
    let _: i32 = eph::SIDM_BABYL_KUGLER1;
    assert_eq!(eph::SIDM_BABYL_KUGLER1, 9);
    let _: i32 = eph::SIDM_BABYL_KUGLER2;
    assert_eq!(eph::SIDM_BABYL_KUGLER2, 10);
    let _: i32 = eph::SIDM_BABYL_KUGLER3;
    assert_eq!(eph::SIDM_BABYL_KUGLER3, 11);
    let _: i32 = eph::SIDM_DELUCE;
    assert_eq!(eph::SIDM_DELUCE, 2);
    let _: i32 = eph::SIDM_DJWHAL_KHUL;
    assert_eq!(eph::SIDM_DJWHAL_KHUL, 6);
    let _: i32 = eph::SIDM_FAGAN_BRADLEY;
    assert_eq!(eph::SIDM_FAGAN_BRADLEY, 0);
    let _: i32 = eph::SIDM_GALALIGN_MARDYKS;
    assert_eq!(eph::SIDM_GALALIGN_MARDYKS, 34);
    let _: i32 = eph::SIDM_GALCENT_0SAG;
    assert_eq!(eph::SIDM_GALCENT_0SAG, 17);
    let _: i32 = eph::SIDM_GALCENT_COCHRANE;
    assert_eq!(eph::SIDM_GALCENT_COCHRANE, 40);
    let _: i32 = eph::SIDM_GALCENT_MULA_WILHELM;
    assert_eq!(eph::SIDM_GALCENT_MULA_WILHELM, 36);
    let _: i32 = eph::SIDM_GALCENT_RGILBRAND;
    assert_eq!(eph::SIDM_GALCENT_RGILBRAND, 30);
    let _: i32 = eph::SIDM_GALEQU_FIORENZA;
    assert_eq!(eph::SIDM_GALEQU_FIORENZA, 41);
    let _: i32 = eph::SIDM_GALEQU_IAU1958;
    assert_eq!(eph::SIDM_GALEQU_IAU1958, 31);
    let _: i32 = eph::SIDM_GALEQU_MULA;
    assert_eq!(eph::SIDM_GALEQU_MULA, 33);
    let _: i32 = eph::SIDM_GALEQU_TRUE;
    assert_eq!(eph::SIDM_GALEQU_TRUE, 32);
    let _: i32 = eph::SIDM_HIPPARCHOS;
    assert_eq!(eph::SIDM_HIPPARCHOS, 15);
    let _: i32 = eph::SIDM_J1900;
    assert_eq!(eph::SIDM_J1900, 19);
    let _: i32 = eph::SIDM_J2000;
    assert_eq!(eph::SIDM_J2000, 18);
    let _: i32 = eph::SIDM_JN_BHASIN;
    assert_eq!(eph::SIDM_JN_BHASIN, 8);
    let _: i32 = eph::SIDM_KRISHNAMURTI;
    assert_eq!(eph::SIDM_KRISHNAMURTI, 5);
    let _: i32 = eph::SIDM_KRISHNAMURTI_VP291;
    assert_eq!(eph::SIDM_KRISHNAMURTI_VP291, 45);
    let _: i32 = eph::SIDM_LAHIRI;
    assert_eq!(eph::SIDM_LAHIRI, 1);
    let _: i32 = eph::SIDM_LAHIRI_1940;
    assert_eq!(eph::SIDM_LAHIRI_1940, 43);
    let _: i32 = eph::SIDM_LAHIRI_ICRC;
    assert_eq!(eph::SIDM_LAHIRI_ICRC, 46);
    let _: i32 = eph::SIDM_LAHIRI_VP285;
    assert_eq!(eph::SIDM_LAHIRI_VP285, 44);
    let _: i32 = eph::SIDM_RAMAN;
    assert_eq!(eph::SIDM_RAMAN, 3);
    let _: i32 = eph::SIDM_SASSANIAN;
    assert_eq!(eph::SIDM_SASSANIAN, 16);
    let _: i32 = eph::SIDM_SS_CITRA;
    assert_eq!(eph::SIDM_SS_CITRA, 26);
    let _: i32 = eph::SIDM_SS_REVATI;
    assert_eq!(eph::SIDM_SS_REVATI, 25);
    let _: i32 = eph::SIDM_SURYASIDDHANTA;
    assert_eq!(eph::SIDM_SURYASIDDHANTA, 21);
    let _: i32 = eph::SIDM_SURYASIDDHANTA_MSUN;
    assert_eq!(eph::SIDM_SURYASIDDHANTA_MSUN, 22);
    let _: i32 = eph::SIDM_TRUE_CITRA;
    assert_eq!(eph::SIDM_TRUE_CITRA, 27);
    let _: i32 = eph::SIDM_TRUE_MULA;
    assert_eq!(eph::SIDM_TRUE_MULA, 35);
    let _: i32 = eph::SIDM_TRUE_PUSHYA;
    assert_eq!(eph::SIDM_TRUE_PUSHYA, 29);
    let _: i32 = eph::SIDM_TRUE_REVATI;
    assert_eq!(eph::SIDM_TRUE_REVATI, 28);
    let _: i32 = eph::SIDM_TRUE_SHEORAN;
    assert_eq!(eph::SIDM_TRUE_SHEORAN, 39);
    let _: i32 = eph::SIDM_USER;
    assert_eq!(eph::SIDM_USER, 255);
    let _: i32 = eph::SIDM_USHASHASHI;
    assert_eq!(eph::SIDM_USHASHASHI, 4);
    let _: i32 = eph::SIDM_VALENS_MOON;
    assert_eq!(eph::SIDM_VALENS_MOON, 42);
    let _: i32 = eph::SIDM_YUKTESHWAR;
    assert_eq!(eph::SIDM_YUKTESHWAR, 7);
    let _: i32 = eph::SPLIT_DEG_KEEP_DEG;
    assert_eq!(eph::SPLIT_DEG_KEEP_DEG, 32);
    let _: i32 = eph::SPLIT_DEG_KEEP_SIGN;
    assert_eq!(eph::SPLIT_DEG_KEEP_SIGN, 16);
    let _: i32 = eph::SPLIT_DEG_NAKSHATRA;
    assert_eq!(eph::SPLIT_DEG_NAKSHATRA, 1024);
    let _: i32 = eph::SPLIT_DEG_ROUND_DEG;
    assert_eq!(eph::SPLIT_DEG_ROUND_DEG, 4);
    let _: i32 = eph::SPLIT_DEG_ROUND_MIN;
    assert_eq!(eph::SPLIT_DEG_ROUND_MIN, 2);
    let _: i32 = eph::SPLIT_DEG_ROUND_SEC;
    assert_eq!(eph::SPLIT_DEG_ROUND_SEC, 1);
    let _: i32 = eph::SPLIT_DEG_ZODIACAL;
    assert_eq!(eph::SPLIT_DEG_ZODIACAL, 8);
    let _: i32 = eph::SUN;
    assert_eq!(eph::SUN, 0);
    let _: f64 = eph::TJD_INVALID;
    assert_eq!(eph::TJD_INVALID.to_bits(), 0x4197d783fc000000);
    let _: i32 = eph::TRUE_NODE;
    assert_eq!(eph::TRUE_NODE, 11);
    let _: i32 = eph::URANUS;
    assert_eq!(eph::URANUS, 7);
    let _: i32 = eph::VARUNA;
    assert_eq!(eph::VARUNA, 30000);
    let _: i32 = eph::VENUS;
    assert_eq!(eph::VENUS, 3);
    let _: i32 = eph::VERTEX;
    assert_eq!(eph::VERTEX, 3);
    let _: i32 = eph::VESTA;
    assert_eq!(eph::VESTA, 20);
    let _: i32 = eph::VULCAN;
    assert_eq!(eph::VULCAN, 55);
    let _: i32 = eph::VULKANUS;
    assert_eq!(eph::VULKANUS, 46);
    let _: i32 = eph::WALDEMATH;
    assert_eq!(eph::WALDEMATH, 58);
    let _: i32 = eph::WHITE_MOON;
    assert_eq!(eph::WHITE_MOON, 56);
    let _: i32 = eph::ZEUS;
    assert_eq!(eph::ZEUS, 42);
}
