//! Complete native buffers compared with an independent C public-header caller.
//! Binary numerical responses remain in memory; assertions identify slots only.
use std::path::{Path, PathBuf};
use std::process::Command;
use swisseph_bindings::*;

/// Build the independent caller against the archive used by this Cargo target.
fn oracle() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Path::new(env!("OUT_DIR"));
    let identity = std::fs::read_to_string(out.join("native-inputs.txt")).unwrap();
    let compiler = identity
        .lines()
        .find_map(|s| s.strip_prefix("compiler: "))
        .unwrap();
    let bin = out.join("full-output-oracle");
    let status = Command::new(compiler)
        .arg("-O2")
        .arg("-I")
        .arg(root.join("swisseph"))
        .arg(root.join("tests/full_output_oracle.c"))
        .arg(out.join("libswisseph.a"))
        .arg("-lm")
        .arg("-o")
        .arg(&bin)
        .status()
        .unwrap();
    assert!(status.success());
    bin
}

/// Fingerprint an identified input, never a native output or residual.
fn fingerprint(path: &Path) -> String {
    let out = Command::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned()
}

/// Compare status, every binary64 slot and the complete native diagnostic.
fn compare(
    bin: &Path,
    data: &str,
    operation: &str,
    jd: f64,
    flags: i32,
    actual: Result<(i32, Vec<f64>, String), Error>,
) {
    let output = Command::new(bin)
        .args([data, operation, &jd.to_string(), &flags.to_string()])
        .env_remove("SE_EPHE_PATH")
        .output()
        .unwrap();
    assert!(output.status.success());
    let bytes = &output.stdout;
    let status = i32::from_ne_bytes(bytes[0..4].try_into().unwrap());
    let count = u32::from_ne_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let text_at = 8 + count * 8;
    let text_len = u32::from_ne_bytes(bytes[text_at..text_at + 4].try_into().unwrap()) as usize;
    assert_eq!(bytes.len(), text_at + 4 + text_len);
    let diagnostic = std::str::from_utf8(&bytes[text_at + 4..]).unwrap();
    if status < 0 {
        let error = actual.expect_err("native failure must stay an error");
        assert_eq!(error.kind(), ErrorKind::Native);
        assert!(error.message().contains(diagnostic));
        return;
    }
    let (actual_status, values, text) = actual.unwrap();
    assert_eq!(actual_status, status);
    assert_eq!(values.len(), count);
    assert_eq!(text, diagnostic);
    for (slot, value) in values.iter().enumerate() {
        assert!(
            value.is_finite(),
            "{operation} inputJD{jd} flags{flags} slot{slot} must be finite"
        );
        let at = 8 + slot * 8;
        let expected_bits = u64::from_ne_bytes(bytes[at..at + 8].try_into().unwrap());
        assert!(
            value.to_bits() == expected_bits,
            "{operation} inputJD{jd} flags{flags} slot{slot}"
        );
    }
}

#[test]
fn complete_geography_and_heliacal_buffers_match_native_c() {
    let Some(data) = std::env::var_os("SWISSEPH_EPHE_DIR") else {
        eprintln!("SKIP full native output data cases: SWISSEPH_EPHE_DIR is not selected");
        return;
    };
    let selected = PathBuf::from(data);
    let mut identities: Vec<_> = ["sepl_18.se1", "semo_18.se1", "seas_18.se1", "sefstars.txt"]
        .into_iter()
        .map(|name| {
            let p = selected.join(name);
            let h = fingerprint(&p);
            (p, h)
        })
        .collect();
    let mut data = selected.to_str().unwrap().to_owned();
    let mut sources = vec![FLG_SWIEPH, FLG_MOSEPH];
    let mut jpl_name = None;
    if let Some(file) = std::env::var_os("SWISSEPH_JPL_FILE") {
        let file = PathBuf::from(file).canonicalize().unwrap();
        let hash = fingerprint(&file);
        assert_eq!(
            hash,
            "29915576d0a6555766b99485ac3056ee415e86df4fce282611c31afb329ad062"
        );
        data.push(':');
        data.push_str(file.parent().unwrap().to_str().unwrap());
        jpl_name = Some(file.file_name().unwrap().to_str().unwrap().to_owned());
        identities.push((file, hash));
        sources.push(FLG_JPLEPH);
    }
    let bin = oracle();
    identities.push((
        Path::new(env!("OUT_DIR")).join("libswisseph.a"),
        fingerprint(&Path::new(env!("OUT_DIR")).join("libswisseph.a")),
    ));
    set_ephe_path(Some(&data)).unwrap();
    if let Some(name) = jpl_name {
        set_jpl_file(&name).unwrap();
    }
    let mut cases = 0;
    for flags in sources {
        for jd in [2451545.0, 2457987.28, 2460409.28] {
            let value = sol_eclipse_where_full(jd, flags).map(|v| {
                (
                    v.eclipse_type,
                    v.geopos.into_iter().chain(v.attributes).collect(),
                    v.diagnostic,
                )
            });
            compare(&bin, &data, "solar", jd, flags, value);
            let value = lun_occult_where_full(jd, SUN, None, flags).map(|v| {
                (
                    v.occult_type,
                    v.geopos.into_iter().chain(v.attributes).collect(),
                    v.diagnostic,
                )
            });
            compare(&bin, &data, "occult", jd, flags, value);
            cases += 2;
        }
    }
    for jd in [2451545.0, 2457987.28, 2460409.28] {
        let value = heliacal_pheno_ut_full(
            jd,
            0.0,
            45.0,
            0.0,
            [1013.25, 15.0, 40.0, 0.0],
            [36.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            "Venus",
            HELIACAL_RISING,
            0,
        )
        .map(|v| (v.status, v.values.into_iter().collect(), v.diagnostic));
        compare(&bin, &data, "heliacal", jd, 0, value);
        cases += 1;
    }
    assert!(sol_eclipse_where_full(f64::NAN, FLG_SWIEPH).is_err());
    assert!(lun_occult_where_full(f64::INFINITY, SUN, None, FLG_SWIEPH).is_err());
    eprintln!(
        "PASS full native output cases={cases}; complete30/50 slots, status and diagnostic; invalid clocks rejected"
    );
}
