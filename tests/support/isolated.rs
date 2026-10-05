//! Process isolation for tests that need a controlled native environment.

/// Return true inside the selected child; the parent runs it without the
/// native path override and returns false. No shared-process environment
/// is mutated, so other test threads may safely call native `getenv`.
pub(crate) fn without_ephe_override(test: &str) -> bool {
    if std::env::var("SWISSEPH_ISOLATED_TEST").as_deref() == Ok(test) {
        return true;
    }
    let child = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env("SWISSEPH_ISOLATED_TEST", test)
        .env_remove("SE_EPHE_PATH")
        .output()
        .expect("run isolated native test");
    assert!(
        child.status.success(),
        "isolated {test} failed:\n{}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    false
}
