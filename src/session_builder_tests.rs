//! Borrowed and consuming builders share validation while owning new snapshots.

use super::*;

#[test]
fn borrowed_and_consuming_builds_preserve_text_selections() {
    let paths = [
        None,
        Some(None),
        Some(Some(String::new())),
        Some(Some(String::from("relative/é/ephemeris"))),
        Some(Some("p".repeat(MAX_EPHE_PATH_LEN))),
    ];
    let names = [
        None,
        Some(None),
        Some(Some(String::new())),
        Some(Some(String::from("éphéméride.440"))),
        Some(Some("j".repeat(MAX_JPL_FILE_LEN))),
    ];
    for path in &paths {
        for name in &names {
            let builder = SessionBuilder {
                ephe_path: path.clone(),
                jpl_file: name.clone(),
                ..SessionBuilder::new()
            };
            let borrowed = builder.build().unwrap();
            let owned = builder.clone().into_session().unwrap();
            let expected_path = path.as_ref().map(|text| text.as_deref().map(str::as_bytes));
            let expected_name = name.as_ref().map(|text| text.as_deref().map(str::as_bytes));
            for session in [&borrowed, &owned] {
                assert_eq!(
                    session
                        .ephe_path
                        .as_ref()
                        .map(|text| text.as_ref().map(|text| text.as_bytes())),
                    expected_path,
                );
                assert_eq!(
                    session
                        .jpl_file
                        .as_ref()
                        .map(|text| text.as_ref().map(|text| text.as_bytes())),
                    expected_name,
                );
            }
            // Reusable builders retain their original owned selections.
            assert_eq!(&builder.ephe_path, path);
            assert_eq!(&builder.jpl_file, name);
        }
    }
}

#[test]
fn borrowed_and_consuming_builds_preserve_validation_precedence() {
    let long_path = "p".repeat(MAX_EPHE_PATH_LEN + 1);
    let long_name = "j".repeat(MAX_JPL_FILE_LEN + 1);
    let cases = [
        (
            format!("{long_path}\0"),
            format!("{long_name}\0"),
            String::from("SessionBuilder: ephemeris path contains an interior NUL byte"),
        ),
        (
            long_path,
            String::from("invalid\0name"),
            format!(
                "SessionBuilder: ephemeris path is {} bytes, longer than the {MAX_EPHE_PATH_LEN}-byte native limit",
                MAX_EPHE_PATH_LEN + 1,
            ),
        ),
        (
            String::from("valid"),
            format!("{long_name}\0"),
            String::from("SessionBuilder: JPL file name contains an interior NUL byte"),
        ),
        (
            String::from("valid"),
            long_name,
            format!(
                "SessionBuilder: JPL file name is {} bytes, longer than the {MAX_JPL_FILE_LEN}-byte native limit",
                MAX_JPL_FILE_LEN + 1,
            ),
        ),
        (
            String::from("valid"),
            String::from("valid"),
            String::from(
                "SessionBuilder: observer longitude, latitude and altitude must be finite",
            ),
        ),
    ];
    for (path, name, message) in cases {
        let mut builder = SessionBuilder::new();
        // A later numeric failure must not displace either text-field error.
        builder
            .ephe_path(&path)
            .jpl_file(&name)
            .topo(f64::NAN, 0.0, 0.0);
        let borrowed = builder.build().unwrap_err();
        let owned = builder.clone().into_session().unwrap_err();
        assert_eq!(borrowed, owned);
        assert_eq!(borrowed.kind(), crate::ErrorKind::InvalidInput);
        assert_eq!(borrowed.message(), message);
        assert_eq!(builder.ephe_path, Some(Some(path)));
        assert_eq!(builder.jpl_file, Some(Some(name)));
    }
}

#[test]
fn each_built_session_owns_fresh_snapshots_while_clones_share_history() {
    let mut builder = SessionBuilder::new();
    builder
        .ephe_path("requested-path")
        .close_jpl_file()
        .capture_file_slots([true, false, true, false, true])
        .refresh_ephe_path_environment(false)
        .reinstall_ephe_path_on_directory_change(false)
        .topo(8.0, 47.0, 400.0)
        .tid_acc(-25.8)
        .delta_t_override(0.001)
        .lapse_rate(0.0065);
    let first = builder.build().unwrap();
    let second = builder.build().unwrap();
    let consumed = builder.into_session().unwrap();
    let cloned = first.clone();
    assert!(Arc::ptr_eq(&first.file_data, &cloned.file_data));
    for independent in [&second, &consumed] {
        assert!(!Arc::ptr_eq(&first.file_data, &independent.file_data));
        assert_eq!(independent.ephe_path, first.ephe_path);
        assert_eq!(independent.jpl_file, first.jpl_file);
        assert_eq!(independent.topo, first.topo);
        assert_eq!(independent.tid_acc, first.tid_acc);
        assert_eq!(independent.delta_t, first.delta_t);
        assert_eq!(independent.lapse_rate, first.lapse_rate);
        assert_eq!(independent.capture_file_slots, first.capture_file_slots);
        assert_eq!(independent.capture_file_data, first.capture_file_data);
        assert_eq!(
            independent.ephe_path_environment_on_installation,
            first.ephe_path_environment_on_installation
        );
        assert_eq!(
            independent.keep_relative_ephe_path,
            first.keep_relative_ephe_path
        );
        assert!(
            independent
                .file_data
                .lock()
                .unwrap()
                .iter()
                .all(Option::is_none)
        );
    }
    assert!(!Arc::ptr_eq(&second.file_data, &consumed.file_data));
    let snapshot = crate::CurrentFileData {
        path: String::from("owned-test-snapshot.se1"),
        tfstart: 0.0,
        tfend: 1.0,
        denum: 0,
    };
    first.file_data.lock().unwrap()[0] = Some(snapshot.clone());
    assert_eq!(
        cloned.file_data.lock().unwrap()[0].as_ref(),
        Some(&snapshot)
    );
    for independent in [&second, &consumed] {
        assert!(
            independent
                .file_data
                .lock()
                .unwrap()
                .iter()
                .all(Option::is_none)
        );
    }
}
