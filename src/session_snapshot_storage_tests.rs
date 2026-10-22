//! Warm snapshot storage reuses capacity while every native read stays fresh.

use super::*;

#[path = "../tests/support/isolated.rs"]
mod isolated;
#[path = "../tests/data_dir.rs"]
mod test_data;

fn snapshot_storage(session: &Session) -> [Option<(usize, usize)>; 5] {
    with_native_access(|| {
        let files = session
            .file_data
            .lock()
            .map_err(|_| Error::lock_poisoned())?;
        Ok(std::array::from_fn(|slot| {
            files[slot]
                .as_ref()
                .map(|file| (file.path.as_ptr() as usize, file.path.capacity()))
        }))
    })
    .unwrap()
}

#[test]
fn warm_file_snapshots_reuse_owned_path_storage() {
    if !isolated::without_ephe_override(
        "session::snapshot_storage_tests::warm_file_snapshots_reuse_owned_path_storage",
    ) {
        return;
    }
    let data = test_data::ephemeris_dir().expect("complete external ephemeris data");
    let session = SessionBuilder::new().ephe_path(&data).build().unwrap();
    let flags = crate::FLG_SWIEPH | crate::FLG_SPEED;
    let first = session.calc_ut(2451545.0, crate::MARS, flags).unwrap();
    assert_ne!(
        first.returned_flags & crate::FLG_SWIEPH,
        0,
        "file source required"
    );
    let report = session
        .get_current_file_data(0)
        .unwrap()
        .expect("opened planet file");
    assert!(!report.path.is_empty());
    let before = snapshot_storage(&session);
    assert!(before[0].is_some());
    let cloned = session.clone();
    let next = cloned.calc_ut(2451546.0, crate::MARS, flags).unwrap();
    assert_ne!(
        next.returned_flags & crate::FLG_SWIEPH,
        0,
        "file source required"
    );
    assert_eq!(
        snapshot_storage(&session),
        before,
        "unchanged path must reuse its owned buffer"
    );
    assert_eq!(
        session.get_current_file_data(0).unwrap(),
        Some(report.clone())
    );
    assert_eq!(cloned.get_current_file_data(0).unwrap(), Some(report));
}
