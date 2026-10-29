//! Negative C status and unmodified diagnostic remain separately accessible.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use swisseph_bindings::global::{calc, calc_pctr, calc_ut, set_ephe_path};
use swisseph_bindings::native::*;

static ORACLE_LOCK: Mutex<()> = Mutex::new(());

/// Compile a public-header caller against this target's identified archive.
fn oracle() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Path::new(env!("OUT_DIR"));
    let identity = std::fs::read_to_string(out.join("native-inputs.txt")).unwrap();
    let compiler = identity
        .lines()
        .find_map(|s| s.strip_prefix("compiler: "))
        .unwrap();
    let bin = out.join("position-error-oracle");
    assert!(
        Command::new(compiler)
            .args(["-O2", "-I"])
            .arg(root.join("swisseph"))
            .arg(root.join("tests/position_error_oracle.c"))
            .arg(out.join("libswisseph.a"))
            .args(["-lm", "-o"])
            .arg(&bin)
            .status()
            .unwrap()
            .success()
    );
    bin
}

#[test]
fn negative_position_status_and_diagnostic_match_independent_c() {
    let _oracle_lock = ORACLE_LOCK.lock().unwrap();
    let bin = oracle();
    set_ephe_path(Some("/tmp/swiss-position-error-missing-data")).unwrap();
    let mut cases = 0;
    for operation in ["calc", "calc_ut"] {
        for flags in [FLG_SWIEPH | FLG_SPEED, FLG_MOSEPH | FLG_SPEED] {
            let expected = Command::new(&bin)
                .args([operation, &flags.to_string()])
                .env_remove("SE_EPHE_PATH")
                .output()
                .unwrap();
            assert!(expected.status.success());
            let status = i32::from_ne_bytes(expected.stdout[..4].try_into().unwrap());
            let length = u32::from_ne_bytes(expected.stdout[4..8].try_into().unwrap()) as usize;
            assert_eq!(expected.stdout.len(), 8 + length);
            let diagnostic = std::str::from_utf8(&expected.stdout[8..]).unwrap();
            let error = if operation == "calc" {
                calc(2451545.0, 999999, flags)
            } else {
                calc_ut(2451545.0, 999999, flags)
            }
            .unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Native);
            assert!(status < 0);
            assert_eq!(error.native_status(), Some(status));
            assert_eq!(error.native_operation(), Some(operation));
            assert!(
                error.native_diagnostic() == Some(diagnostic),
                "original C diagnostic differs"
            );
            assert!(error.message().contains(diagnostic));
            assert!(error.message().contains("body=999999"));
            cases += 1;
        }
    }
    let invalid = calc(f64::NAN, SUN, 0).unwrap_err();
    assert_eq!(invalid.kind(), ErrorKind::InvalidInput);
    assert_eq!(invalid.native_status(), None);
    assert_eq!(invalid.native_diagnostic(), None);
    assert_eq!(invalid.native_operation(), None);
    assert_eq!(cases, 4);
    println!(
        "negative position metadata: four C pairs exact; one preflight has no invented status"
    );
}

#[test]
fn planet_centered_moshier_failures_keep_raw_native_metadata() {
    let _oracle_lock = ORACLE_LOCK.lock().unwrap();
    let bin = oracle();
    let session = SessionBuilder::new().build().unwrap();
    let mut cases = 0;
    for flags in [
        FLG_MOSEPH,
        FLG_MOSEPH | FLG_SPEED,
        FLG_MOSEPH | FLG_EQUATORIAL,
        FLG_MOSEPH | FLG_SPEED | FLG_EQUATORIAL,
    ] {
        let expected = Command::new(&bin)
            .args(["calc_pctr", &flags.to_string()])
            .env_remove("SE_EPHE_PATH")
            .output()
            .unwrap();
        assert!(expected.status.success());
        let status = i32::from_ne_bytes(expected.stdout[..4].try_into().unwrap());
        let length = u32::from_ne_bytes(expected.stdout[4..8].try_into().unwrap()) as usize;
        assert_eq!(expected.stdout.len(), 8 + length);
        let diagnostic = std::str::from_utf8(&expected.stdout[8..]).unwrap();
        assert!(status < 0);
        assert!(!diagnostic.is_empty());
        let error = match calc_pctr(2451545.0, MOON, MARS, flags) {
            Err(error) => error,
            Ok(_) => panic!("native unsupported source must remain an error"),
        };
        assert_eq!(error.kind(), ErrorKind::Native);
        let message =
            format!("calc_pctr: {diagnostic} (jd_et=2451545, body=1, center=4, flags={flags})");
        assert!(
            error.message() == message,
            "original contextual message changed"
        );
        assert!(
            error.to_string() == format!("native error: {message}"),
            "original Display changed"
        );
        assert_eq!(error.native_status(), Some(status));
        assert_eq!(error.native_operation(), Some("calc_pctr"));
        assert!(
            error.native_diagnostic() == Some(diagnostic),
            "raw C diagnostic differs"
        );
        let configured = match session.calc_pctr(2451545.0, MOON, MARS, flags) {
            Err(error) => error,
            Ok(_) => panic!("session must preserve the unsupported-source error"),
        };
        assert!(
            configured == error,
            "free/session metadata or message differs"
        );
        cases += 1;
    }
    let invalid = match calc_pctr(f64::NAN, MOON, MARS, FLG_MOSEPH) {
        Err(error) => error,
        Ok(_) => panic!("non-finite input must remain a preflight error"),
    };
    assert_eq!(invalid.kind(), ErrorKind::InvalidInput);
    assert_eq!(invalid.native_status(), None);
    assert_eq!(invalid.native_operation(), None);
    assert_eq!(invalid.native_diagnostic(), None);
    eprintln!(
        "PASS planet-centered failure metadata: {cases} direct C pairs; original messages/Display and session results unchanged; preflight has no native metadata"
    );
}
