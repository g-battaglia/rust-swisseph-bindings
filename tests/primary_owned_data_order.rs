//! Owned configuration must retain the native data path's numerical preparation.
use swisseph_bindings::{Session, Target, native};

#[path = "support/isolated.rs"]
mod isolated;
#[path = "data_dir.rs"]
mod test_data;

fn directory() -> Option<String> {
    test_data::ephemeris_dir()
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
    // Free setters and their reference calculation are separate native-lock
    // acquisitions. Keep this entire comparison in its own process.
    if !isolated::without_ephe_override(
        "automatic_tidal_selection_matches_the_native_path_before_ut_ayanamsha",
    ) {
        return;
    }
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
                    assert_eq!(actual.returned_flags, expected.returned_flags);
                    assert_eq!(actual.diagnostic, expected.diagnostic);
                }
            }
        }
    }
}

/// Compare full position transport after a foreign owner's clock writes. All
/// references come from the same native engine and stay in memory.
fn compare_source_configuration(path: &str, jpl: Option<&str>, source: i32) {
    for mode in [native::SIDM_FAGAN_BRADLEY, native::SIDM_LAHIRI] {
        for tide in [native::SE_TIDAL_AUTOMATIC, -20.0] {
            for delta_t in [None, Some(0.001)] {
                let mut builder = native::SessionBuilder::new();
                builder
                    .ephe_path(path)
                    .sid_mode(mode, 2451545.0, 23.5)
                    .topo(12.5, 41.9, 1200.0)
                    .tid_acc(tide);
                if let Some(name) = jpl {
                    builder.jpl_file(name);
                }
                if let Some(days) = delta_t {
                    builder.delta_t_override(days);
                }
                let session = native::primary_session(&builder).unwrap();
                for day in [2415020.0, 2451545.0, 2460000.5] {
                    for body in [native::MOON, native::MARS] {
                        // Empty JPL closes all files; nonempty JPL selects data
                        // after the Swiss path. Manual clock values apply last.
                        native::close().unwrap();
                        native::set_tid_acc(native::SE_TIDAL_AUTOMATIC).unwrap();
                        native::set_delta_t_userdef(None).unwrap();
                        if jpl.is_none_or(str::is_empty) {
                            native::set_jpl_file("").unwrap();
                        }
                        native::set_ephe_path(Some(path)).unwrap();
                        if let Some(name) = jpl.filter(|name| !name.is_empty()) {
                            native::set_jpl_file(name).unwrap();
                        }
                        native::set_sid_mode(mode, 2451545.0, 23.5).unwrap();
                        native::set_topo(12.5, 41.9, 1200.0).unwrap();
                        if tide != native::SE_TIDAL_AUTOMATIC {
                            native::set_tid_acc(tide).unwrap();
                        }
                        native::set_delta_t_userdef(delta_t).unwrap();
                        let flags =
                            source | native::FLG_SPEED | native::FLG_SIDEREAL | native::FLG_TOPOCTR;
                        let expected = native::calc_ut(day, body, flags).unwrap();
                        // A missing JPL selection must expose the actual Swiss
                        // fallback; genuine JPL must actually use its archive.
                        let actual_source = if jpl == Some("de440.eph") {
                            native::FLG_JPLEPH
                        } else {
                            native::FLG_SWIEPH
                        };
                        assert_eq!(expected.returned_flags & 7, actual_source);
                        native::set_tid_acc(-30.0).unwrap();
                        native::set_delta_t_userdef(Some(0.05)).unwrap();
                        for _ in 0..2 {
                            let actual = session.calc_ut(day, body, flags).unwrap();
                            assert_eq!(actual.returned_flags, expected.returned_flags);
                            assert_eq!(actual.diagnostic, expected.diagnostic);
                            assert_eq!(
                                actual.values.map(f64::to_bits),
                                expected.values.map(f64::to_bits)
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn owned_swiss_and_missing_jpl_keep_complete_native_positions() {
    if !isolated::without_ephe_override(
        "owned_swiss_and_missing_jpl_keep_complete_native_positions",
    ) {
        return;
    }
    let Some(path) = directory() else {
        return;
    };
    compare_source_configuration(&path, None, native::FLG_SWIEPH);
    compare_source_configuration(&path, Some(""), native::FLG_SWIEPH);
    compare_source_configuration(&path, Some("__owned_missing_jpl__.eph"), native::FLG_JPLEPH);
}

#[test]
fn owned_jpl_keeps_complete_native_positions_and_manual_overrides() {
    if !isolated::without_ephe_override(
        "owned_jpl_keeps_complete_native_positions_and_manual_overrides",
    ) {
        return;
    }
    let Some(path) = directory() else {
        return;
    };
    let Ok(jpl_directory) = std::env::var("SWISSEPH_JPL_DIR") else {
        eprintln!("SKIP owned JPL source: select an existing de440.eph with SWISSEPH_JPL_DIR");
        return;
    };
    assert!(
        std::path::Path::new(&jpl_directory)
            .join("de440.eph")
            .is_file(),
        "SWISSEPH_JPL_DIR must contain the existing de440.eph archive"
    );
    compare_source_configuration(
        &format!("{path};{jpl_directory}"),
        Some("de440.eph"),
        native::FLG_JPLEPH,
    );
}

#[test]
fn scoped_sector_keeps_the_native_path_preparation_and_explicit_observer() {
    if !isolated::without_ephe_override(
        "scoped_sector_keeps_the_native_path_preparation_and_explicit_observer",
    ) {
        return;
    }
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
    if !isolated::without_ephe_override("manual_tide_and_delta_t_remain_final_owned_overrides") {
        return;
    }
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
