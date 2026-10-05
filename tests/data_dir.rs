//! Shared helper for integration tests: locate external ephemeris data.
//!
//! File-based tests must not assume a bundled data directory. The data
//! directory is resolved in order:
//!
//! 1. The `SWISSEPH_EPHE_DIR` environment variable, when set. A set but
//!    missing, incomplete or non-UTF-8 directory is an explicit
//!    misconfiguration and panics with an actionable message.
//! 2. The checkout's `swisseph/ephe` directory, when present (normal local
//!    development with the submodule initialized).
//!
//! The file-based suites use modern planet/Moon/asteroid data and the
//! fixed-star catalog. All four files must exist and be nonempty. This is
//! an availability check; callers must still check actual source flags
//! when a native operation can fall back.
//!
//! When neither has the required files the caller skips its assertions with an
//! explicit `SKIP` notice instead of failing or silently falling back to
//! lower precision. Data-independent tests never consult this helper.
//!
//! Each integration test binary is a separate crate, so consumers include
//! this file with `#[path = "data_dir.rs"] mod test_data;`.

/// Resolve the ephemeris data directory, or `None` when no data is
/// available and the caller must skip its file-based assertions.
pub fn ephemeris_dir() -> Option<String> {
    if let Some(dir) = std::env::var_os("SWISSEPH_EPHE_DIR") {
        let path = std::path::PathBuf::from(&dir);
        assert!(
            path.is_dir(),
            "SWISSEPH_EPHE_DIR points at {dir:?}, which is not a directory"
        );
        let text = path
            .to_str()
            .unwrap_or_else(|| panic!("SWISSEPH_EPHE_DIR path {dir:?} is not valid UTF-8"));
        let missing = missing_files(&path);
        assert!(
            missing.is_empty(),
            "SWISSEPH_EPHE_DIR points at {dir:?}, but required data files are missing or empty: {}; select a complete ephemeris directory",
            missing.join(", ")
        );
        return Some(text.to_owned());
    }
    let candidate = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("swisseph/ephe");
    let missing = missing_files(&candidate);
    if candidate.is_dir() && missing.is_empty() {
        candidate.to_str().map(str::to_owned)
    } else {
        eprintln!(
            "SKIP ephemeris data at {}: required files missing or empty: {}",
            candidate.display(),
            missing.join(", ")
        );
        None
    }
}

/// Required inputs for the shared modern-date file-based test suites.
/// Metadata checks do not read or preserve native numerical fixtures.
fn missing_files(path: &std::path::Path) -> Vec<&'static str> {
    ["sepl_18.se1", "semo_18.se1", "seas_18.se1", "sefstars.txt"]
        .into_iter()
        .filter(|name| {
            !path
                .join(name)
                .metadata()
                .is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
        })
        .collect()
}
