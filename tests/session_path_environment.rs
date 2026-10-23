//! Explicit environment-sampling policies retain the native setter lifecycle.
//! Isolated children compare complete native outputs in memory, never fixtures.

#![cfg(unix)]

use std::ffi::{CStr, c_char};
use swisseph_bindings::{
    FLG_SWIEPH, MARS, SessionBuilder, calc_ut, close, get_current_file_data, require_source_flags,
    set_ephe_path,
};

#[path = "support/isolated.rs"]
mod isolated;
#[path = "data_dir.rs"]
mod test_data;

unsafe extern "C" {
    fn swe_calc_ut(
        jd: f64,
        body: i32,
        flags: i32,
        values: *mut f64,
        diagnostic: *mut c_char,
    ) -> i32;
    fn swe_houses_ex2(
        jd: f64,
        flags: i32,
        lat: f64,
        lon: f64,
        system: i32,
        cusps: *mut f64,
        angles: *mut f64,
        speeds: *mut f64,
        angle_speeds: *mut f64,
        diagnostic: *mut c_char,
    ) -> i32;
}

/// Sample the public C ABI without installing a path or persisting its values.
fn assert_direct_native(session: &swisseph_bindings::Session, jd: f64) {
    let mut values = [0.0; 6];
    let mut diagnostic = [0 as c_char; 256];
    // SAFETY: isolated single-thread child, public ABI and complete live buffers.
    let status = unsafe {
        swe_calc_ut(
            jd,
            MARS,
            FLG_SWIEPH,
            values.as_mut_ptr(),
            diagnostic.as_mut_ptr(),
        )
    };
    assert!(status >= 0, "native position must succeed");
    // SAFETY: native success terminates the initialized diagnostic buffer.
    let text = unsafe { CStr::from_ptr(diagnostic.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    let actual = session.calc_ut(jd, MARS, FLG_SWIEPH).unwrap();
    assert!(actual.returned_flags == status, "native flags changed");
    assert!(
        actual.values.map(f64::to_bits) == values.map(f64::to_bits),
        "native values changed"
    );
    assert!(actual.diagnostic == text, "native diagnostic changed");

    let (mut cusps, mut angles, mut speeds, mut angle_speeds) =
        ([0.0; 13], [0.0; 10], [0.0; 13], [0.0; 10]);
    let mut diagnostic = [0 as c_char; 256];
    // SAFETY: same isolated child, standard P layout and complete native buffers.
    let status = unsafe {
        swe_houses_ex2(
            jd,
            0,
            51.5,
            -0.12,
            i32::from(b'P'),
            cusps.as_mut_ptr(),
            angles.as_mut_ptr(),
            speeds.as_mut_ptr(),
            angle_speeds.as_mut_ptr(),
            diagnostic.as_mut_ptr(),
        )
    };
    assert!(status >= 0, "native houses must succeed");
    let actual = session.houses_ex2(jd, 0, 51.5, -0.12, b'P').unwrap();
    assert!(
        actual
            .cusps
            .iter()
            .zip(&cusps[1..])
            .all(|(a, b)| a.to_bits() == b.to_bits()),
        "native cusps changed"
    );
    assert!(
        actual
            .angles
            .iter()
            .zip(&angles[..8])
            .all(|(a, b)| a.to_bits() == b.to_bits()),
        "native angles changed"
    );
    assert!(
        actual
            .cusp_speeds
            .iter()
            .zip(&speeds[1..])
            .all(|(a, b)| a.to_bits() == b.to_bits()),
        "native cusp speeds changed"
    );
    assert!(
        actual
            .angle_speeds
            .iter()
            .zip(&angle_speeds[..8])
            .all(|(a, b)| a.to_bits() == b.to_bits()),
        "native angle speeds changed"
    );
}

/// Environment mutation is restricted to an isolated single-thread child.
fn environment(path: &std::path::Path) {
    // SAFETY: this executable's selected child has one thread and no concurrent C calls.
    unsafe { std::env::set_var("SE_EPHE_PATH", path) };
}

#[test]
fn installation_policy_matches_c_and_default_policy_still_refreshes() {
    if !isolated::without_ephe_override(
        "installation_policy_matches_c_and_default_policy_still_refreshes",
    ) {
        return;
    }
    let data = test_data::ephemeris_dir().expect("complete external data");
    let empty = std::env::temp_dir().join(format!("swisseph-environment-{}", std::process::id()));
    std::fs::create_dir(&empty).unwrap();
    let requested = "/nonexistent/explicit-session-path";
    let installed = SessionBuilder::new()
        .ephe_path(requested)
        .refresh_ephe_path_environment(false)
        .build()
        .unwrap();
    let refreshing = SessionBuilder::new().ephe_path(requested).build().unwrap();
    environment(std::path::Path::new(&data));
    set_ephe_path(Some(requested)).unwrap();
    require_source_flags(
        &installed.calc_ut(2451545.0, MARS, FLG_SWIEPH).unwrap(),
        FLG_SWIEPH,
    )
    .unwrap();
    environment(&empty);
    assert_direct_native(&installed, 2451546.0);
    assert_direct_native(&installed.clone(), 2451547.0);
    assert!(
        get_current_file_data(0)
            .unwrap()
            .unwrap()
            .path
            .starts_with(&data)
    );

    // Existing default semantics intentionally observe the changed environment.
    let refreshed = refreshing.calc_ut(2451548.0, MARS, FLG_SWIEPH).unwrap();
    assert!(
        require_source_flags(&refreshed, FLG_SWIEPH).is_err(),
        "default no longer refreshes environment"
    );
    assert_direct_native(&installed, 2451549.0);
    environment(std::path::Path::new(&data));
    assert!(
        require_source_flags(
            &installed.calc_ut(2451550.0, MARS, FLG_SWIEPH).unwrap(),
            FLG_SWIEPH
        )
        .is_err(),
        "session-local metadata hid another policy's installation"
    );
    // An explicit setter must sample again, including the identical path spelling.
    set_ephe_path(Some(requested)).unwrap();
    assert_direct_native(&installed, 2451551.0);
    require_source_flags(&calc_ut(2451551.0, MARS, FLG_SWIEPH).unwrap(), FLG_SWIEPH).unwrap();
    environment(&empty);
    set_ephe_path(Some(requested)).unwrap();
    assert!(
        require_source_flags(
            &installed.calc_ut(2451552.0, MARS, FLG_SWIEPH).unwrap(),
            FLG_SWIEPH
        )
        .is_err()
    );
    environment(std::path::Path::new(&data));
    close().unwrap();
    require_source_flags(
        &installed.calc_ut(2451553.0, MARS, FLG_SWIEPH).unwrap(),
        FLG_SWIEPH,
    )
    .unwrap();
    close().unwrap();
    std::fs::remove_dir(empty).unwrap();
}

#[test]
fn installation_policy_uses_globally_installed_environment_for_cwd_checks() {
    if !isolated::without_ephe_override(
        "installation_policy_uses_globally_installed_environment_for_cwd_checks",
    ) {
        return;
    }
    let data = test_data::ephemeris_dir().expect("complete external data");
    let root =
        std::env::temp_dir().join(format!("swisseph-environment-cwd-{}", std::process::id()));
    let full = root.join("full");
    let empty = root.join("empty");
    std::fs::create_dir_all(&full).unwrap();
    std::fs::create_dir(&empty).unwrap();
    for name in ["sepl_18.se1", "semo_18.se1"] {
        std::os::unix::fs::symlink(std::path::Path::new(&data).join(name), full.join(name))
            .unwrap();
    }
    let original = std::env::current_dir().unwrap();
    let session = SessionBuilder::new()
        .ephe_path("/nonexistent/session")
        .refresh_ephe_path_environment(false)
        .build()
        .unwrap();
    environment(&full);
    set_ephe_path(Some("/nonexistent/session")).unwrap();
    require_source_flags(
        &session.calc_ut(2451545.0, MARS, FLG_SWIEPH).unwrap(),
        FLG_SWIEPH,
    )
    .unwrap();
    // Newly changed relative bytes must not reclassify the installed absolute path.
    environment(std::path::Path::new("."));
    std::env::set_current_dir(&empty).unwrap();
    assert_direct_native(&session, 2451546.0);
    require_source_flags(&calc_ut(2451546.0, MARS, FLG_SWIEPH).unwrap(), FLG_SWIEPH).unwrap();
    // Conversely a saved relative path must still notice a real cwd change,
    // even after the environment now spells an absolute missing directory.
    std::env::set_current_dir(&full).unwrap();
    set_ephe_path(Some("/nonexistent/session")).unwrap();
    require_source_flags(
        &session.calc_ut(2451547.0, MARS, FLG_SWIEPH).unwrap(),
        FLG_SWIEPH,
    )
    .unwrap();
    environment(&empty);
    std::env::set_current_dir(&empty).unwrap();
    assert!(
        require_source_flags(
            &session.calc_ut(2451548.0, MARS, FLG_SWIEPH).unwrap(),
            FLG_SWIEPH
        )
        .is_err()
    );
    std::env::set_current_dir(original).unwrap();
    close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
