//! Configured UT nodes/apsides match direct C and preserve the existing path lease.
//! All numerical results stay in process memory or the direct caller's pipe.
use std::path::{Path, PathBuf};
use std::process::Command;
use swisseph_bindings::*;

struct Oracle(PathBuf);
struct Expected {
    status: i32,
    values: [f64; 24],
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
        let bin = out.join("session-nodes-oracle");
        assert!(
            Command::new(compiler)
                .args(["-O2", "-I"])
                .arg(root.join("swisseph"))
                .arg(root.join("tests/session_nodes_oracle.c"))
                .arg(out.join("libswisseph.a"))
                .args(["-lm", "-o"])
                .arg(&bin)
                .status()
                .unwrap()
                .success()
        );
        Self(bin)
    }
    fn read(
        &self,
        path: &Path,
        day: f64,
        body: i32,
        flags: i32,
        method: i32,
        profile: i32,
    ) -> Expected {
        let output = Command::new(&self.0)
            .arg(path)
            .args([
                day.to_string(),
                body.to_string(),
                flags.to_string(),
                method.to_string(),
                profile.to_string(),
            ])
            .env_remove("SE_EPHE_PATH")
            .output()
            .unwrap();
        assert!(output.status.success(), "direct C caller failed");
        let bytes = output.stdout;
        assert!(bytes.len() >= 8 + 24 * 8);
        let status = i32::from_ne_bytes(bytes[..4].try_into().unwrap());
        let length = u32::from_ne_bytes(bytes[4..8].try_into().unwrap()) as usize;
        assert_eq!(bytes.len(), 8 + length + 24 * 8);
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
fn exact(actual: &NodesApsides, expected: &Expected) {
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
        [
            actual.ascending,
            actual.descending,
            actual.perihelion,
            actual.aphelion
        ]
        .iter()
        .flatten()
        .zip(expected.values)
        .all(|(a, b)| a.to_bits() == b.to_bits()),
        "complete native nodes/apsides output differs"
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

fn session(path: &Path, profile: i32) -> Session {
    SessionBuilder::new()
        .ephe_path(path.to_str().unwrap())
        .sid_mode(
            if profile & 1 != 0 {
                SIDM_LAHIRI
            } else {
                SIDM_FAGAN_BRADLEY
            },
            0.0,
            0.0,
        )
        .topo(
            if profile & 2 != 0 { 2.35 } else { 0.0 },
            if profile & 2 != 0 { 48.86 } else { 0.0 },
            if profile & 2 != 0 { 35.0 } else { 0.0 },
        )
        .build()
        .unwrap()
}

#[test]
fn configured_ut_nodes_match_c_and_reuse_or_invalidate_the_selected_path() {
    assert!(
        std::env::var_os("SE_EPHE_PATH").is_none(),
        "run with SE_EPHE_PATH unset"
    );
    let data = data();
    let oracle = Oracle::compile();
    let selected = session(&data, 0);
    let mut pairs = 0;
    for body in [MERCURY, MARS, MOON] {
        for method in [NODBIT_MEAN, NODBIT_OSCU] {
            for flags in [FLG_SWIEPH | FLG_SPEED, FLG_MOSEPH | FLG_SPEED] {
                let expected = oracle.read(&data, 2451545.0, body, flags, method, 0);
                exact(
                    &selected
                        .nod_aps_ut(2451545.0, body, flags, method)
                        .expect("configured UT nodes"),
                    &expected,
                );
                pairs += 1;
            }
        }
    }
    for profile in [1, 2, 3] {
        let flags = FLG_SWIEPH
            | FLG_SPEED
            | if profile & 1 != 0 { FLG_SIDEREAL } else { 0 }
            | if profile & 2 != 0 { FLG_TOPOCTR } else { 0 };
        let configured = session(&data, profile);
        let expected = oracle.read(&data, 2451545.0, MARS, flags, NODBIT_OSCU, profile);
        exact(
            &configured
                .nod_aps_ut(2451545.0, MARS, flags, NODBIT_OSCU)
                .unwrap(),
            &expected,
        );
        pairs += 1;
        // A competing configuration cannot contaminate the next session call.
        let _ = selected
            .nod_aps_ut(2451545.0, MARS, FLG_SWIEPH | FLG_SPEED, NODBIT_OSCU)
            .unwrap();
        exact(
            &configured
                .nod_aps_ut(2451545.0, MARS, flags, NODBIT_OSCU)
                .unwrap(),
            &expected,
        );
    }
    let invalid = match selected.nod_aps_ut(f64::NAN, MERCURY, FLG_SWIEPH, NODBIT_MEAN) {
        Err(error) => error,
        Ok(_) => panic!("non-finite date must remain invalid"),
    };
    assert_eq!(invalid.kind(), ErrorKind::InvalidInput);
    assert!(
        invalid.message() == "nod_aps_ut: Julian Day must be finite",
        "finite-input diagnostic changed"
    );
    let expected_error = match nod_aps_ut(2451545.0, 999999, FLG_SWIEPH, NODBIT_OSCU) {
        Err(error) => error,
        Ok(_) => panic!("free unknown body must remain a native failure"),
    };
    let actual_error = match selected.nod_aps_ut(2451545.0, 999999, FLG_SWIEPH, NODBIT_OSCU) {
        Err(error) => error,
        Ok(_) => panic!("unknown body must remain a native failure"),
    };
    assert!(
        actual_error == expected_error,
        "configured/free failure transport differs"
    );

    #[cfg(unix)]
    {
        let scratch =
            Scratch(std::env::temp_dir().join(format!("swisseph-ut-nodes-{}", std::process::id())));
        std::fs::create_dir(&scratch.0).unwrap();
        let root = scratch.0.canonicalize().unwrap();
        let a = root.join("selected");
        let b = root.join("other");
        std::fs::create_dir(&a).unwrap();
        std::fs::create_dir(&b).unwrap();
        for name in ["sepl_18.se1", "semo_18.se1"] {
            std::os::unix::fs::symlink(data.join(name).canonicalize().unwrap(), a.join(name))
                .unwrap();
        }
        let selected = session(&a, 0);
        let rival = session(&b, 0);
        let flags = FLG_SWIEPH | FLG_SPEED;
        let expected = oracle.read(&a, 2451546.0, MARS, flags, NODBIT_OSCU, 0);
        exact(
            &selected
                .nod_aps_ut(2451546.0, MARS, flags, NODBIT_OSCU)
                .unwrap(),
            &expected,
        );
        let saved = get_current_file_data(0)
            .unwrap()
            .expect("selected planetary file opened");
        let link = a.join("sepl_18.se1");
        let source = data.join("sepl_18.se1").canonicalize().unwrap();
        std::fs::remove_file(&link).unwrap();
        exact(
            &selected
                .nod_aps_ut(2451546.0, MARS, flags, NODBIT_OSCU)
                .unwrap(),
            &expected,
        );
        assert!(
            get_current_file_data(0).unwrap() == Some(saved.clone()),
            "unchanged session must retain the open planetary file"
        );
        let missing = oracle.read(&a, 2451546.0, MARS, flags, NODBIT_OSCU, 0);
        set_ephe_path(Some(a.to_str().unwrap())).unwrap();
        exact(
            &selected
                .nod_aps_ut(2451546.0, MARS, flags, NODBIT_OSCU)
                .unwrap(),
            &missing,
        );
        assert!(
            get_current_file_data(0).unwrap() != Some(saved.clone()),
            "public equal-path setter must still force reload"
        );
        exact(
            &rival
                .nod_aps_ut(2451546.0, MARS, flags, NODBIT_OSCU)
                .unwrap(),
            &oracle.read(&b, 2451546.0, MARS, flags, NODBIT_OSCU, 0),
        );
        std::os::unix::fs::symlink(&source, &link).unwrap();
        exact(
            &selected
                .nod_aps_ut(2451546.0, MARS, flags, NODBIT_OSCU)
                .unwrap(),
            &expected,
        );
        assert!(
            get_current_file_data(0).unwrap() == Some(saved.clone()),
            "changed path must restore its own file"
        );
        std::fs::remove_file(&link).unwrap();
        close().unwrap();
        exact(
            &selected
                .nod_aps_ut(2451546.0, MARS, flags, NODBIT_OSCU)
                .unwrap(),
            &missing,
        );
        assert!(
            selected.get_current_file_data(0).unwrap() != Some(saved),
            "close must invalidate the retained open file"
        );
    }
    eprintln!(
        "PASS configured UT nodes: {pairs} direct-C pairs; full24/status/diagnostic, sidereal/topocentric restoration, same-path reuse, forced reload, rival path, close and errors preserved"
    );
}
