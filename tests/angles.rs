//! checks: angle normalization, signed distances, wrap-aware
//! midpoints, float-to-int conversion and equatorial/ecliptic rotation.
//!
//! These are pure computations over their arguments (no ephemeris files,
//! no process-global configuration), so no sequencing or restore steps
//! are needed. Assertions pin the probed native contracts (ranges,
//! direction convention, rounding, buffer widths) without storing
//! reference vectors beyond exact small integers.

use std::f64::consts::PI;
use swisseph_bindings::{
    ErrorKind, cotrans, cotrans_sp, d2l, deg_midp, degnorm, difdeg2n, difdegn, difrad2n, rad_midp,
    radnorm,
};

#[test]
fn normalization_ranges() {
    assert_eq!(degnorm(-10.0), 350.0);
    assert_eq!(degnorm(360.0), 0.0);
    assert_eq!(degnorm(720.5), 0.5);
    assert!((radnorm(7.0) - (7.0 - 2.0 * PI)).abs() < 1e-15);
    assert_eq!(radnorm(2.0 * PI), 0.0);
}

#[test]
fn signed_distance_ranges() {
    // [0, 360) for `difdegn`, ±180 for `difdeg2n`: the pair below
    // separates the two contracts.
    assert_eq!(difdegn(10.0, 350.0), 20.0);
    assert_eq!(difdegn(350.0, 10.0), 340.0);
    assert_eq!(difdeg2n(10.0, 350.0), 20.0);
    assert_eq!(difdeg2n(350.0, 10.0), -20.0);
    // Radian folding to ±π, including the exact half-turn magnitude.
    assert!((difrad2n(0.1, 6.2) - (0.1 - 6.2 + 2.0 * PI)).abs() < 1e-15);
    assert!((difrad2n(PI, 0.0).abs() - PI).abs() < 1e-15);
}

#[test]
fn midpoints_cross_the_seam() {
    assert_eq!(deg_midp(350.0, 10.0), 0.0);
    assert_eq!(deg_midp(10.0, 350.0), 0.0);
    assert_eq!(deg_midp(10.0, 20.0), 15.0);
    // Short way across the 2π seam, from the wrap contract itself.
    let want = (0.1 + 6.2 - 2.0 * PI) / 2.0;
    assert!((rad_midp(0.1, 6.2) - want).abs() < 1e-15);
}

#[test]
fn d2l_rounds_half_away_and_validates() {
    assert_eq!(d2l(2.5).expect("2.5"), 3);
    assert_eq!(d2l(-2.5).expect("-2.5"), -3);
    assert_eq!(d2l(0.5).expect("0.5"), 1);
    assert_eq!(d2l(-0.5).expect("-0.5"), -1);
    assert_eq!(d2l(3.0).expect("3.0"), 3);
    assert_eq!(d2l(2_147_483_647.0).expect("i32 max"), 2_147_483_647);
    assert_eq!(d2l(-2_147_483_647.0).expect("near i32 min"), -2_147_483_647);
    assert_eq!(
        d2l(2_147_483_647.5_f64.next_down()).expect("last safe positive"),
        i32::MAX
    );
    assert_eq!(
        d2l(-2_147_483_647.5_f64.next_down()).expect("last safe negative"),
        -i32::MAX
    );
    for bad in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        2_147_483_647.5,
        -2_147_483_647.5,
        -2_147_483_647.75,
        -2_147_483_648.0,
        -2_147_483_648.5,
        1e30,
    ] {
        let err = d2l(bad).expect_err("out-of-range must be rejected");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
}

#[test]
fn cotrans_direction_and_round_trip() {
    // North celestial pole with a positive obliquity: equatorial input
    // reads as (RA 0, Dec 90) and lands on ecliptic longitude 90 with
    // the polar distance preserved.
    let ecl = cotrans(0.0, 90.0, 1.0, 23.4392911).expect("forward");
    assert!((ecl[0] - 90.0).abs() < 1e-9, "lon, got {}", ecl[0]);
    assert!((ecl[1] - 66.5607089).abs() < 1e-7, "lat, got {}", ecl[1]);
    assert_eq!(ecl[2], 1.0);
    // Negative obliquity converts back: the ecliptic north pole lands
    // on RA 270 with the same polar distance.
    let equ = cotrans(0.0, 90.0, 1.0, -23.4392911).expect("backward");
    assert!((equ[0] - 270.0).abs() < 1e-9, "RA, got {}", equ[0]);
    assert!((equ[1] - 66.5607089).abs() < 1e-7, "Dec, got {}", equ[1]);
    // Forward-then-backward restores an arbitrary triple to rounding
    // scale with distance untouched.
    let there = cotrans(33.0, -12.5, 2.0, 23.4392911).expect("there");
    let back = cotrans(there[0], there[1], there[2], -23.4392911).expect("back");
    assert!((back[0] - 33.0).abs() < 1e-12, "lon, got {}", back[0]);
    assert!((back[1] + 12.5).abs() < 1e-12, "lat, got {}", back[1]);
    assert_eq!(back[2], 2.0);
}

#[test]
fn cotrans_sp_propagates_speeds() {
    let (pos, speed) =
        cotrans_sp(10.0, 20.0, 1.5, 0.5, -0.25, 0.01, 23.4392911).expect("with speeds");
    // Distance and distance rate pass through the rotation unchanged.
    assert_eq!(pos[2], 1.5);
    assert_eq!(speed[2], 0.01);
    for (index, rate) in speed.iter().enumerate().take(2) {
        assert!(rate.is_finite(), "rate {index} must be finite");
        assert!(rate.abs() < 2.0, "rate {index} off scale: {rate}");
    }
    // The speed-less entry point agrees with the position triple.
    let plain = cotrans(10.0, 20.0, 1.5, 23.4392911).expect("plain");
    assert_eq!(plain, pos, "position triples must agree bitwise");
    // Forward-then-backward restores positions and speeds to rounding
    // scale, proving all six output slots carry the rotation.
    let (back_pos, back_speed) = cotrans_sp(
        pos[0],
        pos[1],
        pos[2],
        speed[0],
        speed[1],
        speed[2],
        -23.4392911,
    )
    .expect("back");
    for (index, (got, want)) in back_pos.iter().zip([10.0, 20.0, 1.5]).enumerate() {
        assert!(
            (got - want).abs() < 1e-12,
            "pos {index} round trip, got {got}"
        );
    }
    for (index, (got, want)) in back_speed.iter().zip([0.5, -0.25, 0.01]).enumerate() {
        assert!(
            (got - want).abs() < 1e-12,
            "speed {index} round trip, got {got}"
        );
    }
}
