//! Owned configuration must retain the native data path's numerical preparation.
use swisseph_bindings::{Session, Target, native};

fn directory() -> Option<String> {
    let directory = std::env::var("SWISSEPH_EPHE_DIR").ok();
    if directory.is_none() {
        eprintln!(
            "SKIP owned data-order checks: provide existing Swiss data with SWISSEPH_EPHE_DIR"
        );
    }
    directory
}

fn reference(path: &str, mode: i32, topo: [f64; 3]) {
    native::close().unwrap();
    native::set_tid_acc(native::SE_TIDAL_AUTOMATIC).unwrap();
    native::set_delta_t_userdef(None).unwrap();
    native::set_ephe_path(Some(path)).unwrap();
    native::set_sid_mode(mode, 2451545.0, 23.5).unwrap();
    native::set_topo(topo[0], topo[1], topo[2]).unwrap();
}

fn primary(path: &str, mode: i32, topo: [f64; 3]) -> Session {
    let mut builder = native::SessionBuilder::new();
    builder
        .ephe_path(path)
        .sid_mode(mode, 2451545.0, 23.5)
        .topo(topo[0], topo[1], topo[2]);
    native::primary_session(&builder).unwrap()
}

#[test]
fn automatic_tidal_selection_matches_the_native_path_before_ut_ayanamsha() {
    let Some(path) = directory() else {
        return;
    };
    for mode in [0, 1, 255, 257] {
        for day in [1700000.5, 2451545.0] {
            reference(&path, mode, [0.0; 3]);
            let expected = native::get_ayanamsa_ut(day).unwrap();
            let session = primary(&path, mode, [0.0; 3]);
            native::close().unwrap();
            for _ in 0..2 {
                let actual = session.get_ayanamsa_ut(day).unwrap();
                assert!(
                    actual.value.to_bits() == expected.to_bits(),
                    "owned mean UT data selection differs"
                );
            }
            for flags in [0, 2, 4, 66] {
                reference(&path, mode, [0.0; 3]);
                let expected = native::get_ayanamsa_ex_ut(day, flags).unwrap();
                native::close().unwrap();
                for _ in 0..2 {
                    let actual = session.get_ayanamsa_ex_ut(day, flags).unwrap();
                    assert!(
                        actual.value.to_bits() == expected.value.to_bits(),
                        "owned extended UT data selection differs"
                    );
                }
            }
        }
    }
}

#[test]
fn scoped_sector_keeps_the_native_path_preparation_and_explicit_observer() {
    let Some(path) = directory() else {
        return;
    };
    for mode in [1, 255] {
        for topo in [[0.0; 3], [12.5, 41.9, 1200.0]] {
            reference(&path, mode, topo);
            let expected = native::gauquelin_sector(
                2451545.0,
                1,
                None,
                32770 | 65536,
                0,
                12.5,
                41.9,
                1200.0,
                900.0,
                -5.0,
            )
            .unwrap();
            let session = primary(&path, mode, topo);
            native::close().unwrap();
            for _ in 0..2 {
                let actual = session
                    .gauquelin_sector(
                        2451545.0,
                        &Target::Body(1),
                        0,
                        [12.5, 41.9, 1200.0],
                        900.0,
                        -5.0,
                        32770 | 65536,
                    )
                    .unwrap();
                assert!(
                    actual.value.to_bits() == expected.value.to_bits(),
                    "owned sector data preparation differs"
                );
            }
        }
    }
}

#[test]
fn manual_tide_and_delta_t_remain_final_owned_overrides() {
    let Some(path) = directory() else {
        return;
    };
    for tide in [-20.0, native::SE_TIDAL_DE431] {
        for delta_t in [None, Some(0.001)] {
            reference(&path, 1, [0.0; 3]);
            native::set_tid_acc(tide).unwrap();
            if let Some(days) = delta_t {
                native::set_delta_t_userdef(Some(days)).unwrap();
            }
            let expected = native::get_ayanamsa_ut(1700000.5).unwrap();
            let mut builder = native::SessionBuilder::new();
            builder
                .ephe_path(&path)
                .sid_mode(1, 2451545.0, 23.5)
                .tid_acc(tide);
            if let Some(days) = delta_t {
                builder.delta_t_override(days);
            }
            let session = native::primary_session(&builder).unwrap();
            native::close().unwrap();
            for _ in 0..2 {
                let actual = session.get_ayanamsa_ut(1700000.5).unwrap();
                assert!(
                    actual.value.to_bits() == expected.to_bits(),
                    "owned manual clock override differs"
                );
            }
        }
    }
}
