//! Focused coverage for the deferred / / rows:
//! `SIDM_USER` (t0/ayan_t0) parameters, the `FLG_SIDEREAL` combined
//! matrix over positions and houses, and the house-system-by-latitude
//! matrix.
//!
//! Sidereal-mode selection is process-global, so every step that depends
//! on the mode runs inside the single sequenced `stateful_sidereal`
//! test, which restores the default mode at the end. The latitude matrix
//! uses tropical houses only and needs no shared configuration.
//! Assertions are structural (bands, difference proofs, error
//! classification), never stored reference vectors.

use swisseph_bindings::{
    ErrorKind, FLG_MOSEPH, FLG_SIDEREAL, FLG_SPEED, SIDM_FAGAN_BRADLEY, SIDM_LAHIRI, SIDM_USER,
    SUN, calc_ut, get_ayanamsa_ex_ut, get_ayanamsa_ut, houses, houses_ex, set_sid_mode,
};

/// J2000.0 in Universal Time.
const J2000_UT: f64 = 2451545.0;

/// Normalize a degree difference into [0, 360).
fn norm360(mut x: f64) -> f64 {
    x %= 360.0;
    if x < 0.0 {
        x += 360.0;
    }
    x
}

/// Sidereal-mode parameters plus the sidereal-flag matrix, sequenced in
/// one test because the mode is process-global.
#[test]
fn stateful_sidereal_user_mode_and_flag_matrix() {
    // Reference point: Lahiri at J2000 sits in the known 23-25 band.
    set_sid_mode(SIDM_LAHIRI, 0.0, 0.0).expect("lahiri mode");
    let lahiri = get_ayanamsa_ut(J2000_UT).expect("lahiri ayanamsa");
    assert!((23.0..25.0).contains(&lahiri), "lahiri band, got {lahiri}");

    // User-defined mode: 24 degrees exactly at J2000.
    set_sid_mode(SIDM_USER, J2000_UT, 24.0).expect("user mode");
    let user = get_ayanamsa_ut(J2000_UT).expect("user ayanamsa");
    assert!((23.5..24.5).contains(&user), "user offset band, got {user}");
    assert!(
        (user - lahiri).abs() > 0.1,
        "user mode must differ from lahiri, got {user} vs {lahiri}"
    );
    // Repeat calls are exactly reproducible under the lock.
    let again = get_ayanamsa_ut(J2000_UT).expect("user ayanamsa repeat");
    assert_eq!(user, again, "ayanamsa repeat must be bitwise");

    // sidereal matrix over positions (Moshier needs no data):
    // tropical minus sidereal longitude is the flag-scoped `_ex`
    // ayanamsha. Established by probing: the calculation paths apply
    // the `_ex` correction chain (bitwise equal to `get_ayanamsa_ex_ut`
    // with matching flags), while the plain `get_ayanamsa_ut` omits it
    // and agrees only to the documented sub-0.01 band — the same
    // mechanism as the house nutation-scale behavior, confirmed here for
    // the USER mode as well. This is native behavior, not a binding
    // conversion error.
    let scoped = get_ayanamsa_ex_ut(J2000_UT, FLG_MOSEPH)
        .expect("scoped ayanamsa")
        .value;
    assert!(
        (scoped - user).abs() < 0.01,
        "plain/ex band, got {scoped} vs {user}"
    );
    let tropical = calc_ut(J2000_UT, SUN, FLG_MOSEPH | FLG_SPEED).expect("tropical sun");
    let sidereal =
        calc_ut(J2000_UT, SUN, FLG_MOSEPH | FLG_SPEED | FLG_SIDEREAL).expect("sidereal sun");
    let shift = norm360(tropical.longitude() - sidereal.longitude());
    assert!(
        (shift - scoped).abs() < 1e-12,
        "sidereal shift must be the scoped ayanamsha, got {shift} vs {scoped}"
    );
    // The sidereal frame rotation is applied to the whole vector, so
    // latitude/distance agree to rounding scale rather than bitwise
    // (probed: ~6e-15 absolute on this input).
    assert!(
        (tropical.latitude() - sidereal.latitude()).abs() < 1e-12,
        "latitude rounding-scale agreement"
    );
    assert!(
        (tropical.distance() - sidereal.distance()).abs() < 1e-12,
        "distance rounding-scale agreement"
    );

    // Same matrix over houses: sidereal cusps shift by the scoped
    // ayanamsha (same `_ex` path as the position calls, per probing).
    let houses_tropical = houses(J2000_UT, 51.5, -0.12, b'P').expect("tropical houses");
    let houses_sidereal =
        houses_ex(J2000_UT, FLG_SIDEREAL, 51.5, -0.12, b'P').expect("sidereal houses");
    for (index, (plain, sid)) in houses_tropical
        .cusps
        .iter()
        .zip(houses_sidereal.cusps.iter())
        .enumerate()
    {
        let cusp_shift = norm360(plain - sid);
        assert!(
            (cusp_shift - scoped).abs() < 1e-9,
            "cusp {index} sidereal shift must be the scoped ayanamsha, got {cusp_shift} vs {scoped}"
        );
    }
    // Repeat sidereal houses are exactly reproducible.
    let repeat = houses_ex(J2000_UT, FLG_SIDEREAL, 51.5, -0.12, b'P').expect("sidereal repeat");
    assert_eq!(houses_sidereal.cusps, repeat.cusps, "cusps repeat bitwise");
    assert_eq!(
        houses_sidereal.angles, repeat.angles,
        "angles repeat bitwise"
    );

    // A different user epoch/offset takes effect as well.
    set_sid_mode(SIDM_USER, J2000_UT, 25.0).expect("second user mode");
    let user2 = get_ayanamsa_ut(J2000_UT).expect("second user ayanamsa");
    assert!(
        (user2 - user - 1.0).abs() < 0.5,
        "offset change must take effect, got {user2} vs {user}"
    );

    set_sid_mode(SIDM_FAGAN_BRADLEY, 0.0, 0.0).expect("restore default mode");
}

/// every system in the matrix either succeeds with finite
/// output or fails with a classified native error — never a panic and
/// never a Rust-side rejection (all letters pass the wrapper).
#[test]
fn house_systems_across_latitudes() {
    const SYSTEMS: [u8; 6] = *b"PKORCE";
    // Equator, mid latitudes: every system is defined.
    for latitude in [0.0, 30.0, 51.5] {
        for system in SYSTEMS {
            let result = houses(J2000_UT, latitude, 0.0, system);
            let named = system as char;
            let layout =
                result.unwrap_or_else(|err| panic!("{named} must succeed at {latitude}: {err}"));
            assert!(
                layout
                    .cusps
                    .iter()
                    .all(|c| c.is_finite() && (0.0..360.0).contains(c)),
                "{named} cusps must be finite angles at {latitude}"
            );
            assert!(
                layout.angles.iter().all(|a| a.is_finite()),
                "{named} angles must be finite at {latitude}"
            );
        }
    }
    // Near the pole the matrix classifies per system: Placidus has no
    // defined cusps (native failure), Porphyry stays defined. Any other
    // outcome must still be a classified native error, never a panic.
    let mut polar_failures = 0;
    for system in SYSTEMS {
        match houses(J2000_UT, 89.9, 0.0, system) {
            Ok(layout) => {
                assert!(
                    layout.cusps.iter().all(|c| c.is_finite()),
                    "{} polar cusps must be finite",
                    system as char
                );
            }
            Err(err) => {
                assert_eq!(
                    err.kind(),
                    ErrorKind::Native,
                    "{} polar failure must be native",
                    system as char
                );
                polar_failures += 1;
            }
        }
    }
    assert!(polar_failures >= 1, "at least Placidus must fail at 89.9");
    let err = houses(J2000_UT, 89.9, 0.0, b'P').expect_err("polar Placidus must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    houses(J2000_UT, 89.9, 0.0, b'O').expect("polar Porphyry stays defined");
}
