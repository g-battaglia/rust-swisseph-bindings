//! Raw C witnesses undefined geometry; safe interfaces preserve current admission.
use swisseph_bindings::native::{FLG_MOSEPH, MOON, SessionBuilder, global, native};

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
fn safe_interfaces_reject_undefined_self_occultation_geometry() {
    let results = [
        native::lun_occult_where_full(2451545.0, MOON, None, FLG_MOSEPH).map(|_| ()),
        SessionBuilder::new()
            .build()
            .unwrap()
            .lun_occult_where_full(2451545.0, MOON, None, FLG_MOSEPH)
            .map(|_| ()),
        global::lun_occult_where(2451545.0, MOON, None, FLG_MOSEPH).map(|_| ()),
    ];
    for result in results {
        match result {
            Err(error) => assert_eq!(
                error.kind(),
                swisseph_bindings::native::ErrorKind::InvalidInput
            ),
            Ok(_) => panic!("undefined Moon self-geometry must be rejected"),
        }
    }
}
