//! Freeze the 1.x function argument order before extension relocation.
//! Retarget only function paths to global/native when those modules land;
//! never derive new parameter order from primary Session methods.
use swisseph_bindings::native::*;
use swisseph_bindings::native::{
    CurrentFileData, HeliacalPhenomenaFull, NodesApsides, OccultSearchOptions,
    OccultationGeometryFull, SolarEclipseGeometryFull, VisibilityFull,
};

// Every function pointer names its extension destination explicitly; a missing
// export cannot fall back to a same-named transitional root function.

#[test]
#[expect(
    clippy::type_complexity,
    reason = "the legacy contract intentionally spells complete fn-pointer types without aliases"
)]
fn legacy_function_types_and_argument_order() {
    let _: fn(f64, f64, f64, f64) -> Result<[f64; 3], Error> = swisseph_bindings::native::cotrans;
    let _: fn(f64, f64, f64, f64, f64, f64, f64) -> Result<([f64; 3], [f64; 3]), Error> =
        swisseph_bindings::native::cotrans_sp;
    let _: fn(f64) -> f64 = swisseph_bindings::native::degnorm;
    let _: fn(f64) -> f64 = swisseph_bindings::native::radnorm;
    let _: fn(f64, f64) -> f64 = swisseph_bindings::native::difdegn;
    let _: fn(f64, f64) -> f64 = swisseph_bindings::native::difdeg2n;
    let _: fn(f64, f64) -> f64 = swisseph_bindings::native::difrad2n;
    let _: fn(f64, f64) -> f64 = swisseph_bindings::native::deg_midp;
    let _: fn(f64, f64) -> f64 = swisseph_bindings::native::rad_midp;
    let _: fn(f64) -> Result<i32, Error> = swisseph_bindings::native::d2l;
    let _: fn(i32) -> i32 = swisseph_bindings::native::csnorm;
    let _: fn(i32, i32) -> Result<i32, Error> = swisseph_bindings::native::difcsn;
    let _: fn(i32, i32) -> Result<i32, Error> = swisseph_bindings::native::difcs2n;
    let _: fn(i32) -> Result<i32, Error> = swisseph_bindings::native::csroundsec;
    let _: fn(i32, u8, bool) -> Result<String, Error> = swisseph_bindings::native::cs2timestr;
    let _: fn(i32, u8, u8) -> Result<String, Error> = swisseph_bindings::native::cs2lonlatstr;
    let _: fn(i32) -> Result<String, Error> = swisseph_bindings::native::cs2degstr;
    let _: fn(f64, i32) -> Result<SplitDeg, Error> = swisseph_bindings::native::split_deg;
    let _: fn(Option<&str>) -> Result<(), Error> = swisseph_bindings::global::set_ephe_path;
    let _: fn(Option<&str>) -> Result<(), Error> = swisseph_bindings::native::ensure_ephe_path;
    let _: fn(f64, f64, f64) -> Result<(), Error> = swisseph_bindings::global::set_topo;
    let _: fn(i32, f64, f64) -> Result<(), Error> = swisseph_bindings::global::set_sid_mode;
    let _: fn(&str) -> Result<(), Error> = swisseph_bindings::native::set_jpl_file;
    let _: fn(f64) -> Result<(), Error> = swisseph_bindings::global::set_tid_acc;
    let _: fn() -> Result<f64, Error> = swisseph_bindings::global::get_tid_acc;
    let _: fn(Option<f64>) -> Result<(), Error> = swisseph_bindings::global::set_delta_t_userdef;
    let _: fn(f64) -> Result<f64, Error> = swisseph_bindings::global::get_ayanamsa_ut;
    let _: fn(f64) -> Result<f64, Error> = swisseph_bindings::global::get_ayanamsa;
    let _: fn(f64, i32) -> Result<Ayanamsha, Error> = swisseph_bindings::global::get_ayanamsa_ex;
    let _: fn(f64, i32) -> Result<Ayanamsha, Error> = swisseph_bindings::global::get_ayanamsa_ex_ut;
    let _: fn(i32) -> Result<String, Error> = swisseph_bindings::native::get_ayanamsa_name;
    let _: fn() -> Result<(), Error> = swisseph_bindings::global::close;
    let _: fn(i32) -> Result<Option<CurrentFileData>, Error> =
        swisseph_bindings::global::get_current_file_data;
    let _: fn(f64, f64, i32) -> Result<LongitudeCrossing, Error> =
        swisseph_bindings::global::solcross;
    let _: fn(f64, f64, i32) -> Result<LongitudeCrossing, Error> =
        swisseph_bindings::global::solcross_ut;
    let _: fn(f64, f64, i32) -> Result<LongitudeCrossing, Error> =
        swisseph_bindings::global::mooncross;
    let _: fn(f64, f64, i32) -> Result<LongitudeCrossing, Error> =
        swisseph_bindings::global::mooncross_ut;
    let _: fn(f64, i32) -> Result<NodeCrossing, Error> = swisseph_bindings::global::mooncross_node;
    let _: fn(f64, i32) -> Result<NodeCrossing, Error> =
        swisseph_bindings::global::mooncross_node_ut;
    let _: fn(i32, f64, f64, i32, i32) -> Result<LongitudeCrossing, Error> =
        swisseph_bindings::global::helio_cross;
    let _: fn(i32, f64, f64, i32, i32) -> Result<LongitudeCrossing, Error> =
        swisseph_bindings::global::helio_cross_ut;
    let _: fn(f64, i32, i32, bool) -> Result<GlobalSolarEclipse, Error> =
        swisseph_bindings::global::sol_eclipse_when_glob;
    let _: fn(f64, i32, i32, bool) -> Result<GlobalLunarEclipse, Error> =
        swisseph_bindings::global::lun_eclipse_when;
    let _: fn(f64, i32) -> Result<SolarEclipseGeometryFull, Error> =
        swisseph_bindings::native::sol_eclipse_where_full;
    let _: fn(f64, i32) -> Result<SolarEclipseGeometry, Error> =
        swisseph_bindings::global::sol_eclipse_where;
    let _: fn(f64, i32, f64, f64, f64) -> Result<SolarEclipseCircumstances, Error> =
        swisseph_bindings::global::sol_eclipse_how;
    let _: fn(f64, i32, Option<(f64, f64, f64)>) -> Result<LunarEclipseCircumstances, Error> =
        swisseph_bindings::global::lun_eclipse_how;
    let _: fn(f64, i32, f64, f64, f64, bool) -> Result<LocalSolarEclipse, Error> =
        swisseph_bindings::global::sol_eclipse_when_loc;
    let _: fn(f64, i32, f64, f64, f64, bool) -> Result<LocalLunarEclipse, Error> =
        swisseph_bindings::global::lun_eclipse_when_loc;
    let _: fn(f64, i32, Option<&str>, i32, i32, bool) -> Result<GlobalOccultation, Error> =
        swisseph_bindings::global::lun_occult_when_glob;
    let _: fn(
        f64,
        i32,
        Option<&str>,
        i32,
        i32,
        OccultSearchOptions,
    ) -> Result<GlobalOccultation, Error> =
        swisseph_bindings::native::lun_occult_when_glob_with_options;
    let _: fn(f64, i32, Option<&str>, i32, f64, f64, f64, bool) -> Result<LocalOccultation, Error> =
        swisseph_bindings::global::lun_occult_when_loc;
    let _: fn(
        f64,
        i32,
        Option<&str>,
        i32,
        f64,
        f64,
        f64,
        OccultSearchOptions,
    ) -> Result<LocalOccultation, Error> =
        swisseph_bindings::native::lun_occult_when_loc_with_options;
    let _: fn(f64, i32, Option<&str>, i32) -> Result<OccultationGeometryFull, Error> =
        swisseph_bindings::native::lun_occult_where_full;
    let _: fn(f64, i32, Option<&str>, i32) -> Result<OccultationGeometry, Error> =
        swisseph_bindings::global::lun_occult_where;
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
    ) -> Result<HeliacalEvent, Error> = swisseph_bindings::global::heliacal_ut;
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
    ) -> Result<HeliacalPhenomenaFull, Error> = swisseph_bindings::native::heliacal_pheno_ut_full;
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
    ) -> Result<HeliacalPhenomena, Error> = swisseph_bindings::global::heliacal_pheno_ut;
    let _: fn(
        f64,
        f64,
        f64,
        f64,
        [f64; 4],
        [f64; 6],
        &str,
        i32,
    ) -> Result<VisibilityOutcome, Error> = swisseph_bindings::global::vis_limit_mag;
    let _: fn(f64, f64, f64, f64, [f64; 4], [f64; 6], &str, i32) -> Result<VisibilityFull, Error> =
        swisseph_bindings::native::vis_limit_mag_full;
    let _: fn(f64, f64, f64, u8) -> Result<Houses, Error> = swisseph_bindings::global::houses;
    let _: fn(f64, i32, f64, f64, u8) -> Result<Houses, Error> =
        swisseph_bindings::global::houses_ex;
    let _: fn(f64, i32, f64, f64, u8) -> Result<HousesWithSpeeds, Error> =
        swisseph_bindings::global::houses_ex2;
    let _: fn(f64, i32, f64, f64) -> Result<GauquelinSectors, Error> =
        swisseph_bindings::global::houses_gauquelin;
    let _: fn(f64, f64, f64, u8) -> Result<Houses, Error> = swisseph_bindings::global::houses_armc;
    let _: fn(f64, f64, f64, u8, f64) -> Result<HousesWithSpeeds, Error> =
        swisseph_bindings::global::houses_armc_ex2;
    let _: fn(f64, f64, f64, u8, f64, f64) -> Result<HousePosition, Error> =
        swisseph_bindings::global::house_pos;
    let _: fn(u8) -> String = swisseph_bindings::native::house_name;
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
    ) -> Result<GauquelinSector, Error> = swisseph_bindings::global::gauquelin_sector;
    let _: fn(f64, i32, i32, i32) -> Result<NodesApsides, Error> =
        swisseph_bindings::global::nod_aps;
    let _: fn(f64, i32, i32, i32) -> Result<NodesApsides, Error> =
        swisseph_bindings::global::nod_aps_ut;
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
    ) -> Result<RiseTransitOutcome, Error> = swisseph_bindings::global::rise_trans;
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
    ) -> Result<RiseTransitOutcome, Error> = swisseph_bindings::global::rise_trans_true_hor;
    let _: fn(f64, i32, f64, f64, f64, f64, f64, f64, f64) -> Result<Horizontal, Error> =
        swisseph_bindings::global::azalt;
    let _: fn(f64, i32, f64, f64, f64, f64, f64) -> Result<Celestial, Error> =
        swisseph_bindings::global::azalt_rev;
    let _: fn(f64, f64, f64, i32) -> Result<f64, Error> = swisseph_bindings::global::refrac;
    let _: fn(f64, f64, f64, f64, f64, i32) -> Result<Refraction, Error> =
        swisseph_bindings::global::refrac_extended;
    let _: fn(f64) -> Result<(), Error> = swisseph_bindings::global::set_lapse_rate;
    let _: fn(f64, i32, i32) -> Result<Phenomena, Error> = swisseph_bindings::global::pheno;
    let _: fn(f64, i32, i32) -> Result<Phenomena, Error> = swisseph_bindings::global::pheno_ut;
    let _: fn(f64, i32, i32) -> Result<OrbitalElements, Error> =
        swisseph_bindings::global::get_orbital_elements;
    let _: fn(f64, i32, i32) -> Result<DistanceExtremes, Error> =
        swisseph_bindings::global::orbit_max_min_true_distance;
    let _: fn(f64, i32, i32) -> Result<Position, Error> = swisseph_bindings::global::calc_ut;
    let _: fn(f64, i32, i32) -> Result<Position, Error> = swisseph_bindings::global::calc;
    let _: fn(f64, i32, i32, i32) -> Result<Position, Error> = swisseph_bindings::global::calc_pctr;
    let _: fn(i32) -> Result<String, Error> = swisseph_bindings::global::get_planet_name;
    let _: fn(&Position, i32) -> Result<(), Error> =
        swisseph_bindings::native::require_source_flags;
    let _: fn(&str, f64, i32) -> Result<StarPosition, Error> = swisseph_bindings::global::fixstar;
    let _: fn(&str, f64, i32) -> Result<StarPosition, Error> =
        swisseph_bindings::global::fixstar_ut;
    let _: fn(&str, f64, i32) -> Result<StarPosition, Error> = swisseph_bindings::global::fixstar2;
    let _: fn(&str, f64, i32) -> Result<StarPosition, Error> =
        swisseph_bindings::global::fixstar2_ut;
    let _: fn(&str) -> Result<StarMagnitude, Error> = swisseph_bindings::global::fixstar_mag;
    let _: fn(&str) -> Result<StarMagnitude, Error> = swisseph_bindings::global::fixstar2_mag;
    let _: fn() -> Result<Vec<StarMagnitude>, Error> =
        swisseph_bindings::native::fixed_star_catalog;
    let _: fn(i32, i32, i32, f64, Calendar) -> f64 = swisseph_bindings::native::julday;
    let _: fn(f64, Calendar) -> Result<CalendarDate, crate::Error> =
        swisseph_bindings::native::revjul;
    let _: fn(i32, i32, i32, i32, i32, f64, Calendar) -> Result<JulDays, Error> =
        swisseph_bindings::global::utc_to_jd;
    let _: fn(f64, Calendar) -> Result<UtcDateTime, Error> = swisseph_bindings::global::jdet_to_utc;
    let _: fn(f64, Calendar) -> Result<UtcDateTime, Error> =
        swisseph_bindings::global::jdut1_to_utc;
    let _: fn(i32, i32, i32, i32, i32, f64, f64) -> Result<UtcDateTime, Error> =
        swisseph_bindings::native::utc_time_zone;
    let _: fn(i32, i32, i32, f64, Calendar) -> Result<f64, Error> =
        swisseph_bindings::native::date_conversion;
    let _: fn(f64) -> Result<i32, Error> = swisseph_bindings::native::day_of_week;
    let _: fn(f64) -> Result<f64, Error> = swisseph_bindings::global::deltat;
    let _: fn(f64, i32) -> Result<DeltaT, Error> = swisseph_bindings::global::deltat_ex;
    let _: fn(f64) -> Result<EquationOfTime, Error> = swisseph_bindings::global::time_equ;
    let _: fn(f64, f64) -> Result<LocalTime, Error> = swisseph_bindings::global::lmt_to_lat;
    let _: fn(f64, f64) -> Result<LocalTime, Error> = swisseph_bindings::global::lat_to_lmt;
    let _: fn(f64) -> Result<f64, Error> = swisseph_bindings::global::sidtime;
    let _: fn(f64, f64, f64) -> Result<f64, Error> = swisseph_bindings::global::sidtime0;
    let _: fn() -> Result<String, crate::Error> = swisseph_bindings::native::version;
    let _: fn() -> Result<String, crate::Error> = swisseph_bindings::native::library_path;
}
