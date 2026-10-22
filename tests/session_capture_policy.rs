//! Optional file snapshots must not change native computation or other owners.

use swisseph_bindings::{FLG_MOSEPH, MOON, SessionBuilder, houses_ex2};

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

    // With explicitly selected data, also require a populated owned snapshot.
    let flags = if std::env::var_os("SWISSEPH_EPHE_DIR").is_some() {
        swisseph_bindings::FLG_SWIEPH
    } else {
        FLG_MOSEPH
    };
    let result = observed.calc_ut(2451545.0, MOON, flags).unwrap();
    swisseph_bindings::require_source_flags(&result, flags).unwrap();
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
            == swisseph_bindings::get_current_file_data(slot).unwrap()),
        "reenabling capture must restore the default snapshot contract"
    );
}
