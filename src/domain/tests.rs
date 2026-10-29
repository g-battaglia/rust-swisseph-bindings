//! Compare the former UT guard's public-C sequence with the current typed path.
//! All native positions, diagnostics and shifts live only in memory. A single
//! native acquisition isolates these controls from other crate test threads.

use std::ffi::{CStr, CString, c_char};

use crate::legacy::{Calendar, Error, ErrorKind, Position};
use crate::{ffi, test_data};

const BCE: f64 = 1705183.2503587962;
const MODERN: f64 = 2451545.0;
const MASK: i32 =
    crate::constants::FLG_SWIEPH | crate::constants::FLG_MOSEPH | crate::constants::FLG_JPLEPH;

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
    crate::config::apply_sid_mode_locked(crate::constants::SIDM_FAGAN_BRADLEY, 0.0, 0.0)
        .expect("install tracked sidereal state");
    crate::config::apply_tid_acc_locked(configuration.tidal).expect("install tidal state");
    crate::config::apply_delta_t_locked(
        configuration
            .delta_t
            .unwrap_or(crate::constants::SE_DELTAT_AUTOMATIC),
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
    for source in [crate::constants::FLG_SWIEPH, crate::constants::FLG_MOSEPH] {
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

struct NativePosition {
    status: i32,
    values: [f64; 6],
    diagnostic: String,
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
    // SAFETY: native writes a terminated diagnostic inside this owned buffer.
    let diagnostic = unsafe { CStr::from_ptr(diagnostic.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    NativePosition {
        status,
        values,
        diagnostic,
    }
}

fn compare_position(expected: NativePosition, actual: Result<Position, Error>) {
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
            tidal: crate::constants::SE_TIDAL_AUTOMATIC,
            delta_t: None,
        },
        Configuration {
            tidal: -26.0,
            delta_t: None,
        },
        Configuration {
            tidal: crate::constants::SE_TIDAL_AUTOMATIC,
            delta_t: Some(0.0),
        },
        Configuration {
            tidal: -20.0,
            delta_t: Some(65.0 / 86400.0),
        },
    ];
    let flags = [
        crate::constants::FLG_SWIEPH | crate::constants::FLG_SPEED,
        crate::constants::FLG_SWIEPH
            | crate::constants::FLG_SPEED
            | crate::constants::FLG_EQUATORIAL,
        crate::constants::FLG_SWIEPH | crate::constants::FLG_SPEED | crate::constants::FLG_SIDEREAL,
        crate::constants::FLG_MOSEPH | crate::constants::FLG_SPEED,
        crate::constants::FLG_JPLEPH | crate::constants::FLG_SPEED,
        crate::constants::FLG_SPEED,
        crate::constants::FLG_SWIEPH | crate::constants::FLG_MOSEPH | crate::constants::FLG_SPEED,
        crate::constants::FLG_SWIEPH | crate::constants::FLG_JPLEPH | crate::constants::FLG_SPEED,
    ];
    crate::state::with_native_access(|| {
        for (path_index, path) in paths.iter().enumerate() {
            for configuration in configurations {
                for jd in [BCE, MODERN] {
                    for flags in flags {
                        guard_case(path, configuration, jd, flags);
                        for body in [
                            crate::constants::SUN,
                            crate::constants::MOON,
                            crate::constants::MARS,
                            crate::constants::ECL_NUT,
                            -999,
                        ] {
                            configure(path, configuration);
                            assert!(former_guard(jd, flags).is_ok(), "admitted native input");
                            let expected = native_position(jd, body, flags);
                            if jd == MODERN
                                && body == crate::constants::MOON
                                && flags
                                    == (crate::constants::FLG_SWIEPH | crate::constants::FLG_SPEED)
                            {
                                let source = if path_index == 0 {
                                    crate::constants::FLG_SWIEPH
                                } else {
                                    crate::constants::FLG_MOSEPH
                                };
                                assert!(
                                    expected.status & MASK == source,
                                    "actual file-backed and missing-data source controls"
                                );
                            }
                            let expected_tidal = tidal_bits();
                            let following = native_position(
                                jd + 0.25,
                                crate::constants::MOON,
                                crate::constants::FLG_MOSEPH | crate::constants::FLG_SPEED,
                            );
                            let following_tidal = tidal_bits();
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
                                crate::constants::MOON,
                                crate::constants::FLG_MOSEPH | crate::constants::FLG_SPEED,
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
        let first = crate::time::julday(i32::MIN, 1, 1, 0.0, Calendar::Gregorian)
            .max(crate::time::julday(i32::MIN, 1, 1, 0.0, Calendar::Julian));
        let end = crate::time::julday(i32::MAX, 12, 31, 24.0, Calendar::Gregorian).min(
            crate::time::julday(i32::MAX, 12, 31, 24.0, Calendar::Julian),
        );
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
                        tidal: crate::constants::SE_TIDAL_AUTOMATIC,
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
