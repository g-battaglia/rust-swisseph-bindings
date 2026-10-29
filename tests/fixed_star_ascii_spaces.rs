//! Exhaustive byte-string control for the generated native catalog overlay.
use std::path::Path;
use std::process::Command;

#[test]
fn compaction_preserves_original_ascii_space_deletion() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Path::new(env!("OUT_DIR"));
    let inputs = std::fs::read_to_string(out.join("native-inputs.txt")).unwrap();
    let compiler = inputs
        .lines()
        .find_map(|line| line.strip_prefix("compiler: "))
        .unwrap();
    assert!(inputs.contains("native-overlay: fixed-star ASCII-space compaction"));
    let binary = out.join("fixed-star-ascii-spaces-control");
    assert!(
        Command::new(compiler)
            .arg("-O3")
            .arg("-I")
            .arg(root.join("src/native"))
            .arg(root.join("tests/fixed_star_ascii_spaces_oracle.c"))
            .arg("-o")
            .arg(&binary)
            .status()
            .unwrap()
            .success()
    );
    let result = Command::new(binary).output().unwrap();
    assert!(
        result.status.success(),
        "native ASCII compaction changed byte order or deletion"
    );
    let cases: usize = std::str::from_utf8(&result.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(cases > 100_000);
}
