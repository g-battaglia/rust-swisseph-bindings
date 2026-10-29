//! Primary calculations retain their actual per-owner file provenance.
use swisseph_bindings::{FLG_MOSEPH, FLG_SPEED, FLG_SWIEPH, MOON, SessionBuilder, native};

#[test]
fn primary_snapshots_retain_complete_native_metadata_across_other_owners() {
    let Some(directory) = std::env::var_os("SWISSEPH_EPHE_DIR") else {
        eprintln!(
            "SKIP primary file snapshot data test: set SWISSEPH_EPHE_DIR to existing Swiss data"
        );
        return;
    };
    let directory = directory.to_str().expect("UTF-8 explicit data directory");
    let mut builder = native::SessionBuilder::new();
    builder.ephe_path(directory);
    let primary = native::primary_session(&builder).unwrap();
    assert!(
        native::primary_current_file_data(&primary, 1)
            .unwrap()
            .is_none()
    );
    primary
        .calc_ut(2451545.0, MOON, FLG_SWIEPH | FLG_SPEED)
        .unwrap();
    let saved = native::primary_current_file_data(&primary, 1)
        .unwrap()
        .expect("actual Moon file");
    let direct = builder.build().unwrap();
    direct
        .calc_ut(2451545.0, MOON, FLG_SWIEPH | FLG_SPEED)
        .unwrap();
    assert!(direct.get_current_file_data(1).unwrap().as_ref() == Some(&saved));
    let clone = primary.clone();
    let mut path = Some(String::with_capacity(1024));
    let capacity = path.as_ref().unwrap().capacity();
    native::copy_primary_file_path_into(&primary, 1, &mut path).unwrap();
    assert_eq!(path.as_deref(), Some(saved.path.as_str()));
    assert_eq!(path.as_ref().unwrap().capacity(), capacity);
    let foreign = SessionBuilder::new().build().unwrap();
    foreign
        .calc_ut(2451545.0, MOON, FLG_MOSEPH | FLG_SPEED)
        .unwrap();
    native::close().unwrap();
    assert!(
        native::primary_current_file_data(&primary, 1)
            .unwrap()
            .as_ref()
            == Some(&saved)
    );
    assert!(
        native::primary_current_file_data(&clone, 1)
            .unwrap()
            .as_ref()
            == Some(&saved)
    );
    for slot in [-1, 5] {
        let before = path.clone();
        assert!(native::copy_primary_file_path_into(&primary, slot, &mut path).is_err());
        assert_eq!(path, before);
    }
    let mut disabled = native::SessionBuilder::new();
    disabled.ephe_path(directory).capture_file_data(false);
    let disabled = native::primary_session(&disabled).unwrap();
    disabled
        .calc_ut(2451545.0, MOON, FLG_SWIEPH | FLG_SPEED)
        .unwrap();
    assert!(
        native::primary_current_file_data(&disabled, 1)
            .unwrap()
            .is_none()
    );
}

#[test]
fn uncomputed_primary_has_no_file_snapshot() {
    let session = SessionBuilder::new().build().unwrap();
    for slot in 0..=4 {
        assert!(
            native::primary_current_file_data(&session, slot)
                .unwrap()
                .is_none()
        );
        let mut path = Some("unrelated previous output".to_owned());
        native::copy_primary_file_path_into(&session, slot, &mut path).unwrap();
        assert!(path.is_none());
    }
    for slot in [-1, 5] {
        assert!(
            native::primary_current_file_data(&session, slot)
                .unwrap()
                .is_none()
        );
        let mut path = Some("preserve on error".to_owned());
        assert!(native::copy_primary_file_path_into(&session, slot, &mut path).is_err());
        assert_eq!(path.as_deref(), Some("preserve on error"));
    }
}
