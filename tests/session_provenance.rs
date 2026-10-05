//! Session metadata must describe its computation, survive other callers
//! and never reset native files. Comparisons are in memory only.

use swisseph_bindings::{
    CALC_RISE, Calendar, FLG_MOSEPH, FLG_SWIEPH, MARS, MOON, SUN, SessionBuilder, calc_ut, close,
    get_current_file_data, julday, require_source_flags,
};

#[path = "data_dir.rs"]
mod test_data;

#[test]
fn session_file_snapshot_is_atomic_owned_and_read_without_reconfiguration() {
    let Some(data) = test_data::ephemeris_dir() else {
        eprintln!("SKIP session provenance: select complete data with SWISSEPH_EPHE_DIR");
        return;
    };
    if !std::path::Path::new(&data).join("sepl_12.se1").is_file() {
        eprintln!(
            "SKIP session provenance: selected directory also needs sepl_12.se1 for year 1600"
        );
        return;
    }
    let session = SessionBuilder::new()
        .ephe_path(&data)
        .build()
        .expect("explicit path session");
    assert_eq!(
        session
            .get_current_file_data(0)
            .expect("before computation"),
        None
    );
    let old = julday(1600, 1, 1, 12.0, Calendar::Gregorian);
    let position = session
        .calc_ut(old, MARS, FLG_SWIEPH)
        .expect("Mars in 1600");
    require_source_flags(&position, FLG_SWIEPH).expect("must use file data");
    let native_old = get_current_file_data(0)
        .expect("native metadata")
        .expect("planet file");
    assert!(native_old.path.ends_with("sepl_12.se1"));
    assert!(native_old.tfstart <= old && old <= native_old.tfend);
    assert_eq!(
        session.get_current_file_data(0).expect("session metadata"),
        Some(native_old.clone())
    );
    assert_eq!(
        get_current_file_data(0).expect("query must not change native metadata"),
        Some(native_old.clone())
    );

    let modern = SessionBuilder::new()
        .ephe_path(&data)
        .build()
        .expect("independent session");
    let other = modern
        .calc(2451545.0, MARS, FLG_SWIEPH)
        .expect("modern Mars");
    require_source_flags(&other, FLG_SWIEPH).expect("modern file source");
    let modern_files = modern
        .get_current_file_data(0)
        .expect("modern provenance")
        .expect("modern planet file");
    assert!(modern_files.path.ends_with("sepl_18.se1"));
    assert_eq!(
        session
            .get_current_file_data(0)
            .expect("after other session"),
        Some(native_old.clone())
    );
    assert_eq!(
        get_current_file_data(0).expect("old query must preserve modern native state"),
        Some(modern_files.clone())
    );

    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                calc_ut(2451545.0, MOON, FLG_SWIEPH).expect("intervening free call");
                close().expect("intervening close");
            })
            .join()
            .expect("other caller");
    });
    assert_eq!(
        session.get_current_file_data(0).expect("after close"),
        Some(native_old.clone())
    );
    assert_eq!(
        modern
            .get_current_file_data(0)
            .expect("other history after close"),
        Some(modern_files)
    );
    assert!(session.calc_ut(f64::NAN, MARS, FLG_SWIEPH).is_err());
    assert!(session.calc(1e100, MARS, FLG_SWIEPH).is_err());
    assert!(session.calc_ut(1e100, MARS, FLG_SWIEPH).is_err());
    assert_eq!(
        session
            .get_current_file_data(0)
            .expect("after rejected input"),
        Some(native_old)
    );
    for slot in [-1, 5, i32::MAX] {
        assert_eq!(
            session.get_current_file_data(slot).expect("invalid slot"),
            None
        );
    }

    // Clones deliberately share computation history. Also cover snapshot
    // capture after an observer-taking operation, not just positions.
    let clone = session.clone();
    clone
        .rise_trans(
            2451545.0, SUN, None, FLG_SWIEPH, CALC_RISE, 12.5, 41.9, 0.0, 0.0, 10.0,
        )
        .expect("rise search");
    let after_rise = get_current_file_data(0).expect("native after rise");
    assert_eq!(
        session
            .get_current_file_data(0)
            .expect("shared clone history"),
        after_rise
    );
    // A native error also replaces the snapshot with the resulting native
    // state, rather than retaining an unrelated successful calculation.
    assert!(clone.calc(2451545.0, 999, FLG_MOSEPH).is_err());
    assert_eq!(
        session
            .get_current_file_data(0)
            .expect("after native error"),
        get_current_file_data(0).expect("native after error")
    );
}
