//! Regression inputs: arithmetic ranges, configured dates and missing catalogs.
//! Only inputs and structural assertions are retained.

use swisseph_bindings::*;
#[path = "support/isolated.rs"]
mod isolated;

fn invalid<T: std::fmt::Debug>(result: Result<T, Error>) {
    assert_eq!(
        result.expect_err("unsafe native domain").kind(),
        ErrorKind::InvalidInput
    );
}

#[test]
fn rounding_and_timezone_intermediates() {
    for x in [-2147483648.0, -2147483647.75, -2147483647.5, 2147483647.5] {
        invalid(d2l(x));
    }
    for cs in [
        i32::MIN,
        i32::MAX,
        -i32::MAX,
        i32::MAX - 49,
        -(i32::MAX - 49),
    ] {
        invalid(cs2lonlatstr(cs, b'E', b'W'));
        invalid(cs2timestr(cs, b':', false));
    }
    for cs in [i32::MAX - 50, -(i32::MAX - 50)] {
        assert!(
            !cs2lonlatstr(cs, b'E', b'W')
                .expect("rounding boundary")
                .is_empty()
        );
        assert!(
            !cs2timestr(cs, b':', false)
                .expect("rounding boundary")
                .is_empty()
        );
    }
    for huge in [-1e100, 1e100, f64::MAX] {
        invalid(utc_time_zone(2000, 1, 1, 12, 0, 0.0, huge));
        invalid(utc_time_zone(2000, 1, 1, 12, 0, huge, 0.0));
    }
    invalid(utc_time_zone(i32::MIN, 1, 1, 12, 0, 0.0, 24.0));
    invalid(utc_time_zone(i32::MAX, 12, 31, 12, 0, 0.0, -24.0));
    assert_eq!(
        utc_time_zone(i32::MIN, 1, 1, 12, 0, 0.0, -24.0)
            .expect("lower year shifted inwards")
            .year,
        i32::MIN
    );
    assert_eq!(
        utc_time_zone(i32::MAX, 12, 31, 12, 0, 0.0, 24.0)
            .expect("upper year shifted inwards")
            .year,
        i32::MAX
    );
}

#[test]
fn calculation_and_search_domains_include_configured_shifts() {
    let jd = 2451545.0;
    set_delta_t_userdef(Some(0.0)).expect("pin state");
    for bad in [f64::NAN, f64::INFINITY, -1e100, 1e100] {
        for flags in [FLG_SWIEPH, FLG_MOSEPH] {
            invalid(calc(bad, MOON, flags));
            invalid(calc_ut(bad, MOON, flags));
            invalid(pheno(bad, MARS, flags));
            invalid(nod_aps(bad, MARS, flags, NODBIT_OSCU));
            invalid(get_orbital_elements(bad, MARS, flags));
        }
        invalid(houses(bad, 51.5, 0.0, b'P'));
        invalid(houses_gauquelin(bad, 0, 51.5, 0.0));
        invalid(sol_eclipse_when_glob(bad, FLG_MOSEPH, 0, false));
        invalid(lun_eclipse_when(bad, FLG_MOSEPH, 0, false));
        invalid(sol_eclipse_when_loc(bad, FLG_MOSEPH, 0.0, 51.5, 0.0, false));
    }
    invalid(houses_gauquelin(jd, 0, 51.5, f64::NAN));
    for bad in [1e20, -1e20] {
        invalid(solcross(0.0, bad, FLG_MOSEPH));
        invalid(mooncross_node(bad, FLG_MOSEPH));
    }
    // Native counter endpoints return classified native range errors;
    // immediately excluded whole days must never reach their casts.
    for edge in [-63_412_861_279.0, 63_417_764_339.0] {
        for backward in [false, true] {
            assert_eq!(
                sol_eclipse_when_glob(edge, FLG_MOSEPH, 0, backward)
                    .expect_err("outside data coverage")
                    .kind(),
                ErrorKind::Native
            );
            assert_eq!(
                lun_eclipse_when(edge, FLG_MOSEPH, 0, backward)
                    .expect_err("outside data coverage")
                    .kind(),
                ErrorKind::Native
            );
        }
    }
    for excluded in [-63_412_861_280.0, 63_417_764_340.0] {
        invalid(sol_eclipse_when_glob(excluded, FLG_MOSEPH, 0, false));
        invalid(lun_eclipse_when(excluded, FLG_MOSEPH, 0, true));
        invalid(sol_eclipse_when_loc(
            excluded, FLG_MOSEPH, 0.0, 51.5, 0.0, false,
        ));
        invalid(lun_eclipse_when_loc(
            excluded, FLG_MOSEPH, 0.0, 51.5, 0.0, true,
        ));
    }
    set_delta_t_userdef(Some(1e100)).expect("finite override remains supported");
    invalid(get_ayanamsa_ut(jd));
    invalid(get_ayanamsa_ex_ut(jd, FLG_MOSEPH));
    invalid(calc_ut(jd, MOON, FLG_SWIEPH));
    invalid(pheno_ut(jd, MARS, FLG_SWIEPH));
    invalid(nod_aps_ut(jd, MARS, FLG_SWIEPH, NODBIT_OSCU));
    invalid(houses(jd, 51.5, 0.0, b'P'));
    invalid(houses_ex(jd, 0, 51.5, 0.0, b'P'));
    invalid(houses_ex2(jd, 0, 51.5, 0.0, b'P'));
    invalid(houses_gauquelin(jd, 0, 51.5, 0.0));
    let mut builder = SessionBuilder::new();
    builder.delta_t_override(1e100);
    let session = builder.build().expect("finite configured override");
    invalid(session.calc_ut(jd, MOON, FLG_SWIEPH));
    invalid(session.calc(1e100, MOON, FLG_SWIEPH));
    invalid(session.houses(jd, 51.5, 0.0, b'P'));
    invalid(session.houses_ex(jd, 0, 51.5, 0.0, b'P'));
    invalid(session.get_ayanamsa_ut(jd));
    set_delta_t_userdef(None).expect("restore state");
    set_sid_mode(SIDM_USER, jd, 15.0).expect("valid reference epoch");
    let previous = get_ayanamsa(jd).expect("configured baseline");
    for bad in [-1e100, 1e100] {
        invalid(set_sid_mode(SIDM_USER, bad, 0.0));
        invalid(get_ayanamsa(bad));
        invalid(get_ayanamsa_ex(bad, FLG_MOSEPH));
        assert_eq!(
            get_ayanamsa(jd).expect("setter rejection preserves state"),
            previous
        );
        assert!(
            SessionBuilder::new()
                .sid_mode(SIDM_USER, bad, 0.0)
                .build()
                .is_err()
        );
    }
    set_sid_mode(SIDM_FAGAN_BRADLEY, 0.0, 0.0).expect("restore sidereal state");
    set_topo(0.0, 51.5, 0.0).expect("valid observer height");
    let observer_baseline =
        calc_ut(jd, MOON, FLG_MOSEPH | FLG_TOPOCTR).expect("valid observer baseline");
    for bad in [-1e100, 1e100, f64::MAX] {
        invalid(set_topo(0.0, 51.5, bad));
        assert!(SessionBuilder::new().topo(0.0, 51.5, bad).build().is_err());
        invalid(rise_trans(
            jd, SUN, None, FLG_MOSEPH, CALC_RISE, 0.0, 51.5, bad, 0.0, 15.0,
        ));
        invalid(rise_trans_true_hor(
            jd, SUN, None, FLG_MOSEPH, CALC_RISE, 0.0, 51.5, bad, 0.0, 15.0, 0.0,
        ));
        invalid(gauquelin_sector(
            jd, SUN, None, FLG_MOSEPH, 2, 0.0, 51.5, bad, 0.0, 15.0,
        ));
        invalid(sol_eclipse_when_loc(jd, FLG_MOSEPH, 0.0, 51.5, bad, false));
        invalid(lun_eclipse_when_loc(jd, FLG_MOSEPH, 0.0, 51.5, bad, false));
        invalid(sol_eclipse_how(jd, FLG_MOSEPH, 0.0, 51.5, bad));
        invalid(lun_eclipse_how(jd, FLG_MOSEPH, Some((0.0, 51.5, bad))));
        invalid(lun_occult_when_loc(
            jd, VENUS, None, FLG_MOSEPH, 0.0, 51.5, bad, false,
        ));
        invalid(heliacal_ut(
            jd,
            0.0,
            51.5,
            bad,
            [0.0; 4],
            [0.0; 6],
            "Venus",
            HELIACAL_RISING,
            FLG_MOSEPH,
        ));
        invalid(heliacal_pheno_ut(
            jd,
            0.0,
            51.5,
            bad,
            [0.0; 4],
            [0.0; 6],
            "Venus",
            HELIACAL_RISING,
            FLG_MOSEPH,
        ));
        invalid(vis_limit_mag(
            jd, 0.0, 51.5, bad, [0.0; 4], [0.0; 6], "Venus", FLG_MOSEPH,
        ));
        assert_eq!(
            calc_ut(jd, MOON, FLG_MOSEPH | FLG_TOPOCTR)
                .expect("setter rejection preserves observer"),
            observer_baseline
        );
    }
    let ceiling = f64::from(i32::MAX) * AUNIT_TO_KM * 1000.0;
    let excluded = f64::from_bits(ceiling.to_bits() + 1);
    invalid(set_topo(0.0, 51.5, excluded));
    invalid(set_topo(0.0, 51.5, -excluded));
    for height in [ceiling, -ceiling, 10_000_000.0] {
        set_topo(0.0, 51.5, height).expect("admitted observer-height boundary");
        if let Ok(position) = calc_ut(jd, MOON, FLG_MOSEPH | FLG_TOPOCTR) {
            assert!(position.values.iter().all(|value| value.is_finite()));
        }
    }
    set_topo(0.0, 0.0, 0.0).expect("restore observer");
    assert!(calc_ut(jd, MOON, FLG_MOSEPH).is_ok());
    assert!(houses(jd, 51.5, 0.0, b'P').is_ok());
}

#[test]
fn extended_catalog_absence_is_actionable() {
    if !isolated::without_ephe_override("extended_catalog_absence_is_actionable") {
        return;
    }
    let empty =
        std::env::temp_dir().join(format!("swisseph-absent-catalog-{}", std::process::id()));
    std::fs::create_dir(&empty).expect("owned empty catalog directory");
    set_ephe_path(Some(empty.to_str().expect("native path"))).expect("select empty path");
    for error in [
        fixstar2("Sirius", 2451545.0, FLG_MOSEPH).unwrap_err(),
        fixstar2_ut("Sirius", 2451545.0, FLG_MOSEPH).unwrap_err(),
        fixstar2_mag("Sirius").unwrap_err(),
    ] {
        assert_eq!(error.kind(), ErrorKind::Native);
        assert!(error.message().contains("catalog") && error.message().contains("sefstars.txt"));
    }
    set_ephe_path(None).expect("reset native path");
    std::fs::remove_dir(empty).expect("remove owned directory");
}
