//! Optional file snapshots must not change native computation or other owners.

use swisseph_bindings::{FLG_MOSEPH, MOON, SessionBuilder, global::houses_ex2};

#[test]
fn disabled_file_capture_preserves_native_houses_and_independent_snapshots() {
    let mut builder = SessionBuilder::new();
    builder.default_ephe_path().sid_mode(1, 0.0, 0.0);
    if let Ok(path) = std::env::var("SWISSEPH_EPHE_DIR") {
        builder.ephe_path(&path);
    }
    let observed = builder.build().expect("observed session");
    let unobserved = builder
        .capture_file_data(false)
        .build()
        .expect("unobserved session");
    let reenabled = builder
        .capture_file_data(true)
        .build()
        .expect("reenabled session");

    for day in [1700000.5, 2451545.0, 2460310.5] {
        for flags in [0, 8192, 65536, 65536 | 8192] {
            for system in *b"PEW" {
                for session in [&observed, &unobserved, &reenabled] {
                    let actual = session
                        .houses_ex2(day, flags, 41.9, 12.5, system)
                        .expect("session houses");
                    let expected =
                        houses_ex2(day, flags, 41.9, 12.5, system).expect("direct native houses");
                    assert!(
                        actual
                            .cusps
                            .iter()
                            .chain(&actual.angles)
                            .chain(&actual.cusp_speeds)
                            .chain(&actual.angle_speeds)
                            .zip(
                                expected
                                    .cusps
                                    .iter()
                                    .chain(&expected.angles)
                                    .chain(&expected.cusp_speeds)
                                    .chain(&expected.angle_speeds)
                            )
                            .all(|(a, b)| a.to_bits() == b.to_bits()),
                        "snapshot policy must preserve every native house value"
                    );
                }
            }
        }
    }
    assert!(
        unobserved
            .houses_ex2(f64::NAN, 0, 41.9, 12.5, b'P')
            .is_err(),
        "disabling capture must preserve input validation"
    );

    // Both metadata policies retain input-admission and native-failure details.
    for (day, latitude) in [(f64::NAN, 41.9), (2451545.0, 89.5)] {
        let expected = observed
            .houses_ex2(day, 0, latitude, 12.5, b'P')
            .unwrap_err();
        let actual = unobserved
            .houses_ex2(day, 0, latitude, 12.5, b'P')
            .unwrap_err();
        assert!(
            actual.kind() == expected.kind() && actual.to_string() == expected.to_string(),
            "disabled snapshots must preserve input/native errors exactly"
        );
    }

    // With explicitly selected data, also require a populated owned snapshot.
    let flags = if std::env::var_os("SWISSEPH_EPHE_DIR").is_some() {
        swisseph_bindings::FLG_SWIEPH
    } else {
        FLG_MOSEPH
    };
    let result = observed.calc_ut(2451545.0, MOON, flags).unwrap();
    swisseph_bindings::native::require_source_flags(&result, flags).unwrap();
    let before: Vec<_> = (0..5)
        .map(|slot| observed.get_current_file_data(slot).unwrap())
        .collect();
    if flags == swisseph_bindings::FLG_SWIEPH {
        assert!(
            before.iter().any(Option::is_some),
            "explicit data must populate a snapshot"
        );
    }
    let clone = unobserved.clone();
    clone.calc_ut(2460310.5, MOON, flags).unwrap();
    for (slot, previous) in before.into_iter().enumerate() {
        assert!(
            observed.get_current_file_data(slot as i32).unwrap() == previous,
            "another session must not replace the owned snapshot"
        );
        assert!(
            unobserved
                .get_current_file_data(slot as i32)
                .unwrap()
                .is_none()
        );
        assert!(clone.get_current_file_data(slot as i32).unwrap().is_none());
        let mut path = Some("previous destination".to_owned());
        clone
            .copy_captured_file_path_into(slot as i32, &mut path)
            .unwrap();
        assert!(path.is_none());
    }
    reenabled.calc_ut(2451545.0, MOON, flags).unwrap();
    assert!(
        (0..5).all(|slot| reenabled.get_current_file_data(slot).unwrap()
            == swisseph_bindings::global::get_current_file_data(slot).unwrap()),
        "reenabling capture must restore the default snapshot contract"
    );
}

#[test]
fn selected_file_slots_preserve_native_positions_and_owned_provenance() {
    use swisseph_bindings::{
        CHIRON, EARTH, FLG_EQUATORIAL, FLG_SIDEREAL, FLG_SPEED, FLG_SWIEPH, JUPITER, MERCURY,
        PLUTO, SUN, global::calc_ut, global::get_current_file_data, native::require_source_flags,
    };

    let path = std::env::var("SWISSEPH_EPHE_DIR").expect("explicit native data directory");
    let mut builder = SessionBuilder::new();
    builder.ephe_path(&path).sid_mode(1, 0.0, 0.0);
    let complete = builder.build().unwrap();
    let seed = complete
        .calc_ut(2451545.0, CHIRON, FLG_SWIEPH | FLG_SPEED)
        .unwrap();
    require_source_flags(&seed, FLG_SWIEPH).unwrap();
    assert!(
        complete.get_current_file_data(2).unwrap().is_some(),
        "asteroid metadata fixture must be populated"
    );
    let selected = builder
        .capture_file_slots([true, true, false, false, false])
        .build()
        .unwrap();

    for day in [2451545.0, 2460310.5, 2460676.5] {
        for flags in [
            FLG_MOSEPH | FLG_SPEED,
            FLG_SWIEPH | FLG_SPEED,
            FLG_SWIEPH | FLG_SPEED | FLG_EQUATORIAL,
            FLG_SWIEPH | FLG_SPEED | FLG_SIDEREAL,
        ] {
            for body in [SUN, MOON, MERCURY, JUPITER, PLUTO, EARTH] {
                let baseline = complete.calc_ut(day, body, flags).unwrap();
                let actual = selected.calc_ut(day, body, flags).unwrap();
                let direct = calc_ut(day, body, flags).unwrap();
                for expected in [&baseline, &direct] {
                    assert!(
                        actual
                            .values
                            .iter()
                            .zip(expected.values)
                            .all(|(a, b)| a.to_bits() == b.to_bits())
                            && actual.returned_flags == expected.returned_flags
                            && actual.diagnostic == expected.diagnostic,
                        "selected metadata slots must preserve original native values, flags and diagnostics"
                    );
                }
                for slot in 0..2 {
                    assert!(
                        selected.get_current_file_data(slot).unwrap()
                            == get_current_file_data(slot).unwrap(),
                        "selected owned metadata must match the native snapshot"
                    );
                }
                for slot in 2..5 {
                    assert!(
                        selected.get_current_file_data(slot).unwrap().is_none(),
                        "unselected metadata must not be captured"
                    );
                }
            }
        }
    }
    let before: Vec<_> = (0..5)
        .map(|slot| complete.get_current_file_data(slot).unwrap())
        .collect();
    let clone = selected.clone();
    clone
        .calc_ut(2461000.5, MOON, FLG_SWIEPH | FLG_SPEED)
        .unwrap();
    assert!(
        (0..5).all(|slot| complete.get_current_file_data(slot).unwrap() == before[slot as usize]),
        "selected session clones must not replace another owner's history"
    );
    assert!(
        clone.get_current_file_data(1).unwrap() == selected.get_current_file_data(1).unwrap(),
        "clones must retain the same selected history"
    );
    assert!(
        selected.calc_ut(f64::NAN, MOON, FLG_SWIEPH).is_err(),
        "selected snapshots must not weaken input admission"
    );
    let restored = builder.capture_file_data(true).build().unwrap();
    restored
        .calc_ut(2451545.0, MOON, FLG_SWIEPH | FLG_SPEED)
        .unwrap();
    assert!(
        (0..5).all(|slot| restored.get_current_file_data(slot).unwrap()
            == get_current_file_data(slot).unwrap()),
        "enabling all snapshots must override an earlier slot selection"
    );
    let reselected = builder
        .capture_file_data(false)
        .capture_file_slots([false, true, false, false, false])
        .build()
        .unwrap();
    reselected
        .calc_ut(2451545.0, MOON, FLG_SWIEPH | FLG_SPEED)
        .unwrap();
    assert!(
        reselected.get_current_file_data(1).unwrap().is_some()
            && reselected.get_current_file_data(0).unwrap().is_none(),
        "slot selection must override disabled snapshots"
    );
}
