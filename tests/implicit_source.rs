//! Implicit UT source selection must match an independent native C call.
use std::path::{Path, PathBuf};
use std::process::Command;
use swisseph_bindings::*;

/// Compile against the same identified archive and public C header.
fn oracle() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Path::new(env!("OUT_DIR"));
    let identity = std::fs::read_to_string(out.join("native-inputs.txt")).unwrap();
    let compiler = identity
        .lines()
        .find_map(|line| line.strip_prefix("compiler: "))
        .unwrap();
    let binary = out.join("implicit-source-oracle");
    assert!(
        Command::new(compiler)
            .args(["-O2", "-I"])
            .arg(root.join("swisseph"))
            .arg(root.join("tests/implicit_source_oracle.c"))
            .arg(out.join("libswisseph.a"))
            .args(["-lm", "-o"])
            .arg(&binary)
            .status()
            .unwrap()
            .success()
    );
    binary
}

#[test]
fn implicit_ut_source_matches_native_before_and_after_analytical_calls() {
    let oracle = oracle();
    let missing = std::env::temp_dir().join(format!(
        "swiss-implicit-source-empty-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&missing).unwrap();
    let mut paths = vec![missing.clone()];
    if let Some(path) = std::env::var_os("SWISSEPH_EPHE_DIR") {
        paths.push(path.into());
    }
    let mut pairs = 0;
    let mut mismatches = Vec::new();
    for path in &paths {
        for jd in [2451545.0, 1700000.5] {
            for body in [ECL_NUT, SUN, MOON, MARS] {
                for flags in [0, FLG_SWIEPH, FLG_MOSEPH] {
                    let output = Command::new(&oracle)
                        .arg(path)
                        .args([jd.to_string(), body.to_string(), flags.to_string()])
                        .env_remove("SE_EPHE_PATH")
                        .output()
                        .unwrap();
                    assert!(output.status.success());
                    assert_eq!(output.stdout.len(), 4 + 6 * 8);
                    let status = i32::from_ne_bytes(output.stdout[..4].try_into().unwrap());
                    set_ephe_path(Some(path.to_str().unwrap())).unwrap();
                    calc_ut(jd, MARS, FLG_MOSEPH | FLG_SPEED | FLG_RADIANS).unwrap();
                    let actual = calc_ut(jd, body, flags).unwrap();
                    let words_equal = actual
                        .values
                        .iter()
                        .zip(output.stdout[4..].as_chunks::<8>().0.iter())
                        .all(|(value, bytes)| {
                            value.to_bits() == f64::from_ne_bytes(*bytes).to_bits()
                        });
                    if actual.returned_flags != status || !words_equal {
                        mismatches.push((jd, body, flags, path == &missing));
                    }
                    pairs += 1;
                }
            }
        }
    }
    for path in &paths {
        let output = Command::new(&oracle)
            .arg(path)
            .arg("sequence")
            .env_remove("SE_EPHE_PATH")
            .output()
            .unwrap();
        assert!(output.status.success());
        let (records, remainder) = output.stdout.as_chunks::<52>();
        assert!(remainder.is_empty());
        let mut records = records.iter();
        let mut builder = SessionBuilder::new();
        builder.ephe_path(path.to_str().unwrap());
        let session = builder.build().unwrap();
        for jd in [2451545.0, 1700000.5] {
            let inputs = [SUN, MOON, MARS]
                .into_iter()
                .flat_map(|body| [4, 260, 2308, 4356, 8452].map(|flags| (body, flags)))
                .chain([0, 4, 260].map(|flags| (ECL_NUT, flags)));
            for (body, flags) in inputs {
                let expected = records.next().unwrap();
                let status = i32::from_ne_bytes(expected[..4].try_into().unwrap());
                let actual = session.calc_ut(jd, body, flags).unwrap();
                let words_equal = actual
                    .values
                    .iter()
                    .zip(expected[4..].as_chunks::<8>().0.iter())
                    .all(|(value, bytes)| value.to_bits() == f64::from_ne_bytes(*bytes).to_bits());
                if actual.returned_flags != status || !words_equal {
                    mismatches.push((jd, body, flags, path == &missing));
                }
                pairs += 1;
            }
        }
        assert!(records.next().is_none());
    }
    std::fs::remove_dir(&missing).unwrap();
    assert!(
        mismatches.is_empty(),
        "native implicit-source disagreement for input/status selectors: {mismatches:?}"
    );
    println!("implicit source: {pairs} complete C result pairs exact; no vectors retained");
}
