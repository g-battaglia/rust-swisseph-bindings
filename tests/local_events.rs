//! Integration checks: local eclipse searches  and lunar
//! occultations.
//!
//! Native configuration is process-global, so every state-dependent step
//! runs inside the sequenced `stateful_*` tests. Standalone tests use
//! pure-Rust input validation only, which reaches no native code.
//! Assertions are structural (ranges, ordering, error classification,
//! same-engine consistency), never stored reference vectors.

use swisseph_bindings::{
    ECL_1ST_VISIBLE, ECL_2ND_VISIBLE, ECL_3RD_VISIBLE, ECL_4TH_VISIBLE, ECL_ANNULAR, ECL_CENTRAL,
    ECL_MAX_VISIBLE, ECL_PARTIAL, ECL_PENUMBRAL, ECL_TOTAL, ECL_VISIBLE, ErrorKind, FLG_MOSEPH,
    FLG_SWIEPH, MOON, SUN, VENUS, calc_ut, lun_eclipse_when_loc, lun_occult_when_glob,
    lun_occult_when_loc, lun_occult_where, require_source_flags, set_ephe_path,
    sol_eclipse_when_loc,
};

#[path = "data_dir.rs"]
mod test_data;

/// J2000.0 as a Julian Day label.
const J2000: f64 = 2451545.0;
/// 2024-01-01 00:00 UT, search epoch for the eclipse tests.
const Y2024: f64 = 2460310.5;

/// Event type flags do not identify the ephemeris source. Require actual
/// SWIEPH Sun/Moon positions before accepting file-based event checks.
fn require_eclipse_data(jd: f64) {
    for body in [SUN, MOON] {
        let position = calc_ut(jd, body, FLG_SWIEPH).expect("eclipse data preflight");
        require_source_flags(&position, FLG_SWIEPH).expect("eclipse data must use SWIEPH");
    }
}

#[test]
fn local_calls_reject_non_finite_input() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for result in [
            sol_eclipse_when_loc(bad, FLG_SWIEPH, -96.8, 32.8, 0.0, false).map(|_| ()),
            sol_eclipse_when_loc(Y2024, FLG_SWIEPH, bad, 32.8, 0.0, false).map(|_| ()),
            sol_eclipse_when_loc(Y2024, FLG_SWIEPH, -96.8, 32.8, bad, false).map(|_| ()),
            lun_eclipse_when_loc(bad, FLG_SWIEPH, 0.0, 51.5, 0.0, false).map(|_| ()),
            lun_eclipse_when_loc(Y2024, FLG_SWIEPH, 0.0, bad, 0.0, false).map(|_| ()),
            lun_occult_when_glob(bad, VENUS, None, FLG_MOSEPH, 0, false).map(|_| ()),
            lun_occult_where(bad, VENUS, None, FLG_MOSEPH).map(|_| ()),
            lun_occult_when_loc(bad, VENUS, None, FLG_MOSEPH, 0.0, 51.5, 0.0, false).map(|_| ()),
            lun_occult_when_loc(J2000, VENUS, None, FLG_MOSEPH, bad, 51.5, 0.0, false).map(|_| ()),
        ] {
            let err = result.expect_err("non-finite input must be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidInput);
        }
    }
}

#[test]
fn occult_calls_reject_bad_star_and_moon_body_path() {
    let overlong = "x".repeat(512);
    for result in [
        lun_occult_when_glob(J2000, VENUS, Some("Alde\0baran"), FLG_MOSEPH, 0, false).map(|_| ()),
        lun_occult_when_glob(J2000, VENUS, Some(&overlong), FLG_MOSEPH, 0, false).map(|_| ()),
        lun_occult_when_loc(
            J2000,
            VENUS,
            Some("Alde\0baran"),
            FLG_MOSEPH,
            0.0,
            51.5,
            0.0,
            false,
        )
        .map(|_| ()),
        lun_occult_where(J2000, VENUS, Some(&overlong), FLG_MOSEPH).map(|_| ()),
    ] {
        let err = result.expect_err("bad star name must be rejected");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
    // The Moon has no lunar occultation of its own: the native search
    // does not terminate, so the body path is rejected before any call.
    // A real star name still selects the star path and is allowed.
    for result in [
        lun_occult_when_glob(J2000, MOON, None, FLG_MOSEPH, 0, false).map(|_| ()),
        lun_occult_when_glob(J2000, MOON, Some(""), FLG_MOSEPH, 0, false).map(|_| ()),
        lun_occult_when_loc(J2000, MOON, None, FLG_MOSEPH, 0.0, 51.5, 0.0, false).map(|_| ()),
        lun_occult_where(J2000, MOON, None, FLG_MOSEPH).map(|_| ()),
    ] {
        let err = result.expect_err("Moon body path must be rejected");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
}

#[test]
fn stateful_local_solar_eclipse() {
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP file-based assertions: no ephemeris data found; set SWISSEPH_EPHE_DIR \
             to a directory containing the .se1 files"
        );
        return;
    };
    set_ephe_path(Some(&data_dir)).expect("data path");
    require_eclipse_data(Y2024);

    // --- next solar eclipse visible from Dallas after 2024-01-01: the
    // --- 2024-04-08 total eclipse, every contact visible ---
    let ecl =
        sol_eclipse_when_loc(Y2024, FLG_SWIEPH, -96.8, 32.8, 0.0, false).expect("Dallas search");
    assert_eq!(
        ecl.eclipse_type,
        ECL_TOTAL
            | ECL_VISIBLE
            | ECL_MAX_VISIBLE
            | ECL_1ST_VISIBLE
            | ECL_2ND_VISIBLE
            | ECL_3RD_VISIBLE
            | ECL_4TH_VISIBLE
    );
    assert!(ecl.maximum() > Y2024 && ecl.maximum() < Y2024 + 120.0);
    assert!(ecl.first_contact() <= ecl.maximum() && ecl.maximum() <= ecl.fourth_contact());
    assert!(ecl.second_contact() > 0.0 && ecl.third_contact() > ecl.second_contact());
    // No sunrise/sunset span: the whole event happens while the Sun is up.
    assert_eq!(ecl.sunrise(), 0.0);
    assert_eq!(ecl.sunset(), 0.0);
    // Total circumstances at maximum: saros 139/30, magnitude above one.
    assert!(ecl.magnitude() > 1.0);
    assert_eq!(ecl.saros_series(), 139.0);
    assert_eq!(ecl.saros_member(), 30.0);
    assert!(ecl.attributes[11..20].iter().all(|&v| v == 0.0));
    // Bitwise repeat calls.
    assert_eq!(
        sol_eclipse_when_loc(Y2024, FLG_SWIEPH, -96.8, 32.8, 0.0, false).expect("repeat"),
        ecl
    );

    // --- backward search from just past the maximum finds the same event ---
    let back = sol_eclipse_when_loc(2460409.3, FLG_SWIEPH, -96.8, 32.8, 0.0, true)
        .expect("backward search");
    assert!(back.maximum() < 2460409.3);
    assert_eq!(back.maximum(), ecl.maximum());

    // --- observer altitude outside -500..25000 m fails natively ---
    let err = sol_eclipse_when_loc(Y2024, FLG_SWIEPH, -96.8, 32.8, 30000.0, false)
        .expect_err("altitude range must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(err.to_string().contains("must be between"), "{}", err);
}

#[test]
fn stateful_local_lunar_eclipse() {
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP file-based assertions: no ephemeris data found; set SWISSEPH_EPHE_DIR \
             to a directory containing the .se1 files"
        );
        return;
    };
    set_ephe_path(Some(&data_dir)).expect("data path");
    require_eclipse_data(Y2024);

    // --- next lunar eclipse visible from London after 2024-02-15: the
    // --- 2024-03-25 penumbral eclipse, penumbral phases only ---
    let pen = lun_eclipse_when_loc(2460355.5, FLG_SWIEPH, 0.0, 51.5, 0.0, false)
        .expect("penumbral search");
    assert_eq!(
        pen.eclipse_type & (ECL_PENUMBRAL | ECL_VISIBLE),
        ECL_PENUMBRAL | ECL_VISIBLE
    );
    assert!(pen.maximum() > 2460355.5 && pen.maximum() < 2460355.5 + 60.0);
    // No partial/total phase: those contacts stay absent.
    assert_eq!(pen.partial_begin(), 0.0);
    assert_eq!(pen.partial_end(), 0.0);
    assert_eq!(pen.total_begin(), 0.0);
    assert_eq!(pen.total_end(), 0.0);
    assert!(pen.penumbral_begin() > 0.0);
    // The penumbral end is clipped by moonset: it reads 0.0 while the
    // moonset slot carries the event end (which coincides with maximum
    // here).
    assert_eq!(pen.penumbral_end(), 0.0);
    assert_eq!(pen.moonset(), pen.maximum());
    assert_eq!(pen.umbral_magnitude(), 0.0);
    assert!(pen.penumbral_magnitude() > 0.0);

    // --- next one after the penumbral event: the 2024-09-18 partial
    // --- eclipse, partial phases visible from London ---
    let part =
        lun_eclipse_when_loc(2460400.0, FLG_SWIEPH, 0.0, 51.5, 0.0, false).expect("partial search");
    assert_eq!(
        part.eclipse_type & (ECL_PARTIAL | ECL_VISIBLE),
        ECL_PARTIAL | ECL_VISIBLE
    );
    assert!(part.maximum() > 2460400.0 && part.maximum() < 2460400.0 + 200.0);
    assert!(part.partial_begin() <= part.maximum() && part.maximum() <= part.partial_end());
    assert!(part.penumbral_begin() <= part.partial_begin());
    assert!(part.partial_end() <= part.penumbral_end());
    // September 2024 was partial, not total.
    assert_eq!(part.total_begin(), 0.0);
    assert_eq!(part.total_end(), 0.0);
    assert!(part.umbral_magnitude() > 0.0);
    assert!(part.penumbral_magnitude() > part.umbral_magnitude());
    assert!(part.attributes[11..20].iter().all(|&v| v == 0.0));
    assert_eq!(
        lun_eclipse_when_loc(2460400.0, FLG_SWIEPH, 0.0, 51.5, 0.0, false).expect("repeat"),
        part
    );

    // --- backward search finds the penumbral event again ---
    let back =
        lun_eclipse_when_loc(2460400.0, FLG_SWIEPH, 0.0, 51.5, 0.0, true).expect("backward search");
    assert!(back.maximum() < 2460400.0);
    assert_eq!(back.maximum(), pen.maximum());
}

#[test]
fn stateful_global_occultation() {
    // The star path resolves through sefstars.txt, so the data path is
    // sequenced here even though the planets use Moshier arithmetic.
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP file-based assertions: no ephemeris data found; set SWISSEPH_EPHE_DIR \
             to a directory containing the .se1 files"
        );
        return;
    };
    set_ephe_path(Some(&data_dir)).expect("data path");
    // --- next lunar occultation of Venus after J2000: total, central ---
    let occ = lun_occult_when_glob(J2000, VENUS, None, FLG_MOSEPH, 0, false).expect("Venus search");
    assert_eq!(occ.occult_type, ECL_TOTAL | ECL_CENTRAL);
    assert!(occ.maximum() > J2000 && occ.maximum() < J2000 + 100.0);
    assert!(occ.begin() <= occ.maximum() && occ.maximum() <= occ.end());
    assert!(occ.totality_begin() <= occ.maximum() && occ.maximum() <= occ.totality_end());
    assert!(occ.centerline_begin() <= occ.maximum() && occ.maximum() <= occ.centerline_end());
    // Reserved hybrid slots stay zero.
    assert_eq!(occ.times[8], 0.0);
    assert_eq!(occ.times[9], 0.0);
    // Bitwise repeat calls; empty star name behaves as the body path.
    assert_eq!(
        lun_occult_when_glob(J2000, VENUS, None, FLG_MOSEPH, 0, false).expect("repeat"),
        occ
    );
    assert_eq!(
        lun_occult_when_glob(J2000, VENUS, Some(""), FLG_MOSEPH, 0, false)
            .expect("empty star path"),
        occ
    );

    // --- backward search from just past the maximum finds the same event ---
    let back = lun_occult_when_glob(occ.maximum() + 0.1, VENUS, None, FLG_MOSEPH, 0, true)
        .expect("backward search");
    assert!(back.maximum() < occ.maximum() + 0.1);
    assert_eq!(back.maximum(), occ.maximum());

    // --- impossible type combinations fail natively with diagnostics ---
    let err = lun_occult_when_glob(
        J2000,
        VENUS,
        None,
        FLG_MOSEPH,
        ECL_PARTIAL | ECL_CENTRAL,
        false,
    )
    .expect_err("central partial must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    let err = lun_occult_when_glob(J2000, VENUS, None, FLG_MOSEPH, ECL_ANNULAR, false)
        .expect_err("annular Venus must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(err.to_string().contains("annular"), "{}", err);

    // --- unoccultable star fails natively with the latitude diagnostic ---
    let err = lun_occult_when_glob(J2000, 0, Some("Sirius"), FLG_MOSEPH, 0, false)
        .expect_err("Sirius must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(err.to_string().contains("ecl. lat."), "{}", err);

    // --- fixed-star path: Aldebaran is occulted shortly after J2000 ---
    let ald = lun_occult_when_glob(J2000, 0, Some("Aldebaran"), FLG_MOSEPH, 0, false)
        .expect("Aldebaran search");
    assert_eq!(ald.occult_type & ECL_TOTAL, ECL_TOTAL);
    assert!(ald.maximum() > J2000 && ald.maximum() < J2000 + 30.0);
    assert!(ald.begin() <= ald.maximum() && ald.maximum() <= ald.end());
}

#[test]
fn stateful_local_occultation() {
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP file-based assertions: no ephemeris data found; set SWISSEPH_EPHE_DIR \
             to a directory containing the .se1 files"
        );
        return;
    };
    set_ephe_path(Some(&data_dir)).expect("data path");

    // --- next lunar occultation of Venus visible from Greenwich ---
    let occ = lun_occult_when_loc(J2000, VENUS, None, FLG_SWIEPH, 0.0, 51.5, 0.0, false)
        .expect("Venus local search");
    assert_eq!(
        occ.occult_type & (ECL_TOTAL | ECL_VISIBLE),
        ECL_TOTAL | ECL_VISIBLE
    );
    assert!(occ.maximum() > J2000);
    assert!(occ.first_contact() <= occ.maximum() && occ.maximum() <= occ.fourth_contact());
    // Total event: the Moon covers more than the Venus diameter.
    assert!(occ.magnitude() > 1.0);
    // Reserved slots: engine-zeroed tret tail and the solar-only
    // NASA/saros fields stay zero for a planetary target.
    assert!(occ.times[7..10].iter().all(|&v| v == 0.0));
    assert!(occ.attributes[8..20].iter().all(|&v| v == 0.0));
    assert_eq!(
        lun_occult_when_loc(J2000, VENUS, None, FLG_SWIEPH, 0.0, 51.5, 0.0, false).expect("repeat"),
        occ
    );

    // --- backward search from just past the maximum finds the same event ---
    let back = lun_occult_when_loc(
        occ.maximum() + 0.1,
        VENUS,
        None,
        FLG_SWIEPH,
        0.0,
        51.5,
        0.0,
        true,
    )
    .expect("backward search");
    assert_eq!(back.maximum(), occ.maximum());

    // --- observer altitude outside -500..25000 m fails natively ---
    let err = lun_occult_when_loc(J2000, VENUS, None, FLG_SWIEPH, 0.0, 51.5, 30000.0, false)
        .expect_err("altitude range must fail");
    assert_eq!(err.kind(), ErrorKind::Native);

    // --- fixed-star path at the same observer ---
    let star = lun_occult_when_loc(
        J2000,
        0,
        Some("Aldebaran"),
        FLG_SWIEPH,
        0.0,
        51.5,
        0.0,
        false,
    )
    .expect("Aldebaran local search");
    assert_eq!(star.occult_type & ECL_VISIBLE, ECL_VISIBLE);
    assert!(star.first_contact() <= star.maximum() && star.maximum() <= star.fourth_contact());
}

#[test]
fn stateful_occult_where() {
    // Same catalog-path sequencing as in `stateful_global_occultation`.
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP file-based assertions: no ephemeris data found; set SWISSEPH_EPHE_DIR \
             to a directory containing the .se1 files"
        );
        return;
    };
    set_ephe_path(Some(&data_dir)).expect("data path");
    // --- circumstances at the maximum of the next Venus occultation ---
    let glob =
        lun_occult_when_glob(J2000, VENUS, None, FLG_MOSEPH, 0, false).expect("Venus search");
    let geo = lun_occult_where(glob.maximum(), VENUS, None, FLG_MOSEPH).expect("where call");
    assert_ne!(geo.occult_type, 0);
    assert!(geo.magnitude() > 0.0);
    assert!(geo.longitude >= -180.0 && geo.longitude <= 180.0);
    assert!(geo.latitude >= -90.0 && geo.latitude <= 90.0);
    // Solar-only fields and the reserved tail stay zero.
    assert!(geo.attributes[8..20].iter().all(|&v| v == 0.0));
    assert_eq!(
        lun_occult_where(glob.maximum(), VENUS, None, FLG_MOSEPH).expect("repeat"),
        geo
    );

    // --- quiet instant: type 0 is data with the diagnostic kept ---
    let quiet = lun_occult_where(J2000, VENUS, None, FLG_MOSEPH).expect("quiet where");
    assert_eq!(quiet.occult_type, 0);
    assert!(!quiet.diagnostic.is_empty());

    // --- star path at its own maximum agrees it is an event ---
    let ald = lun_occult_when_glob(J2000, 0, Some("Aldebaran"), FLG_MOSEPH, 0, false)
        .expect("Aldebaran search");
    let ageo =
        lun_occult_where(ald.maximum(), 0, Some("Aldebaran"), FLG_MOSEPH).expect("star where call");
    assert_ne!(ageo.occult_type, 0);
    assert!(ageo.magnitude() > 0.0);
}
