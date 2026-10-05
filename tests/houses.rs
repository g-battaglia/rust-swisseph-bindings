//! Integration checks: house speeds (`houses_ex2`), the Gauquelin
//! sector layout, ARMC variants, `house_pos`/`house_name`, the extended
//! ayanamsha calls and `gauquelin_sector`.
//!
//! Native configuration is process-global, so every sidereal-mode step
//! runs inside the single sequenced `stateful_ayanamsha` test.
//! Standalone tests use tropical houses with flags that need no data
//! files, or the Moshier model, and so depend on no shared
//! configuration. Assertions are structural (ranges, agreement,
//! provenance, error classification), never stored reference vectors.

use swisseph_bindings::{
    ErrorKind, FLG_MOSEPH, FLG_SWIEPH, MARS, SIDM_FAGAN_BRADLEY, SIDM_LAHIRI, gauquelin_sector,
    get_ayanamsa, get_ayanamsa_ex, get_ayanamsa_ex_ut, get_ayanamsa_name, get_ayanamsa_ut,
    house_name, house_pos, houses, houses_armc, houses_armc_ex2, houses_ex2, houses_gauquelin,
    set_sid_mode,
};

/// J2000.0 in Universal Time.
const J2000_UT: f64 = 2451545.0;
/// London: latitude north, longitude east (negative = west).
const LONDON_LAT: f64 = 51.5;
const LONDON_LON: f64 = -0.12;
/// Paris: latitude north, longitude east.
const PARIS_LAT: f64 = 48.85;
const PARIS_LON: f64 = 2.35;

#[test]
fn ex2_positions_agree_with_houses() {
    // Same engine, same inputs: speeds must not move the positions
    //.
    let plain = houses(J2000_UT, LONDON_LAT, LONDON_LON, b'P').expect("houses Placidus");
    let ext = houses_ex2(J2000_UT, 0, LONDON_LAT, LONDON_LON, b'P').expect("houses_ex2 Placidus");
    assert_eq!(ext.cusps, plain.cusps, "cusps must agree bitwise");
    assert_eq!(ext.angles, plain.angles, "angles must agree bitwise");
    // House cusps sweep with the diurnal rotation (~361 deg/day), so
    // speeds are large: finiteness plus a diurnal-scale band guards
    // against ABI garbage without pinning engine values.
    for (index, speed) in ext.cusp_speeds.iter().enumerate() {
        assert!(
            speed.is_finite(),
            "cusp {index} speed must be finite, got {speed}"
        );
        assert!(
            (0.0..1500.0).contains(speed),
            "cusp {index} daily speed off the diurnal scale: {speed}"
        );
    }
    for (index, speed) in ext.angle_speeds.iter().enumerate() {
        assert!(
            speed.is_finite(),
            "angle {index} speed must be finite, got {speed}"
        );
    }
    // Opposite cusps (6 apart) share the same motion at this latitude.
    for pair in 0..6 {
        assert!(
            (ext.cusp_speeds[pair] - ext.cusp_speeds[pair + 6]).abs() < 1e-6,
            "opposite-cusp speed pair {pair} must agree"
        );
    }
    // ARMC advances at the sidereal rotation rate (~360.9856 deg/day).
    assert!(
        (ext.angle_speeds[2] - 360.9856).abs() < 0.01,
        "ARMC speed must be the rotation rate, got {}",
        ext.angle_speeds[2]
    );
}

#[test]
fn ex2_rejects_gauquelin_and_non_letters() {
    let err =
        houses_ex2(J2000_UT, 0, LONDON_LAT, LONDON_LON, b'G').expect_err("G must be rejected");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    let err =
        houses_ex2(J2000_UT, 0, LONDON_LAT, LONDON_LON, b'g').expect_err("g must be rejected");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    let err =
        houses_ex2(J2000_UT, 0, LONDON_LAT, LONDON_LON, b'1').expect_err("digit must be rejected");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
}

#[test]
fn gauquelin_layout_has_36_sectors() {
    let sectors = houses_gauquelin(J2000_UT, 0, PARIS_LAT, PARIS_LON).expect("gauquelin Paris");
    assert_eq!(sectors.sectors.len(), 36);
    assert_eq!(sectors.sector_speeds.len(), 36);
    for (index, sector) in sectors.sectors.iter().enumerate() {
        assert!(
            (0.0..360.0).contains(sector),
            "sector {index} out of range: {sector}"
        );
    }
    for speed in sectors
        .sector_speeds
        .iter()
        .chain(sectors.angle_speeds.iter())
    {
        assert!(speed.is_finite(), "sector speed must be finite");
    }
    assert!(
        (0.0..360.0).contains(&sectors.angles[0]),
        "gauquelin ascendant out of range"
    );
}

#[test]
fn armc_echoes_input_and_rejects_layouts() {
    // angles[2] is the ARMC echo: the engine normalizes it to 0-360.
    let houses = houses_armc(280.0, LONDON_LAT, 23.44, b'P').expect("armc Placidus");
    assert!(
        (houses.angles[2] - 280.0).abs() < 1e-9,
        "ARMC echo, got {:?}",
        houses.angles
    );
    assert_eq!(houses.cusps.len(), 12);
    assert!(houses.cusps.iter().all(|c| (0.0..360.0).contains(c)));
    // Negative ARMC normalizes into range.
    let wrapped = houses_armc(-10.0, LONDON_LAT, 23.44, b'P').expect("negative armc");
    assert!((wrapped.angles[2] - 350.0).abs() < 1e-9);
    // Layout policy matches houses(): Gauquelin and non-letters rejected.
    let err = houses_armc(280.0, LONDON_LAT, 23.44, b'G').expect_err("G must be rejected");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    let err = houses_armc(280.0, LONDON_LAT, 23.44, b' ').expect_err("space must be rejected");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    let err = houses_armc(f64::NAN, LONDON_LAT, 23.44, b'P').expect_err("NaN must be rejected");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
}

#[test]
fn armc_ex2_speeds_and_sunshine_input() {
    let ext = houses_armc_ex2(280.0, LONDON_LAT, 23.44, b'P', 0.0).expect("armc_ex2 Placidus");
    assert_eq!(ext.cusps.len(), 12);
    assert!(ext.cusp_speeds.iter().all(|s| s.is_finite()));
    assert!(ext.angle_speeds.iter().all(|s| s.is_finite()));
    // Positions agree with the non-speed variant (same engine, same inputs).
    let plain = houses_armc(280.0, LONDON_LAT, 23.44, b'P').expect("armc Placidus");
    assert_eq!(ext.cusps, plain.cusps);
    assert_eq!(ext.angles, plain.angles);
    // The Sunshine declination input is ignored by non-Sunshine systems.
    let other = houses_armc_ex2(280.0, LONDON_LAT, 23.44, b'P', 20.0).expect("other declination");
    assert_eq!(
        ext.cusps, other.cusps,
        "declination must be ignored outside 'I'"
    );
    // Sunshine accepts an explicit declination and stays in range.
    let sunshine = houses_armc_ex2(280.0, LONDON_LAT, 23.44, b'I', 10.0).expect("sunshine");
    assert!(sunshine.cusps.iter().all(|c| (0.0..360.0).contains(c)));
    let err = houses_armc_ex2(280.0, LONDON_LAT, 23.44, b'P', f64::INFINITY)
        .expect_err("infinite declination must be rejected");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
}

#[test]
fn house_pos_round_trips_through_cusps() {
    // A cusp longitude must resolve back into its own house (self
    // consistency without any external reference data).
    let armc = 280.0;
    let houses = houses_armc(armc, LONDON_LAT, 23.44, b'P').expect("armc Placidus");
    for (index, cusp) in houses.cusps.iter().enumerate() {
        let pos = house_pos(armc, LONDON_LAT, 23.44, b'P', *cusp, 0.0).expect("house_pos");
        let house = pos.value.floor() as usize;
        assert_eq!(
            house,
            index + 1,
            "cusp {index} ({cusp}) must sit in house {}, got {} ({:?})",
            index + 1,
            pos.value,
            pos.diagnostic,
        );
    }
    // Gauquelin sectors span 1-37 (native folds 'g' to 'G' itself).
    let sector = house_pos(armc, PARIS_LAT, 23.44, b'g', 100.0, 5.0).expect("sector pos");
    assert!(
        (1.0..37.0).contains(&sector.value),
        "sector out of range: {}",
        sector.value
    );
    let err =
        house_pos(f64::NAN, LONDON_LAT, 23.44, b'P', 100.0, 0.0).expect_err("NaN must be rejected");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
}

#[test]
fn koch_circumpolar_failure_is_zero_with_diagnostic() {
    // Inside the circumpolar region Koch has no defined position: the
    // native sentinel is exactly 0.0 with the reason kept in the
    // diagnostic — data-shaped, never a Rust error.
    let pos = house_pos(0.0, 75.0, 23.44, b'K', 0.0, 66.0).expect("koch circumpolar");
    assert_eq!(pos.value, 0.0, "koch failure sentinel must be 0.0");
    assert!(!pos.diagnostic.is_empty(), "failure reason must be kept");
}

#[test]
fn house_names_match_native_table() {
    assert_eq!(house_name(b'P'), "Placidus");
    assert_eq!(house_name(b'K'), "Koch");
    assert_eq!(house_name(b'G'), "Gauquelin sectors");
    assert_eq!(house_name(b'g'), "Gauquelin sectors");
    assert_eq!(house_name(b'W'), "equal/ whole sign");
    // Unknown selectors fall through to Placidus inside the native
    // table; the mapping is preserved, not second-guessed.
    assert_eq!(house_name(b'Z'), "Placidus");
}

#[test]
fn polar_placidus_fails_natively() {
    // near the pole Placidus has no defined cusps; the engine
    // reports ERR and the binding returns a classified native error
    // with no silent fallback to another system.
    let err = houses(J2000_UT, 89.9, 0.0, b'P').expect_err("polar Placidus must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    let err = houses_ex2(J2000_UT, 0, 89.9, 0.0, b'P').expect_err("polar ex2 must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    // Porphyry stays defined at the same place: the failure is
    // system-specific, not a latitude rejection.
    let porphyry = houses(J2000_UT, 89.9, 0.0, b'O').expect("polar Porphyry");
    assert!(porphyry.cusps.iter().all(|c| c.is_finite()));
}

#[test]
fn gauquelin_sector_geometric_path() {
    // Moshier needs no data files; method 0 is purely geometric.
    let sector = gauquelin_sector(
        J2000_UT, MARS, None, FLG_MOSEPH, 0, PARIS_LON, PARIS_LAT, 0.0, 0.0, 0.0,
    )
    .expect("gauquelin Mars");
    assert!(
        (1.0..37.0).contains(&sector.value),
        "sector out of range: {}",
        sector.value
    );
    // Without latitude the result differs in general but stays in range.
    let flat = gauquelin_sector(
        J2000_UT, MARS, None, FLG_MOSEPH, 1, PARIS_LON, PARIS_LAT, 0.0, 0.0, 0.0,
    )
    .expect("gauquelin flat");
    assert!((1.0..37.0).contains(&flat.value));
    // An empty star name follows the body path, like None.
    let empty = gauquelin_sector(
        J2000_UT,
        MARS,
        Some(""),
        FLG_MOSEPH,
        0,
        PARIS_LON,
        PARIS_LAT,
        0.0,
        0.0,
        0.0,
    )
    .expect("empty star name");
    assert_eq!(empty.value, sector.value);
    // Out-of-range methods fail inside the engine with a diagnostic.
    let err = gauquelin_sector(
        J2000_UT, MARS, None, FLG_MOSEPH, 99, PARIS_LON, PARIS_LAT, 0.0, 0.0, 0.0,
    )
    .expect_err("bad method must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(
        err.message().contains("invalid method"),
        "got: {}",
        err.message()
    );
}

#[test]
fn gauquelin_sector_rejects_bad_inputs() {
    let long_name = "x".repeat(256);
    let err = gauquelin_sector(
        J2000_UT,
        MARS,
        Some(&long_name),
        FLG_MOSEPH,
        0,
        PARIS_LON,
        PARIS_LAT,
        0.0,
        0.0,
        0.0,
    )
    .expect_err("overlong star name must be rejected");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    let err = gauquelin_sector(
        f64::NAN,
        MARS,
        None,
        FLG_MOSEPH,
        0,
        PARIS_LON,
        PARIS_LAT,
        0.0,
        0.0,
        0.0,
    )
    .expect_err("NaN time must be rejected");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
}

/// Sidereal-mode configuration plus every ayanamsha read, sequenced in
/// one test because the mode is process-global.
#[test]
fn stateful_ayanamsha() {
    set_sid_mode(SIDM_LAHIRI, 0.0, 0.0).expect("lahiri mode");
    let ut = get_ayanamsa_ut(J2000_UT).expect("ayanamsa ut");
    assert!((23.0..25.0).contains(&ut), "lahiri band, got {ut}");
    let et = get_ayanamsa(J2000_UT).expect("ayanamsa et");
    assert!((23.0..25.0).contains(&et), "lahiri band, got {et}");
    // ET and UT inputs differ by Delta T (~64 s at J2000), so the two
    // values agree closely without being identical computations.
    assert!((ut - et).abs() < 1e-3, "ut/et split, got {ut} vs {et}");
    // Explicit-flag variants keep provenance and diagnostics.
    let ex = get_ayanamsa_ex(J2000_UT, FLG_SWIEPH).expect("ayanamsa ex");
    assert!((23.0..25.0).contains(&ex.value));
    assert!(ex.returned_flags >= 0, "retflag, got {}", ex.returned_flags);
    let ex_ut = get_ayanamsa_ex_ut(J2000_UT, FLG_SWIEPH).expect("ayanamsa ex ut");
    assert!((23.0..25.0).contains(&ex_ut.value));
    // The `_ex` path adds nutation while the plain path does not, so
    // they agree to nutation scale (~1 arcsec), not bitwise.
    assert!(
        (ex_ut.value - ut).abs() < 0.01,
        "nutation-scale agreement, got {} vs {ut}",
        ex_ut.value
    );
    assert_eq!(
        get_ayanamsa_name(SIDM_LAHIRI).expect("lahiri name"),
        "Lahiri"
    );
    assert_eq!(
        get_ayanamsa_name(SIDM_FAGAN_BRADLEY).expect("fagan name"),
        "Fagan/Bradley"
    );
    // Modes without a predefined name fail natively (null lookup).
    let err = get_ayanamsa_name(100).expect_err("nameless mode must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    let err = get_ayanamsa_name(-1).expect_err("negative mode must be rejected");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    set_sid_mode(SIDM_FAGAN_BRADLEY, 0.0, 0.0).expect("restore default mode");
}
