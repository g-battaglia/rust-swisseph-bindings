//! Session path reuse preserves open files while explicit writes still reload.
//! Native comparisons remain in memory and use the selected external data.

#![cfg(unix)]

use swisseph_bindings::{
    FLG_SWIEPH, MARS, MOON, SessionBuilder, calc_ut, close, get_current_file_data, get_tid_acc,
    require_source_flags, set_ephe_path, set_tid_acc,
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
                FLG_SWIEPH | swisseph_bindings::FLG_SPEED,
                FLG_SWIEPH | swisseph_bindings::FLG_EQUATORIAL,
                swisseph_bindings::FLG_MOSEPH | swisseph_bindings::FLG_SPEED,
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
    let flags = swisseph_bindings::FLG_MOSEPH;
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
fn relative_session_path_tracks_working_directory() {
    if !isolated::without_ephe_override("relative_session_path_tracks_working_directory") {
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
    std::env::set_current_dir(&full).unwrap();
    let session = SessionBuilder::new().ephe_path(".").build().unwrap();
    let position = session.calc_ut(JD, MARS, FLG_SWIEPH).unwrap();
    require_source_flags(&position, FLG_SWIEPH).expect("relative data lookup");
    std::env::set_current_dir(&empty).unwrap();
    let position = session.calc_ut(JD, MARS, FLG_SWIEPH).unwrap();
    assert!(
        require_source_flags(&position, FLG_SWIEPH).is_err(),
        "same relative spelling must observe the changed working directory"
    );
    std::env::set_current_dir(original).unwrap();
    close().unwrap();
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
