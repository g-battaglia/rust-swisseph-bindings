//! Native close retains its search path without eagerly reading modern data.
//! Numerical values are compared in memory against the unmodified C operation.

use std::ffi::{CStr, CString, c_char};
use swisseph_bindings::{
    FLG_EQUATORIAL, FLG_SIDEREAL, FLG_SPEED, FLG_SWIEPH, SIDM_LAHIRI, SIDM_USER, SessionBuilder,
    close_preserving_path, set_ephe_path, set_sid_mode,
};

#[path = "support/isolated.rs"]
mod isolated;
#[path = "data_dir.rs"]
mod test_data;

unsafe extern "C" {
    fn swe_set_ephe_path(path: *const c_char);
    fn swe_set_sid_mode(mode: i32, epoch: f64, offset: f64);
    fn swe_close();
    fn swe_fixstar_ut(
        name: *mut c_char,
        day: f64,
        flags: i32,
        values: *mut f64,
        diagnostic: *mut c_char,
    ) -> i32;
}

#[test]
fn close_drops_files_but_keeps_explicit_delta_t() {
    if !isolated::without_ephe_override("close_drops_files_but_keeps_explicit_delta_t") {
        return;
    }
    let data = test_data::ephemeris_dir().expect("complete explicit modern data");
    set_ephe_path(Some(&data)).unwrap();
    let session = SessionBuilder::new().ephe_path(&data).build().unwrap();
    session
        .calc_ut(2451545.0, swisseph_bindings::MARS, FLG_SWIEPH)
        .unwrap();
    assert!(
        swisseph_bindings::get_current_file_data(0)
            .unwrap()
            .is_some()
    );
    let delta_t = 65.0 / 86400.0;
    swisseph_bindings::set_delta_t_userdef(Some(delta_t)).unwrap();
    close_preserving_path().unwrap();
    assert!(
        swisseph_bindings::get_current_file_data(0)
            .unwrap()
            .is_none()
    );
    assert!(swisseph_bindings::deltat(1700000.5).unwrap().to_bits() == delta_t.to_bits());
    // Session validation must retain the same explicit override as native C.
    session
        .fixstar_ut("Regulus", 1700000.5, FLG_SWIEPH)
        .unwrap();
    assert!(swisseph_bindings::deltat(1700000.5).unwrap().to_bits() == delta_t.to_bits());
    swisseph_bindings::set_delta_t_userdef(None).unwrap();
    close_preserving_path().unwrap();
}

#[cfg(unix)]
#[test]
fn close_does_not_relabel_relative_path_identity() {
    if !isolated::without_ephe_override("close_does_not_relabel_relative_path_identity") {
        return;
    }
    let data = test_data::ephemeris_dir().expect("complete explicit modern data");
    let root = std::env::temp_dir().join(format!("swisseph-close-path-{}", std::process::id()));
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
    let before = session
        .calc_ut(2451545.0, swisseph_bindings::MARS, FLG_SWIEPH)
        .unwrap();
    swisseph_bindings::require_source_flags(&before, FLG_SWIEPH).unwrap();
    // Native path installation happened in `full`, not the reset directory.
    std::env::set_current_dir(&empty).unwrap();
    close_preserving_path().unwrap();
    let after = session
        .calc_ut(2451545.0, swisseph_bindings::MARS, FLG_SWIEPH)
        .unwrap();
    assert!(swisseph_bindings::require_source_flags(&after, FLG_SWIEPH).is_err());
    close_preserving_path().unwrap();
    std::env::set_current_dir(original).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

struct Observation {
    bits: [u64; 6],
    name: String,
    flags: i32,
    diagnostic: String,
}

fn direct_star(query: &str, day: f64, flags: i32) -> Observation {
    let mut name = [0 as c_char; 513];
    for (target, byte) in name.iter_mut().zip(query.bytes()) {
        *target = byte as c_char;
    }
    let mut values = [0.0; 6];
    let mut diagnostic = [0 as c_char; 256];
    // SAFETY: isolated child, complete initialized writable buffers and no
    // concurrent native caller. Strings remain alive throughout the C call.
    let returned = unsafe {
        swe_fixstar_ut(
            name.as_mut_ptr(),
            day,
            flags,
            values.as_mut_ptr(),
            diagnostic.as_mut_ptr(),
        )
    };
    assert!(returned >= 0, "direct native star must resolve");
    // SAFETY: successful native calls terminate both initialized buffers.
    let (name, diagnostic) = unsafe {
        (
            CStr::from_ptr(name.as_ptr()).to_str().unwrap().to_owned(),
            CStr::from_ptr(diagnostic.as_ptr())
                .to_str()
                .unwrap()
                .to_owned(),
        )
    };
    Observation {
        bits: values.map(f64::to_bits),
        name,
        flags: returned,
        diagnostic,
    }
}

#[test]
fn configured_stars_after_close_match_direct_native_sequence() {
    if !isolated::without_ephe_override("configured_stars_after_close_match_direct_native_sequence")
    {
        return;
    }
    let data = test_data::ephemeris_dir().expect("complete explicit modern data");
    let path = CString::new(data.as_str()).unwrap();
    let queries = [
        "Regulus",
        "Regulus,alLeo",
        ",alLeo",
        "Algenib",
        "1",
        "2",
        ",alOri",
    ];
    let flags = [
        FLG_SWIEPH | FLG_SPEED,
        FLG_SWIEPH | FLG_SPEED | FLG_EQUATORIAL,
        FLG_SWIEPH | FLG_SPEED | FLG_SIDEREAL,
    ];
    for mode in [0, SIDM_LAHIRI, SIDM_USER] {
        // SAFETY: isolated child; scalar native configuration and a live path.
        unsafe {
            swe_close();
            swe_set_ephe_path(path.as_ptr());
            swe_set_sid_mode(mode, 2451545.0, 24.0);
        }
        let _modern = direct_star("Regulus", 2451545.0, flags[0]);
        // SAFETY: isolated child; close leaves the native search path installed.
        unsafe { swe_close() };
        let expected: Vec<_> = flags
            .iter()
            .flat_map(|flags| {
                queries
                    .iter()
                    .map(move |query| (*flags, *query, direct_star(query, 1700000.5, *flags)))
            })
            .collect();

        set_ephe_path(Some(&data)).unwrap();
        set_sid_mode(mode, 2451545.0, 24.0).unwrap();
        let session = SessionBuilder::new().ephe_path(&data).build().unwrap();
        session.fixstar_ut("Regulus", 2451545.0, flags[0]).unwrap();
        close_preserving_path().unwrap();
        for (flags, query, expected) in expected {
            let actual = session.fixstar_ut(query, 1700000.5, flags).unwrap();
            assert!(
                actual.values.map(f64::to_bits) == expected.bits,
                "post-close native coordinate/rate words differ"
            );
            assert!(
                actual.resolved_name == expected.name,
                "resolved name differs"
            );
            assert!(
                actual.returned_flags == expected.flags,
                "returned flags differ"
            );
            assert!(
                actual.diagnostic == expected.diagnostic,
                "native diagnostic differs"
            );
        }
    }
}
