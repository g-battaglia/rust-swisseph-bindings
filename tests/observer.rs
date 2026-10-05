//! Integration checks: orbital nodes/apsides and rise/set/transit
//! searches.
//!
//! Native configuration is process-global, so every state-dependent step
//! runs inside the single sequenced `stateful_nodes_and_rises` test.
//! Standalone tests use pure-Rust input validation only, which reaches no
//! native code. Assertions are structural (ranges, method effects, error
//! classification, same-engine consistency), never stored reference
//! vectors.

use swisseph_bindings::{
    BIT_DISC_CENTER, BIT_GEOCTR_NO_ECL_LAT, BIT_HINDU_RISING, BIT_NO_REFRACTION, CALC_ITRANSIT,
    CALC_MTRANSIT, CALC_RISE, CALC_SET, ErrorKind, FLG_MOSEPH, FLG_SPEED, FLG_SWIEPH, MARS, MOON,
    NODBIT_FOPOINT, NODBIT_MEAN, NODBIT_OSCU, NODBIT_OSCU_BAR, RiseTransitOutcome, SUN, nod_aps,
    nod_aps_ut, rise_trans, rise_trans_true_hor, set_ephe_path,
};

#[path = "data_dir.rs"]
mod test_data;

/// J2000.0 as a Julian Day label.
const J2000: f64 = 2451545.0;
/// Search start: 2000-01-01 00:00 UT.
const SEARCH_START: f64 = 2451544.5;
/// Greenwich observer.
const GREENWICH_LON: f64 = 0.0;
const GREENWICH_LAT: f64 = 51.5;
/// Standard atmosphere for refraction.
const PRESSURE: f64 = 1013.25;
const TEMP: f64 = 15.0;

#[test]
fn observer_constants_match_pinned_header() {
    assert_eq!(NODBIT_MEAN, 1);
    assert_eq!(NODBIT_OSCU, 2);
    assert_eq!(NODBIT_OSCU_BAR, 4);
    assert_eq!(NODBIT_FOPOINT, 256);
    assert_eq!(CALC_RISE, 1);
    assert_eq!(CALC_SET, 2);
    assert_eq!(CALC_MTRANSIT, 4);
    assert_eq!(CALC_ITRANSIT, 8);
    assert_eq!(
        BIT_HINDU_RISING,
        BIT_DISC_CENTER | BIT_NO_REFRACTION | BIT_GEOCTR_NO_ECL_LAT
    );
    assert_eq!(BIT_HINDU_RISING, 896);
}

#[test]
fn nodes_reject_non_finite_time() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for result in [
            nod_aps(bad, MARS, 0, NODBIT_MEAN).map(|_| ()),
            nod_aps_ut(bad, MARS, 0, NODBIT_MEAN).map(|_| ()),
        ] {
            let err = result.expect_err("non-finite date must be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidInput);
        }
    }
}

#[test]
fn rise_rejects_bad_input() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for result in [
            rise_trans(
                bad,
                SUN,
                None,
                0,
                CALC_RISE,
                GREENWICH_LON,
                GREENWICH_LAT,
                0.0,
                PRESSURE,
                TEMP,
            )
            .map(|_| ()),
            rise_trans(
                SEARCH_START,
                SUN,
                None,
                0,
                CALC_RISE,
                bad,
                GREENWICH_LAT,
                0.0,
                PRESSURE,
                TEMP,
            )
            .map(|_| ()),
            rise_trans(
                SEARCH_START,
                SUN,
                None,
                0,
                CALC_RISE,
                GREENWICH_LON,
                GREENWICH_LAT,
                0.0,
                bad,
                TEMP,
            )
            .map(|_| ()),
            rise_trans_true_hor(
                SEARCH_START,
                SUN,
                None,
                0,
                CALC_RISE,
                GREENWICH_LON,
                GREENWICH_LAT,
                0.0,
                PRESSURE,
                TEMP,
                bad,
            )
            .map(|_| ()),
        ] {
            let err = result.expect_err("non-finite input must be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidInput);
        }
    }
    let nul = "Siri\0us";
    for result in [
        rise_trans(
            SEARCH_START,
            SUN,
            Some(nul),
            0,
            CALC_RISE,
            GREENWICH_LON,
            GREENWICH_LAT,
            0.0,
            PRESSURE,
            TEMP,
        )
        .map(|_| ()),
        rise_trans_true_hor(
            SEARCH_START,
            SUN,
            Some(nul),
            0,
            CALC_RISE,
            GREENWICH_LON,
            GREENWICH_LAT,
            0.0,
            PRESSURE,
            TEMP,
            0.0,
        )
        .map(|_| ()),
    ] {
        let err = result.expect_err("NUL star name must be rejected");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
    let overlong = "S".repeat(256);
    for result in [
        rise_trans(
            SEARCH_START,
            SUN,
            Some(&overlong),
            0,
            CALC_RISE,
            GREENWICH_LON,
            GREENWICH_LAT,
            0.0,
            PRESSURE,
            TEMP,
        )
        .map(|_| ()),
        rise_trans_true_hor(
            SEARCH_START,
            SUN,
            Some(&overlong),
            0,
            CALC_RISE,
            GREENWICH_LON,
            GREENWICH_LAT,
            0.0,
            PRESSURE,
            TEMP,
            0.0,
        )
        .map(|_| ()),
    ] {
        let err = result.expect_err("overlong star name must be rejected");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
}

/// Sequenced stateful checks: nodes/apsides methods, rise/set/transit
/// events, circumpolar no-event, star path and error classification.
/// Order matters because native state is process-global; the default
/// search path is restored before the test ends.
#[test]
fn stateful_nodes_and_rises() {
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP stateful_nodes_and_rises: no ephemeris data found; set SWISSEPH_EPHE_DIR \
             to a directory containing the .se1 files"
        );
        return;
    };
    set_ephe_path(Some(&data_dir)).expect("point at shipped data");

    // 1. Mars mean nodes/apsides: all 24 components finite, angles in
    // range, native OK status with no diagnostic.
    let mars = nod_aps(J2000, MARS, FLG_SWIEPH | FLG_SPEED, NODBIT_MEAN).expect("mars mean");
    for vector in [
        &mars.ascending,
        &mars.descending,
        &mars.perihelion,
        &mars.aphelion,
    ] {
        assert!(
            vector.iter().all(|v| v.is_finite()),
            "all components finite"
        );
        assert!((0.0..360.0).contains(&vector[0]), "longitude in range");
    }
    assert_eq!(mars.returned_flags, 0, "native OK status, no source bits");
    assert!(mars.diagnostic.is_empty());

    // 2. The UT entry point agrees with the ET one up to the UT↔TT
    // offset: a loose 1° band, not a precision claim.
    let mars_ut = nod_aps_ut(J2000, MARS, FLG_SWIEPH | FLG_SPEED, NODBIT_MEAN).expect("mars ut");
    for (index, (et, ut)) in mars
        .ascending
        .iter()
        .zip(mars_ut.ascending.iter())
        .enumerate()
    {
        if index < 2 {
            assert!(
                (et - ut).abs() < 1.0,
                "angle component {index} must agree across time scales: {et} vs {ut}"
            );
        }
    }

    // 3. The method bit is honored: osculating nodes differ from mean
    // ones for Mars, and the focal-point bit changes the aphelion slot.
    let oscu = nod_aps(J2000, MARS, FLG_SWIEPH | FLG_SPEED, NODBIT_OSCU).expect("mars oscu");
    assert_ne!(
        oscu.ascending, mars.ascending,
        "method must change the nodes"
    );
    let fopoint = nod_aps(
        J2000,
        MARS,
        FLG_SWIEPH | FLG_SPEED,
        NODBIT_MEAN | NODBIT_FOPOINT,
    )
    .expect("mars focal point");
    assert_ne!(
        fopoint.aphelion, mars.aphelion,
        "focal point replaces aphelion"
    );
    assert_eq!(fopoint.perihelion, mars.perihelion, "perihelion unaffected");

    // 4. The Moon resolves as well, and repeated calls are exactly
    // reproducible under the lock.
    let moon = nod_aps_ut(J2000, MOON, FLG_SWIEPH | FLG_SPEED, NODBIT_MEAN).expect("moon");
    assert!((0.0..360.0).contains(&moon.ascending[0]));
    let again = nod_aps(J2000, MARS, FLG_SWIEPH | FLG_SPEED, NODBIT_MEAN).expect("mars repeat");
    assert_eq!(again.ascending, mars.ascending);
    assert_eq!(again.aphelion, mars.aphelion);

    // 5. Bodies with no built-in or file elements fail inside the engine
    // with the diagnostic kept.
    for result in [
        nod_aps(J2000, 999, FLG_SWIEPH | FLG_SPEED, NODBIT_MEAN).map(|_| ()),
        nod_aps_ut(J2000, 999, FLG_SWIEPH | FLG_SPEED, NODBIT_MEAN).map(|_| ()),
    ] {
        let err = result.expect_err("unknown body must fail");
        assert_eq!(err.kind(), ErrorKind::Native);
        assert!(!err.message().is_empty());
    }

    // 6. Sunrise/sunset over Greenwich: events found inside the search
    // day, rise before set.
    let rise = rise_trans(
        SEARCH_START,
        SUN,
        None,
        FLG_SWIEPH,
        CALC_RISE,
        GREENWICH_LON,
        GREENWICH_LAT,
        0.0,
        PRESSURE,
        TEMP,
    )
    .expect("sunrise");
    let RiseTransitOutcome::Event {
        time_ut: sunrise, ..
    } = rise
    else {
        panic!("sunrise must be an event, got {rise:?}");
    };
    let set = rise_trans(
        SEARCH_START,
        SUN,
        None,
        FLG_SWIEPH,
        CALC_SET,
        GREENWICH_LON,
        GREENWICH_LAT,
        0.0,
        PRESSURE,
        TEMP,
    )
    .expect("sunset");
    let RiseTransitOutcome::Event {
        time_ut: sunset, ..
    } = set
    else {
        panic!("sunset must be an event, got {set:?}");
    };
    assert!(
        (SEARCH_START..SEARCH_START + 1.0).contains(&sunrise),
        "sunrise inside the search day: {sunrise}"
    );
    assert!(
        (SEARCH_START..SEARCH_START + 1.0).contains(&sunset),
        "sunset inside the search day: {sunset}"
    );
    assert!(sunrise < sunset, "rise precedes set");

    // 7. The upper transit falls between rise and set; the true-horizon
    // variant with a zero horizon agrees with the plain search.
    let transit = rise_trans(
        SEARCH_START,
        SUN,
        None,
        FLG_SWIEPH,
        CALC_MTRANSIT,
        GREENWICH_LON,
        GREENWICH_LAT,
        0.0,
        PRESSURE,
        TEMP,
    )
    .expect("mtransit");
    let RiseTransitOutcome::Event {
        time_ut: culmination,
        ..
    } = transit
    else {
        panic!("transit must be an event, got {transit:?}");
    };
    assert!(
        (sunrise..sunset).contains(&culmination),
        "culmination between rise and set: {culmination}"
    );
    let flat = rise_trans_true_hor(
        SEARCH_START,
        SUN,
        None,
        FLG_SWIEPH,
        CALC_RISE,
        GREENWICH_LON,
        GREENWICH_LAT,
        0.0,
        PRESSURE,
        TEMP,
        0.0,
    )
    .expect("true horizon rise");
    let RiseTransitOutcome::Event {
        time_ut: flat_rise, ..
    } = flat
    else {
        panic!("flat-horizon rise must be an event, got {flat:?}");
    };
    assert!(
        (flat_rise - sunrise).abs() < 1e-4,
        "zero horizon must agree with the plain search: {flat_rise} vs {sunrise}"
    );

    // 8. A circumpolar rise/set has no event: the absence is preserved
    // as data, never an error (the June Sun never sets at 89°N).
    let polar = rise_trans(
        SEARCH_START,
        SUN,
        None,
        FLG_SWIEPH,
        CALC_SET,
        0.0,
        89.0,
        0.0,
        PRESSURE,
        TEMP,
    )
    .expect("circumpolar search must not error");
    assert!(
        matches!(polar, RiseTransitOutcome::Circumpolar { .. }),
        "polar sunset must be a no-event, got {polar:?}"
    );

    // 9. The fixed-star path resolves through the catalog; an empty
    // name behaves as the body path; unknown stars fail natively.
    let sirius = rise_trans(
        SEARCH_START,
        SUN,
        Some("Sirius"),
        FLG_SWIEPH,
        CALC_RISE,
        GREENWICH_LON,
        GREENWICH_LAT,
        0.0,
        PRESSURE,
        TEMP,
    )
    .expect("sirius rise");
    assert!(
        matches!(sirius, RiseTransitOutcome::Event { .. }),
        "sirius rise must be an event, got {sirius:?}"
    );
    let empty_name = rise_trans(
        SEARCH_START,
        SUN,
        Some(""),
        FLG_SWIEPH,
        CALC_RISE,
        GREENWICH_LON,
        GREENWICH_LAT,
        0.0,
        PRESSURE,
        TEMP,
    )
    .expect("empty star name");
    assert_eq!(empty_name, rise, "empty name must behave as the body path");
    let err = rise_trans(
        SEARCH_START,
        SUN,
        Some("NoSuchStarXYZ"),
        FLG_SWIEPH,
        CALC_RISE,
        GREENWICH_LON,
        GREENWICH_LAT,
        0.0,
        PRESSURE,
        TEMP,
    )
    .expect_err("unknown star must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(
        err.message().contains("not found"),
        "got {:?}",
        err.message()
    );

    // 10. Mean nodes need no data files: they succeed with an empty
    // search path (Moshier selection agrees structurally).
    let empty = std::env::temp_dir().join("swisseph-bindings-empty-observer");
    std::fs::create_dir_all(&empty).expect("empty observer dir");
    set_ephe_path(Some(empty.to_str().expect("utf8 temp path"))).expect("empty data path");
    let filefree =
        nod_aps(J2000, MARS, FLG_SWIEPH | FLG_SPEED, NODBIT_MEAN).expect("mean w/o data");
    assert!(filefree.ascending.iter().all(|v| v.is_finite()));
    let moshier = nod_aps(J2000, MARS, FLG_MOSEPH | FLG_SPEED, NODBIT_MEAN).expect("moshier");
    assert!((moshier.ascending[0] - filefree.ascending[0]).abs() < 1.0);

    // 11. Restore the default search path for a clean handoff.
    set_ephe_path(None).expect("restore default path");
}
