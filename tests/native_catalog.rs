//! Selected catalogue listing compared with independent public C calls.
//! Numerical reference responses stay in process memory, never a fixture.
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use swisseph_bindings::{ErrorKind, SessionBuilder, fixed_star_catalog, fixstar2_mag};

#[path = "support/isolated.rs"]
mod isolated;

/// Read a length-prefixed native string without saving the binary response.
fn take_text<'a>(bytes: &'a [u8], at: &mut usize) -> &'a str {
    let n = u32::from_ne_bytes(bytes[*at..*at + 4].try_into().unwrap()) as usize;
    *at += 4;
    let text = std::str::from_utf8(&bytes[*at..*at + n]).unwrap();
    *at += n;
    text
}
/// Build the public-header caller with this exact native archive/compiler.
fn oracle() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Path::new(env!("OUT_DIR"));
    let identity = std::fs::read_to_string(out.join("native-inputs.txt")).unwrap();
    let compiler = identity
        .lines()
        .find_map(|s| s.strip_prefix("compiler: "))
        .unwrap();
    let bin = out.join("catalog-oracle");
    assert!(
        Command::new(compiler)
            .args(["-O2", "-I"])
            .arg(root.join("swisseph"))
            .arg(root.join("tests/catalog_oracle.c"))
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
fn selected_catalog_matches_numeric_c_order_and_end() {
    if !isolated::without_ephe_override("selected_catalog_matches_numeric_c_order_and_end") {
        return;
    }
    let data = std::env::var("SWISSEPH_EPHE_DIR").expect("select identified catalog data");
    let path = Path::new(&data).join("sefstars.txt");
    let before = std::fs::read(&path).unwrap();
    let mut b = SessionBuilder::new();
    b.ephe_path(&data);
    let session = b.build().unwrap();
    let actual = session.fixed_star_catalog().unwrap();
    let output = Command::new(oracle())
        .arg(&data)
        .env_remove("SE_EPHE_PATH")
        .output()
        .unwrap();
    assert!(output.status.success());
    let bytes = &output.stdout;
    let mut at = 4;
    assert!(i32::from_ne_bytes(bytes[..4].try_into().unwrap()) >= 0);
    take_text(bytes, &mut at);
    let mut index = 0;
    loop {
        let status = i32::from_ne_bytes(bytes[at..at + 4].try_into().unwrap());
        at += 4;
        let name = take_text(bytes, &mut at);
        let diagnostic = take_text(bytes, &mut at);
        if status < 0 {
            assert!(diagnostic.contains(&format!(
                "sequential fixed star number {} is not available",
                index + 1
            )));
            break;
        }
        let bits = u64::from_ne_bytes(bytes[at..at + 8].try_into().unwrap());
        at += 8;
        let entry = &actual[index];
        assert_eq!(entry.resolved_name, name);
        assert_eq!(entry.diagnostic, diagnostic);
        assert!(
            entry.magnitude.to_bits() == bits,
            "numeric catalog slot{}",
            index + 1
        );
        index += 1;
    }
    assert_eq!(index, actual.len());
    assert_eq!(at, bytes.len());
    assert!(!actual.is_empty());
    let free = fixed_star_catalog().unwrap();
    assert!(actual == free);
    // Repeated native numeric queries retain the actual name/magnitude; no dedup.
    for n in [1, index, index / 2] {
        let one = fixstar2_mag(&n.to_string()).unwrap();
        let two = fixstar2_mag(&n.to_string()).unwrap();
        assert!(one == two && one == actual[n - 1]);
    }
    assert!(
        std::fs::read(path).unwrap() == before,
        "catalog data changed during comparison"
    );
    println!("native catalog entries{index}; exact names/order/diagnostics/magnitude bits");
}
#[test]
fn unavailable_or_empty_catalog_is_an_error_not_empty_success() {
    if !isolated::without_ephe_override(
        "unavailable_or_empty_catalog_is_an_error_not_empty_success",
    ) {
        return;
    }
    let dir = std::env::temp_dir().join(format!("native-catalog-missing-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut b = SessionBuilder::new();
    b.ephe_path(dir.to_str().unwrap());
    let session = b.build().unwrap();
    for empty in [false, true] {
        if empty {
            std::fs::write(dir.join("sefstars.txt"), b"").unwrap();
        }
        let error = match session.fixed_star_catalog() {
            Err(error) => error,
            Ok(_) => panic!("missing/empty unexpectedly returned a catalogue"),
        };
        assert_eq!(error.kind(), ErrorKind::Native);
        assert!(error.message().contains("catalog unavailable"));
    }
    std::fs::remove_dir_all(dir).unwrap();
}
