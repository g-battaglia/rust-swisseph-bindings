//! Session path reuse preserves open files while explicit writes still reload.
//! Native comparisons remain in memory and use the selected external data.

#![cfg(unix)]

use swisseph_bindings::native::{
    CHIRON, FLG_SWIEPH, MARS, MOON, SessionBuilder, global::calc_ut, global::close,
    global::get_current_file_data, global::get_tid_acc, global::set_ephe_path, global::set_tid_acc,
    native::require_source_flags,
};

#[path = "support/isolated.rs"]
mod isolated;
#[path = "data_dir.rs"]
mod test_data;

const JD: f64 = 2451545.0;

unsafe extern "C" {
    fn swe_set_ephe_path(path: *const std::ffi::c_char);
    fn swe_calc_ut(
        jd: f64,
        body: i32,
        flags: i32,
        values: *mut f64,
        diagnostic: *mut std::ffi::c_char,
    ) -> i32;
    fn swe_deltat(jd: f64) -> f64;
}

#[test]
fn default_path_snapshot_matches_native_reuse_and_explicit_reload() {
    if !isolated::without_ephe_override(
        "default_path_snapshot_matches_native_reuse_and_explicit_reload",
    ) {
        return;
    }
    let data = test_data::ephemeris_dir().expect("complete external ephemeris data");
    let directory = std::env::temp_dir().join(format!(
        "swisseph-default-path-history-{}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    for name in ["sepl_18.se1", "semo_18.se1", "seas_18.se1"] {
        std::os::unix::fs::symlink(std::path::Path::new(&data).join(name), directory.join(name))
            .unwrap();
    }
    let previous_directory = std::env::current_dir().unwrap();
    std::env::set_current_dir(&directory).unwrap();
    let history =
        || std::array::from_fn::<_, 5, _>(|slot| get_current_file_data(slot as i32).unwrap());
    let mut values = [0.0; 6];
    let mut diagnostic = [0 as std::ffi::c_char; 256];
    // SAFETY: isolated single-thread child, public C ABI, complete initialized
    // output buffers. NULL selects the compiled path, whose first entry is cwd.
    let native_status = unsafe {
        swe_set_ephe_path(std::ptr::null());
        swe_calc_ut(
            JD,
            CHIRON,
            FLG_SWIEPH,
            values.as_mut_ptr(),
            diagnostic.as_mut_ptr(),
        )
    };
    assert!(native_status & FLG_SWIEPH != 0, "native Swiss source");
    // SAFETY: scalar public C call in the same isolated native sequence.
    let expected = unsafe { swe_deltat(JD) };
    let retained = history();
    assert!(
        retained[2].is_some(),
        "native asteroid file must be present"
    );
    // SAFETY: explicit public C path write intentionally closes open files.
    unsafe { swe_set_ephe_path(std::ptr::null()) };
    let reloaded = history();
    assert!(
        retained != reloaded,
        "explicit setter changes native history"
    );

    // Repeat the same setup through public safe APIs. A default-path Session
    // does not imply a fresh explicit path write before every computation.
    close().unwrap();
    set_ephe_path(None).unwrap();
    let session = SessionBuilder::new().default_ephe_path().build().unwrap();
    // An owned session resets foreign model caches on first acquisition. Seed
    // the file history through the same owner before testing steady reuse.
    let seeded = session.calc_ut(JD, CHIRON, FLG_SWIEPH).unwrap();
    require_source_flags(&seeded, FLG_SWIEPH).unwrap();
    assert_eq!(session.deltat(JD).unwrap().to_bits(), expected.to_bits());
    for (slot, expected) in retained.iter().enumerate() {
        assert_eq!(
            session.get_current_file_data(slot as i32).unwrap(),
            *expected
        );
    }
    set_ephe_path(None).unwrap();
    assert_eq!(history(), reloaded);
    // Explicit global path writes invalidate ownership; the next calculation
    // must reinstall complete settings before it can reuse its files.
    session.calc_ut(JD, CHIRON, FLG_SWIEPH).unwrap();
    session.deltat(JD).unwrap();
    for (slot, expected) in retained.iter().enumerate() {
        assert_eq!(
            session.get_current_file_data(slot as i32).unwrap(),
            *expected
        );
    }
    close().unwrap();
    std::env::set_current_dir(previous_directory).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn one_session_reinstalls_its_data_path_across_distinct_threads() {
    if !isolated::without_ephe_override(
        "one_session_reinstalls_its_data_path_across_distinct_threads",
    ) {
        return;
    }
    let data = test_data::ephemeris_dir().expect("complete external ephemeris data");
    let session = SessionBuilder::new().ephe_path(&data).build().unwrap();
    let flags = FLG_SWIEPH | swisseph_bindings::native::FLG_SPEED;
    let expected = session.calc_ut(JD, MARS, flags).unwrap();
    require_source_flags(&expected, FLG_SWIEPH).unwrap();
    let parent = std::thread::current().id();
    let mut identities = std::collections::HashSet::new();
    for _ in 0..4 {
        let worker_session = session.clone();
        let worker_expected = expected.clone();
        let directory = data.clone();
        let identity = std::thread::spawn(move || {
            for _ in 0..3 {
                let actual = worker_session.calc_ut(JD, MARS, flags).unwrap();
                assert!(
                    actual == worker_expected,
                    "thread migration changed the native result"
                );
                let file = get_current_file_data(0).unwrap().unwrap();
                assert!(
                    file.path.starts_with(&directory),
                    "native data path differs"
                );
            }
            std::thread::current().id()
        })
        .join()
        .expect("native worker succeeds");
        assert_ne!(identity, parent);
        assert!(
            identities.insert(identity),
            "worker identities must remain distinct"
        );
        assert!(session.calc_ut(JD, MARS, flags).unwrap() == expected);
    }
}

#[test]
fn reused_session_positions_match_direct_native_calls() {
    if !isolated::without_ephe_override("reused_session_positions_match_direct_native_calls") {
        return;
    }
    let data = test_data::ephemeris_dir().expect("complete external ephemeris data");
    let path = std::ffi::CString::new(data.as_str()).unwrap();
    let mut expected = Vec::new();
    for jd in [JD, 2440000.5] {
        for body in [MARS, MOON] {
            for flags in [
                FLG_SWIEPH | swisseph_bindings::native::FLG_SPEED,
                FLG_SWIEPH | swisseph_bindings::native::FLG_EQUATORIAL,
                swisseph_bindings::native::FLG_MOSEPH | swisseph_bindings::native::FLG_SPEED,
            ] {
                let mut values = [0.0; 6];
                let mut diagnostic = [0 as std::ffi::c_char; 256];
                // SAFETY: isolated single-test child; no other native callers.
                // Public C ABI, live C string and complete initialized buffers.
                let status = unsafe {
                    swe_set_ephe_path(path.as_ptr());
                    swe_calc_ut(
                        jd,
                        body,
                        flags,
                        values.as_mut_ptr(),
                        diagnostic.as_mut_ptr(),
                    )
                };
                assert!(status >= 0, "direct native case must succeed");
                // SAFETY: successful native call writes a terminated diagnostic
                // into this initialized public-ABI-sized buffer.
                let text = unsafe { std::ffi::CStr::from_ptr(diagnostic.as_ptr()) }
                    .to_str()
                    .unwrap()
                    .to_owned();
                expected.push((jd, body, flags, status, values.map(f64::to_bits), text));
            }
        }
    }
    let session = SessionBuilder::new().ephe_path(&data).build().unwrap();
    for _ in 0..2 {
        for (jd, body, flags, status, bits, diagnostic) in &expected {
            let actual = session.calc_ut(*jd, *body, *flags).unwrap();
            assert!(
                actual.returned_flags == *status,
                "native returned flags differ"
            );
            assert!(
                actual.values.map(f64::to_bits) == *bits,
                "native position slots differ"
            );
            assert!(
                actual.diagnostic == *diagnostic,
                "native diagnostic differs"
            );
        }
    }
}

#[test]
fn unchanged_session_path_keeps_files_and_respects_explicit_configuration() {
    if !isolated::without_ephe_override(
        "unchanged_session_path_keeps_files_and_respects_explicit_configuration",
    ) {
        return;
    }
    let data = test_data::ephemeris_dir().expect("complete external ephemeris data");
    let directory =
        std::env::temp_dir().join(format!("swisseph-session-cache-{}", std::process::id()));
    std::fs::create_dir(&directory).expect("owned data directory");
    for name in ["sepl_18.se1", "semo_18.se1", "seas_18.se1", "sefstars.txt"] {
        std::os::unix::fs::symlink(std::path::Path::new(&data).join(name), directory.join(name))
            .expect("owned data link");
    }
    let data = directory.to_str().expect("native path").to_owned();
    let session = SessionBuilder::new()
        .ephe_path(&data)
        .build()
        .expect("configured session");
    let baseline = session.calc_ut(JD, MARS, FLG_SWIEPH).expect("file Mars");
    require_source_flags(&baseline, FLG_SWIEPH).expect("actual Swiss file source");
    let planets = get_current_file_data(0)
        .expect("file metadata")
        .expect("planet file open");
    let next = session.calc_ut(JD + 1.0, MARS, FLG_SWIEPH).unwrap();
    require_source_flags(&next, FLG_SWIEPH).expect("next-day actual Swiss source");
    let planet_link = directory.join("sepl_18.se1");
    let original_planets = std::fs::read_link(&planet_link).unwrap();
    std::fs::remove_file(&planet_link).expect("remove owned name, retaining native open file");
    let repeated = session.calc_ut(JD + 1.0, MARS, FLG_SWIEPH).unwrap();
    require_source_flags(&repeated, FLG_SWIEPH)
        .expect("unchanged session must keep the already-open file");
    assert!(
        repeated == next,
        "open-file reuse preserves every returned value and flag"
    );

    // The public setter is an explicit reload even for the same spelling.
    set_ephe_path(Some(&data)).expect("forced same-path reload");
    let reloaded = calc_ut(JD + 1.0, MARS, FLG_SWIEPH).unwrap();
    assert!(
        require_source_flags(&reloaded, FLG_SWIEPH).is_err(),
        "forced reload must observe the removed name, rather than retain cached positions"
    );
    std::os::unix::fs::symlink(original_planets, &planet_link).unwrap();
    set_ephe_path(Some(&data)).expect("reload restored owned name");
    assert!(session.calc_ut(JD, MARS, FLG_SWIEPH).unwrap() == baseline);

    // Another owner can select a different path. The session restores its own.
    let other = format!("{data}/.");
    let rival = SessionBuilder::new().ephe_path(&other).build().unwrap();
    assert!(rival.calc_ut(JD, MARS, FLG_SWIEPH).unwrap() == baseline);
    assert!(
        get_current_file_data(0)
            .unwrap()
            .unwrap()
            .path
            .starts_with(&other)
    );
    assert!(session.calc_ut(JD, MARS, FLG_SWIEPH).unwrap() == baseline);
    assert_eq!(get_current_file_data(0).unwrap(), Some(planets));
    set_ephe_path(Some(&other)).expect("intervening free setter");
    assert!(session.calc_ut(JD, MARS, FLG_SWIEPH).unwrap() == baseline);

    // Reuse must preserve the path setter's automatic-tidal side effect.
    let automatic = get_tid_acc().unwrap();
    set_tid_acc(-20.0).expect("intervening manual tidal mode");
    let forced = {
        set_ephe_path(Some(&data)).unwrap();
        calc_ut(JD, MOON, FLG_SWIEPH).unwrap()
    };
    set_tid_acc(-20.0).unwrap();
    assert!(session.calc_ut(JD, MOON, FLG_SWIEPH).unwrap() == forced);
    assert_eq!(get_tid_acc().unwrap(), automatic);
    // An equal numerical value can still select manual mode. It must also
    // invalidate reuse, since later automatic model selection differs.
    set_tid_acc(automatic).unwrap();
    let flags = swisseph_bindings::native::FLG_MOSEPH;
    set_ephe_path(Some(&data)).unwrap();
    let automatic_model = calc_ut(JD, MOON, flags).unwrap();
    set_tid_acc(automatic).unwrap();
    assert!(session.calc_ut(JD, MOON, flags).unwrap() == automatic_model);

    close().expect("explicit process-wide reset");
    assert_eq!(get_current_file_data(0).unwrap(), None);
    assert!(session.calc_ut(JD, MARS, FLG_SWIEPH).unwrap() == baseline);
    close().unwrap();
    std::fs::remove_dir_all(directory).expect("remove only owned links");
}

#[test]
fn relative_session_path_requires_rebuild_after_working_directory_change() {
    if !isolated::without_ephe_override(
        "relative_session_path_requires_rebuild_after_working_directory_change",
    ) {
        return;
    }
    let data = test_data::ephemeris_dir().expect("complete external ephemeris data");
    let root = std::env::temp_dir().join(format!("swisseph-session-cwd-{}", std::process::id()));
    let full = root.join("full");
    let empty = root.join("empty");
    std::fs::create_dir_all(&full).unwrap();
    std::fs::create_dir(&empty).unwrap();
    for name in ["sepl_18.se1", "semo_18.se1"] {
        std::os::unix::fs::symlink(std::path::Path::new(&data).join(name), full.join(name))
            .unwrap();
    }
    let original = std::env::current_dir().unwrap();
    for requested in [
        None,
        Some("."),
        Some("/nonexistent;."),
        Some("/nonexistent:."),
    ] {
        std::env::set_current_dir(&full).unwrap();
        let mut builder = SessionBuilder::new();
        let builder = match requested {
            Some(path) => builder.ephe_path(path),
            None => builder.default_ephe_path(),
        };
        let session = swisseph_bindings::native::primary_session(builder).unwrap();
        let position = session.calc_ut(JD, MARS, FLG_SWIEPH).unwrap();
        assert_ne!(
            position.returned_flags & FLG_SWIEPH,
            0,
            "relative data lookup"
        );
        let remembered = get_current_file_data(0).unwrap();
        std::env::set_current_dir(&empty).unwrap();
        let error = match session.calc_ut(JD, MARS, FLG_SWIEPH) {
            Err(error) => error,
            Ok(_) => panic!("primary session adopted changed lookup context"),
        };
        assert_eq!(error.kind(), swisseph_bindings::ErrorKind::InvalidInput);
        assert_eq!(get_current_file_data(0).unwrap(), remembered);
        assert_eq!(get_current_file_data(0).unwrap(), remembered);
        let rebuilt = swisseph_bindings::native::primary_session(builder).unwrap();
        let changed = rebuilt.calc_ut(JD, MARS, FLG_SWIEPH).unwrap();
        assert!(
            changed.returned_flags & FLG_SWIEPH == 0,
            "only an explicit rebuild adopts the new relative lookup context"
        );
        std::env::set_current_dir(&full).unwrap();
        assert_eq!(session.calc_ut(JD, MARS, FLG_SWIEPH).unwrap(), position);
        close().unwrap();
    }
    std::env::set_current_dir(original).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn session_path_respects_native_environment_precedence() {
    let data = test_data::ephemeris_dir().expect("complete external ephemeris data");
    if std::env::var_os("SWISSEPH_PATH_ENV_CHILD").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "session_path_respects_native_environment_precedence",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("SWISSEPH_PATH_ENV_CHILD", "1")
            .env("SE_EPHE_PATH", &data)
            .output()
            .expect("isolated environment child");
        assert!(output.status.success(), "environment child failed");
        return;
    }
    let requested = "/nonexistent/session-ephemeris";
    let session = SessionBuilder::new().ephe_path(requested).build().unwrap();
    let result = session.calc_ut(JD, MARS, FLG_SWIEPH).unwrap();
    require_source_flags(&result, FLG_SWIEPH).expect("environment supplies files");
    let file = get_current_file_data(0).unwrap().unwrap();
    assert!(file.path.starts_with(&data));
    session.calc_ut(JD, MOON, FLG_SWIEPH).unwrap();
    assert_eq!(get_current_file_data(0).unwrap(), Some(file));
}

#[test]
fn absolute_session_paths_keep_open_files_across_directory_changes() {
    let child_mode = std::env::var("SWISSEPH_ABSOLUTE_CWD_CHILD");
    if child_mode.is_err() {
        let mut failures = Vec::new();
        for mode in ["argument", "environment"] {
            let root = std::env::temp_dir().join(format!(
                "swisseph-absolute-cwd-{}-{mode}",
                std::process::id()
            ));
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "absolute_session_paths_keep_open_files_across_directory_changes",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("SWISSEPH_ABSOLUTE_CWD_CHILD", mode)
                .env("SWISSEPH_ABSOLUTE_CWD_ROOT", &root)
                .env_remove("SE_EPHE_PATH");
            if mode == "environment" {
                command.env("SE_EPHE_PATH", root.join("full"));
            }
            let output = command.output().expect("isolated absolute-path child");
            if !output.status.success() {
                failures.push((mode, String::from_utf8_lossy(&output.stderr).into_owned()));
            }
        }
        assert!(
            failures.is_empty(),
            "absolute-path children failed: {failures:?}"
        );
        return;
    }
    let data = test_data::ephemeris_dir().expect("complete external ephemeris data");
    let root = std::path::PathBuf::from(std::env::var_os("SWISSEPH_ABSOLUTE_CWD_ROOT").unwrap());
    let full = root.join("full");
    let empty = root.join("empty");
    std::fs::create_dir_all(&full).unwrap();
    std::fs::create_dir(&empty).unwrap();
    for name in ["sepl_18.se1", "semo_18.se1"] {
        std::os::unix::fs::symlink(std::path::Path::new(&data).join(name), full.join(name))
            .unwrap();
    }
    let requested = if child_mode.unwrap() == "environment" {
        "."
    } else {
        full.to_str().unwrap()
    };
    let session = SessionBuilder::new().ephe_path(requested).build().unwrap();
    let first = session.calc_ut(JD, MARS, FLG_SWIEPH).unwrap();
    require_source_flags(&first, FLG_SWIEPH).expect("absolute data lookup");
    std::fs::remove_file(full.join("sepl_18.se1")).unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(&empty).unwrap();
    let repeated = session.calc_ut(JD + 1.0, MARS, FLG_SWIEPH).unwrap();
    std::env::set_current_dir(original).unwrap();
    close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
    require_source_flags(&repeated, FLG_SWIEPH)
        .expect("an unrelated cwd change must keep the absolute-path file open");
}
