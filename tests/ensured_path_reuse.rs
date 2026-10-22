//! Guarded free-operation setup reuses the session path rules without changing explicit setters.
//! All reference positions remain in memory in isolated single-test children.
#![cfg(unix)]
use swisseph_bindings::*;
#[path = "support/isolated.rs"]
mod isolated;
#[path = "data_dir.rs"]
mod test_data;
const JD: f64 = 2451545.0;
fn data() -> String {
    let path = test_data::ephemeris_dir().expect("complete external ephemeris data");
    for (name, size) in [("sepl_18.se1", 484061), ("semo_18.se1", 1304771)] {
        assert_eq!(
            std::fs::metadata(std::path::Path::new(&path).join(name))
                .unwrap()
                .len(),
            size,
            "required ephemeris filename/size admission"
        );
    }
    path
}

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
fn ensured_path_reads_match_direct_native_calls() {
    if !isolated::without_ephe_override("ensured_path_reads_match_direct_native_calls") {
        return;
    }
    let data = data();
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
    close().unwrap();
    for _ in 0..2 {
        for (jd, body, flags, status, bits, diagnostic) in &expected {
            ensure_ephe_path(Some(&data)).unwrap();
            let actual = calc_ut(*jd, *body, *flags).unwrap();
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

struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = close();
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn read(path: Option<&str>, day: f64, flags: i32) -> Position {
    ensure_ephe_path(path).unwrap();
    calc_ut(day, MARS, flags).unwrap()
}

#[test]
fn ensured_path_reuses_files_and_respects_explicit_setters_close_and_tidal_state() {
    if !isolated::without_ephe_override(
        "ensured_path_reuses_files_and_respects_explicit_setters_close_and_tidal_state",
    ) {
        return;
    }
    let data = data();
    let scratch =
        Scratch(std::env::temp_dir().join(format!("swisseph-ensured-path-{}", std::process::id())));
    std::fs::create_dir(&scratch.0).unwrap();
    let root = scratch.0.canonicalize().unwrap();
    let full = root.join("full");
    let empty = root.join("empty");
    std::fs::create_dir(&full).unwrap();
    std::fs::create_dir(&empty).unwrap();
    for name in ["sepl_18.se1", "semo_18.se1"] {
        std::os::unix::fs::symlink(std::path::Path::new(&data).join(name), full.join(name))
            .unwrap();
    }
    let path = full.to_str().unwrap();
    let flags = FLG_SWIEPH | FLG_SPEED;
    let expected = read(Some(path), JD, flags);
    require_source_flags(&expected, FLG_SWIEPH).unwrap();
    let saved = get_current_file_data(0)
        .unwrap()
        .expect("selected planet file");
    let link = full.join("sepl_18.se1");
    let source = std::fs::read_link(&link).unwrap();
    std::fs::remove_file(&link).unwrap();
    let repeated = read(Some(path), JD, flags);
    assert!(
        repeated == expected,
        "unchanged guard must preserve exact native output"
    );
    assert!(
        get_current_file_data(0).unwrap() == Some(saved.clone()),
        "unchanged guard must retain the opened file"
    );
    set_ephe_path(Some(path)).unwrap();
    let missing = calc_ut(JD, MARS, flags).unwrap();
    assert!(
        require_source_flags(&missing, FLG_SWIEPH).is_err(),
        "explicit equal-path setter must force reload"
    );
    assert!(
        read(Some(path), JD, flags) == missing,
        "guard must retain the explicit setter's state"
    );
    let _ = read(empty.to_str(), JD, flags);
    std::os::unix::fs::symlink(&source, &link).unwrap();
    assert!(
        read(Some(path), JD, flags) == expected,
        "changed path must restore its own exact native output"
    );
    assert!(
        get_current_file_data(0).unwrap() == Some(saved.clone()),
        "changed path must restore its own file"
    );
    let automatic = get_tid_acc().unwrap();
    set_tid_acc(-20.0).unwrap();
    set_ephe_path(Some(path)).unwrap();
    let expected_tidal = calc_ut(JD, MOON, flags).unwrap();
    set_tid_acc(-20.0).unwrap();
    ensure_ephe_path(Some(path)).unwrap();
    assert!(
        calc_ut(JD, MOON, flags).unwrap() == expected_tidal,
        "manual tidal state must invalidate path reuse"
    );
    assert!(
        get_tid_acc().unwrap().to_bits() == automatic.to_bits(),
        "guard must restore automatic tidal selection"
    );
    set_tid_acc(automatic).unwrap();
    ensure_ephe_path(Some(path)).unwrap();
    let equal_manual = calc_ut(JD, MOON, FLG_MOSEPH).unwrap();
    set_tid_acc(automatic).unwrap();
    set_ephe_path(Some(path)).unwrap();
    assert!(
        calc_ut(JD, MOON, FLG_MOSEPH).unwrap() == equal_manual,
        "equal-valued manual writes must still reset to automatic"
    );
    std::fs::remove_file(&link).unwrap();
    close().unwrap();
    assert!(
        read(Some(path), JD, flags) == missing,
        "close must invalidate retained files"
    );
    assert!(
        get_current_file_data(0).unwrap() != Some(saved),
        "close must remove the previous file state"
    );
    // Admission is shared with the explicit setter and must not alter native state.
    for path in ["bad\0path".to_owned(), "x".repeat(MAX_EPHE_PATH_LEN + 1)] {
        let before = get_current_file_data(0).unwrap();
        let actual = ensure_ephe_path(Some(&path)).unwrap_err();
        let expected = set_ephe_path(Some(&path)).unwrap_err();
        assert!(
            actual == expected,
            "guard and setter must share exact input admission"
        );
        assert!(
            get_current_file_data(0).unwrap() == before,
            "invalid input must not configure native state"
        );
    }
    // A new native thread must not inherit the previous thread's path marker.
    std::os::unix::fs::symlink(&source, &link).unwrap();
    set_ephe_path(Some(path)).unwrap();
    let _ = read(Some(path), JD, flags);
    std::fs::remove_file(&link).unwrap();
    let owned = path.to_owned();
    assert!(
        std::thread::spawn(move || require_source_flags(
            &read(Some(&owned), JD, flags),
            FLG_SWIEPH
        )
        .is_err())
        .join()
        .unwrap(),
        "another thread must apply its own path"
    );
}

#[test]
fn ensured_default_relative_and_environment_paths_track_the_working_directory() {
    let child_mode = std::env::var("SWISSEPH_ENSURE_CWD_CHILD");
    if child_mode.is_err() {
        for mode in [
            "relative",
            "default",
            "environment-relative",
            "environment-absolute",
        ] {
            let root = std::env::temp_dir().join(format!(
                "swisseph-ensured-cwd-{}-{mode}",
                std::process::id()
            ));
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "ensured_default_relative_and_environment_paths_track_the_working_directory",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("SWISSEPH_ENSURE_CWD_CHILD", mode)
                .env("SWISSEPH_ENSURE_CWD_ROOT", &root)
                .env_remove("SE_EPHE_PATH");
            if mode == "environment-relative" {
                command.env("SE_EPHE_PATH", ".");
            }
            if mode == "environment-absolute" {
                command.env("SE_EPHE_PATH", root.join("full"));
            }
            let result = command.output().unwrap();
            assert!(
                result.status.success(),
                "guarded {mode} child failed: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        return;
    }
    let data = data();
    let scratch = Scratch(std::path::PathBuf::from(
        std::env::var_os("SWISSEPH_ENSURE_CWD_ROOT").unwrap(),
    ));
    let full = scratch.0.join("full");
    let empty = scratch.0.join("empty");
    std::fs::create_dir_all(&full).unwrap();
    std::fs::create_dir(&empty).unwrap();
    for name in ["sepl_18.se1", "semo_18.se1"] {
        std::os::unix::fs::symlink(std::path::Path::new(&data).join(name), full.join(name))
            .unwrap();
    }
    let mode = child_mode.unwrap();
    let requested = match mode.as_str() {
        "relative" => Some("."),
        "default" => None,
        _ => Some("/nonexistent/ignored-by-native-environment"),
    };
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(&full).unwrap();
    require_source_flags(&read(requested, JD, FLG_SWIEPH), FLG_SWIEPH)
        .expect("native path must select real files");
    let saved = get_current_file_data(0)
        .unwrap()
        .expect("native file selection");
    std::fs::remove_file(full.join("sepl_18.se1")).unwrap();
    std::env::set_current_dir(&empty).unwrap();
    let changed = read(requested, JD + 1.0, FLG_SWIEPH);
    if mode == "environment-absolute" {
        require_source_flags(&changed, FLG_SWIEPH)
            .expect("absolute environment path must keep files across unrelated cwd changes");
        assert!(
            get_current_file_data(0).unwrap() == Some(saved),
            "absolute path metadata must stay unchanged"
        );
    } else {
        assert!(
            require_source_flags(&changed, FLG_SWIEPH).is_err(),
            "relative/default path must follow the changed cwd"
        );
        assert!(
            get_current_file_data(0).unwrap() != Some(saved),
            "relative/default path must invalidate prior file state"
        );
    }
    std::env::set_current_dir(original).unwrap();
}
