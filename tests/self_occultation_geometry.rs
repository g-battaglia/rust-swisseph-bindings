//! Preserve the pinned C engine's undefined fixed-instant geometry, not a fake error.
use swisseph_bindings::{FLG_MOSEPH, MOON, SessionBuilder, global, native};

#[test]
fn pinned_c_witness_confirms_the_undefined_geography() {
    use std::path::Path;
    use std::process::Command;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Path::new(env!("OUT_DIR"));
    let identity = std::fs::read_to_string(output.join("native-inputs.txt")).unwrap();
    let compiler = identity
        .lines()
        .find_map(|line| line.strip_prefix("compiler: "))
        .unwrap();
    let executable = output.join("self-occultation-geometry-oracle");
    assert!(
        Command::new(compiler)
            .args(["-O2", "-I"])
            .arg(root.join("swisseph"))
            .arg(root.join("tests/self_occultation_geometry_oracle.c"))
            .arg(output.join("libswisseph.a"))
            .args(["-lm", "-o"])
            .arg(&executable)
            .status()
            .unwrap()
            .success()
    );
    let witness = Command::new(executable)
        .env_remove("SE_EPHE_PATH")
        .output()
        .unwrap();
    assert!(
        witness.status.success(),
        "retain the actual native non-finite result"
    );
    eprintln!("{}", std::str::from_utf8(&witness.stdout).unwrap().trim());
}

#[test]
fn session_and_projection_retain_every_fixed_instant_native_word() {
    let native = native::lun_occult_where_full(2451545.0, MOON, None, FLG_MOSEPH).unwrap();
    let session = SessionBuilder::new()
        .build()
        .unwrap()
        .lun_occult_where_full(2451545.0, MOON, None, FLG_MOSEPH)
        .unwrap();
    assert_eq!(native.occult_type, session.occult_type);
    assert_eq!(native.diagnostic, session.diagnostic);
    assert!(
        native
            .geopos
            .into_iter()
            .chain(native.attributes)
            .zip(session.geopos.into_iter().chain(session.attributes))
            .all(|(a, b)| a.to_bits() == b.to_bits())
    );
    let projected = global::lun_occult_where(2451545.0, MOON, None, FLG_MOSEPH).unwrap();
    assert_eq!(projected.longitude.to_bits(), native.geopos[0].to_bits());
    assert_eq!(projected.latitude.to_bits(), native.geopos[1].to_bits());
}
