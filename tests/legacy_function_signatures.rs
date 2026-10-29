//! Freeze the 1.x function argument order before extension relocation.
//! Retarget only function paths to global/native when those modules land;
//! never derive new parameter order from primary Session methods.
use swisseph_bindings::*;

#[test]
#[expect(
    clippy::type_complexity,
    reason = "the legacy contract intentionally spells complete fn-pointer types without aliases"
)]
fn legacy_function_types_and_argument_order() {
    let _: fn(f64, f64, f64, f64) -> Result<[f64; 3], Error> = swisseph_bindings::cotrans;
    let _: fn(f64, f64, f64, f64, f64, f64, f64) -> Result<([f64; 3], [f64; 3]), Error> =
        swisseph_bindings::cotrans_sp;
    let _: fn(f64) -> f64 = swisseph_bindings::degnorm;
    let _: fn(f64) -> f64 = swisseph_bindings::radnorm;
    let _: fn(f64, f64) -> f64 = swisseph_bindings::difdegn;
    let _: fn(f64, f64) -> f64 = swisseph_bindings::difdeg2n;
    let _: fn(f64, f64) -> f64 = swisseph_bindings::difrad2n;
    let _: fn(f64, f64) -> f64 = swisseph_bindings::deg_midp;
    let _: fn(f64, f64) -> f64 = swisseph_bindings::rad_midp;
    let _: fn(f64) -> Result<i32, Error> = swisseph_bindings::d2l;
    let _: fn(i32) -> i32 = swisseph_bindings::csnorm;
    let _: fn(i32, i32) -> Result<i32, Error> = swisseph_bindings::difcsn;
    let _: fn(i32, i32) -> Result<i32, Error> = swisseph_bindings::difcs2n;
    let _: fn(i32) -> Result<i32, Error> = swisseph_bindings::csroundsec;
    let _: fn(i32, u8, bool) -> Result<String, Error> = swisseph_bindings::cs2timestr;
    let _: fn(i32, u8, u8) -> Result<String, Error> = swisseph_bindings::cs2lonlatstr;
    let _: fn(i32) -> Result<String, Error> = swisseph_bindings::cs2degstr;
    let _: fn(f64, i32) -> Result<SplitDeg, Error> = swisseph_bindings::split_deg;
    let _: fn(Option<&str>) -> Result<(), Error> = swisseph_bindings::set_ephe_path;
    let _: fn(Option<&str>) -> Result<(), Error> = swisseph_bindings::ensure_ephe_path;
    let _: fn(f64, f64, f64) -> Result<(), Error> = swisseph_bindings::set_topo;
    let _: fn(i32, f64, f64) -> Result<(), Error> = swisseph_bindings::set_sid_mode;
    let _: fn(&str) -> Result<(), Error> = swisseph_bindings::set_jpl_file;
    let _: fn(f64) -> Result<(), Error> = swisseph_bindings::set_tid_acc;
    let _: fn() -> Result<f64, Error> = swisseph_bindings::get_tid_acc;
    let _: fn(Option<f64>) -> Result<(), Error> = swisseph_bindings::set_delta_t_userdef;
    let _: fn(f64) -> Result<f64, Error> = swisseph_bindings::get_ayanamsa_ut;
    let _: fn(f64) -> Result<f64, Error> = swisseph_bindings::get_ayanamsa;
    let _: fn(f64, i32) -> Result<Ayanamsha, Error> = swisseph_bindings::get_ayanamsa_ex;
    let _: fn(f64, i32) -> Result<Ayanamsha, Error> = swisseph_bindings::get_ayanamsa_ex_ut;
    let _: fn(i32) -> Result<String, Error> = swisseph_bindings::get_ayanamsa_name;
    let _: fn() -> Result<(), Error> = swisseph_bindings::close;
    let _: fn(i32) -> Result<Option<CurrentFileData>, Error> =
        swisseph_bindings::get_current_file_data;
    let _: fn(f64, f64, i32) -> Result<LongitudeCrossing, Error> = swisseph_bindings::solcross;
    let _: fn(f64, f64, i32) -> Result<LongitudeCrossing, Error> = swisseph_bindings::solcross_ut;
    let _: fn(f64, f64, i32) -> Result<LongitudeCrossing, Error> = swisseph_bindings::mooncross;
    let _: fn(f64, f64, i32) -> Result<LongitudeCrossing, Error> = swisseph_bindings::mooncross_ut;
    let _: fn(f64, i32) -> Result<NodeCrossing, Error> = swisseph_bindings::mooncross_node;
    let _: fn(f64, i32) -> Result<NodeCrossing, Error> = swisseph_bindings::mooncross_node_ut;
    let _: fn(i32, f64, f64, i32, i32) -> Result<LongitudeCrossing, Error> =
        swisseph_bindings::helio_cross;
    let _: fn(i32, f64, f64, i32, i32) -> Result<LongitudeCrossing, Error> =
        swisseph_bindings::helio_cross_ut;
    let _: fn(f64, i32, i32, bool) -> Result<GlobalSolarEclipse, Error> =
        swisseph_bindings::sol_eclipse_when_glob;
    let _: fn(f64, i32, i32, bool) -> Result<GlobalLunarEclipse, Error> =
        swisseph_bindings::lun_eclipse_when;
    let _: fn(f64, i32) -> Result<SolarEclipseGeometryFull, Error> =
        swisseph_bindings::sol_eclipse_where_full;
    let _: fn(f64, i32) -> Result<SolarEclipseGeometry, Error> =
        swisseph_bindings::sol_eclipse_where;
    let _: fn(f64, i32, f64, f64, f64) -> Result<SolarEclipseCircumstances, Error> =
        swisseph_bindings::sol_eclipse_how;
    let _: fn(f64, i32, Option<(f64, f64, f64)>) -> Result<LunarEclipseCircumstances, Error> =
        swisseph_bindings::lun_eclipse_how;
    let _: fn(f64, i32, f64, f64, f64, bool) -> Result<LocalSolarEclipse, Error> =
        swisseph_bindings::sol_eclipse_when_loc;
    let _: fn(f64, i32, f64, f64, f64, bool) -> Result<LocalLunarEclipse, Error> =
        swisseph_bindings::lun_eclipse_when_loc;
    let _: fn(f64, i32, Option<&str>, i32, i32, bool) -> Result<GlobalOccultation, Error> =
        swisseph_bindings::lun_occult_when_glob;
    let _: fn(
        f64,
        i32,
        Option<&str>,
        i32,
        i32,
        OccultSearchOptions,
    ) -> Result<GlobalOccultation, Error> = swisseph_bindings::lun_occult_when_glob_with_options;
    let _: fn(f64, i32, Option<&str>, i32, f64, f64, f64, bool) -> Result<LocalOccultation, Error> =
        swisseph_bindings::lun_occult_when_loc;
    let _: fn(
        f64,
        i32,
        Option<&str>,
        i32,
        f64,
        f64,
        f64,
        OccultSearchOptions,
    ) -> Result<LocalOccultation, Error> = swisseph_bindings::lun_occult_when_loc_with_options;
    let _: fn(f64, i32, Option<&str>, i32) -> Result<OccultationGeometryFull, Error> =
        swisseph_bindings::lun_occult_where_full;
    let _: fn(f64, i32, Option<&str>, i32) -> Result<OccultationGeometry, Error> =
        swisseph_bindings::lun_occult_where;
    let _: fn(
        f64,
        f64,
        f64,
        f64,
        [f64; 4],
        [f64; 6],
        &str,
        i32,
        i32,
    ) -> Result<HeliacalEvent, Error> = swisseph_bindings::heliacal_ut;
    let _: fn(
        f64,
        f64,
        f64,
        f64,
        [f64; 4],
        [f64; 6],
        &str,
        i32,
        i32,
    ) -> Result<HeliacalPhenomenaFull, Error> = swisseph_bindings::heliacal_pheno_ut_full;
    let _: fn(
        f64,
        f64,
        f64,
        f64,
        [f64; 4],
        [f64; 6],
        &str,
        i32,
        i32,
    ) -> Result<HeliacalPhenomena, Error> = swisseph_bindings::heliacal_pheno_ut;
    let _: fn(
        f64,
        f64,
        f64,
        f64,
        [f64; 4],
        [f64; 6],
        &str,
        i32,
    ) -> Result<VisibilityOutcome, Error> = swisseph_bindings::vis_limit_mag;
    let _: fn(f64, f64, f64, f64, [f64; 4], [f64; 6], &str, i32) -> Result<VisibilityFull, Error> =
        swisseph_bindings::vis_limit_mag_full;
    let _: fn(f64, f64, f64, u8) -> Result<Houses, Error> = swisseph_bindings::houses;
    let _: fn(f64, i32, f64, f64, u8) -> Result<Houses, Error> = swisseph_bindings::houses_ex;
    let _: fn(f64, i32, f64, f64, u8) -> Result<HousesWithSpeeds, Error> =
        swisseph_bindings::houses_ex2;
    let _: fn(f64, i32, f64, f64) -> Result<GauquelinSectors, Error> =
        swisseph_bindings::houses_gauquelin;
    let _: fn(f64, f64, f64, u8) -> Result<Houses, Error> = swisseph_bindings::houses_armc;
    let _: fn(f64, f64, f64, u8, f64) -> Result<HousesWithSpeeds, Error> =
        swisseph_bindings::houses_armc_ex2;
    let _: fn(f64, f64, f64, u8, f64, f64) -> Result<HousePosition, Error> =
        swisseph_bindings::house_pos;
    let _: fn(u8) -> String = swisseph_bindings::house_name;
    let _: fn(
        f64,
        i32,
        Option<&str>,
        i32,
        i32,
        f64,
        f64,
        f64,
        f64,
        f64,
    ) -> Result<GauquelinSector, Error> = swisseph_bindings::gauquelin_sector;
    let _: fn(f64, i32, i32, i32) -> Result<NodesApsides, Error> = swisseph_bindings::nod_aps;
    let _: fn(f64, i32, i32, i32) -> Result<NodesApsides, Error> = swisseph_bindings::nod_aps_ut;
    let _: fn(
        f64,
        i32,
        Option<&str>,
        i32,
        i32,
        f64,
        f64,
        f64,
        f64,
        f64,
    ) -> Result<RiseTransitOutcome, Error> = swisseph_bindings::rise_trans;
    let _: fn(
        f64,
        i32,
        Option<&str>,
        i32,
        i32,
        f64,
        f64,
        f64,
        f64,
        f64,
        f64,
    ) -> Result<RiseTransitOutcome, Error> = swisseph_bindings::rise_trans_true_hor;
    let _: fn(f64, i32, f64, f64, f64, f64, f64, f64, f64) -> Result<Horizontal, Error> =
        swisseph_bindings::azalt;
    let _: fn(f64, i32, f64, f64, f64, f64, f64) -> Result<Celestial, Error> =
        swisseph_bindings::azalt_rev;
    let _: fn(f64, f64, f64, i32) -> Result<f64, Error> = swisseph_bindings::refrac;
    let _: fn(f64, f64, f64, f64, f64, i32) -> Result<Refraction, Error> =
        swisseph_bindings::refrac_extended;
    let _: fn(f64) -> Result<(), Error> = swisseph_bindings::set_lapse_rate;
    let _: fn(f64, i32, i32) -> Result<Phenomena, Error> = swisseph_bindings::pheno;
    let _: fn(f64, i32, i32) -> Result<Phenomena, Error> = swisseph_bindings::pheno_ut;
    let _: fn(f64, i32, i32) -> Result<OrbitalElements, Error> =
        swisseph_bindings::get_orbital_elements;
    let _: fn(f64, i32, i32) -> Result<DistanceExtremes, Error> =
        swisseph_bindings::orbit_max_min_true_distance;
    let _: fn(f64, i32, i32) -> Result<Position, Error> = swisseph_bindings::calc_ut;
    let _: fn(f64, i32, i32) -> Result<Position, Error> = swisseph_bindings::calc;
    let _: fn(f64, i32, i32, i32) -> Result<Position, Error> = swisseph_bindings::calc_pctr;
    let _: fn(i32) -> Result<String, Error> = swisseph_bindings::get_planet_name;
    let _: fn(&Position, i32) -> Result<(), Error> = swisseph_bindings::require_source_flags;
    let _: fn(&str, f64, i32) -> Result<StarPosition, Error> = swisseph_bindings::fixstar;
    let _: fn(&str, f64, i32) -> Result<StarPosition, Error> = swisseph_bindings::fixstar_ut;
    let _: fn(&str, f64, i32) -> Result<StarPosition, Error> = swisseph_bindings::fixstar2;
    let _: fn(&str, f64, i32) -> Result<StarPosition, Error> = swisseph_bindings::fixstar2_ut;
    let _: fn(&str) -> Result<StarMagnitude, Error> = swisseph_bindings::fixstar_mag;
    let _: fn(&str) -> Result<StarMagnitude, Error> = swisseph_bindings::fixstar2_mag;
    let _: fn() -> Result<Vec<StarMagnitude>, Error> = swisseph_bindings::fixed_star_catalog;
    let _: fn(i32, i32, i32, f64, Calendar) -> f64 = swisseph_bindings::julday;
    let _: fn(f64, Calendar) -> Result<CalendarDate, crate::Error> = swisseph_bindings::revjul;
    let _: fn(i32, i32, i32, i32, i32, f64, Calendar) -> Result<JulDays, Error> =
        swisseph_bindings::utc_to_jd;
    let _: fn(f64, Calendar) -> Result<UtcDateTime, Error> = swisseph_bindings::jdet_to_utc;
    let _: fn(f64, Calendar) -> Result<UtcDateTime, Error> = swisseph_bindings::jdut1_to_utc;
    let _: fn(i32, i32, i32, i32, i32, f64, f64) -> Result<UtcDateTime, Error> =
        swisseph_bindings::utc_time_zone;
    let _: fn(i32, i32, i32, f64, Calendar) -> Result<f64, Error> =
        swisseph_bindings::date_conversion;
    let _: fn(f64) -> Result<i32, Error> = swisseph_bindings::day_of_week;
    let _: fn(f64) -> Result<f64, Error> = swisseph_bindings::deltat;
    let _: fn(f64, i32) -> Result<DeltaT, Error> = swisseph_bindings::deltat_ex;
    let _: fn(f64) -> Result<EquationOfTime, Error> = swisseph_bindings::time_equ;
    let _: fn(f64, f64) -> Result<LocalTime, Error> = swisseph_bindings::lmt_to_lat;
    let _: fn(f64, f64) -> Result<LocalTime, Error> = swisseph_bindings::lat_to_lmt;
    let _: fn(f64) -> Result<f64, Error> = swisseph_bindings::sidtime;
    let _: fn(f64, f64, f64) -> Result<f64, Error> = swisseph_bindings::sidtime0;
    let _: fn() -> Result<String, crate::Error> = swisseph_bindings::version;
    let _: fn() -> Result<String, crate::Error> = swisseph_bindings::library_path;
}
