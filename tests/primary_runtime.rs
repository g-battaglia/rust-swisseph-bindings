//! Primary operation reachability and complete owned-configuration isolation.
//! Numerical controls live only in memory, never in persisted reference vectors.
use swisseph_bindings as eph;

fn configured() -> eph::native::SessionBuilder {
    let data = std::env::var("SWISSEPH_EPHE_DIR").expect("select primary runtime data");
    let mut builder = eph::native::SessionBuilder::new();
    builder.ephe_path(&data);
    builder
}

#[test]
fn all_configured_primary_operations_reach_the_native_engine() {
    let session = eph::native::primary_session(&configured()).unwrap();
    let target = eph::Target::Body(eph::SUN);
    let mut count = 0;
    macro_rules! run { ($operation:ident($($arg:expr),*)) => {{
        let result = session.$operation($($arg),*).unwrap_or_else(|error| panic!("{}: {error}", stringify!($operation)));
        std::hint::black_box(result); count += 1;
    }}; }
    run!(calc(2451545.0, eph::MARS, eph::FLG_MOSEPH | eph::FLG_SPEED));
    run!(calc_ut(
        2451545.0,
        eph::MARS,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(pheno(
        2451545.0,
        eph::MARS,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(pheno_ut(
        2451545.0,
        eph::MARS,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(nod_aps(
        2451545.0,
        eph::MARS,
        eph::NODBIT_OSCU,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(nod_aps_ut(
        2451545.0,
        eph::MARS,
        eph::NODBIT_OSCU,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(get_orbital_elements(
        2451545.0,
        eph::MARS,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(orbit_max_min_true_distance(
        2451545.0,
        eph::MARS,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(calc_pctr(
        2451545.0,
        eph::MARS,
        eph::JUPITER,
        eph::FLG_SWIEPH | eph::FLG_SPEED
    ));
    run!(get_planet_name(eph::MARS));
    run!(fixstar(
        "Sirius",
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(fixstar_ut(
        "Sirius",
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(fixstar2(
        "Sirius",
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(fixstar2_ut(
        "Sirius",
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(fixstar_mag("Sirius"));
    run!(fixstar2_mag("Sirius"));
    run!(houses(2451545.0, 51.5, 0.0, b'P'));
    run!(houses_ex(
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED,
        51.5,
        0.0,
        b'P'
    ));
    run!(houses_ex2(
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED,
        51.5,
        0.0,
        b'P'
    ));
    run!(houses_gauquelin(
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED,
        51.5,
        0.0
    ));
    run!(houses_armc(280.0, 51.5, 23.44, b'P'));
    run!(houses_armc_ex2(280.0, 51.5, 23.44, b'P', 0.0));
    run!(house_pos(280.0, 51.5, 23.44, b'P', 0.0, 0.0));
    run!(gauquelin_sector(
        2451545.0,
        &target,
        0,
        [0.0, 51.5, 0.0],
        1013.25,
        15.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(get_ayanamsa(2451545.0));
    run!(get_ayanamsa_ut(2451545.0));
    run!(get_ayanamsa_ex(2451545.0, eph::FLG_MOSEPH | eph::FLG_SPEED));
    run!(get_ayanamsa_ex_ut(
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(utc_to_jd(2000, 1, 1, 12, 0, 0.0, eph::Calendar::Gregorian));
    run!(jdet_to_utc(2451545.0, eph::Calendar::Gregorian));
    run!(jdut1_to_utc(2451545.0, eph::Calendar::Gregorian));
    run!(deltat(2451545.0));
    run!(deltat_ex(2451545.0, eph::FLG_MOSEPH | eph::FLG_SPEED));
    run!(time_equ(2451545.0));
    run!(lat_to_lmt(2451545.0, 0.0));
    run!(lmt_to_lat(2451545.0, 0.0));
    run!(sidtime(2451545.0));
    run!(sidtime0(2451545.0, 23.44, 0.0));
    run!(solcross(0.0, 2451545.0, eph::FLG_MOSEPH | eph::FLG_SPEED));
    run!(solcross_ut(
        0.0,
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(mooncross(0.0, 2451545.0, eph::FLG_MOSEPH | eph::FLG_SPEED));
    run!(mooncross_ut(
        0.0,
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(mooncross_node(2451545.0, eph::FLG_MOSEPH | eph::FLG_SPEED));
    run!(mooncross_node_ut(
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(helio_cross(
        eph::MARS,
        120.0,
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED,
        1
    ));
    run!(helio_cross_ut(
        eph::MARS,
        120.0,
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED,
        1
    ));
    run!(rise_trans(
        2451545.0,
        &target,
        eph::CALC_RISE,
        [0.0, 51.5, 0.0],
        1013.25,
        15.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(rise_trans_true_hor(
        2451545.0,
        &target,
        eph::CALC_RISE,
        [0.0, 51.5, 0.0],
        1013.25,
        15.0,
        0.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(azalt(
        2451545.0,
        0,
        [0.0, 51.5, 0.0],
        1013.25,
        15.0,
        0.0,
        51.5
    ));
    run!(azalt_rev(
        2451545.0,
        eph::native::HOR2EQU,
        [0.0, 51.5, 0.0],
        180.0,
        10.0
    ));
    run!(refrac(10.0, 1013.25, 15.0, 0));
    run!(refrac_extended(10.0, 0.0, 1013.25, 15.0, 0.0065, 0));
    run!(sol_eclipse_when_glob(
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED,
        0,
        false
    ));
    run!(lun_eclipse_when(
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED,
        0,
        false
    ));
    run!(sol_eclipse_where(
        2451545.0,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(sol_eclipse_how(
        2451545.0,
        [0.0, 51.5, 0.0],
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(lun_eclipse_how(
        2451545.0,
        [0.0, 51.5, 0.0],
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(sol_eclipse_when_loc(
        2451545.0,
        [0.0, 51.5, 0.0],
        eph::FLG_MOSEPH | eph::FLG_SPEED,
        false
    ));
    run!(lun_eclipse_when_loc(
        2451545.0,
        [0.0, 51.5, 0.0],
        eph::FLG_MOSEPH | eph::FLG_SPEED,
        false
    ));
    run!(lun_occult_where(
        2451545.0,
        &target,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(lun_occult_when_glob(
        2451545.0,
        &target,
        eph::FLG_MOSEPH | eph::FLG_SPEED,
        0,
        0
    ));
    run!(lun_occult_when_loc(
        2451545.0,
        &target,
        [0.0, 51.5, 0.0],
        eph::FLG_MOSEPH | eph::FLG_SPEED,
        0
    ));
    run!(heliacal_ut(
        2451545.0,
        [0.0, 51.5, 0.0],
        [0.0; 4],
        [0.0; 6],
        "Venus",
        eph::HELIACAL_RISING,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(heliacal_pheno_ut(
        2451545.0,
        [0.0, 51.5, 0.0],
        [0.0; 4],
        [0.0; 6],
        "Venus",
        eph::HELIACAL_RISING,
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    run!(vis_limit_mag(
        2451545.0,
        [0.0, 51.5, 0.0],
        [0.0; 4],
        [0.0; 6],
        "Venus",
        eph::FLG_MOSEPH | eph::FLG_SPEED
    ));
    assert_eq!(count, 65);
    for system in *b"Gg" {
        let values = session.houses_armc(280.0, 51.5, 23.44, system).unwrap();
        assert_eq!(values.cusps.len(), 36);
        let rates = session
            .houses_armc_ex2(280.0, 51.5, 23.44, system, 0.0)
            .unwrap();
        assert_eq!(rates.cusps.len(), 36);
        assert_eq!(rates.cusp_speeds.len(), 36);
    }
    for system in [b'G', b'g', 0, 255] {
        assert_eq!(
            session
                .houses(2451545.0, 51.5, 0.0, system)
                .unwrap_err()
                .kind(),
            eph::ErrorKind::InvalidInput
        );
    }
    let long = eph::Target::Star("x".repeat(1024));
    assert_eq!(
        session
            .rise_trans(
                2451545.0,
                &long,
                eph::CALC_RISE,
                [0.0, 51.5, 0.0],
                0.0,
                0.0,
                eph::FLG_MOSEPH
            )
            .unwrap_err()
            .kind(),
        eph::ErrorKind::InvalidInput
    );
    assert!(session.deltat(f64::NAN).is_err());
}

fn exact(actual: eph::Position, expected: &eph::Position) {
    assert_eq!(actual.returned_flags, expected.returned_flags);
    assert_eq!(actual.diagnostic, expected.diagnostic);
    assert_eq!(actual.warnings, expected.warnings);
    assert_eq!(actual.messages, expected.messages);
    assert!(
        actual
            .values
            .iter()
            .zip(expected.values)
            .all(|(a, b)| a.to_bits() == b.to_bits())
    );
}
#[test]
fn primary_sessions_restore_owned_defaults_in_eight_thread_aba_sequences() {
    let a = eph::native::primary_session(&configured()).unwrap();
    let b = eph::native::primary_session(
        configured()
            .sid_mode(eph::SIDM_LAHIRI, 0.0, 0.0)
            .topo(12.5, 41.9, 150.0)
            .tid_acc(-25.8)
            .delta_t_override(0.001)
            .lapse_rate(0.007),
    )
    .unwrap();
    let flags = eph::FLG_MOSEPH | eph::FLG_SPEED | eph::FLG_SIDEREAL | eph::FLG_TOPOCTR;
    let va = a.calc_ut(2451545.0, eph::MARS, flags).unwrap();
    let vb = b.calc_ut(2451545.0, eph::MARS, flags).unwrap();
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let (a, b, va, vb) = (&a, &b, &va, &vb);
            scope.spawn(move || {
                for _ in 0..8 {
                    eph::global::set_sid_mode(eph::SIDM_FAGAN_BRADLEY | 8192, 0.0, 0.0).unwrap();
                    eph::global::set_delta_t_userdef(Some(0.05)).unwrap();
                    exact(a.calc_ut(2451545.0, eph::MARS, flags).unwrap(), va);
                    exact(b.calc_ut(2451545.0, eph::MARS, flags).unwrap(), vb);
                    exact(a.calc_ut(2451545.0, eph::MARS, flags).unwrap(), va);
                }
            });
        }
    });
}

#[test]
fn sunshine_point_positions_reject_unrepresented_native_history() {
    use std::{path::Path, process::Command};
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Path::new(env!("OUT_DIR"));
    let metadata = std::fs::read_to_string(out.join("native-inputs.txt")).unwrap();
    let compiler = metadata
        .lines()
        .find_map(|line| line.strip_prefix("compiler: "))
        .unwrap();
    let binary = out.join("primary-sunshine-context-oracle");
    assert!(
        Command::new(compiler)
            .args(["-O2", "-Wall", "-Wextra", "-Werror", "-I"])
            .arg(root.join("swisseph"))
            .arg(root.join("tests/primary_sunshine_context_oracle.c"))
            .arg(out.join("libswisseph.a"))
            .args(["-lm", "-o"])
            .arg(&binary)
            .status()
            .unwrap()
            .success()
    );
    let output = Command::new(binary)
        .env_remove("SE_EPHE_PATH")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"history_dependent=1 statuses=0,0\n");
    let session = eph::SessionBuilder::new().build().unwrap();
    for system in *b"Ii" {
        let error = match session.house_pos(100.0, 45.0, 23.44, system, 80.0, 0.0) {
            Err(error) => error,
            Ok(_) => panic!("unrepresented solar context was inherited"),
        };
        assert_eq!(error.kind(), eph::ErrorKind::Unsupported);
        assert_eq!(error.operation(), Some("house_pos"));
        assert!(
            error.native_status().is_none()
                && error.native_scalar().is_none()
                && error.diagnostic().is_none()
        );
        assert!(
            std::error::Error::source(&error).is_none(),
            "no C operation was called"
        );
    }
}
