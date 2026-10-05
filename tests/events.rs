//! Integration checks: longitude crossings  and global
//! eclipse searches/circumstances.
//!
//! Native configuration is process-global, so every state-dependent step
//! runs inside the sequenced `stateful_*` tests. Standalone tests use
//! pure-Rust input validation only, which reaches no native code.
//! Assertions are structural (ranges, round trips, ordering, error
//! classification, same-engine consistency), never stored reference
//! vectors.

use swisseph_bindings::{
    ECL_1ST_VISIBLE, ECL_2ND_VISIBLE, ECL_3RD_VISIBLE, ECL_4TH_VISIBLE, ECL_ALLTYPES_LUNAR,
    ECL_ALLTYPES_SOLAR, ECL_ANNULAR, ECL_ANNULAR_TOTAL, ECL_CENTRAL, ECL_HYBRID, ECL_MAX_VISIBLE,
    ECL_NONCENTRAL, ECL_OCC_BEG_DAYLIGHT, ECL_OCC_END_DAYLIGHT, ECL_ONE_TRY, ECL_PARTBEG_VISIBLE,
    ECL_PARTEND_VISIBLE, ECL_PARTIAL, ECL_PENUMBBEG_VISIBLE, ECL_PENUMBEND_VISIBLE, ECL_PENUMBRAL,
    ECL_TOTAL, ECL_TOTBEG_VISIBLE, ECL_TOTEND_VISIBLE, ECL_VISIBLE, ErrorKind, FLG_HELCTR,
    FLG_MOSEPH, FLG_SWIEPH, MARS, MOON, SUN, calc, helio_cross, helio_cross_ut, lun_eclipse_how,
    lun_eclipse_when, mooncross, mooncross_node, mooncross_node_ut, mooncross_ut, set_ephe_path,
    sol_eclipse_how, sol_eclipse_when_glob, sol_eclipse_where, solcross, solcross_ut,
};

#[path = "data_dir.rs"]
mod test_data;

/// J2000.0 as a Julian Day label.
const J2000: f64 = 2451545.0;
/// 2024-01-01 00:00 UT, search epoch for the eclipse tests.
const Y2024: f64 = 2460310.5;

#[test]
fn eclipse_constants_match_pinned_header() {
    assert_eq!(ECL_CENTRAL, 1);
    assert_eq!(ECL_NONCENTRAL, 2);
    assert_eq!(ECL_TOTAL, 4);
    assert_eq!(ECL_ANNULAR, 8);
    assert_eq!(ECL_PARTIAL, 16);
    assert_eq!(ECL_ANNULAR_TOTAL, 32);
    assert_eq!(ECL_HYBRID, 32);
    assert_eq!(ECL_PENUMBRAL, 64);
    assert_eq!(
        ECL_ALLTYPES_SOLAR,
        ECL_CENTRAL | ECL_NONCENTRAL | ECL_TOTAL | ECL_ANNULAR | ECL_PARTIAL | ECL_ANNULAR_TOTAL
    );
    assert_eq!(ECL_ALLTYPES_LUNAR, ECL_TOTAL | ECL_PARTIAL | ECL_PENUMBRAL);
    assert_eq!(ECL_VISIBLE, 128);
    assert_eq!(ECL_MAX_VISIBLE, 256);
    assert_eq!(ECL_1ST_VISIBLE, 512);
    assert_eq!(ECL_PARTBEG_VISIBLE, 512);
    assert_eq!(ECL_2ND_VISIBLE, 1024);
    assert_eq!(ECL_TOTBEG_VISIBLE, 1024);
    assert_eq!(ECL_3RD_VISIBLE, 2048);
    assert_eq!(ECL_TOTEND_VISIBLE, 2048);
    assert_eq!(ECL_4TH_VISIBLE, 4096);
    assert_eq!(ECL_PARTEND_VISIBLE, 4096);
    assert_eq!(ECL_PENUMBBEG_VISIBLE, 8192);
    assert_eq!(ECL_PENUMBEND_VISIBLE, 16384);
    assert_eq!(ECL_OCC_BEG_DAYLIGHT, 8192);
    assert_eq!(ECL_OCC_END_DAYLIGHT, 16384);
    assert_eq!(ECL_ONE_TRY, 32 * 1024);
}

#[test]
fn crossings_reject_non_finite_input() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for result in [
            solcross(bad, J2000, FLG_MOSEPH).map(|_| ()),
            solcross(0.0, bad, FLG_MOSEPH).map(|_| ()),
            solcross_ut(0.0, bad, FLG_MOSEPH).map(|_| ()),
            mooncross(bad, J2000, FLG_MOSEPH).map(|_| ()),
            mooncross_ut(0.0, bad, FLG_MOSEPH).map(|_| ()),
            mooncross_node(bad, FLG_MOSEPH).map(|_| ()),
            mooncross_node_ut(bad, FLG_MOSEPH).map(|_| ()),
            helio_cross(MARS, bad, J2000, FLG_MOSEPH, 1).map(|_| ()),
            helio_cross(MARS, 0.0, bad, FLG_MOSEPH, 1).map(|_| ()),
            helio_cross_ut(MARS, 0.0, bad, FLG_MOSEPH, -1).map(|_| ()),
        ] {
            let err = result.expect_err("non-finite input must be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidInput);
        }
    }
}

#[test]
fn eclipse_calls_reject_non_finite_input() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for result in [
            sol_eclipse_when_glob(bad, FLG_MOSEPH, 0, false).map(|_| ()),
            lun_eclipse_when(bad, FLG_MOSEPH, 0, false).map(|_| ()),
            sol_eclipse_where(bad, FLG_MOSEPH).map(|_| ()),
            sol_eclipse_how(bad, FLG_MOSEPH, 0.0, 51.5, 0.0).map(|_| ()),
            sol_eclipse_how(Y2024, FLG_MOSEPH, bad, 51.5, 0.0).map(|_| ()),
            lun_eclipse_how(bad, FLG_MOSEPH, None).map(|_| ()),
            lun_eclipse_how(Y2024, FLG_MOSEPH, Some((bad, 51.5, 0.0))).map(|_| ()),
        ] {
            let err = result.expect_err("non-finite input must be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidInput);
        }
    }
}

#[test]
fn stateful_crossings() {
    // Moshier-only sequencing: no data files needed.

    // --- solcross: next Aries ingress after J2000 (2000-03-20) ---
    let cross = solcross(0.0, J2000, FLG_MOSEPH).expect("solcross succeeds");
    assert!(cross.time > J2000 && cross.time < J2000 + 100.0);
    // The Sun is at the target longitude at the reported instant.
    let sun = calc(cross.time, SUN, FLG_MOSEPH).expect("Sun position");
    let unsigned = (sun.values[0] % 360.0 + 360.0) % 360.0;
    let distance = unsigned.min(360.0 - unsigned);
    assert!(
        distance < 0.01,
        "Sun longitude at crossing: {}",
        sun.values[0]
    );
    // Bitwise repeat calls.
    assert_eq!(solcross(0.0, J2000, FLG_MOSEPH).expect("repeat"), cross);
    // Target normalization: 720° behaves as 0°.
    assert_eq!(
        solcross(720.0, J2000, FLG_MOSEPH).expect("normalized"),
        cross
    );

    // --- solcross_ut agrees with solcross up to Delta T ---
    let cross_ut = solcross_ut(0.0, J2000, FLG_MOSEPH).expect("solcross_ut succeeds");
    assert!((cross.time - cross_ut.time).abs() < 0.005);

    // --- mooncross: next 0° crossing within one sidereal month ---
    let moon = mooncross(0.0, J2000, FLG_MOSEPH).expect("mooncross succeeds");
    assert!(moon.time > J2000 && moon.time < J2000 + 30.0);
    let moon_pos = calc(moon.time, MOON, FLG_MOSEPH).expect("Moon position");
    let unsigned = (moon_pos.values[0] % 360.0 + 360.0) % 360.0;
    let distance = unsigned.min(360.0 - unsigned);
    assert!(
        distance < 0.01,
        "Moon longitude at crossing: {}",
        moon_pos.values[0]
    );
    let moon_ut = mooncross_ut(0.0, J2000, FLG_MOSEPH).expect("mooncross_ut succeeds");
    assert!((moon.time - moon_ut.time).abs() < 0.005);

    // --- mooncross_node: next node passage within half a nodal month ---
    let node = mooncross_node(J2000, FLG_MOSEPH).expect("node succeeds");
    assert!(node.time > J2000 && node.time < J2000 + 15.0);
    assert!(node.latitude.abs() < 1e-3);
    assert!((0.0..360.0).contains(&node.longitude));
    assert_eq!(mooncross_node(J2000, FLG_MOSEPH).expect("repeat"), node);
    let node_ut = mooncross_node_ut(J2000, FLG_MOSEPH).expect("node_ut succeeds");
    assert!((node.time - node_ut.time).abs() < 0.005);

    // --- helio_cross: Mars forward and backward ---
    let helio = helio_cross(MARS, 0.0, J2000, FLG_MOSEPH, 1).expect("helio forward succeeds");
    assert!(helio.time > J2000 && helio.time < J2000 + 700.0);
    let mars = calc(helio.time, MARS, FLG_MOSEPH | FLG_HELCTR).expect("Mars helio position");
    let unsigned = (mars.values[0] % 360.0 + 360.0) % 360.0;
    let distance = unsigned.min(360.0 - unsigned);
    assert!(
        distance < 0.01,
        "Mars helio longitude at crossing: {}",
        mars.values[0]
    );
    let back = helio_cross(MARS, 0.0, J2000, FLG_MOSEPH, -1).expect("helio backward succeeds");
    assert!(back.time < J2000);
    assert_eq!(
        helio_cross(MARS, 0.0, J2000, FLG_MOSEPH, 1).expect("repeat"),
        helio
    );
    let helio_ut = helio_cross_ut(MARS, 0.0, J2000, FLG_MOSEPH, 1).expect("helio_ut succeeds");
    assert!((helio.time - helio_ut.time).abs() < 0.005);

    // --- helio_cross rejects bodies without a heliocentric longitude ---
    for body in [SUN, MOON] {
        let err =
            helio_cross(body, 0.0, J2000, FLG_MOSEPH, 1).expect_err("Sun/Moon must be rejected");
        assert_eq!(err.kind(), ErrorKind::Native);
        assert!(
            err.message().contains("not possible"),
            "unexpected diagnostic: {}",
            err.message()
        );
    }
}

#[test]
fn stateful_global_eclipses() {
    // Moshier sequencing first (no data files needed).
    // --- next total solar eclipse after 2024-01-01: 2024-04-08 ---
    let sol = sol_eclipse_when_glob(Y2024, FLG_MOSEPH, ECL_TOTAL, false).expect("solar search");
    assert!(sol.maximum() > Y2024 && sol.maximum() < Y2024 + 120.0);
    assert_eq!(sol.eclipse_type & ECL_TOTAL, ECL_TOTAL);
    assert_eq!(sol.eclipse_type & ECL_CENTRAL, ECL_CENTRAL);
    assert!(sol.begin() <= sol.maximum() && sol.maximum() <= sol.end());
    assert!(sol.totality_begin() <= sol.maximum() && sol.maximum() <= sol.totality_end());
    assert!(sol.centerline_begin() <= sol.maximum() && sol.maximum() <= sol.centerline_end());
    assert!(sol.begin() < sol.totality_begin());
    assert!(sol.totality_end() < sol.end());
    // Hybrid-transition slots are reserved.
    assert_eq!(sol.times[8], 0.0);
    assert_eq!(sol.times[9], 0.0);
    // Bitwise repeat.
    assert_eq!(
        sol_eclipse_when_glob(Y2024, FLG_MOSEPH, ECL_TOTAL, false).expect("repeat"),
        sol
    );

    // --- previous solar eclipse before 2024-01-01: 2023-10-14 annular ---
    let prev = sol_eclipse_when_glob(Y2024, FLG_MOSEPH, 0, true).expect("backward search");
    assert!(prev.maximum() < Y2024 && prev.maximum() > Y2024 - 120.0);
    assert_eq!(prev.eclipse_type & ECL_ANNULAR, ECL_ANNULAR);

    // --- impossible type combination fails natively ---
    let err = sol_eclipse_when_glob(Y2024, FLG_MOSEPH, ECL_CENTRAL | ECL_PARTIAL, false)
        .expect_err("central partial must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(
        err.message().contains("central partial"),
        "unexpected diagnostic: {}",
        err.message()
    );

    // --- next total lunar eclipse after 2024-01-01: 2025-03-14 ---
    let lun = lun_eclipse_when(Y2024, FLG_MOSEPH, ECL_TOTAL, false).expect("lunar search");
    assert!(lun.maximum() > Y2024 && lun.maximum() < Y2024 + 500.0);
    assert_eq!(lun.eclipse_type & ECL_TOTAL, ECL_TOTAL);
    assert!(lun.partial_begin() <= lun.maximum() && lun.maximum() <= lun.partial_end());
    assert!(lun.total_begin() <= lun.maximum() && lun.maximum() <= lun.total_end());
    assert!(lun.penumbral_begin() <= lun.maximum() && lun.maximum() <= lun.penumbral_end());
    assert!(lun.penumbral_begin() < lun.partial_begin());
    assert!(lun.partial_end() < lun.penumbral_end());
    // Reserved slots stay zero.
    assert_eq!(lun.times[1], 0.0);
    assert_eq!(lun.times[8], 0.0);
    assert_eq!(lun.times[9], 0.0);
    assert_eq!(
        lun_eclipse_when(Y2024, FLG_MOSEPH, ECL_TOTAL, false).expect("repeat"),
        lun
    );

    // --- previous lunar eclipse before 2024-01-01: 2023-10-28 partial ---
    let lun_prev = lun_eclipse_when(Y2024, FLG_MOSEPH, 0, true).expect("lunar backward");
    assert!(lun_prev.maximum() < Y2024 && lun_prev.maximum() > Y2024 - 120.0);
    assert_eq!(lun_prev.eclipse_type & ECL_PARTIAL, ECL_PARTIAL);

    // --- annular-only lunar requests fail natively ---
    let err = lun_eclipse_when(Y2024, FLG_MOSEPH, ECL_ANNULAR, false)
        .expect_err("annular lunar must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(
        err.message().contains("annular"),
        "unexpected diagnostic: {}",
        err.message()
    );

    // --- sol_eclipse_where at the solar maximum: greatest eclipse ---
    let geo = sol_eclipse_where(sol.maximum(), FLG_MOSEPH).expect("where succeeds");
    assert_ne!(geo.eclipse_type, 0);
    assert_eq!(geo.eclipse_type & ECL_TOTAL, ECL_TOTAL);
    assert!((-180.0..180.0).contains(&geo.longitude));
    assert!((-90.0..90.0).contains(&geo.latitude));
    assert!(geo.attributes[0] > 1.0, "magnitude {}", geo.attributes[0]);
    assert!((geo.attributes[1] - 1.0).abs() < 0.1);
    assert!(geo.attributes[2] > 1.0, "obscuration {}", geo.attributes[2]);
    assert!(geo.attributes[3] != 0.0, "core width {}", geo.attributes[3]);
    assert!(geo.attributes[9] > 0.0, "saros {}", geo.attributes[9]);
    // Reserved tail stays zero.
    assert!(geo.attributes[11..20].iter().all(|&v| v == 0.0));

    // --- sol_eclipse_where with no eclipse in progress: dedicated Ok state ---
    let none = sol_eclipse_where(Y2024, FLG_MOSEPH).expect("quiet instant");
    assert_eq!(none.eclipse_type, 0);
    assert!(
        none.diagnostic.contains("no solar eclipse"),
        "unexpected diagnostic: {}",
        none.diagnostic
    );

    // --- sol_eclipse_how at the greatest-eclipse point: local totality ---
    let loc = sol_eclipse_how(sol.maximum(), FLG_MOSEPH, geo.longitude, geo.latitude, 0.0)
        .expect("how succeeds");
    assert_eq!(loc.eclipse_type & ECL_TOTAL, ECL_TOTAL);
    assert!(loc.magnitude() > 1.0);
    assert!(loc.obscuration() > 1.0);
    assert_eq!(loc.attributes, geo.attributes);

    // --- sol_eclipse_how with no eclipse: Ok with type 0 ---
    let quiet = sol_eclipse_how(Y2024, FLG_MOSEPH, 0.0, 51.5, 0.0).expect("quiet how");
    assert_eq!(quiet.eclipse_type, 0);

    // --- sol_eclipse_how rejects out-of-range altitudes natively ---
    let err = sol_eclipse_how(sol.maximum(), FLG_MOSEPH, 0.0, 51.5, 30_000.0)
        .expect_err("high altitude must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(
        err.message().contains("between -500 and 25000"),
        "unexpected diagnostic: {}",
        err.message()
    );

    // --- lun_eclipse_how at the lunar maximum (geocentric) ---
    let lhow = lun_eclipse_how(lun.maximum(), FLG_MOSEPH, None).expect("lunar how");
    assert_eq!(lhow.eclipse_type & ECL_TOTAL, ECL_TOTAL);
    assert!(lhow.umbral_magnitude() > 1.0);
    assert!(lhow.penumbral_magnitude() > 1.0);
    assert_eq!(lhow.attributes[8], lhow.attributes[0]);
    assert!(lhow.attributes[9] > 0.0, "saros {}", lhow.attributes[9]);
    // Unused slots stay zero; geocentric azimuth/altitude stay zero.
    assert_eq!(lhow.attributes[2], 0.0);
    assert_eq!(lhow.attributes[3], 0.0);
    assert_eq!(lhow.attributes[4], 0.0);
    assert_eq!(lhow.attributes[5], 0.0);
    assert_eq!(lhow.attributes[6], 0.0);
    assert!(lhow.attributes[11..20].iter().all(|&v| v == 0.0));

    // --- lun_eclipse_how with an observer: runs, azimuth/altitude filled ---
    let lobs = lun_eclipse_how(lun.maximum(), FLG_MOSEPH, Some((12.5, 41.9, 0.0)))
        .expect("observer lunar how");
    assert!((lobs.attributes[0] - lhow.attributes[0]).abs() < 1e-9);
    assert!(lobs.attributes[4] != 0.0 || lobs.attributes[5] != 0.0);

    // --- lun_eclipse_how with no eclipse: Ok with type 0 ---
    let lquiet = lun_eclipse_how(Y2024, FLG_MOSEPH, None).expect("quiet lunar how");
    assert_eq!(lquiet.eclipse_type, 0);

    // --- file-based SWIEPH agrees with Moshier on the solar maximum ---
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP file-based assertions: no ephemeris data found; set SWISSEPH_EPHE_DIR \
             to a directory containing the .se1 files"
        );
        return;
    };
    set_ephe_path(Some(&data_dir)).expect("data path");
    let sol_file = sol_eclipse_when_glob(Y2024, FLG_SWIEPH, ECL_TOTAL, false).expect("file search");
    assert!((sol_file.maximum() - sol.maximum()).abs() < 0.01);
    assert_eq!(sol_file.eclipse_type, sol.eclipse_type);
}
