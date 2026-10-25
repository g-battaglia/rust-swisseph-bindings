//! Compare the former UT guard's public-C sequence with the current typed path.
//! All native positions, diagnostics and shifts live only in memory. A single
//! native acquisition isolates these controls from other crate test threads.

use std::ffi::{CStr, CString, c_char};

use crate::{Calendar, Error, ErrorKind, Position, ffi, test_data};

const BCE: f64 = 1705183.2503587962;
const MODERN: f64 = 2451545.0;
const MASK: i32 = crate::FLG_SWIEPH | crate::FLG_MOSEPH | crate::FLG_JPLEPH;

#[derive(Clone, Copy)]
struct Configuration {
    tidal: f64,
    delta_t: Option<f64>,
}

fn configure(path: &CString, configuration: Configuration) {
    // SAFETY: the test holds native access; this is the original public C
    // close, followed by tracked setters so the typed sidereal/domain state
    // agrees with the native state on every side of each comparison.
    unsafe { ffi::swe_close() };
    crate::config::apply_ephe_path_locked(Some(path)).expect("install native data path");
    crate::config::apply_sid_mode_locked(crate::SIDM_FAGAN_BRADLEY, 0.0, 0.0)
        .expect("install tracked sidereal state");
    crate::config::apply_tid_acc_locked(configuration.tidal).expect("install tidal state");
    crate::config::apply_delta_t_locked(
        configuration.delta_t.unwrap_or(crate::SE_DELTAT_AUTOMATIC),
    )
    .expect("install Delta-T state");
}

fn tidal_bits() -> u64 {
    // SAFETY: scalar native read under the same acquisition as the operation.
    unsafe { ffi::swe_get_tid_acc() }.to_bits()
}

fn delta_t(jd: f64, flags: i32) -> f64 {
    // SAFETY: callers first admit a finite, native-calendar-bounded date.
    // Diagnostics may be null, exactly as in the former validation sequence.
    unsafe { ffi::swe_deltat_ex(jd, flags, std::ptr::null_mut()) }
}

/// Independent native forward calendars define both half-open year domains.
/// Return only the offending effective input, without attempting unsafe C
/// inverse conversion or storing an astronomical reference value.
fn check_calendar(jd: f64) -> Result<(), f64> {
    // Public C calendar flags: Gregorian 1, Julian 0. Do not use a private
    // Rust conversion helper as the authority for these independent bounds.
    for calendar_flag in [1, 0] {
        // SAFETY: public pure forward conversion, complete scalar arguments.
        let (first, end) = unsafe {
            (
                ffi::swe_julday(i32::MIN, 1, 1, 0.0, calendar_flag),
                ffi::swe_julday(i32::MAX, 12, 31, 24.0, calendar_flag),
            )
        };
        if !jd.is_finite() || jd < first || jd >= end {
            return Err(jd);
        }
    }
    Ok(())
}

/// Frozen former sequence: requested, Swiss, Moshier, requested restoration.
/// This C control intentionally retains the now-redundant Swiss call.
fn former_guard(jd: f64, flags: i32) -> Result<(), f64> {
    check_calendar(jd)?;
    check_calendar(jd + delta_t(jd, flags))?;
    if flags == -1 {
        return Ok(());
    }
    let mut result = Ok(());
    for source in [crate::FLG_SWIEPH, crate::FLG_MOSEPH] {
        if let Err(rejected) = check_calendar(jd + delta_t(jd, (flags & !MASK) | source)) {
            result = Err(rejected);
            break;
        }
    }
    let _restored = delta_t(jd, flags);
    result
}

fn compare_guard(expected: Result<(), f64>, actual: Result<(), Error>) {
    match (expected, actual) {
        (Ok(()), Ok(())) => {}
        (Err(rejected), Err(error)) => {
            assert!(
                error.kind() == ErrorKind::InvalidInput,
                "domain error category"
            );
            let expected = format!(
                "calc_ut: Julian Day must be finite with a calendar year representable as i32 ({rejected})"
            );
            assert!(error.message() == expected, "original domain error message");
            assert!(
                error.native_status().is_none(),
                "no requested C calculation on domain rejection"
            );
        }
        _ => panic!("native-calendar admission differs"),
    }
}

#[derive(PartialEq, Eq)]
struct NativeFile {
    path: Vec<u8>,
    start: u64,
    end: u64,
    denum: i32,
}

fn file_observations() -> [Option<NativeFile>; 5] {
    std::array::from_fn(|slot| {
        let mut start = 0.0;
        let mut end = 0.0;
        let mut denum = 0;
        // SAFETY: native access is held, each output has a live scalar slot,
        // and a returned path is copied before another native operation.
        let path = unsafe {
            ffi::swe_get_current_file_data(
                i32::try_from(slot).expect("native file slot"),
                &mut start,
                &mut end,
                &mut denum,
            )
        };
        if path.is_null() {
            None
        } else {
            // SAFETY: non-null native static path, copied under native access.
            let path = unsafe { CStr::from_ptr(path) }.to_bytes().to_vec();
            Some(NativeFile {
                path,
                start: start.to_bits(),
                end: end.to_bits(),
                denum,
            })
        }
    })
}

struct NativePosition {
    status: i32,
    values: [f64; 6],
    diagnostic: String,
    diagnostic_bytes: [c_char; 256],
    files: [Option<NativeFile>; 5],
}

fn native_position(jd: f64, body: i32, flags: i32) -> NativePosition {
    let mut values = [0.0; 6];
    let mut diagnostic = [0 as c_char; 256];
    // SAFETY: admitted finite UT input; complete initialized public-ABI buffers,
    // live through the call, with native access held across the whole sequence.
    let status = unsafe {
        ffi::swe_calc_ut(
            jd,
            body,
            flags,
            values.as_mut_ptr(),
            diagnostic.as_mut_ptr(),
        )
    };
    let diagnostic_bytes = diagnostic;
    // SAFETY: native writes a terminated diagnostic inside this owned buffer.
    let diagnostic = unsafe { CStr::from_ptr(diagnostic.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    NativePosition {
        status,
        values,
        diagnostic,
        diagnostic_bytes,
        files: file_observations(),
    }
}

fn compare_native_position(expected: &NativePosition, actual: &NativePosition) {
    assert!(
        actual.status == expected.status,
        "raw native status and flags"
    );
    assert!(
        actual.values.map(f64::to_bits) == expected.values.map(f64::to_bits),
        "raw six native position bits"
    );
    assert!(
        actual.diagnostic_bytes == expected.diagnostic_bytes,
        "all 256 raw diagnostic bytes"
    );
    assert!(actual.files == expected.files, "native file observations");
}

fn compare_position(expected: NativePosition, actual: Result<Position, Error>) {
    assert!(
        file_observations() == expected.files,
        "typed native file observations"
    );
    if expected.status >= 0 {
        let actual = match actual {
            Ok(position) => position,
            Err(_) => panic!("native successful position rejected"),
        };
        assert!(
            expected.values.iter().all(|value| value.is_finite()),
            "finite C position"
        );
        assert!(
            actual.values.iter().all(|value| value.is_finite()),
            "finite typed position"
        );
        assert!(
            actual.values.map(f64::to_bits) == expected.values.map(f64::to_bits),
            "all six native position bits"
        );
        assert!(
            actual.returned_flags == expected.status,
            "actual source and returned flags"
        );
        assert!(
            actual.diagnostic == expected.diagnostic,
            "successful native diagnostic"
        );
    } else {
        let error = match actual {
            Err(error) => error,
            Ok(_) => panic!("native failing position accepted"),
        };
        assert!(error.kind() == ErrorKind::Native, "native error category");
        assert!(
            error.native_status() == Some(expected.status),
            "native failure status"
        );
        assert!(
            error.native_operation() == Some("calc_ut"),
            "native operation identity"
        );
        assert!(
            error.native_diagnostic() == Some(expected.diagnostic.as_str()),
            "native failure diagnostic"
        );
    }
}

fn guard_case(path: &CString, configuration: Configuration, jd: f64, flags: i32) {
    configure(path, configuration);
    let expected = former_guard(jd, flags);
    let expected_tidal = tidal_bits();
    // Inherited selection is a subsequent native read, not an alternative
    // source probe. Its value and side effects must still match after a guard.
    let expected_following = delta_t(BCE, -1).to_bits();
    configure(path, configuration);
    let actual = super::check_ut_date_locked("calc_ut", jd, flags);
    let actual_tidal = tidal_bits();
    let actual_following = delta_t(BCE, -1).to_bits();
    compare_guard(expected, actual);
    assert!(
        actual_tidal == expected_tidal,
        "tidal selection after validation/restoration"
    );
    assert!(
        actual_following == expected_following,
        "subsequent inherited Delta-T read"
    );
}

#[test]
fn ut_guard_matches_independent_native_sequence() {
    assert!(
        std::env::var_os("SE_EPHE_PATH").is_none(),
        "run this controlled file/fallback comparison without SE_EPHE_PATH"
    );
    let data = test_data::ephemeris_dir().expect("complete external ephemeris data required");
    let empty = std::env::temp_dir().join(format!(
        "swisseph-ut-probe-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&empty).expect("create private missing-data path");
    let paths = [
        CString::new(data).unwrap(),
        CString::new(empty.to_str().unwrap()).unwrap(),
    ];
    let configurations = [
        Configuration {
            tidal: crate::SE_TIDAL_AUTOMATIC,
            delta_t: None,
        },
        Configuration {
            tidal: -26.0,
            delta_t: None,
        },
        Configuration {
            tidal: crate::SE_TIDAL_AUTOMATIC,
            delta_t: Some(0.0),
        },
        Configuration {
            tidal: -20.0,
            delta_t: Some(65.0 / 86400.0),
        },
    ];
    let flags = [
        crate::FLG_SWIEPH | crate::FLG_SPEED,
        crate::FLG_SWIEPH | crate::FLG_SPEED | crate::FLG_EQUATORIAL,
        crate::FLG_SWIEPH | crate::FLG_SPEED | crate::FLG_SIDEREAL,
        crate::FLG_MOSEPH | crate::FLG_SPEED,
        crate::FLG_JPLEPH | crate::FLG_SPEED,
        crate::FLG_SPEED,
        crate::FLG_SWIEPH | crate::FLG_MOSEPH | crate::FLG_SPEED,
        crate::FLG_SWIEPH | crate::FLG_JPLEPH | crate::FLG_SPEED,
    ];
    crate::state::with_native_access(|| {
        for (path_index, path) in paths.iter().enumerate() {
            for configuration in configurations {
                for jd in [BCE, MODERN] {
                    for flags in flags {
                        guard_case(path, configuration, jd, flags);
                        for body in [crate::SUN, crate::MOON, crate::MARS, crate::ECL_NUT, -999] {
                            configure(path, configuration);
                            assert!(former_guard(jd, flags).is_ok(), "admitted native input");
                            let expected = native_position(jd, body, flags);
                            if jd == MODERN
                                && body == crate::MOON
                                && flags == (crate::FLG_SWIEPH | crate::FLG_SPEED)
                            {
                                let source = if path_index == 0 {
                                    crate::FLG_SWIEPH
                                } else {
                                    crate::FLG_MOSEPH
                                };
                                assert!(
                                    expected.status & MASK == source,
                                    "actual file-backed and missing-data source controls"
                                );
                            }
                            let expected_tidal = tidal_bits();
                            let following = native_position(
                                jd + 0.25,
                                crate::MOON,
                                crate::FLG_MOSEPH | crate::FLG_SPEED,
                            );
                            let following_tidal = tidal_bits();
                            configure(path, configuration);
                            assert!(
                                super::check_before_calc_ut_locked("calc_ut", jd, flags).is_ok(),
                                "scoped native preflight"
                            );
                            let scoped = native_position(jd, body, flags);
                            compare_native_position(&expected, &scoped);
                            assert!(
                                tidal_bits() == expected_tidal,
                                "tidal state after scoped native prefix"
                            );
                            configure(path, configuration);
                            // This is the exact typed operation called by both
                            // the free calc_ut and Session::calc_ut under lock.
                            let actual = crate::positions::calc_ut_locked(jd, body, flags);
                            assert!(
                                tidal_bits() == expected_tidal,
                                "tidal state after typed calculation"
                            );
                            compare_position(expected, actual);
                            let actual_following = crate::positions::calc_ut_locked(
                                jd + 0.25,
                                crate::MOON,
                                crate::FLG_MOSEPH | crate::FLG_SPEED,
                            );
                            compare_position(following, actual_following);
                            assert!(
                                tidal_bits() == following_tidal,
                                "subsequent calculation state"
                            );
                        }
                    }
                }
            }
        }
        // Pin Delta-T for extreme calendar edges: do not ask astronomical
        // date models to operate beyond their own arithmetic preconditions.
        let first = crate::julday(i32::MIN, 1, 1, 0.0, Calendar::Gregorian).max(crate::julday(
            i32::MIN,
            1,
            1,
            0.0,
            Calendar::Julian,
        ));
        let end = crate::julday(i32::MAX, 12, 31, 24.0, Calendar::Gregorian).min(crate::julday(
            i32::MAX,
            12,
            31,
            24.0,
            Calendar::Julian,
        ));
        for flags in flags.into_iter().chain([-1]) {
            for (jd, shift) in [
                (first.next_down(), 0.0),
                (first, 0.0),
                (first, -1.0),
                (first, 1.0),
                (end.next_down(), 0.0),
                (end.next_down(), 1.0),
                (end.next_down(), -1.0),
                (end, 0.0),
                (BCE, 1e100),
                (BCE, -1e100),
            ] {
                guard_case(
                    &paths[0],
                    Configuration {
                        tidal: crate::SE_TIDAL_AUTOMATIC,
                        delta_t: Some(shift),
                    },
                    jd,
                    flags,
                );
            }
        }
        // Leave tracked and native configuration in agreement after the raw-C
        // control, with no reference to the soon-removed private empty path.
        configure(&paths[0], configurations[0]);
        Ok(())
    })
    .expect("exclusive native controls");
    std::fs::remove_dir(empty).expect("remove private missing-data path");
}

#[test]
fn scoped_calc_ut_preflight_defers_only_the_unused_success_restore() {
    let path = CString::new(test_data::ephemeris_dir().expect("external ephemeris data")).unwrap();
    let configuration = Configuration {
        tidal: crate::SE_TIDAL_AUTOMATIC,
        delta_t: None,
    };
    crate::state::with_native_access(|| {
        for flags in [
            crate::FLG_SWIEPH | crate::FLG_SPEED,
            crate::FLG_MOSEPH | crate::FLG_SPEED,
            crate::FLG_JPLEPH | crate::FLG_SPEED,
            crate::FLG_SPEED,
            MASK | crate::FLG_SPEED,
        ] {
            configure(&path, configuration);
            assert!(former_guard(BCE, flags).is_ok(), "reference input admitted");
            let requested_tidal = tidal_bits();
            let expected = native_position(BCE, crate::MOON, flags);
            let expected_tidal = tidal_bits();
            configure(&path, configuration);
            let _fallback_shift = delta_t(BCE, (flags & !MASK) | crate::FLG_MOSEPH);
            let fallback_tidal = tidal_bits();
            if flags == (crate::FLG_SWIEPH | crate::FLG_SPEED) {
                // A positive control distinguishes a deferred restore from
                // an accidentally unchanged eager implementation.
                assert!(
                    requested_tidal != fallback_tidal,
                    "distinct automatic selections"
                );
            }
            configure(&path, configuration);
            assert!(
                super::check_before_calc_ut_locked("calc_ut", BCE, flags).is_ok(),
                "scoped input admitted"
            );
            // Test-only scalar observation; it neither changes configuration
            // nor performs a calculation before the production C prefix.
            assert!(
                tidal_bits() == fallback_tidal,
                "successful restore is deferred"
            );
            let actual = native_position(BCE, crate::MOON, flags);
            compare_native_position(&expected, &actual);
            assert!(
                tidal_bits() == expected_tidal,
                "native prefix restores selection"
            );
        }
        configure(&path, configuration);
        Ok(())
    })
    .expect("exclusive deferred-restoration control");
}

#[test]
fn scoped_calc_ut_preflight_preserves_sidereal_inherited_and_error_paths() {
    let path = CString::new(test_data::ephemeris_dir().expect("external ephemeris data")).unwrap();
    let configuration = Configuration {
        tidal: crate::SE_TIDAL_AUTOMATIC,
        delta_t: Some(1.0),
    };
    // Public native SE_SIDBIT_USER_UT. The tracked setter stores this same
    // explicit input; the C reference independently checks its UT epoch.
    const USER_UT: i32 = 1024;
    crate::state::with_native_access(|| {
        let end = crate::julday(i32::MAX, 12, 31, 24.0, Calendar::Gregorian).min(crate::julday(
            i32::MAX,
            12,
            31,
            24.0,
            Calendar::Julian,
        ));
        let mut sidereal_rejections = 0;
        for epoch in [MODERN, end.next_down()] {
            for flags in [
                crate::FLG_SWIEPH | crate::FLG_SPEED | crate::FLG_SIDEREAL,
                -1,
            ] {
                configure(&path, configuration);
                crate::config::apply_sid_mode_locked(crate::SIDM_USER | USER_UT, epoch, 0.0)
                    .expect("install tracked UT reference");
                let expected = former_guard(BCE, flags).and_then(|()| {
                    if flags != -1 && flags & crate::FLG_SIDEREAL != 0 {
                        former_guard(epoch, flags)
                    } else {
                        Ok(())
                    }
                });
                let expected_tidal = tidal_bits();
                let expected_files = file_observations();
                configure(&path, configuration);
                crate::config::apply_sid_mode_locked(crate::SIDM_USER | USER_UT, epoch, 0.0)
                    .expect("reinstall tracked UT reference");
                let actual = super::check_before_calc_ut_locked("calc_ut", BCE, flags);
                if flags != -1 && flags & crate::FLG_SIDEREAL != 0 && expected.is_err() {
                    sidereal_rejections += 1;
                }
                compare_guard(expected, actual);
                assert!(
                    tidal_bits() == expected_tidal,
                    "retained preflight tidal state"
                );
                assert!(
                    file_observations() == expected_files,
                    "retained preflight files"
                );
            }
        }
        assert!(sidereal_rejections == 1, "UT reference rejection exercised");
        for flags in [crate::FLG_SWIEPH | crate::FLG_SPEED, -1] {
            for shift in [1e100, -1e100] {
                let invalid = Configuration {
                    delta_t: Some(shift),
                    ..configuration
                };
                configure(&path, invalid);
                let expected = former_guard(BCE, flags);
                assert!(expected.is_err(), "reference date rejection exercised");
                let expected_tidal = tidal_bits();
                let expected_files = file_observations();
                configure(&path, invalid);
                let actual = super::check_before_calc_ut_locked("calc_ut", BCE, flags);
                compare_guard(expected, actual);
                assert!(tidal_bits() == expected_tidal, "error-path tidal selection");
                assert!(file_observations() == expected_files, "error-path files");
            }
        }
        configure(
            &path,
            Configuration {
                delta_t: None,
                ..configuration
            },
        );
        Ok(())
    })
    .expect("exclusive preflight-retention controls");
}
