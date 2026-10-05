//! Generated operation accounting; see validation/generate-native-parity.py.

/// Every public free operation, with stable alphabetical wire identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Op {
    /// Public `azalt`; native `swe_azalt`.
    Azalt = 0,
    /// Public `azalt_rev`; native `swe_azalt_rev`.
    AzaltRev = 1,
    /// Public `calc`; native `swe_calc`.
    Calc = 2,
    /// Public `calc_pctr`; native `swe_calc_pctr`.
    CalcPctr = 3,
    /// Public `calc_ut`; native `swe_calc_ut`.
    CalcUt = 4,
    /// Public `close`; native `swe_close`.
    Close = 5,
    /// Public `cotrans`; native `swe_cotrans`.
    Cotrans = 6,
    /// Public `cotrans_sp`; native `swe_cotrans_sp`.
    CotransSp = 7,
    /// Public `cs2degstr`; native `swe_cs2degstr`.
    Cs2degstr = 8,
    /// Public `cs2lonlatstr`; native `swe_cs2lonlatstr`.
    Cs2lonlatstr = 9,
    /// Public `cs2timestr`; native `swe_cs2timestr`.
    Cs2timestr = 10,
    /// Public `csnorm`; native `swe_csnorm`.
    Csnorm = 11,
    /// Public `csroundsec`; native `swe_csroundsec`.
    Csroundsec = 12,
    /// Public `d2l`; native `swe_d2l`.
    D2l = 13,
    /// Public `date_conversion`; native `swe_date_conversion`.
    DateConversion = 14,
    /// Public `day_of_week`; native `swe_day_of_week`.
    DayOfWeek = 15,
    /// Public `deg_midp`; native `swe_deg_midp`.
    DegMidp = 16,
    /// Public `degnorm`; native `swe_degnorm`.
    Degnorm = 17,
    /// Public `deltat`; native `swe_deltat`.
    Deltat = 18,
    /// Public `deltat_ex`; native `swe_deltat_ex`.
    DeltatEx = 19,
    /// Public `difcs2n`; native `swe_difcs2n`.
    Difcs2n = 20,
    /// Public `difcsn`; native `swe_difcsn`.
    Difcsn = 21,
    /// Public `difdeg2n`; native `swe_difdeg2n`.
    Difdeg2n = 22,
    /// Public `difdegn`; native `swe_difdegn`.
    Difdegn = 23,
    /// Public `difrad2n`; native `swe_difrad2n`.
    Difrad2n = 24,
    /// Public `fixstar`; native `swe_fixstar`.
    Fixstar = 25,
    /// Public `fixstar2`; native `swe_fixstar2`.
    Fixstar2 = 26,
    /// Public `fixstar2_mag`; native `swe_fixstar2_mag`.
    Fixstar2Mag = 27,
    /// Public `fixstar2_ut`; native `swe_fixstar2_ut`.
    Fixstar2Ut = 28,
    /// Public `fixstar_mag`; native `swe_fixstar_mag`.
    FixstarMag = 29,
    /// Public `fixstar_ut`; native `swe_fixstar_ut`.
    FixstarUt = 30,
    /// Public `gauquelin_sector`; native `swe_gauquelin_sector`.
    GauquelinSector = 31,
    /// Public `get_ayanamsa`; native `swe_get_ayanamsa`.
    GetAyanamsa = 32,
    /// Public `get_ayanamsa_ex`; native `swe_get_ayanamsa_ex`.
    GetAyanamsaEx = 33,
    /// Public `get_ayanamsa_ex_ut`; native `swe_get_ayanamsa_ex_ut`.
    GetAyanamsaExUt = 34,
    /// Public `get_ayanamsa_name`; native `swe_get_ayanamsa_name`.
    GetAyanamsaName = 35,
    /// Public `get_ayanamsa_ut`; native `swe_get_ayanamsa_ut`.
    GetAyanamsaUt = 36,
    /// Public `get_current_file_data`; native `swe_get_current_file_data`.
    GetCurrentFileData = 37,
    /// Public `get_orbital_elements`; native `swe_get_orbital_elements`.
    GetOrbitalElements = 38,
    /// Public `get_planet_name`; native `swe_get_planet_name`.
    GetPlanetName = 39,
    /// Public `get_tid_acc`; native `swe_get_tid_acc`.
    GetTidAcc = 40,
    /// Public `heliacal_pheno_ut`; native `swe_heliacal_pheno_ut`.
    HeliacalPhenoUt = 41,
    /// Public `heliacal_ut`; native `swe_heliacal_ut`.
    HeliacalUt = 42,
    /// Public `helio_cross`; native `swe_helio_cross`.
    HelioCross = 43,
    /// Public `helio_cross_ut`; native `swe_helio_cross_ut`.
    HelioCrossUt = 44,
    /// Public `house_name`; native `swe_house_name`.
    HouseName = 45,
    /// Public `house_pos`; native `swe_house_pos`.
    HousePos = 46,
    /// Public `houses`; native `swe_houses`.
    Houses = 47,
    /// Public `houses_armc`; native `swe_houses_armc`.
    HousesArmc = 48,
    /// Public `houses_armc_ex2`; native `swe_houses_armc_ex2`.
    HousesArmcEx2 = 49,
    /// Public `houses_ex`; native `swe_houses_ex`.
    HousesEx = 50,
    /// Public `houses_ex2`; native `swe_houses_ex2`.
    HousesEx2 = 51,
    /// Public `houses_gauquelin`; native `swe_houses_ex2`.
    HousesGauquelin = 52,
    /// Public `jdet_to_utc`; native `swe_jdet_to_utc`.
    JdetToUtc = 53,
    /// Public `jdut1_to_utc`; native `swe_jdut1_to_utc`.
    Jdut1ToUtc = 54,
    /// Public `julday`; native `swe_julday`.
    Julday = 55,
    /// Public `lat_to_lmt`; native `swe_lat_to_lmt`.
    LatToLmt = 56,
    /// Public `library_path`; native `swe_get_library_path`.
    LibraryPath = 57,
    /// Public `lmt_to_lat`; native `swe_lmt_to_lat`.
    LmtToLat = 58,
    /// Public `lun_eclipse_how`; native `swe_lun_eclipse_how`.
    LunEclipseHow = 59,
    /// Public `lun_eclipse_when`; native `swe_lun_eclipse_when`.
    LunEclipseWhen = 60,
    /// Public `lun_eclipse_when_loc`; native `swe_lun_eclipse_when_loc`.
    LunEclipseWhenLoc = 61,
    /// Public `lun_occult_when_glob`; native `swe_lun_occult_when_glob`.
    LunOccultWhenGlob = 62,
    /// Public `lun_occult_when_glob_with_options`; native `swe_lun_occult_when_glob`.
    LunOccultWhenGlobWithOptions = 63,
    /// Public `lun_occult_when_loc`; native `swe_lun_occult_when_loc`.
    LunOccultWhenLoc = 64,
    /// Public `lun_occult_when_loc_with_options`; native `swe_lun_occult_when_loc`.
    LunOccultWhenLocWithOptions = 65,
    /// Public `lun_occult_where`; native `swe_lun_occult_where`.
    LunOccultWhere = 66,
    /// Public `mooncross`; native `swe_mooncross`.
    Mooncross = 67,
    /// Public `mooncross_node`; native `swe_mooncross_node`.
    MooncrossNode = 68,
    /// Public `mooncross_node_ut`; native `swe_mooncross_node_ut`.
    MooncrossNodeUt = 69,
    /// Public `mooncross_ut`; native `swe_mooncross_ut`.
    MooncrossUt = 70,
    /// Public `nod_aps`; native `swe_nod_aps`.
    NodAps = 71,
    /// Public `nod_aps_ut`; native `swe_nod_aps_ut`.
    NodApsUt = 72,
    /// Public `orbit_max_min_true_distance`; native `swe_orbit_max_min_true_distance`.
    OrbitMaxMinTrueDistance = 73,
    /// Public `pheno`; native `swe_pheno`.
    Pheno = 74,
    /// Public `pheno_ut`; native `swe_pheno_ut`.
    PhenoUt = 75,
    /// Public `rad_midp`; native `swe_rad_midp`.
    RadMidp = 76,
    /// Public `radnorm`; native `swe_radnorm`.
    Radnorm = 77,
    /// Public `refrac`; native `swe_refrac`.
    Refrac = 78,
    /// Public `refrac_extended`; native `swe_refrac_extended`.
    RefracExtended = 79,
    /// Public `require_source_flags`; native `Rust-only helper`.
    RequireSourceFlags = 80,
    /// Public `revjul`; native `swe_revjul`.
    Revjul = 81,
    /// Public `rise_trans`; native `swe_rise_trans`.
    RiseTrans = 82,
    /// Public `rise_trans_true_hor`; native `swe_rise_trans_true_hor`.
    RiseTransTrueHor = 83,
    /// Public `set_delta_t_userdef`; native `swe_set_delta_t_userdef`.
    SetDeltaTUserdef = 84,
    /// Public `set_ephe_path`; native `swe_set_ephe_path`.
    SetEphePath = 85,
    /// Public `set_jpl_file`; native `swe_set_jpl_file`.
    SetJplFile = 86,
    /// Public `set_lapse_rate`; native `swe_set_lapse_rate`.
    SetLapseRate = 87,
    /// Public `set_sid_mode`; native `swe_set_sid_mode`.
    SetSidMode = 88,
    /// Public `set_tid_acc`; native `swe_set_tid_acc`.
    SetTidAcc = 89,
    /// Public `set_topo`; native `swe_set_topo`.
    SetTopo = 90,
    /// Public `sidtime`; native `swe_sidtime`.
    Sidtime = 91,
    /// Public `sidtime0`; native `swe_sidtime0`.
    Sidtime0 = 92,
    /// Public `sol_eclipse_how`; native `swe_sol_eclipse_how`.
    SolEclipseHow = 93,
    /// Public `sol_eclipse_when_glob`; native `swe_sol_eclipse_when_glob`.
    SolEclipseWhenGlob = 94,
    /// Public `sol_eclipse_when_loc`; native `swe_sol_eclipse_when_loc`.
    SolEclipseWhenLoc = 95,
    /// Public `sol_eclipse_where`; native `swe_sol_eclipse_where`.
    SolEclipseWhere = 96,
    /// Public `solcross`; native `swe_solcross`.
    Solcross = 97,
    /// Public `solcross_ut`; native `swe_solcross_ut`.
    SolcrossUt = 98,
    /// Public `split_deg`; native `swe_split_deg`.
    SplitDeg = 99,
    /// Public `time_equ`; native `swe_time_equ`.
    TimeEqu = 100,
    /// Public `utc_time_zone`; native `swe_utc_time_zone`.
    UtcTimeZone = 101,
    /// Public `utc_to_jd`; native `swe_utc_to_jd`.
    UtcToJd = 102,
    /// Public `version`; native `swe_version`.
    Version = 103,
    /// Public `vis_limit_mag`; native `swe_vis_limit_mag`.
    VisLimitMag = 104,
}

/// Independent C-buffer to public-result projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// `Ayanamsha` field ordering is defined in the independent comparator.
    Ayanamsha,
    /// `Calendar` field ordering is defined in the independent comparator.
    Calendar,
    /// `Circumstances` field ordering is defined in the independent comparator.
    Circumstances,
    /// `Crossing` field ordering is defined in the independent comparator.
    Crossing,
    /// `Distances` field ordering is defined in the independent comparator.
    Distances,
    /// `File` field ordering is defined in the independent comparator.
    File,
    /// `Geometry` field ordering is defined in the independent comparator.
    Geometry,
    /// `GlobalEvent` field ordering is defined in the independent comparator.
    GlobalEvent,
    /// `Heliacal` field ordering is defined in the independent comparator.
    Heliacal,
    /// `HeliacalPhenomena` field ordering is defined in the independent comparator.
    HeliacalPhenomena,
    /// `HouseSpeeds` field ordering is defined in the independent comparator.
    HouseSpeeds,
    /// `Houses` field ordering is defined in the independent comparator.
    Houses,
    /// `Integer` field ordering is defined in the independent comparator.
    Integer,
    /// `JulDays` field ordering is defined in the independent comparator.
    JulDays,
    /// `LocalEvent` field ordering is defined in the independent comparator.
    LocalEvent,
    /// `LocalSolar` field ordering is defined in the independent comparator.
    LocalSolar,
    /// `Magnitude` field ordering is defined in the independent comparator.
    Magnitude,
    /// `NodeCrossing` field ordering is defined in the independent comparator.
    NodeCrossing,
    /// `Nodes` field ordering is defined in the independent comparator.
    Nodes,
    /// `Orbital` field ordering is defined in the independent comparator.
    Orbital,
    /// `Phenomena` field ordering is defined in the independent comparator.
    Phenomena,
    /// `Position` field ordering is defined in the independent comparator.
    Position,
    /// `PureScalar` field ordering is defined in the independent comparator.
    PureScalar,
    /// `Rise` field ordering is defined in the independent comparator.
    Rise,
    /// `Scalar` field ordering is defined in the independent comparator.
    Scalar,
    /// `ScalarDiagnostic` field ordering is defined in the independent comparator.
    ScalarDiagnostic,
    /// `Sectors` field ordering is defined in the independent comparator.
    Sectors,
    /// `Split` field ordering is defined in the independent comparator.
    Split,
    /// `Star` field ordering is defined in the independent comparator.
    Star,
    /// `Text` field ordering is defined in the independent comparator.
    Text,
    /// `Unit` field ordering is defined in the independent comparator.
    Unit,
    /// `Utc` field ordering is defined in the independent comparator.
    Utc,
    /// `Vector` field ordering is defined in the independent comparator.
    Vector,
    /// `Visibility` field ordering is defined in the independent comparator.
    Visibility,
}

/// Declaration inventory entry, not a claim of executed coverage.
pub struct Operation {
    /// Public free function identifier.
    pub op: Op,
    /// Canonical public name.
    pub name: &'static str,
    /// C symbol; empty only for the Rust-only source policy.
    pub native: &'static str,
    /// Owned Rust family containing the public entry point.
    pub family: &'static str,
    /// Result projection, including native padding.
    pub layout: Layout,
}

/// Complete mapped public surface.
pub const OPERATIONS: &[Operation] = &[
    Operation {
        op: Op::Azalt,
        name: "azalt",
        native: "swe_azalt",
        family: "observer",
        layout: Layout::Vector,
    },
    Operation {
        op: Op::AzaltRev,
        name: "azalt_rev",
        native: "swe_azalt_rev",
        family: "observer",
        layout: Layout::Vector,
    },
    Operation {
        op: Op::Calc,
        name: "calc",
        native: "swe_calc",
        family: "positions",
        layout: Layout::Position,
    },
    Operation {
        op: Op::CalcPctr,
        name: "calc_pctr",
        native: "swe_calc_pctr",
        family: "positions",
        layout: Layout::Position,
    },
    Operation {
        op: Op::CalcUt,
        name: "calc_ut",
        native: "swe_calc_ut",
        family: "positions",
        layout: Layout::Position,
    },
    Operation {
        op: Op::Close,
        name: "close",
        native: "swe_close",
        family: "config",
        layout: Layout::Unit,
    },
    Operation {
        op: Op::Cotrans,
        name: "cotrans",
        native: "swe_cotrans",
        family: "angles",
        layout: Layout::Vector,
    },
    Operation {
        op: Op::CotransSp,
        name: "cotrans_sp",
        native: "swe_cotrans_sp",
        family: "angles",
        layout: Layout::Vector,
    },
    Operation {
        op: Op::Cs2degstr,
        name: "cs2degstr",
        native: "swe_cs2degstr",
        family: "angles",
        layout: Layout::Text,
    },
    Operation {
        op: Op::Cs2lonlatstr,
        name: "cs2lonlatstr",
        native: "swe_cs2lonlatstr",
        family: "angles",
        layout: Layout::Text,
    },
    Operation {
        op: Op::Cs2timestr,
        name: "cs2timestr",
        native: "swe_cs2timestr",
        family: "angles",
        layout: Layout::Text,
    },
    Operation {
        op: Op::Csnorm,
        name: "csnorm",
        native: "swe_csnorm",
        family: "angles",
        layout: Layout::Integer,
    },
    Operation {
        op: Op::Csroundsec,
        name: "csroundsec",
        native: "swe_csroundsec",
        family: "angles",
        layout: Layout::Integer,
    },
    Operation {
        op: Op::D2l,
        name: "d2l",
        native: "swe_d2l",
        family: "angles",
        layout: Layout::Integer,
    },
    Operation {
        op: Op::DateConversion,
        name: "date_conversion",
        native: "swe_date_conversion",
        family: "time",
        layout: Layout::Scalar,
    },
    Operation {
        op: Op::DayOfWeek,
        name: "day_of_week",
        native: "swe_day_of_week",
        family: "time",
        layout: Layout::Integer,
    },
    Operation {
        op: Op::DegMidp,
        name: "deg_midp",
        native: "swe_deg_midp",
        family: "angles",
        layout: Layout::PureScalar,
    },
    Operation {
        op: Op::Degnorm,
        name: "degnorm",
        native: "swe_degnorm",
        family: "angles",
        layout: Layout::PureScalar,
    },
    Operation {
        op: Op::Deltat,
        name: "deltat",
        native: "swe_deltat",
        family: "time",
        layout: Layout::Scalar,
    },
    Operation {
        op: Op::DeltatEx,
        name: "deltat_ex",
        native: "swe_deltat_ex",
        family: "time",
        layout: Layout::ScalarDiagnostic,
    },
    Operation {
        op: Op::Difcs2n,
        name: "difcs2n",
        native: "swe_difcs2n",
        family: "angles",
        layout: Layout::Integer,
    },
    Operation {
        op: Op::Difcsn,
        name: "difcsn",
        native: "swe_difcsn",
        family: "angles",
        layout: Layout::Integer,
    },
    Operation {
        op: Op::Difdeg2n,
        name: "difdeg2n",
        native: "swe_difdeg2n",
        family: "angles",
        layout: Layout::PureScalar,
    },
    Operation {
        op: Op::Difdegn,
        name: "difdegn",
        native: "swe_difdegn",
        family: "angles",
        layout: Layout::PureScalar,
    },
    Operation {
        op: Op::Difrad2n,
        name: "difrad2n",
        native: "swe_difrad2n",
        family: "angles",
        layout: Layout::PureScalar,
    },
    Operation {
        op: Op::Fixstar,
        name: "fixstar",
        native: "swe_fixstar",
        family: "stars",
        layout: Layout::Star,
    },
    Operation {
        op: Op::Fixstar2,
        name: "fixstar2",
        native: "swe_fixstar2",
        family: "stars",
        layout: Layout::Star,
    },
    Operation {
        op: Op::Fixstar2Mag,
        name: "fixstar2_mag",
        native: "swe_fixstar2_mag",
        family: "stars",
        layout: Layout::Magnitude,
    },
    Operation {
        op: Op::Fixstar2Ut,
        name: "fixstar2_ut",
        native: "swe_fixstar2_ut",
        family: "stars",
        layout: Layout::Star,
    },
    Operation {
        op: Op::FixstarMag,
        name: "fixstar_mag",
        native: "swe_fixstar_mag",
        family: "stars",
        layout: Layout::Magnitude,
    },
    Operation {
        op: Op::FixstarUt,
        name: "fixstar_ut",
        native: "swe_fixstar_ut",
        family: "stars",
        layout: Layout::Star,
    },
    Operation {
        op: Op::GauquelinSector,
        name: "gauquelin_sector",
        native: "swe_gauquelin_sector",
        family: "houses",
        layout: Layout::ScalarDiagnostic,
    },
    Operation {
        op: Op::GetAyanamsa,
        name: "get_ayanamsa",
        native: "swe_get_ayanamsa",
        family: "config",
        layout: Layout::Scalar,
    },
    Operation {
        op: Op::GetAyanamsaEx,
        name: "get_ayanamsa_ex",
        native: "swe_get_ayanamsa_ex",
        family: "config",
        layout: Layout::Ayanamsha,
    },
    Operation {
        op: Op::GetAyanamsaExUt,
        name: "get_ayanamsa_ex_ut",
        native: "swe_get_ayanamsa_ex_ut",
        family: "config",
        layout: Layout::Ayanamsha,
    },
    Operation {
        op: Op::GetAyanamsaName,
        name: "get_ayanamsa_name",
        native: "swe_get_ayanamsa_name",
        family: "config",
        layout: Layout::Text,
    },
    Operation {
        op: Op::GetAyanamsaUt,
        name: "get_ayanamsa_ut",
        native: "swe_get_ayanamsa_ut",
        family: "config",
        layout: Layout::Scalar,
    },
    Operation {
        op: Op::GetCurrentFileData,
        name: "get_current_file_data",
        native: "swe_get_current_file_data",
        family: "config",
        layout: Layout::File,
    },
    Operation {
        op: Op::GetOrbitalElements,
        name: "get_orbital_elements",
        native: "swe_get_orbital_elements",
        family: "observer",
        layout: Layout::Orbital,
    },
    Operation {
        op: Op::GetPlanetName,
        name: "get_planet_name",
        native: "swe_get_planet_name",
        family: "positions",
        layout: Layout::Text,
    },
    Operation {
        op: Op::GetTidAcc,
        name: "get_tid_acc",
        native: "swe_get_tid_acc",
        family: "config",
        layout: Layout::Scalar,
    },
    Operation {
        op: Op::HeliacalPhenoUt,
        name: "heliacal_pheno_ut",
        native: "swe_heliacal_pheno_ut",
        family: "events",
        layout: Layout::HeliacalPhenomena,
    },
    Operation {
        op: Op::HeliacalUt,
        name: "heliacal_ut",
        native: "swe_heliacal_ut",
        family: "events",
        layout: Layout::Heliacal,
    },
    Operation {
        op: Op::HelioCross,
        name: "helio_cross",
        native: "swe_helio_cross",
        family: "events",
        layout: Layout::Crossing,
    },
    Operation {
        op: Op::HelioCrossUt,
        name: "helio_cross_ut",
        native: "swe_helio_cross_ut",
        family: "events",
        layout: Layout::Crossing,
    },
    Operation {
        op: Op::HouseName,
        name: "house_name",
        native: "swe_house_name",
        family: "houses",
        layout: Layout::Text,
    },
    Operation {
        op: Op::HousePos,
        name: "house_pos",
        native: "swe_house_pos",
        family: "houses",
        layout: Layout::ScalarDiagnostic,
    },
    Operation {
        op: Op::Houses,
        name: "houses",
        native: "swe_houses",
        family: "houses",
        layout: Layout::Houses,
    },
    Operation {
        op: Op::HousesArmc,
        name: "houses_armc",
        native: "swe_houses_armc",
        family: "houses",
        layout: Layout::Houses,
    },
    Operation {
        op: Op::HousesArmcEx2,
        name: "houses_armc_ex2",
        native: "swe_houses_armc_ex2",
        family: "houses",
        layout: Layout::HouseSpeeds,
    },
    Operation {
        op: Op::HousesEx,
        name: "houses_ex",
        native: "swe_houses_ex",
        family: "houses",
        layout: Layout::Houses,
    },
    Operation {
        op: Op::HousesEx2,
        name: "houses_ex2",
        native: "swe_houses_ex2",
        family: "houses",
        layout: Layout::HouseSpeeds,
    },
    Operation {
        op: Op::HousesGauquelin,
        name: "houses_gauquelin",
        native: "swe_houses_ex2",
        family: "houses",
        layout: Layout::Sectors,
    },
    Operation {
        op: Op::JdetToUtc,
        name: "jdet_to_utc",
        native: "swe_jdet_to_utc",
        family: "time",
        layout: Layout::Utc,
    },
    Operation {
        op: Op::Jdut1ToUtc,
        name: "jdut1_to_utc",
        native: "swe_jdut1_to_utc",
        family: "time",
        layout: Layout::Utc,
    },
    Operation {
        op: Op::Julday,
        name: "julday",
        native: "swe_julday",
        family: "time",
        layout: Layout::PureScalar,
    },
    Operation {
        op: Op::LatToLmt,
        name: "lat_to_lmt",
        native: "swe_lat_to_lmt",
        family: "time",
        layout: Layout::ScalarDiagnostic,
    },
    Operation {
        op: Op::LibraryPath,
        name: "library_path",
        native: "swe_get_library_path",
        family: "version",
        layout: Layout::Text,
    },
    Operation {
        op: Op::LmtToLat,
        name: "lmt_to_lat",
        native: "swe_lmt_to_lat",
        family: "time",
        layout: Layout::ScalarDiagnostic,
    },
    Operation {
        op: Op::LunEclipseHow,
        name: "lun_eclipse_how",
        native: "swe_lun_eclipse_how",
        family: "events",
        layout: Layout::Circumstances,
    },
    Operation {
        op: Op::LunEclipseWhen,
        name: "lun_eclipse_when",
        native: "swe_lun_eclipse_when",
        family: "events",
        layout: Layout::GlobalEvent,
    },
    Operation {
        op: Op::LunEclipseWhenLoc,
        name: "lun_eclipse_when_loc",
        native: "swe_lun_eclipse_when_loc",
        family: "events",
        layout: Layout::LocalEvent,
    },
    Operation {
        op: Op::LunOccultWhenGlob,
        name: "lun_occult_when_glob",
        native: "swe_lun_occult_when_glob",
        family: "events",
        layout: Layout::GlobalEvent,
    },
    Operation {
        op: Op::LunOccultWhenGlobWithOptions,
        name: "lun_occult_when_glob_with_options",
        native: "swe_lun_occult_when_glob",
        family: "events",
        layout: Layout::GlobalEvent,
    },
    Operation {
        op: Op::LunOccultWhenLoc,
        name: "lun_occult_when_loc",
        native: "swe_lun_occult_when_loc",
        family: "events",
        layout: Layout::LocalEvent,
    },
    Operation {
        op: Op::LunOccultWhenLocWithOptions,
        name: "lun_occult_when_loc_with_options",
        native: "swe_lun_occult_when_loc",
        family: "events",
        layout: Layout::LocalEvent,
    },
    Operation {
        op: Op::LunOccultWhere,
        name: "lun_occult_where",
        native: "swe_lun_occult_where",
        family: "events",
        layout: Layout::Geometry,
    },
    Operation {
        op: Op::Mooncross,
        name: "mooncross",
        native: "swe_mooncross",
        family: "events",
        layout: Layout::Crossing,
    },
    Operation {
        op: Op::MooncrossNode,
        name: "mooncross_node",
        native: "swe_mooncross_node",
        family: "events",
        layout: Layout::NodeCrossing,
    },
    Operation {
        op: Op::MooncrossNodeUt,
        name: "mooncross_node_ut",
        native: "swe_mooncross_node_ut",
        family: "events",
        layout: Layout::NodeCrossing,
    },
    Operation {
        op: Op::MooncrossUt,
        name: "mooncross_ut",
        native: "swe_mooncross_ut",
        family: "events",
        layout: Layout::Crossing,
    },
    Operation {
        op: Op::NodAps,
        name: "nod_aps",
        native: "swe_nod_aps",
        family: "observer",
        layout: Layout::Nodes,
    },
    Operation {
        op: Op::NodApsUt,
        name: "nod_aps_ut",
        native: "swe_nod_aps_ut",
        family: "observer",
        layout: Layout::Nodes,
    },
    Operation {
        op: Op::OrbitMaxMinTrueDistance,
        name: "orbit_max_min_true_distance",
        native: "swe_orbit_max_min_true_distance",
        family: "observer",
        layout: Layout::Distances,
    },
    Operation {
        op: Op::Pheno,
        name: "pheno",
        native: "swe_pheno",
        family: "observer",
        layout: Layout::Phenomena,
    },
    Operation {
        op: Op::PhenoUt,
        name: "pheno_ut",
        native: "swe_pheno_ut",
        family: "observer",
        layout: Layout::Phenomena,
    },
    Operation {
        op: Op::RadMidp,
        name: "rad_midp",
        native: "swe_rad_midp",
        family: "angles",
        layout: Layout::PureScalar,
    },
    Operation {
        op: Op::Radnorm,
        name: "radnorm",
        native: "swe_radnorm",
        family: "angles",
        layout: Layout::PureScalar,
    },
    Operation {
        op: Op::Refrac,
        name: "refrac",
        native: "swe_refrac",
        family: "observer",
        layout: Layout::Scalar,
    },
    Operation {
        op: Op::RefracExtended,
        name: "refrac_extended",
        native: "swe_refrac_extended",
        family: "observer",
        layout: Layout::Vector,
    },
    Operation {
        op: Op::RequireSourceFlags,
        name: "require_source_flags",
        native: "",
        family: "positions",
        layout: Layout::Unit,
    },
    Operation {
        op: Op::Revjul,
        name: "revjul",
        native: "swe_revjul",
        family: "time",
        layout: Layout::Calendar,
    },
    Operation {
        op: Op::RiseTrans,
        name: "rise_trans",
        native: "swe_rise_trans",
        family: "observer",
        layout: Layout::Rise,
    },
    Operation {
        op: Op::RiseTransTrueHor,
        name: "rise_trans_true_hor",
        native: "swe_rise_trans_true_hor",
        family: "observer",
        layout: Layout::Rise,
    },
    Operation {
        op: Op::SetDeltaTUserdef,
        name: "set_delta_t_userdef",
        native: "swe_set_delta_t_userdef",
        family: "config",
        layout: Layout::Unit,
    },
    Operation {
        op: Op::SetEphePath,
        name: "set_ephe_path",
        native: "swe_set_ephe_path",
        family: "config",
        layout: Layout::Unit,
    },
    Operation {
        op: Op::SetJplFile,
        name: "set_jpl_file",
        native: "swe_set_jpl_file",
        family: "config",
        layout: Layout::Unit,
    },
    Operation {
        op: Op::SetLapseRate,
        name: "set_lapse_rate",
        native: "swe_set_lapse_rate",
        family: "observer",
        layout: Layout::Unit,
    },
    Operation {
        op: Op::SetSidMode,
        name: "set_sid_mode",
        native: "swe_set_sid_mode",
        family: "config",
        layout: Layout::Unit,
    },
    Operation {
        op: Op::SetTidAcc,
        name: "set_tid_acc",
        native: "swe_set_tid_acc",
        family: "config",
        layout: Layout::Unit,
    },
    Operation {
        op: Op::SetTopo,
        name: "set_topo",
        native: "swe_set_topo",
        family: "config",
        layout: Layout::Unit,
    },
    Operation {
        op: Op::Sidtime,
        name: "sidtime",
        native: "swe_sidtime",
        family: "time",
        layout: Layout::Scalar,
    },
    Operation {
        op: Op::Sidtime0,
        name: "sidtime0",
        native: "swe_sidtime0",
        family: "time",
        layout: Layout::Scalar,
    },
    Operation {
        op: Op::SolEclipseHow,
        name: "sol_eclipse_how",
        native: "swe_sol_eclipse_how",
        family: "events",
        layout: Layout::Circumstances,
    },
    Operation {
        op: Op::SolEclipseWhenGlob,
        name: "sol_eclipse_when_glob",
        native: "swe_sol_eclipse_when_glob",
        family: "events",
        layout: Layout::GlobalEvent,
    },
    Operation {
        op: Op::SolEclipseWhenLoc,
        name: "sol_eclipse_when_loc",
        native: "swe_sol_eclipse_when_loc",
        family: "events",
        layout: Layout::LocalSolar,
    },
    Operation {
        op: Op::SolEclipseWhere,
        name: "sol_eclipse_where",
        native: "swe_sol_eclipse_where",
        family: "events",
        layout: Layout::Geometry,
    },
    Operation {
        op: Op::Solcross,
        name: "solcross",
        native: "swe_solcross",
        family: "events",
        layout: Layout::Crossing,
    },
    Operation {
        op: Op::SolcrossUt,
        name: "solcross_ut",
        native: "swe_solcross_ut",
        family: "events",
        layout: Layout::Crossing,
    },
    Operation {
        op: Op::SplitDeg,
        name: "split_deg",
        native: "swe_split_deg",
        family: "angles",
        layout: Layout::Split,
    },
    Operation {
        op: Op::TimeEqu,
        name: "time_equ",
        native: "swe_time_equ",
        family: "time",
        layout: Layout::ScalarDiagnostic,
    },
    Operation {
        op: Op::UtcTimeZone,
        name: "utc_time_zone",
        native: "swe_utc_time_zone",
        family: "time",
        layout: Layout::Utc,
    },
    Operation {
        op: Op::UtcToJd,
        name: "utc_to_jd",
        native: "swe_utc_to_jd",
        family: "time",
        layout: Layout::JulDays,
    },
    Operation {
        op: Op::Version,
        name: "version",
        native: "swe_version",
        family: "version",
        layout: Layout::Text,
    },
    Operation {
        op: Op::VisLimitMag,
        name: "vis_limit_mag",
        native: "swe_vis_limit_mag",
        family: "events",
        layout: Layout::Visibility,
    },
];

impl Op {
    /// Reject unknown wire identifiers instead of guessing a declaration.
    pub fn from_wire(id: u32) -> Option<Self> {
        OPERATIONS.get(id as usize).map(|entry| entry.op)
    }
    /// Retrieve the frozen declaration and result projection.
    pub fn entry(self) -> &'static Operation {
        &OPERATIONS[self as usize]
    }
}
