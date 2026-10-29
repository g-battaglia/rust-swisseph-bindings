//! Owned session operations compared to serial free calls and conflicting threads.
//! Expected arrays stay in memory; assertions print only operation and case ids.
use std::sync::Arc;
use swisseph_bindings::*;
use swisseph_bindings::{
    global::calc_pctr, global::fixstar, global::fixstar_mag, global::fixstar_ut, global::fixstar2,
    global::fixstar2_mag, global::fixstar2_ut, global::get_ayanamsa, global::get_ayanamsa_ex,
    global::get_ayanamsa_ex_ut, global::houses_ex2, global::houses_gauquelin,
    global::set_delta_t_userdef, global::set_ephe_path, global::set_sid_mode, global::set_topo,
    global::time_equ, native::heliacal_pheno_ut_full, native::lun_occult_where_full,
    native::sol_eclipse_where_full,
};

// Serial scripts use non-atomic global setters to construct independent
// expectations. Their internally spawned contention threads remain concurrent.
static EXPECTATION_SCRIPT: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn complete_session_operations_remain_configured_under_contention() {
    let _script = EXPECTATION_SCRIPT.lock().unwrap();
    let data = std::env::var("SWISSEPH_EPHE_DIR").expect("select identified data");
    let mut records = Vec::new();
    for (mode, lon, lat, offset) in [
        (SIDM_FAGAN_BRADLEY, 12.5, 41.9, 0.001),
        (SIDM_LAHIRI, -75.0, -20.0, 0.002),
    ] {
        set_ephe_path(Some(&data)).unwrap();
        set_sid_mode(mode, 0.0, 0.0).unwrap();
        set_topo(lon, lat, 123.0).unwrap();
        set_delta_t_userdef(Some(offset)).unwrap();
        let mut b = SessionBuilder::new();
        b.ephe_path(&data)
            .sid_mode(mode, 0.0, 0.0)
            .topo(lon, lat, 123.0)
            .delta_t_override(offset);
        let session = Arc::new(b.build().unwrap());
        let jd = 2451545.0;
        let flags = FLG_MOSEPH | FLG_SIDEREAL | FLG_SPEED;
        let expected = (
            get_ayanamsa(jd).unwrap(),
            get_ayanamsa_ex(jd, flags).unwrap(),
            get_ayanamsa_ex_ut(jd, flags).unwrap(),
            houses_ex2(jd, FLG_SIDEREAL, lat, lon, b'P').unwrap(),
            calc_pctr(jd, MOON, EARTH, FLG_SWIEPH | FLG_SPEED).unwrap(),
            time_equ(jd).unwrap(),
            sol_eclipse_where_full(jd, FLG_MOSEPH).unwrap(),
            lun_occult_where_full(jd, SUN, None, FLG_MOSEPH).unwrap(),
            fixstar2_ut("Sirius", jd, flags).unwrap(),
            fixstar2_mag("Sirius").unwrap(),
        );
        records.push((session, lat, lon, expected));
    }
    let threads: Vec<_> = records
        .into_iter()
        .map(|(s, lat, lon, e)| {
            std::thread::spawn(move || {
                for _ in 0..24 {
                    let jd = 2451545.0;
                    let flags = FLG_MOSEPH | FLG_SIDEREAL | FLG_SPEED;
                    assert!(
                        s.get_ayanamsa(jd).unwrap().to_bits() == e.0.to_bits(),
                        "meanTT"
                    );
                    assert!(s.get_ayanamsa_ex(jd, flags).unwrap() == e.1, "extendedTT");
                    assert!(
                        s.get_ayanamsa_ex_ut(jd, flags).unwrap() == e.2,
                        "extendedUT"
                    );
                    assert!(
                        s.houses_ex2(jd, FLG_SIDEREAL, lat, lon, b'P').unwrap() == e.3,
                        "house speeds"
                    );
                    assert!(
                        s.calc_pctr(jd, MOON, EARTH, FLG_SWIEPH | FLG_SPEED)
                            .unwrap()
                            == e.4,
                        "center body"
                    );
                    assert!(s.time_equ(jd).unwrap() == e.5, "equation of time");
                    assert!(
                        s.sol_eclipse_where_full(jd, FLG_MOSEPH).unwrap() == e.6,
                        "solar geography"
                    );
                    assert!(
                        s.lun_occult_where_full(jd, SUN, None, FLG_MOSEPH).unwrap() == e.7,
                        "occult geography"
                    );
                    assert!(
                        s.fixstar2_ut("Sirius", jd, flags).unwrap() == e.8,
                        "fixed star"
                    );
                    assert!(s.fixstar2_mag("Sirius").unwrap() == e.9, "magnitude");
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    println!("2owned configurations x24iterations x10operations; exact values/flags/diagnostics");
}

#[test]
fn additional_native_session_routes_keep_complete_results_and_errors() {
    let _script = EXPECTATION_SCRIPT.lock().unwrap();
    let data = std::env::var("SWISSEPH_EPHE_DIR").expect("select identified data");
    set_ephe_path(Some(&data)).unwrap();
    set_sid_mode(SIDM_LAHIRI, 0.0, 0.0).unwrap();
    let mut builder = SessionBuilder::new();
    builder.ephe_path(&data).sid_mode(SIDM_LAHIRI, 0.0, 0.0);
    let session = builder.build().unwrap();
    let jd = 2451545.0;
    let flags = FLG_MOSEPH | FLG_SIDEREAL;
    assert!(
        session.houses_gauquelin(jd, flags, 45.0, 0.0).unwrap()
            == houses_gauquelin(jd, flags, 45.0, 0.0).unwrap()
    );
    assert!(session.fixstar("Sirius", jd, flags).unwrap() == fixstar("Sirius", jd, flags).unwrap());
    assert!(
        session.fixstar_ut("Sirius", jd, flags).unwrap()
            == fixstar_ut("Sirius", jd, flags).unwrap()
    );
    assert!(
        session.fixstar2("Sirius", jd, flags).unwrap() == fixstar2("Sirius", jd, flags).unwrap()
    );
    assert!(session.fixstar_mag("Sirius").unwrap() == fixstar_mag("Sirius").unwrap());
    let atm = [1013.25, 15.0, 40.0, 0.0];
    let obs = [36.0, 1.0, 0.0, 0.0, 0.0, 0.0];
    assert!(
        session
            .heliacal_pheno_ut_full(jd, 0.0, 45.0, 0.0, atm, obs, "Venus", 1, FLG_MOSEPH)
            .unwrap()
            == heliacal_pheno_ut_full(jd, 0.0, 45.0, 0.0, atm, obs, "Venus", 1, FLG_MOSEPH)
                .unwrap()
    );
    for bad in [f64::NAN, f64::INFINITY] {
        let error = session.get_ayanamsa(bad).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
    }
    assert!(
        session.calc_pctr(jd, MOON, EARTH, FLG_MOSEPH).is_err(),
        "native unsupported source remains an error"
    );
}
