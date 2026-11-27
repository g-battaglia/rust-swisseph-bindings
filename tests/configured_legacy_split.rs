//! Check explicit mode transport against the legacy C routine and ambient mode.
use swisseph_bindings::native::{
    ErrorKind, SIDM_FAGAN_BRADLEY, SIDM_TRUE_SHEORAN, SIDM_USER, SPLIT_DEG_NAKSHATRA, set_sid_mode,
    split_deg_2_10_03, split_deg_2_10_03_with_sidereal_mode,
};

#[test]
fn requested_mode_matches_the_native_setter_without_changing_ambient_state() {
    let degrees = [-30.5, -0.0, 0.0, 0.1, 13.33333333333333, 29.99999, 360.0];
    let flags = [0, 8, 1024, 1025, 1056, -1];

    // This test is the only ambient-mode writer in its executable.
    for mode in (-1..=48).chain([SIDM_USER, 256 + SIDM_TRUE_SHEORAN, 256 + 47]) {
        set_sid_mode(mode, 2451545.0, 23.0).unwrap();
        for degree in degrees {
            for flag in flags {
                let expected = split_deg_2_10_03(degree, flag).unwrap();
                let actual = split_deg_2_10_03_with_sidereal_mode(degree, flag, mode).unwrap();
                assert_eq!(actual, expected, "{mode}, {degree}, {flag}");
                assert_eq!(actual.secfr.to_bits(), expected.secfr.to_bits());
            }
        }
    }

    let flags = SPLIT_DEG_NAKSHATRA;
    set_sid_mode(SIDM_TRUE_SHEORAN, 0.0, 0.0).unwrap();
    let sheoran = split_deg_2_10_03(0.0, flags).unwrap();
    set_sid_mode(SIDM_FAGAN_BRADLEY, 0.0, 0.0).unwrap();
    let fagan = split_deg_2_10_03(0.0, flags).unwrap();
    assert_ne!(sheoran, fagan);
    std::thread::scope(|scope| {
        for (mode, expected) in [(SIDM_TRUE_SHEORAN, sheoran), (SIDM_FAGAN_BRADLEY, fagan)] {
            scope.spawn(move || {
                for _ in 0..64 {
                    let actual = split_deg_2_10_03_with_sidereal_mode(0.0, flags, mode).unwrap();
                    assert_eq!(actual, expected);
                    assert_eq!(actual.secfr.to_bits(), expected.secfr.to_bits());
                }
            });
        }
    });
    assert_eq!(split_deg_2_10_03(0.0, flags).unwrap(), fagan);
    assert_eq!(
        split_deg_2_10_03_with_sidereal_mode(f64::NAN, flags, SIDM_TRUE_SHEORAN)
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(split_deg_2_10_03(0.0, flags).unwrap(), fagan);
}
