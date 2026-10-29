//! Complete native buffers compared with an independent C public-header caller.
//! Binary numerical responses remain in memory; assertions identify slots only.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

static CONFIGURATION: Mutex<()> = Mutex::new(());
use swisseph_bindings::native::*;
use swisseph_bindings::native::{
    global::calc_ut, global::heliacal_ut, global::set_delta_t_userdef, global::set_ephe_path,
    global::vis_limit_mag, native::heliacal_pheno_ut_full, native::lun_occult_where_full,
    native::require_source_flags, native::set_jpl_file, native::sol_eclipse_where_full,
    native::version, native::vis_limit_mag_full,
};

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
    let _configuration = CONFIGURATION.lock().unwrap();
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

/// Read the complete initialized visibility buffer from the independent caller.
/// The source and data path are identical to the safe binding call.
fn visibility_oracle(
    bin: &Path,
    data: &str,
    flags: i32,
    jd: f64,
    object: &str,
) -> (i32, Vec<f64>, String) {
    let output = Command::new(bin)
        .args([data, "vislim", &jd.to_string(), &flags.to_string(), object])
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
    let values = bytes[8..text_at]
        .as_chunks::<8>()
        .0
        .iter()
        .map(|word| f64::from_ne_bytes(*word))
        .collect();
    let diagnostic = std::str::from_utf8(&bytes[text_at + 4..])
        .unwrap()
        .to_owned();
    (status, values, diagnostic)
}

#[test]
fn complete_visibility_buffer_matches_native_c() {
    let _configuration = CONFIGURATION.lock().unwrap();
    let data = std::env::var("SWISSEPH_EPHE_DIR").unwrap_or_default();
    let mut sources = vec![FLG_MOSEPH];
    if !data.is_empty() {
        for (name, size) in [("sepl_18.se1", 484061), ("semo_18.se1", 1304771)] {
            assert_eq!(
                std::fs::metadata(Path::new(&data).join(name))
                    .unwrap()
                    .len(),
                size,
                "identified visibility data file {name}"
            );
        }
        sources.push(FLG_SWIEPH);
    }
    set_ephe_path(Some(&data)).unwrap();
    set_delta_t_userdef(None).unwrap();
    let source = calc_ut(2451545.0, VENUS, FLG_MOSEPH).unwrap();
    require_source_flags(&source, FLG_MOSEPH).unwrap();
    let event = heliacal_ut(
        2451545.0,
        0.0,
        51.5,
        0.0,
        [0.0; 4],
        [0.0; 6],
        "Venus",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .unwrap();
    let bin = oracle();
    let mut cases = 0;
    for flags in sources {
        let source = calc_ut(2451545.0, VENUS, flags).unwrap();
        require_source_flags(&source, flags).unwrap();
        let event = heliacal_ut(
            2451545.0,
            0.0,
            51.5,
            0.0,
            [0.0; 4],
            [0.0; 6],
            "Venus",
            HELIACAL_RISING,
            flags,
        )
        .unwrap();
        for (label, jd, below) in [
            ("visible", event.optimum(), false),
            ("below-horizon", event.optimum() - 0.25, true),
        ] {
            let (expected_status, expected, diagnostic) =
                visibility_oracle(&bin, &data, flags, jd, "Venus");
            assert_eq!(expected.len(), 10);
            assert_eq!(expected_status == HELFLAG_BELOW_HORIZON, below);
            let actual =
                vis_limit_mag_full(jd, 0.0, 51.5, 0.0, [0.0; 4], [0.0; 6], "Venus", flags).unwrap();
            assert_eq!(actual.status, expected_status);
            assert_eq!(actual.diagnostic, diagnostic);
            assert_eq!(actual.values.len(), expected.len());
            for (slot, actual) in actual.values.iter().enumerate() {
                assert!(
                    actual.to_bits() == expected[slot].to_bits(),
                    "{label} slot{slot}"
                );
            }
            let (status, values, text) = match vis_limit_mag(
                jd, 0.0, 51.5, 0.0, [0.0; 4], [0.0; 6], "Venus", flags,
            )
            .unwrap()
            {
                VisibilityOutcome::Visible(value) => (value.status, value.values, value.diagnostic),
                VisibilityOutcome::BelowHorizon(value) => {
                    (HELFLAG_BELOW_HORIZON, value.values, value.diagnostic)
                }
            };
            assert_eq!(status, actual.status);
            assert_eq!(text, actual.diagnostic);
            assert_eq!(values.len(), 8);
            assert!(
                values
                    .iter()
                    .zip(&actual.values)
                    .all(|(a, b)| a.to_bits() == b.to_bits()),
                "{label} retained eight-slot projection"
            );
            eprintln!(
                "PASS visibility {label}: source_flags={flags}, status={}, full_slots=10, legacy_slots=8, direct_c_slots=10",
                actual.status
            );
            cases += 1;
        }
    }
    for object in ["Sun", "NoSuchBodyXYZ"] {
        let (status, values, diagnostic) =
            visibility_oracle(&bin, &data, FLG_MOSEPH, 2451545.0, object);
        assert_eq!(status, -1);
        assert!(values.is_empty());
        assert!(!diagnostic.is_empty());
        let error = vis_limit_mag_full(
            2451545.0, 0.0, 51.5, 0.0, [0.0; 4], [0.0; 6], object, FLG_MOSEPH,
        )
        .map_or_else(|error| error, |_| panic!("expected visibility error"));
        assert_eq!(error.kind(), ErrorKind::Native);
        assert!(error.message().contains(&diagnostic));
        let legacy = vis_limit_mag(
            2451545.0, 0.0, 51.5, 0.0, [0.0; 4], [0.0; 6], object, FLG_MOSEPH,
        )
        .map_or_else(|error| error, |_| panic!("expected visibility error"));
        assert_eq!(error.message(), legacy.message());
        cases += 1;
    }
    let session = SessionBuilder::new()
        .delta_t_override(0.001)
        .build()
        .unwrap();
    set_delta_t_userdef(Some(0.001)).unwrap();
    let expected = vis_limit_mag_full(
        event.optimum(),
        0.0,
        51.5,
        0.0,
        [0.0; 4],
        [0.0; 6],
        "Venus",
        FLG_MOSEPH,
    )
    .unwrap();
    set_delta_t_userdef(Some(0.002)).unwrap();
    let actual = session
        .vis_limit_mag_full(
            event.optimum(),
            0.0,
            51.5,
            0.0,
            [0.0; 4],
            [0.0; 6],
            "Venus",
            FLG_MOSEPH,
        )
        .unwrap();
    assert!(
        actual == expected,
        "owned session must apply its configuration"
    );
    set_delta_t_userdef(None).unwrap();
    for (jd, object) in [(f64::NAN, "Venus"), (2451545.0, "Ve\0nus")] {
        let error = vis_limit_mag_full(jd, 0.0, 51.5, 0.0, [0.0; 4], [0.0; 6], object, FLG_MOSEPH)
            .map_or_else(|error| error, |_| panic!("expected visibility error"));
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
        assert_eq!(
            session
                .vis_limit_mag_full(jd, 0.0, 51.5, 0.0, [0.0; 4], [0.0; 6], object, FLG_MOSEPH,)
                .map_or_else(|error| error, |_| panic!("expected visibility error"))
                .kind(),
            ErrorKind::InvalidInput
        );
    }
    set_delta_t_userdef(None).unwrap();
    let archive = Path::new(env!("OUT_DIR")).join("libswisseph.a");
    eprintln!(
        "PASS visibility cases={cases}; session configuration and invalid inputs; engine={} configured_data={} archive_sha256={}",
        version().unwrap(),
        !data.is_empty(),
        fingerprint(&archive)
    );
}

/// The closed primary route retains full native channels, not legacy subsets.
#[test]
fn primary_geography_heliacal_and_visibility_preserve_full_c_transports() {
    let _configuration = CONFIGURATION.lock().unwrap();
    let data = std::env::var("SWISSEPH_EPHE_DIR").expect("select identified primary output data");
    for name in ["sepl_18.se1", "semo_18.se1", "seas_18.se1", "sefstars.txt"] {
        assert_eq!(fingerprint(&Path::new(&data).join(name)).len(), 64);
    }
    let session = primary_session(SessionBuilder::new().ephe_path(&data)).unwrap();
    let bin = oracle();
    fn original(error: swisseph_bindings::Error) -> Error {
        use std::error::Error as _;
        assert_eq!(error.kind(), swisseph_bindings::ErrorKind::Calculation);
        assert!(error.native_status().is_some());
        error
            .source()
            .unwrap()
            .downcast_ref::<Error>()
            .expect("genuine native cause")
            .clone()
    }
    for flags in [FLG_MOSEPH, FLG_SWIEPH] {
        for jd in [2451545.0, 2457987.28, 2460409.28] {
            compare(
                &bin,
                &data,
                "solar",
                jd,
                flags,
                session
                    .sol_eclipse_where(jd, flags)
                    .map(|v| {
                        (
                            v.status,
                            v.geography.into_iter().chain(v.attributes).collect(),
                            v.diagnostic.unwrap(),
                        )
                    })
                    .map_err(original),
            );
            compare(
                &bin,
                &data,
                "occult",
                jd,
                flags,
                session
                    .lun_occult_where(jd, &swisseph_bindings::Target::Body(SUN), flags)
                    .map(|v| {
                        (
                            v.status,
                            v.geography.into_iter().chain(v.attributes).collect(),
                            v.diagnostic.unwrap(),
                        )
                    })
                    .map_err(original),
            );
            compare(
                &bin,
                &data,
                "heliacal-event",
                jd,
                flags,
                session
                    .heliacal_ut(
                        jd,
                        [0.0, 51.5, 0.0],
                        [0.0; 4],
                        [0.0; 6],
                        "Venus",
                        HELIACAL_RISING,
                        flags,
                    )
                    .map(|v| (v.status.unwrap(), v.values, v.diagnostic.unwrap()))
                    .map_err(original),
            );
            let (status, expected, text) = visibility_oracle(&bin, &data, flags, jd, "Venus");
            let v = session
                .vis_limit_mag(jd, [0.0, 51.5, 0.0], [0.0; 4], [0.0; 6], "Venus", flags)
                .unwrap();
            assert_eq!(v.status, Some(status));
            assert_eq!(v.scalar, None);
            assert_eq!(v.diagnostic.as_deref(), Some(text.as_str()));
            assert!(
                v.values
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| a.to_bits() == b.to_bits())
            );
        }
    }
    for jd in [2451545.0, 2457987.28, 2460409.28] {
        compare(
            &bin,
            &data,
            "heliacal",
            jd,
            0,
            session
                .heliacal_pheno_ut(
                    jd,
                    [0.0, 45.0, 0.0],
                    [1013.25, 15.0, 40.0, 0.0],
                    [36.0, 1.0, 0.0, 0.0, 0.0, 0.0],
                    "Venus",
                    HELIACAL_RISING,
                    0,
                )
                .map(|v| {
                    (
                        v.status.unwrap(),
                        v.values.into_iter().collect(),
                        v.diagnostic.unwrap(),
                    )
                })
                .map_err(original),
        );
    }
}
