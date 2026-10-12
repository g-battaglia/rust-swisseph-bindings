//! Native valid domains must survive Rust admission without losing real errors.
use std::path::{Path, PathBuf};
use std::process::Command;
use swisseph_bindings::*;

/// Build the public-header reference against this target's C archive.
fn oracle() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Path::new(env!("OUT_DIR"));
    let identity = std::fs::read_to_string(out.join("native-inputs.txt")).unwrap();
    let compiler = identity
        .lines()
        .find_map(|s| s.strip_prefix("compiler: "))
        .unwrap();
    let bin = out.join("admission-oracle");
    assert!(
        Command::new(compiler)
            .args(["-O2", "-I"])
            .arg(root.join("swisseph"))
            .arg(root.join("tests/admission_oracle.c"))
            .arg(out.join("libswisseph.a"))
            .args(["-lm", "-o"])
            .arg(&bin)
            .status()
            .unwrap()
            .success()
    );
    bin
}

/// Read one native result in RAM; no response is saved or printed.
fn reference(bin: &Path, op: &str, input: &str, flags: i32) -> (i32, Vec<u64>, String, String) {
    let out = Command::new(bin)
        .args([op, input, &flags.to_string()])
        .env_remove("SE_EPHE_PATH")
        .output()
        .unwrap();
    assert!(out.status.success());
    let bytes = out.stdout;
    let status = i32::from_ne_bytes(bytes[..4].try_into().unwrap());
    let count = u32::from_ne_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let values = bytes[8..8 + count * 8]
        .as_chunks::<8>()
        .0
        .iter()
        .map(|v| u64::from_ne_bytes(*v))
        .collect();
    let text_at = 8 + count * 8;
    let length = u32::from_ne_bytes(bytes[text_at..text_at + 4].try_into().unwrap()) as usize;
    let text = std::str::from_utf8(&bytes[text_at + 4..text_at + 4 + length])
        .unwrap()
        .to_owned();
    let at = text_at + 4 + length;
    let name_length = u32::from_ne_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    assert_eq!(bytes.len(), at + 4 + name_length);
    let name = std::str::from_utf8(&bytes[at + 4..]).unwrap().to_owned();
    (status, values, text, name)
}

#[test]
fn built_in_positions_and_fixed_instant_moon_geometry_match_c() {
    let bin = oracle();
    set_ephe_path(Some("/tmp/swiss-admission-missing-data")).unwrap();
    let mut cases = 0;
    for query in [
        "Spica",
        "spica-suffix",
        "Revati",
        ",zePsc",
        "Pushya",
        ",deCnc",
        "Mula",
        ",laSco",
        ",SgrA*",
        ",GP1958",
        ",GPol",
    ] {
        for flags in [FLG_SWIEPH, FLG_MOSEPH | FLG_SPEED] {
            for op in ["fixstar2", "fixstar2_ut"] {
                let (status, bits, diagnostic, name) = reference(&bin, op, query, flags);
                let value = if op == "fixstar2" {
                    fixstar2(query, 2451545.0, flags)
                } else {
                    fixstar2_ut(query, 2451545.0, flags)
                }
                .expect("valid native built-in entry refused");
                assert_eq!(value.returned_flags, status);
                assert!(
                    value.values.iter().map(|v| v.to_bits()).eq(bits),
                    "{op} {query} flags{flags} values differ"
                );
                assert_eq!(value.resolved_name, name);
                assert_eq!(value.diagnostic, diagnostic);
                cases += 1;
            }
        }
    }
    for jd in [2451545.0, 2460310.5] {
        for flags in [FLG_SWIEPH, FLG_MOSEPH] {
            let (status, bits, diagnostic, _) = reference(&bin, "where", &jd.to_string(), flags);
            let value =
                lun_occult_where_full(jd, MOON, None, flags).expect("finite C geometry refused");
            assert_eq!(value.occult_type, status);
            assert!(
                value
                    .geopos
                    .into_iter()
                    .chain(value.attributes)
                    .map(f64::to_bits)
                    .eq(bits),
                "where inputJD{jd} flags{flags} values differ"
            );
            assert_eq!(value.diagnostic, diagnostic);
            cases += 1;
        }
    }
    // The catalog itself and non-built-in stars still need the selected file.
    assert!(fixed_star_catalog().is_err());
    assert!(fixstar2("Sirius", 2451545.0, FLG_MOSEPH).is_err());
    println!("native admission: {cases} C pairs exact; absent catalog remains an error");
    assert_eq!(cases, 48);
}

#[test]
fn crossing_failure_retains_original_empty_diagnostic() {
    let bin = oracle();
    set_ephe_path(Some("/tmp/swiss-admission-missing-data")).unwrap();
    let (_, bits, diagnostic, _) = reference(&bin, "mooncross_ut", "90", FLG_SWIEPH | FLG_HELCTR);
    let error = mooncross_ut(90.0, 2460310.5, FLG_SWIEPH | FLG_HELCTR).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Native);
    assert!(
        error.native_diagnostic() == Some(diagnostic.as_str()),
        "actual empty C buffer lost"
    );
    assert_eq!(bits.len(), 1);
    assert_eq!(error.native_operation(), Some("mooncross_ut"));
    assert!(
        error.native_scalar_return().unwrap().to_bits() == bits[0],
        "original C sentinel differs"
    );
}
