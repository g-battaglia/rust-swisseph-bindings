//! Resolve intentionally incomplete external data in child processes so
//! environment overrides cannot race with other integration tests.

use std::process::Command;

#[path = "data_dir.rs"]
mod test_data;

#[test]
fn selected_directory_child() {
    if std::env::var_os("SWISSEPH_REVIEW_DATA_CHILD").is_some() {
        assert!(test_data::ephemeris_dir().is_some());
    }
}

#[test]
fn explicit_empty_partial_and_zero_length_data_are_rejected() {
    let root = std::env::temp_dir().join(format!("swisseph-review-data-{}", std::process::id()));
    std::fs::create_dir(&root).expect("create empty test directory");
    for stage in 0..3 {
        if stage == 1 {
            // Availability-only sentinel, never passed to native Swiss.
            std::fs::write(root.join("sepl_18.se1"), b"availability sentinel")
                .expect("partial directory");
        } else if stage == 2 {
            for name in ["semo_18.se1", "seas_18.se1", "sefstars.txt"] {
                std::fs::write(root.join(name), b"").expect("empty required file");
            }
        }
        let child = Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", "selected_directory_child", "--nocapture"])
            .env("SWISSEPH_REVIEW_DATA_CHILD", "1")
            .env("SWISSEPH_EPHE_DIR", &root)
            .output()
            .expect("run isolated data resolver");
        assert!(
            !child.status.success(),
            "incomplete data must not enable file-based tests"
        );
        let diagnostic = String::from_utf8_lossy(&child.stderr);
        assert!(
            diagnostic.contains("required data files are missing or empty"),
            "{diagnostic}"
        );
        assert!(
            diagnostic.contains("semo_18.se1"),
            "missing input must be named: {diagnostic}"
        );
    }
    std::fs::remove_dir_all(root).expect("remove owned test directory");
}
