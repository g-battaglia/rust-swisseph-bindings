//! Regression inputs for path capacity, nonterminating flags, effective
//! sidereal epochs, strict component sources and inherited tidal selection.
//! Native comparisons stay in memory; no astronomical vectors are stored.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use swisseph_bindings::*;

#[path = "support/isolated.rs"]
mod isolated;
#[path = "data_dir.rs"]
mod test_data;

const JD: f64 = 2451545.0;
const USER_UT: i32 = SIDM_USER | 1024;

fn invalid<T: std::fmt::Debug>(result: Result<T, Error>) {
    assert_eq!(
        result.expect_err("invalid native request").kind(),
        ErrorKind::InvalidInput
    );
}

/// Bound an isolated regression so a native hang cannot hang the harness.
fn run_child(mut command: Command) {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("isolated child");
    let start = Instant::now();
    loop {
        if child.try_wait().expect("child status").is_some() {
            let output = child.wait_with_output().expect("child output");
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        if start.elapsed() > Duration::from_secs(45) {
            child.kill().expect("stop watchdog child");
            child.wait().expect("reap watchdog child");
            panic!("isolated regression exceeded watchdog");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn library_path_handles_long_executable_paths() {
    if std::env::var_os("SWISSEPH_LONG_PATH_CHILD").is_some() {
        let executable = std::env::current_exe()
            .expect("executable pathname")
            .canonicalize()
            .expect("native loader's canonical pathname");
        let expected = executable.to_str().expect("ASCII regression path");
        assert!(expected.len() > 255);
        let actual = library_path().expect("owned native pathname");
        assert!(
            actual.as_bytes() == &expected.as_bytes()[..256],
            "native pathname has its documented truncation"
        );
        set_ephe_path(Some("/short/ephemeris/path")).expect("independent ephemeris setting");
        assert_eq!(library_path().expect("independent executable path"), actual);
        return;
    }
    let root = std::env::temp_dir().join(format!("swisseph-long-path-{}", std::process::id()));
    let directory = root
        .join("a".repeat(90))
        .join("b".repeat(90))
        .join("c".repeat(90));
    std::fs::create_dir_all(&directory).expect("owned long directory");
    let copied = directory.join("binding-regression");
    std::fs::copy(std::env::current_exe().expect("test executable"), &copied)
        .expect("copy owned test binary");
    let mut command = Command::new(&copied);
    command
        .args([
            "--exact",
            "library_path_handles_long_executable_paths",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("SWISSEPH_LONG_PATH_CHILD", "1")
        .env_remove("SE_EPHE_PATH");
    run_child(command);
    std::fs::remove_dir_all(root).expect("remove owned long-path fixture");
}

#[test]
fn heliocentric_solar_crossings_reject_without_holding_native_access() {
    if std::env::var_os("SWISSEPH_SOLAR_CROSSING_CHILD").is_some() {
        let (done, receive) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            for target in [0.0, 90.0] {
                invalid(solcross(target, JD, FLG_MOSEPH | FLG_HELCTR));
                invalid(solcross_ut(target, JD, FLG_MOSEPH | FLG_HELCTR));
            }
            done.send(()).expect("completion");
        });
        let (version_done, version_receive) = std::sync::mpsc::channel();
        let reader = std::thread::spawn(move || {
            version_done.send(version()).expect("version completion");
        });
        receive
            .recv_timeout(Duration::from_secs(2))
            .expect("crossings reject promptly");
        assert!(
            !version_receive
                .recv_timeout(Duration::from_secs(2))
                .expect("native reader cannot be trapped")
                .expect("version read")
                .is_empty()
        );
        worker.join().expect("crossing thread");
        reader.join().expect("reader thread");
        assert!(
            solcross(90.0, JD, FLG_MOSEPH)
                .expect("valid solar search")
                .time
                > JD
        );
        return;
    }
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command
        .args([
            "--exact",
            "heliocentric_solar_crossings_reject_without_holding_native_access",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("SWISSEPH_SOLAR_CROSSING_CHILD", "1");
    run_child(command);
}

#[test]
fn strict_source_checks_reported_component_fallbacks() {
    let mut position = Position {
        values: [0.0; 6],
        returned_flags: FLG_SWIEPH,
        diagnostic: "using Moshier eph. for moon;".into(),
    };
    assert_eq!(
        require_source_flags(&position, FLG_SWIEPH)
            .unwrap_err()
            .kind(),
        ErrorKind::Native
    );
    require_source_flags(&position, FLG_SWIEPH | FLG_MOSEPH)
        .expect("both reported sources permitted");
    require_source_flags(&position, 0).expect("unrestricted default chain");
    position.diagnostic = "unrelated warning mentions Moshier without a fallback".into();
    require_source_flags(&position, FLG_SWIEPH).expect("warning is not automatically a fallback");
    position.diagnostic = "USING SWISS EPH. for component".into();
    position.returned_flags = FLG_JPLEPH;
    assert!(require_source_flags(&position, FLG_JPLEPH).is_err());
}

unsafe extern "C" {
    fn swe_houses(
        jd: f64,
        latitude: f64,
        longitude: f64,
        system: i32,
        cusps: *mut f64,
        angles: *mut f64,
    ) -> i32;
    fn swe_sidtime(jd: f64) -> f64;
    fn swe_get_ayanamsa_ut(jd: f64) -> f64;
}

#[test]
fn configured_sidereal_epochs_and_tidal_selection() {
    if !isolated::without_ephe_override("configured_sidereal_epochs_and_tidal_selection") {
        return;
    }
    set_tid_acc(-26.0).expect("first tidal setter");
    set_delta_t_userdef(Some(0.0)).expect("valid active shift");
    assert_eq!(
        get_tid_acc().expect("first-use configuration preserved"),
        -26.0
    );
    set_sid_mode(USER_UT, JD, 15.0).expect("UT reference");
    let baseline = get_ayanamsa(JD).expect("reference baseline");
    set_delta_t_userdef(Some(1e100)).expect("finite override");
    invalid(set_sid_mode(USER_UT, JD, 30.0));
    invalid(get_ayanamsa(JD));
    invalid(get_ayanamsa_ex(JD, FLG_MOSEPH));
    invalid(calc(JD, MOON, FLG_MOSEPH | FLG_SIDEREAL));
    assert!(
        calc(JD, MOON, FLG_MOSEPH).is_ok(),
        "unused sidereal mode does not block tropical ET"
    );
    invalid(SessionBuilder::new().sid_mode(USER_UT, JD, 30.0).build());
    invalid(
        SessionBuilder::new()
            .sid_mode(USER_UT, JD, 30.0)
            .delta_t_override(1e100)
            .build(),
    );
    invalid(
        SessionBuilder::new()
            .sid_mode(USER_UT, JD, 30.0)
            .tid_acc(SE_TIDAL_AUTOMATIC)
            .build(),
    );
    let valid = SessionBuilder::new()
        .sid_mode(USER_UT, JD, 15.0)
        .delta_t_override(0.0)
        .build()
        .expect("explicit safe shift overrides inherited huge one");
    assert_eq!(deltat(JD).expect("builder preserves override"), 1e100);
    let cleared = SessionBuilder::new()
        .sid_mode(USER_UT, JD, 15.0)
        .clear_delta_t_override()
        .build()
        .expect("clear computed reference without lasting changes");
    assert_eq!(
        deltat(JD).expect("temporary preflight restores override"),
        1e100
    );
    assert!(
        cleared
            .get_ayanamsa_ut(JD)
            .expect("computed shift")
            .is_finite()
    );
    valid.get_ayanamsa_ut(JD).expect("explicit valid session");
    assert_eq!(
        get_ayanamsa(JD).expect("rejected setter preserved preceding mode"),
        baseline
    );
    set_delta_t_userdef(Some(1e100)).expect("change inherited shift after build");
    let inherited = SessionBuilder::new()
        .build()
        .expect("inherited state is checked at call time");
    invalid(inherited.calc(JD, MOON, FLG_MOSEPH | FLG_SIDEREAL));
    close().expect("reset tracked write-only native settings");
    assert!(
        get_ayanamsa(JD)
            .expect("native default after close")
            .is_finite()
    );

    let historical = julday(1850, 1, 1, 12.0, Calendar::Gregorian);
    let model = SessionBuilder::new()
        .sid_mode(USER_UT, historical, 15.0)
        .tid_acc(1e100)
        .clear_delta_t_override()
        .build()
        .expect("computed model validated with complete session at call time");
    invalid(model.get_ayanamsa_ut(JD));
    close().expect("reset after rejected configured computation");

    let Some(data) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP file-specific fallback/tidal comparisons: select complete SWISSEPH_EPHE_DIR"
        );
        return;
    };
    let partial =
        std::env::temp_dir().join(format!("swisseph-partial-moon-{}", std::process::id()));
    std::fs::create_dir(&partial).expect("owned partial data directory");
    std::fs::copy(
        std::path::Path::new(&data).join("sepl_18.se1"),
        partial.join("sepl_18.se1"),
    )
    .expect("copy planet data only");
    set_ephe_path(Some(partial.to_str().expect("native path"))).expect("partial files");
    let mixed = calc_ut(JD, MARS, FLG_SWIEPH).expect("reported native component fallback");
    assert_ne!(mixed.returned_flags & FLG_SWIEPH, 0);
    assert!(mixed.diagnostic.contains("using Moshier eph. for moon"));
    let error = require_source_flags(&mixed, FLG_SWIEPH).expect_err("strict component gate");
    assert!(error.message().contains(&mixed.diagnostic));
    set_ephe_path(Some(&data)).expect("complete data");
    std::fs::remove_dir_all(partial).expect("remove only owned data copy");
    set_tid_acc(SE_TIDAL_AUTOMATIC).expect("automatic tidal selection");
    calc_ut(JD, MARS, FLG_SWIEPH).expect("install file-dependent tidal selection");
    let selected = get_tid_acc().expect("Swiss file selection");
    assert_eq!(selected, SE_TIDAL_DE441);
    let date = julday(1900, 1, 1, 12.0, Calendar::Gregorian);
    let mut cusps = [0.0; 13];
    let mut angles = [0.0; 10];
    assert_eq!(
        // SAFETY: isolated single-test child, no concurrent raw/binding calls;
        // public ABI with ordinary finite inputs and initialized native widths.
        unsafe {
            swe_houses(
                date,
                51.5,
                0.0,
                i32::from(b'P'),
                cusps.as_mut_ptr(),
                angles.as_mut_ptr(),
            )
        },
        0
    );
    assert_eq!(
        get_tid_acc().expect("direct native houses preserve selection"),
        selected
    );
    let bound = houses(date, 51.5, 0.0, b'P').expect("checked houses");
    assert!(
        bound
            .cusps
            .iter()
            .zip(&cusps[1..])
            .all(|(a, b)| a.to_bits() == b.to_bits())
    );
    assert!(
        bound
            .angles
            .iter()
            .zip(angles)
            .all(|(a, b)| a.to_bits() == b.to_bits())
    );
    assert_eq!(
        get_tid_acc().expect("house preflight preserves inherited selection"),
        selected
    );
    // SAFETY: same isolated native access discipline and ordinary date.
    let raw_sidtime = unsafe { swe_sidtime(date) };
    assert_eq!(
        sidtime(date).expect("sidereal time").to_bits(),
        raw_sidtime.to_bits()
    );
    // SAFETY: same isolated discipline; scalar public ABI.
    let raw_ayanamsha = unsafe { swe_get_ayanamsa_ut(date) };
    assert_eq!(
        get_ayanamsa_ut(date).expect("ayanamsha").to_bits(),
        raw_ayanamsha.to_bits()
    );
    assert_eq!(
        get_tid_acc().expect("inherited scalar preflights"),
        selected
    );
    calc_ut(JD, SUN, FLG_MOSEPH).expect("subsequent automatic source change");
    assert_eq!(
        get_tid_acc().expect("automatic mode remains enabled"),
        SE_TIDAL_MOSEPH
    );
    close().expect("reset native state");
}
