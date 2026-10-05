//! Integration checks: fixed-star positions, magnitudes and catalog
//! behavior.
//!
//! Native configuration is process-global, so every path-dependent step
//! runs inside the single sequenced `stateful_star_catalog` test.
//! Standalone tests use pure-Rust input validation only, which reaches no
//! native code. Assertions are structural (ranges, name resolution, error
//! classification, same-engine consistency), never stored reference
//! vectors.

use swisseph_bindings::{
    ErrorKind, FLG_MOSEPH, FLG_SPEED, FLG_SWIEPH, MARS, fixstar, fixstar_mag, fixstar_ut, fixstar2,
    fixstar2_mag, fixstar2_ut, gauquelin_sector, set_ephe_path,
};

#[path = "data_dir.rs"]
mod test_data;

/// J2000.0 as a Universal-Time label.
const J2000_UT: f64 = 2451545.0;
/// Paris observer for the Gauquelin star-path check.
const PARIS_LON: f64 = 2.35;
const PARIS_LAT: f64 = 48.85;

/// Overlong query: 512 bytes never fit the native star buffer with its
/// NUL terminator.
fn overlong_star() -> String {
    "S".repeat(512)
}

#[test]
fn star_positions_reject_bad_input() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for result in [
            fixstar("Sirius", bad, 0),
            fixstar_ut("Sirius", bad, 0),
            fixstar2("Sirius", bad, 0),
            fixstar2_ut("Sirius", bad, 0),
        ] {
            let err = result.expect_err("non-finite date must be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidInput);
        }
    }
    let nul = "Siri\0us";
    for result in [
        fixstar(nul, J2000_UT, 0).map(|_| ()),
        fixstar_ut(nul, J2000_UT, 0).map(|_| ()),
        fixstar2(nul, J2000_UT, 0).map(|_| ()),
        fixstar2_ut(nul, J2000_UT, 0).map(|_| ()),
        fixstar_mag(nul).map(|_| ()),
        fixstar2_mag(nul).map(|_| ()),
    ] {
        let err = result.expect_err("NUL query must be rejected");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
    let overlong = overlong_star();
    for result in [
        fixstar(&overlong, J2000_UT, 0).map(|_| ()),
        fixstar_ut(&overlong, J2000_UT, 0).map(|_| ()),
        fixstar2(&overlong, J2000_UT, 0).map(|_| ()),
        fixstar2_ut(&overlong, J2000_UT, 0).map(|_| ()),
        fixstar_mag(&overlong).map(|_| ()),
        fixstar2_mag(&overlong).map(|_| ()),
    ] {
        let err = result.expect_err("overlong query must be rejected");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
}

/// Sequenced stateful checks: catalog loading, positions, search forms,
/// magnitudes, unknown-star errors and path isolation. Order matters
/// because native state is process-global; the default search path is
/// restored before the test ends.
#[test]
fn stateful_star_catalog() {
    // 1. Without catalog data the engine reports the gap instead of
    // inventing a star. This runs before any successful catalog load so
    // no cached catalog can mask the missing files.
    let empty = std::env::temp_dir().join("swisseph-bindings-empty-star-catalog");
    std::fs::create_dir_all(&empty).expect("empty star dir");
    set_ephe_path(Some(empty.to_str().expect("utf8 temp path"))).expect("empty data path");
    let err = fixstar_ut("Sirius", J2000_UT, FLG_SWIEPH | FLG_SPEED)
        .expect_err("star lookup without catalog must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(!err.message().is_empty(), "native diagnostic must be kept");

    // 2. With the shipped catalog, the traditional name resolves and all
    // six components are finite with source provenance. Without data the
    // empty-catalog check above still ran; the rest is skipped.
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP stateful_star_catalog file-based sections: no ephemeris data found; set \
             SWISSEPH_EPHE_DIR to a directory containing sefstars.txt and the .se1 files"
        );
        set_ephe_path(None).expect("restore default path");
        return;
    };
    set_ephe_path(Some(&data_dir)).expect("point at shipped data");
    let sirius_ut = fixstar_ut("Sirius", J2000_UT, FLG_SWIEPH | FLG_SPEED).expect("sirius ut");
    assert!(sirius_ut.resolved_name.contains("Sirius"));
    assert!(sirius_ut.values.iter().all(|v| v.is_finite()));
    assert!((0.0..360.0).contains(&sirius_ut.longitude()));
    assert!(
        sirius_ut.returned_flags & FLG_SWIEPH != 0,
        "source bit must echo the request, got {:#X}",
        sirius_ut.returned_flags
    );

    // 3. The ET entry point agrees with the UT one up to the UT↔TT
    // offset (about a minute of Earth-rotation scale at J2000): a loose
    // 1° band, not a precision claim.
    let sirius_et = fixstar("Sirius", J2000_UT, FLG_SWIEPH | FLG_SPEED).expect("sirius et");
    assert_eq!(sirius_et.resolved_name, sirius_ut.resolved_name);
    for (index, (et, ut)) in sirius_et
        .values
        .iter()
        .zip(sirius_ut.values.iter())
        .enumerate()
    {
        if index < 2 {
            assert!(
                (et - ut).abs() < 1.0,
                "angle component {index} must agree across time scales: {et} vs {ut}"
            );
        }
    }

    // 4. The extended search resolves the same entry through the
    // nomenclature, and the full "Name,Nomenclature" form is echoed.
    let by_traditional = fixstar2_ut("Sirius", J2000_UT, FLG_SWIEPH | FLG_SPEED).expect("v2 name");
    let by_nomenclature =
        fixstar2_ut(",alCMa", J2000_UT, FLG_SWIEPH | FLG_SPEED).expect("v2 nomenclature");
    assert!(
        by_traditional.resolved_name.contains("Sirius"),
        "got {:?}",
        by_traditional.resolved_name
    );
    assert_eq!(by_traditional.resolved_name, by_nomenclature.resolved_name);
    assert_eq!(by_traditional.values, by_nomenclature.values);
    let wildcard = fixstar2("Siri%", J2000_UT, FLG_SWIEPH | FLG_SPEED).expect("v2 wildcard");
    assert!(
        wildcard.resolved_name.contains("Sirius"),
        "got {:?}",
        wildcard.resolved_name
    );

    // 5. Repeated calls are exactly reproducible (catalog + engine state
    // are stable under the lock).
    let again = fixstar_ut("Sirius", J2000_UT, FLG_SWIEPH | FLG_SPEED).expect("sirius repeat");
    assert_eq!(again.values, sirius_ut.values);
    assert_eq!(again.resolved_name, sirius_ut.resolved_name);
    assert_eq!(again.returned_flags, sirius_ut.returned_flags);

    // 6. Magnitudes come from the catalog without a position computation.
    let mag = fixstar_mag("Sirius").expect("sirius magnitude");
    assert!(
        (mag.magnitude - (-1.46)).abs() < 0.01,
        "Sirius magnitude must match the catalog, got {}",
        mag.magnitude
    );
    assert!(mag.resolved_name.contains("Sirius"));
    let mag2 = fixstar2_mag(",alCMa").expect("v2 magnitude");
    assert_eq!(mag2.magnitude, mag.magnitude);
    assert_eq!(mag2.resolved_name, mag.resolved_name);
    let regulus = fixstar_mag("Regulus").expect("regulus magnitude");
    assert!(
        (regulus.magnitude - 1.4).abs() < 0.05,
        "Regulus magnitude must match the catalog, got {}",
        regulus.magnitude
    );

    // 7. Unknown stars fail inside the engine with the diagnostic kept;
    // the failure is a catalog verdict, never invented data.
    for result in [
        fixstar_ut("NoSuchStarXYZ", J2000_UT, FLG_SWIEPH | FLG_SPEED).map(|_| ()),
        fixstar("NoSuchStarXYZ", J2000_UT, FLG_SWIEPH | FLG_SPEED).map(|_| ()),
        fixstar2_ut("NoSuchStarXYZ%", J2000_UT, FLG_SWIEPH | FLG_SPEED).map(|_| ()),
        fixstar_mag("NoSuchStarXYZ").map(|_| ()),
        fixstar2_mag("NoSuchStarXYZ").map(|_| ()),
    ] {
        let err = result.expect_err("unknown star must fail");
        assert_eq!(err.kind(), ErrorKind::Native);
        assert!(!err.message().is_empty());
    }

    // 8. The Gauquelin star path now has catalog semantics: a catalog
    // name succeeds where the body path would compute a planet.
    let star_sector = gauquelin_sector(
        J2000_UT,
        MARS,
        Some("Sirius"),
        FLG_SWIEPH,
        0,
        PARIS_LON,
        PARIS_LAT,
        0.0,
        0.0,
        0.0,
    )
    .expect("gauquelin star path");
    assert!(
        (1.0..37.0).contains(&star_sector.value),
        "star sector out of range: {}",
        star_sector.value
    );
    // Moshier selection is equally usable for the star path (the catalog
    // lookup itself is data-driven, the ephemeris model is flagged).
    let moshier_star = gauquelin_sector(
        J2000_UT,
        MARS,
        Some("Sirius"),
        FLG_MOSEPH,
        0,
        PARIS_LON,
        PARIS_LAT,
        0.0,
        0.0,
        0.0,
    )
    .expect("gauquelin star moshier");
    assert!((1.0..37.0).contains(&moshier_star.value));

    // 9. Restore the default search path for a clean handoff.
    set_ephe_path(None).expect("restore default path");
}
