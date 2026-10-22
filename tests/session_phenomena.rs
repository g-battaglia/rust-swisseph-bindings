//! Configured UT phenomena match direct C and preserve the existing path lease.
//! All numerical results stay in process memory or the direct caller's pipe.
use std::path::{Path, PathBuf};
use std::process::Command;
use swisseph_bindings::*;

struct Oracle(PathBuf);
struct Expected {
    status: i32,
    values: [f64; 20],
    diagnostic: String,
}
impl Oracle {
    fn compile() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let out = Path::new(env!("OUT_DIR"));
        let identity = std::fs::read_to_string(out.join("native-inputs.txt")).unwrap();
        let compiler = identity
            .lines()
            .find_map(|line| line.strip_prefix("compiler: "))
            .unwrap();
        let bin = out.join("session-phenomena-oracle");
        assert!(
            Command::new(compiler)
                .args(["-O2", "-I"])
                .arg(root.join("swisseph"))
                .arg(root.join("tests/session_phenomena_oracle.c"))
                .arg(out.join("libswisseph.a"))
                .args(["-lm", "-o"])
                .arg(&bin)
                .status()
                .unwrap()
                .success()
        );
        Self(bin)
    }
    fn read(&self, path: &Path, day: f64, body: i32, flags: i32) -> Expected {
        let output = Command::new(&self.0)
            .arg(path)
            .args([day.to_string(), body.to_string(), flags.to_string()])
            .env_remove("SE_EPHE_PATH")
            .output()
            .unwrap();
        assert!(output.status.success(), "direct C caller failed");
        let bytes = output.stdout;
        assert!(bytes.len() >= 8 + 20 * 8);
        let status = i32::from_ne_bytes(bytes[..4].try_into().unwrap());
        let length = u32::from_ne_bytes(bytes[4..8].try_into().unwrap()) as usize;
        assert_eq!(bytes.len(), 8 + length + 20 * 8);
        let diagnostic = String::from_utf8(bytes[8..8 + length].to_vec()).unwrap();
        let values = std::array::from_fn(|slot| {
            let start = 8 + length + slot * 8;
            f64::from_ne_bytes(bytes[start..start + 8].try_into().unwrap())
        });
        Expected {
            status,
            values,
            diagnostic,
        }
    }
}
fn exact(actual: &Phenomena, expected: &Expected) {
    assert!(expected.status >= 0, "C reference must serve this request");
    assert!(
        actual.returned_flags == expected.status,
        "native status/flags differ"
    );
    assert!(
        actual.diagnostic == expected.diagnostic,
        "native diagnostic differs"
    );
    assert!(
        actual
            .values
            .iter()
            .zip(expected.values)
            .all(|(a, b)| a.to_bits() == b.to_bits()),
        "complete native phenomena output differs"
    );
}
fn data() -> PathBuf {
    let path = PathBuf::from(
        std::env::var_os("SWISSEPH_EPHE_DIR").expect("explicit external ephemeris directory"),
    );
    for (name, size) in [("sepl_18.se1", 484061), ("semo_18.se1", 1304771)] {
        assert_eq!(
            std::fs::metadata(path.join(name))
                .expect("required identified data")
                .len(),
            size,
            "required filename/size admission"
        );
    }
    path
}
#[cfg(unix)]
struct Scratch(PathBuf);
#[cfg(unix)]
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = close();
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn configured_ut_phenomena_match_c_and_reuse_or_invalidate_the_selected_path() {
    assert!(
        std::env::var_os("SE_EPHE_PATH").is_none(),
        "run with SE_EPHE_PATH unset"
    );
    let data = data();
    let oracle = Oracle::compile();
    let session = SessionBuilder::new()
        .ephe_path(data.to_str().unwrap())
        .build()
        .unwrap();
    let mut pairs = 0;
    for body in [MERCURY, VENUS, MOON] {
        for flags in [FLG_SWIEPH | FLG_SPEED, FLG_MOSEPH | FLG_SPEED] {
            let expected = oracle.read(&data, 2451545.0, body, flags);
            let actual = session
                .pheno_ut(2451545.0, body, flags)
                .expect("configured UT phenomena");
            exact(&actual, &expected);
            pairs += 1;
        }
    }
    let invalid = match session.pheno_ut(f64::NAN, MERCURY, FLG_SWIEPH) {
        Err(error) => error,
        Ok(_) => panic!("non-finite date must remain invalid"),
    };
    assert_eq!(invalid.kind(), ErrorKind::InvalidInput);
    let expected_error = match pheno_ut(2451545.0, 999999, FLG_SWIEPH) {
        Err(error) => error,
        Ok(_) => panic!("free unknown body must remain a native failure"),
    };
    let actual_error = match session.pheno_ut(2451545.0, 999999, FLG_SWIEPH) {
        Err(error) => error,
        Ok(_) => panic!("unknown body must remain a native failure"),
    };
    assert!(
        actual_error == expected_error,
        "configured/free failure transport differs"
    );

    #[cfg(unix)]
    {
        let scratch = Scratch(
            std::env::temp_dir().join(format!("swisseph-ut-phenomena-{}", std::process::id())),
        );
        std::fs::create_dir(&scratch.0).unwrap();
        let root = scratch.0.canonicalize().unwrap();
        let a = root.join("selected");
        let b = root.join("other");
        std::fs::create_dir(&a).unwrap();
        std::fs::create_dir(&b).unwrap();
        let link = a.join("sepl_18.se1");
        let source = data.join("sepl_18.se1").canonicalize().unwrap();
        std::os::unix::fs::symlink(&source, &link).unwrap();
        let selected = SessionBuilder::new()
            .ephe_path(a.to_str().unwrap())
            .build()
            .unwrap();
        let rival = SessionBuilder::new()
            .ephe_path(b.to_str().unwrap())
            .build()
            .unwrap();
        let flags = FLG_SWIEPH | FLG_SPEED;
        let expected = oracle.read(&a, 2451546.0, MERCURY, flags);
        exact(
            &selected.pheno_ut(2451546.0, MERCURY, flags).unwrap(),
            &expected,
        );
        let saved = get_current_file_data(0)
            .unwrap()
            .expect("selected planetary file opened");
        std::fs::remove_file(&link).unwrap();
        exact(
            &selected.pheno_ut(2451546.0, MERCURY, flags).unwrap(),
            &expected,
        );
        assert!(
            get_current_file_data(0).unwrap() == Some(saved.clone()),
            "unchanged session must retain the open planetary file"
        );
        let missing = oracle.read(&a, 2451546.0, MERCURY, flags);
        set_ephe_path(Some(a.to_str().unwrap())).unwrap();
        exact(
            &selected.pheno_ut(2451546.0, MERCURY, flags).unwrap(),
            &missing,
        );
        assert!(
            get_current_file_data(0).unwrap() != Some(saved.clone()),
            "public equal-path setter must still force reload"
        );
        exact(
            &rival.pheno_ut(2451546.0, MERCURY, flags).unwrap(),
            &oracle.read(&b, 2451546.0, MERCURY, flags),
        );
        std::os::unix::fs::symlink(&source, &link).unwrap();
        exact(
            &selected.pheno_ut(2451546.0, MERCURY, flags).unwrap(),
            &expected,
        );
        assert!(
            get_current_file_data(0).unwrap() == Some(saved),
            "changed path must restore its own file"
        );
        std::fs::remove_file(&link).unwrap();
        close().unwrap();
        exact(
            &selected.pheno_ut(2451546.0, MERCURY, flags).unwrap(),
            &missing,
        );
        assert!(
            selected.get_current_file_data(0).unwrap().is_none(),
            "close must invalidate the retained open file"
        );
    }
    eprintln!(
        "PASS configured UT phenomena: {pairs} direct-C pairs; same-path reuse, forced reload, rival path, close and errors preserved"
    );
}
