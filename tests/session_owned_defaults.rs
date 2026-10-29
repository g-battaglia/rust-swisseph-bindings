//! Owned defaults must cover native model side effects, not just seven setters.
//! Expected numerical values remain in memory and come from the actual engine.
#[path = "support/isolated.rs"]
mod isolated;
use std::path::Path;
use std::process::Command;
use swisseph_bindings as eph;

#[test]
fn default_session_rejects_inherited_original_precession_models() {
    if !isolated::without_ephe_override(
        "default_session_rejects_inherited_original_precession_models",
    ) {
        return;
    }
    // Header-defined SE_SIDBIT_PREC_ORIG: this genuine global option also writes
    // precession/nutation models. Re-selecting Fagan alone does not clear them.
    const ORIGINAL_PRECESSION: i32 = 8192;
    let session = eph::native::primary_session(
        eph::native::SessionBuilder::new().ephe_path("/nonexistent/owned-session-model-witness"),
    )
    .unwrap();
    let mut controls = Vec::new();
    for jd in [2415020.0, 2451545.0, 2460000.5] {
        for body in [eph::MOON, eph::MARS] {
            for shape in [0, eph::FLG_SIDEREAL, eph::FLG_EQUATORIAL] {
                let flags = eph::FLG_MOSEPH | eph::FLG_SPEED | shape;
                let before = session.calc(jd, body, flags).unwrap();
                controls.push((jd, body, flags, before));
            }
        }
    }
    for mode in [eph::SIDM_FAGAN_BRADLEY, eph::SIDM_LAHIRI] {
        for (jd, body, flags, before) in &controls {
            eph::global::set_sid_mode(mode | ORIGINAL_PRECESSION, 0.0, 0.0).unwrap();
            // Also populate native numerical caches using the foreign model.
            eph::global::calc(*jd, *body, *flags).unwrap();
            let after = session.calc(*jd, *body, *flags).unwrap();
            assert_eq!(before.returned_flags, after.returned_flags);
            assert_eq!(before.diagnostic, after.diagnostic);
            assert!(
                before
                    .values
                    .iter()
                    .zip(after.values)
                    .all(|(a, b)| a.to_bits() == b.to_bits()),
                "a global original-precession option leaked into the default session"
            );
        }
    }
}

#[test]
fn public_header_witness_reproduces_and_resets_model_side_effect() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Path::new(env!("OUT_DIR"));
    let identity = std::fs::read_to_string(out.join("native-inputs.txt")).unwrap();
    let compiler = identity
        .lines()
        .find_map(|line| line.strip_prefix("compiler: "))
        .unwrap();
    let oracle = out.join("session-models-oracle");
    assert!(
        Command::new(compiler)
            .args(["-O2", "-Wall", "-Wextra", "-Werror", "-I"])
            .arg(root.join("swisseph"))
            .arg(root.join("tests/session_models_oracle.c"))
            .arg(out.join("libswisseph.a"))
            .args(["-lm", "-o"])
            .arg(&oracle)
            .status()
            .unwrap()
            .success()
    );
    let output = Command::new(oracle)
        .env_remove("SE_EPHE_PATH")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "native witness did not reproduce the model side effect"
    );
    let report = String::from_utf8(output.stdout).unwrap();
    assert!(report.contains("defaults_restore_exact=1"));
    eprint!("{report}");
}
