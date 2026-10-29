//! Exact native positions under Session sidereal reuse and configuration resets.
//! The independent public-C baseline repeats the setter before every call;
//! results, diagnostics and reference arrays remain in memory only.

use std::ffi::{CStr, CString, c_char};
use swisseph_bindings::native::*;
use swisseph_bindings::native::{
    global::close, global::set_ephe_path, global::set_sid_mode, native::close_preserving_path,
    native::set_jpl_file,
};

#[path = "support/isolated.rs"]
mod isolated;

/// Public-header sidereal option selecting the mode's original precession model.
const SIDBIT_PREC_ORIG: i32 = 8192;

unsafe extern "C" {
    fn swe_close();
    fn swe_set_ephe_path(path: *const c_char);
    fn swe_set_sid_mode(mode: i32, epoch: f64, offset: f64);
    fn swe_calc_ut(
        jd: f64,
        body: i32,
        flags: i32,
        values: *mut f64,
        diagnostic: *mut c_char,
    ) -> i32;
}

/// Input identity and the complete public position from independent C calls.
type Case = (f64, i32, i32, Position);

/// Build an in-memory baseline using the former repeated-setter lifecycle.
fn reference(path: &str, mode: i32, epoch: f64, offset: f64) -> Vec<Case> {
    let path = CString::new(path).unwrap();
    // SAFETY: isolated native test, live NUL-terminated path and no concurrent
    // callers. Public close gives each mode an independent initial C state.
    unsafe {
        swe_close();
        swe_set_ephe_path(path.as_ptr());
    }
    let mut cases = Vec::new();
    for jd in [2451545.0, 2460310.5] {
        for body in [SUN, MOON, MERCURY, MARS, JUPITER, PLUTO] {
            for flags in [
                FLG_SWIEPH | FLG_SPEED | FLG_SIDEREAL,
                FLG_SWIEPH | FLG_SPEED | FLG_SIDEREAL | FLG_EQUATORIAL,
                FLG_SWIEPH | FLG_SPEED,
            ] {
                let mut values = [0.0; 6];
                let mut diagnostic = [0 as c_char; 256];
                // SAFETY: finite admitted inputs and complete initialized
                // public-ABI output buffers, in this isolated serial caller.
                let returned_flags = unsafe {
                    swe_set_sid_mode(mode, epoch, offset);
                    swe_calc_ut(
                        jd,
                        body,
                        flags,
                        values.as_mut_ptr(),
                        diagnostic.as_mut_ptr(),
                    )
                };
                assert!(returned_flags >= 0, "native position succeeds");
                assert!(returned_flags & FLG_SWIEPH != 0, "actual Swiss source");
                // SAFETY: successful C call writes a NUL-terminated diagnostic
                // into the initialized buffer; copy before another C call.
                let diagnostic = unsafe { CStr::from_ptr(diagnostic.as_ptr()) }
                    .to_str()
                    .unwrap()
                    .to_owned();
                cases.push((
                    jd,
                    body,
                    flags,
                    Position {
                        values,
                        returned_flags,
                        diagnostic,
                    },
                ));
            }
        }
    }
    cases
}

/// Compare all six exact bits, source/flags and complete native diagnostic.
fn compare(session: &Session, cases: &[Case]) {
    for (jd, body, flags, expected) in cases {
        let actual = session.calc_ut(*jd, *body, *flags).unwrap();
        assert!(
            actual.values.map(f64::to_bits) == expected.values.map(f64::to_bits)
                && actual.returned_flags == expected.returned_flags
                && actual.diagnostic == expected.diagnostic,
            "sidereal lifecycle differs from independent C"
        );
    }
}

#[test]
fn configured_sidereal_reuse_matches_c_across_all_invalidation_paths() {
    if !isolated::without_ephe_override(
        "configured_sidereal_reuse_matches_c_across_all_invalidation_paths",
    ) {
        return;
    }
    let data = std::env::var("SWISSEPH_EPHE_DIR").expect("explicit complete Swiss data");
    for (mode, epoch, offset) in [
        (SIDM_LAHIRI, 0.0, 0.0),
        (SIDM_FAGAN_BRADLEY | SIDBIT_PREC_ORIG, 0.0, 0.0),
        (SIDM_TRUE_SHEORAN, 0.0, 0.0),
        (SIDM_USER | 1024, 2451545.0, 17.5),
    ] {
        let cases = reference(&data, mode, epoch, offset);
        close().unwrap();
        let session = SessionBuilder::new()
            .ephe_path(&data)
            .sid_mode(mode, epoch, offset)
            .build()
            .unwrap();
        compare(&session, &cases);

        // Each operation can replace mode state or clear native models while
        // leaving mode fields unchanged. The next Session call must restore
        // its complete configuration, including PREC_ORIG's precession model.
        let resets: [fn(&str); 5] = [
            |_| set_sid_mode(SIDM_RAMAN, 0.0, 0.0).unwrap(),
            |path| set_ephe_path(Some(path)).unwrap(),
            |_| set_jpl_file("__sidereal_reuse_missing__.eph").unwrap(),
            |_| close().unwrap(),
            |_| close_preserving_path().unwrap(),
        ];
        for reset in resets {
            reset(&data);
            compare(&session, &cases);
        }
        // A repeated public write remains a forced native setter. Its state
        // is already installed when the Session next acquires native access.
        set_sid_mode(mode, epoch, offset).unwrap();
        compare(&session, &cases);
        for _ in 0..2 {
            let moved = session.clone();
            let expected = cases.clone();
            std::thread::spawn(move || compare(&moved, &expected))
                .join()
                .unwrap();
        }
        compare(&session, &cases);
    }
    close().unwrap();
}
