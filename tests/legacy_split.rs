//! Versioned split output contracts and common native input admission.
use swisseph_bindings::{ErrorKind, native::split_deg, native::split_deg_2_10_03};

#[test]
fn unrounded_contracts_agree_and_rounding_retains_legacy_lower_fields() {
    for degree in [
        -450.0, -30.5, -0.0, 0.0, 12.345678, 29.99999, 45.5, 359.99999, 450.0,
    ] {
        for flags in [0, 8, 16, 32, 1024] {
            assert_eq!(
                split_deg_2_10_03(degree, flags).unwrap(),
                split_deg(degree, flags).unwrap()
            );
        }
        for flags in [1, 2, 4, 12, 28, 1025, -1] {
            let legacy = split_deg_2_10_03(degree, flags).unwrap();
            let current = split_deg(degree, flags).unwrap();
            assert_eq!(legacy.deg, current.deg);
            assert_eq!(legacy.sign, current.sign);
            assert_eq!(legacy.secfr, f64::from(legacy.sec));
            assert_eq!(current.secfr, 0.0);
        }
    }
}

#[test]
fn versioned_utility_keeps_finite_capacity_errors() {
    for degree in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        2147483648.0,
        -2147483648.0,
    ] {
        assert_eq!(
            split_deg_2_10_03(degree, 0).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
        assert_eq!(
            split_deg(degree, 0).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
    }
}
