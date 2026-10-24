//! Warm snapshot storage reuses capacity while every native read stays fresh.

use super::*;
use crate::test_data;

#[path = "../tests/support/isolated.rs"]
mod isolated;

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

#[test]
fn copied_file_paths_reuse_capacity_and_remain_owned_snapshot_bytes() {
    if !isolated::without_ephe_override(
        "session::snapshot_storage_tests::copied_file_paths_reuse_capacity_and_remain_owned_snapshot_bytes",
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
        .expect("planet file");
    assert!(!report.path.is_empty());
    let mut path = None;
    session.get_current_file_path_into(0, &mut path).unwrap();
    assert_eq!(path.as_deref(), Some(report.path.as_str()));
    let output = path.as_mut().unwrap();
    output.reserve(32);
    let storage = (output.as_ptr(), output.capacity());
    // This mutates only the caller's owned copy, not the stored snapshot.
    output.push('!');
    assert_eq!(
        session.get_current_file_data(0).unwrap(),
        Some(report.clone())
    );
    let clone = session.clone();
    let second = clone.calc_ut(2451546.0, crate::MARS, flags).unwrap();
    assert_ne!(
        second.returned_flags & crate::FLG_SWIEPH,
        0,
        "file source required"
    );
    clone.get_current_file_path_into(0, &mut path).unwrap();
    let output = path.as_ref().unwrap();
    assert_eq!((output.as_ptr(), output.capacity()), storage);
    assert_eq!(output, &report.path);

    let moon = clone.calc_ut(2451546.0, crate::MOON, flags).unwrap();
    assert_ne!(
        moon.returned_flags & crate::FLG_SWIEPH,
        0,
        "file source required"
    );
    let moon_file = clone.get_current_file_data(1).unwrap().expect("Moon file");
    assert_ne!(
        moon_file.path, report.path,
        "different native slots required"
    );
    session.get_current_file_path_into(1, &mut path).unwrap();
    let output = path.as_ref().unwrap();
    assert_eq!((output.as_ptr(), output.capacity()), storage);
    assert_eq!(output, &moon_file.path);
    crate::close().unwrap();
    session.get_current_file_path_into(1, &mut path).unwrap();
    assert_eq!(path.as_deref(), Some(moon_file.path.as_str()));
}

#[test]
fn copied_file_paths_clear_empty_slots_and_preserve_target_on_invalid_slots() {
    if !isolated::without_ephe_override(
        "session::snapshot_storage_tests::copied_file_paths_clear_empty_slots_and_preserve_target_on_invalid_slots",
    ) {
        return;
    }
    let session = SessionBuilder::new().build().unwrap();
    let mut path = Some(String::from("retained path"));
    let storage = {
        let path = path.as_ref().unwrap();
        (path.as_ptr(), path.capacity())
    };
    for slot in [-1, 5, i32::MAX] {
        let error = session
            .get_current_file_path_into(slot, &mut path)
            .unwrap_err();
        assert_eq!(error.kind(), crate::ErrorKind::InvalidInput);
        let path = path.as_ref().unwrap();
        assert_eq!(path, "retained path");
        assert_eq!((path.as_ptr(), path.capacity()), storage);
        assert_eq!(session.get_current_file_data(slot).unwrap(), None);
    }
    for slot in 0..=4 {
        path = Some(String::from("retained path"));
        session.get_current_file_path_into(slot, &mut path).unwrap();
        assert_eq!(path, None);
    }
}

#[test]
fn copied_file_paths_preserve_target_on_reentry_and_poisoned_locks() {
    if !isolated::without_ephe_override(
        "session::snapshot_storage_tests::copied_file_paths_preserve_target_on_reentry_and_poisoned_locks",
    ) {
        return;
    }
    let session = SessionBuilder::new().build().unwrap();
    let mut path = Some(String::from("retained path"));
    let storage = {
        let path = path.as_ref().unwrap();
        (path.as_ptr(), path.capacity())
    };
    with_native_access(|| {
        let error = session
            .get_current_file_path_into(0, &mut path)
            .unwrap_err();
        assert_eq!(error.kind(), crate::ErrorKind::InvalidInput);
        Ok(())
    })
    .unwrap();
    assert_eq!(path.as_deref(), Some("retained path"));
    assert_eq!(
        (
            path.as_ref().unwrap().as_ptr(),
            path.as_ref().unwrap().capacity()
        ),
        storage
    );

    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _files = session.file_data.lock().unwrap();
        panic!("intentional metadata mutex poison");
    }));
    assert!(caught.is_err());
    for slot in [0, -1] {
        let error = session
            .get_current_file_path_into(slot, &mut path)
            .unwrap_err();
        assert_eq!(error.kind(), crate::ErrorKind::LockPoisoned);
        assert_eq!(path.as_deref(), Some("retained path"));
        assert_eq!(
            (
                path.as_ref().unwrap().as_ptr(),
                path.as_ref().unwrap().capacity()
            ),
            storage
        );
    }
    // Run only in this isolated child: a poisoned native mutex is permanent.
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: Result<(), Error> = with_native_access(|| {
            panic!("intentional native mutex poison");
        });
    }));
    assert!(caught.is_err());
    let clean = SessionBuilder::new().build().unwrap();
    let error = clean.get_current_file_path_into(0, &mut path).unwrap_err();
    assert_eq!(error.kind(), crate::ErrorKind::LockPoisoned);
    assert_eq!(path.as_deref(), Some("retained path"));
    assert_eq!(
        (
            path.as_ref().unwrap().as_ptr(),
            path.as_ref().unwrap().capacity()
        ),
        storage
    );
}

#[test]
fn owned_snapshot_projection_does_not_reenter_native_access() {
    if !isolated::without_ephe_override(
        "session::snapshot_storage_tests::owned_snapshot_projection_does_not_reenter_native_access",
    ) {
        return;
    }
    let session = SessionBuilder::new().build().unwrap();
    session.file_data.lock().unwrap()[0] = Some(crate::CurrentFileData {
        path: String::from("owned-snapshot.se1"),
        tfstart: 0.0,
        tfend: 1.0,
        denum: 0,
    });
    let mut target = Some(String::from("caller-owned-storage"));
    with_native_access(|| {
        session.copy_captured_file_path_into(0, &mut target)?;
        assert_eq!(target.as_deref(), Some("owned-snapshot.se1"));
        session.copy_captured_file_path_into(1, &mut target)?;
        assert!(target.is_none());
        target = Some(String::from("unchanged-on-error"));
        assert_eq!(
            session
                .copy_captured_file_path_into(5, &mut target)
                .unwrap_err()
                .kind(),
            crate::ErrorKind::InvalidInput,
        );
        assert_eq!(target.as_deref(), Some("unchanged-on-error"));
        Ok(())
    })
    .unwrap();
}
