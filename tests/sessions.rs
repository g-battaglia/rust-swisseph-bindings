//! atomic configured-operation regression.
//!
//! Free `set_*` + dependent-computation pairs are two acquisitions of the
//! process-wide native lock, so a racy interleaving observes the wrong
//! configuration (the review's A/B probe). [`Session`] applies its owned
//! configuration and runs one dependent computation in a single
//! acquisition of the same lock.
//!
//! This file proves, against in-memory serial baselines only (no numeric
//! reference vectors are stored):
//!
//! 1. Interleaved topocentric sessions each reproduce their own serial
//!    baseline bitwise, never the rival configuration.
//! 2. Interleaved sidereal sessions each reproduce their own serial
//!    baseline bitwise.
//! 3. A session event search with its own observer agrees bitwise with
//!    the serial free call, and its documented global-observer side
//!    effect lands deterministically.
//! 4. Invalid builders and pre-lock rejections preserve global settings;
//!    dependent checks may reject after installing session settings, while
//!    preserving file history. [`close`] does not break later session calls.
//! 5. Mixed free + session work across threads shares the one lock
//!    without deadlocking; sequential reuse never trips the reentry
//!    guard (same-thread nesting is structurally impossible — no
//!    callbacks or guard objects exist — with a mutex-level backstop
//!    converting any future accidental nesting into an error).
//!
//! The tests in this file MUST NOT overlap: they drive process-global
//! state. A file-local serial mutex enforces that; it nests outside the
//! native lock (never the reverse), so no deadlock is possible. All
//! computations use the analytical Moshier model, so no data files are
//! needed.

use std::sync::{Arc, Barrier, Mutex, MutexGuard};
use std::thread;

use swisseph_bindings::{
    CALC_RISE, ErrorKind, FLG_MOSEPH, FLG_SIDEREAL, FLG_SPEED, FLG_TOPOCTR, MOON,
    SIDM_FAGAN_BRADLEY, SIDM_LAHIRI, SIDM_USER, SUN, SessionBuilder, calc_ut, close,
    get_ayanamsa_ut, rise_trans, set_sid_mode, set_topo,
};

/// Serializes the tests in this file. Held for the whole test body,
/// always outside the native lock, so no deadlock is possible.
static TEST_SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    TEST_SERIAL.lock().expect("session serial lock")
}

/// J2000.0 in Universal Time.
const J2000: f64 = 2451545.0;
/// Munich observer (east deg, north deg, meters).
const MUNICH: (f64, f64, f64) = (11.34, 48.14, 520.0);
/// Sydney observer (east deg, north deg, meters).
const SYDNEY: (f64, f64, f64) = (151.21, -33.87, 50.0);
/// Rome observer for event searches.
const ROME: (f64, f64, f64) = (12.50, 41.90, 50.0);

fn topo_flags() -> i32 {
    FLG_MOSEPH | FLG_SPEED | FLG_TOPOCTR
}

fn restore_defaults() {
    set_sid_mode(SIDM_FAGAN_BRADLEY, 0.0, 0.0).expect("restore sidereal default");
    set_topo(0.0, 0.0, 0.0).expect("restore observer default");
}

#[test]
fn session_topocentric_calc_matches_own_serial_baseline() {
    let _serial = serial();
    // Serial baselines through the free functions.
    set_topo(MUNICH.0, MUNICH.1, MUNICH.2).expect("munich observer");
    let base_a = calc_ut(J2000, MOON, topo_flags()).expect("munich moon");
    set_topo(SYDNEY.0, SYDNEY.1, SYDNEY.2).expect("sydney observer");
    let base_b = calc_ut(J2000, MOON, topo_flags()).expect("sydney moon");
    assert_ne!(base_a, base_b, "observers must differ");

    let mut builder_a = SessionBuilder::new();
    builder_a.topo(MUNICH.0, MUNICH.1, MUNICH.2);
    let session_a = builder_a.build().expect("valid session A");
    let mut builder_b = SessionBuilder::new();
    builder_b.topo(SYDNEY.0, SYDNEY.1, SYDNEY.2);
    let session_b = builder_b.build().expect("valid session B");

    // Interleave hard: every worker starts together and alternates tight
    // session calls, so free set_*/calc_* pairs would observe the rival
    // configuration here.
    let barrier = Arc::new(Barrier::new(4));
    let workers: Vec<_> = (0..4)
        .map(|id| {
            let barrier = Arc::clone(&barrier);
            let (session, baseline) = if id % 2 == 0 {
                (session_a.clone(), base_a.clone())
            } else {
                (session_b.clone(), base_b.clone())
            };
            thread::spawn(move || {
                barrier.wait();
                for _ in 0..50 {
                    let got = session
                        .calc_ut(J2000, MOON, topo_flags())
                        .expect("session calc");
                    assert_eq!(
                        got, baseline,
                        "worker {id} must see its own observer, never the rival's"
                    );
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("worker panicked");
    }
    restore_defaults();
}

#[test]
fn session_sidereal_ayanamsha_matches_own_serial_baseline() {
    let _serial = serial();
    set_sid_mode(SIDM_LAHIRI, 0.0, 0.0).expect("lahiri mode");
    let base_lahiri = get_ayanamsa_ut(J2000).expect("lahiri ayanamsha");
    let calc_lahiri =
        calc_ut(J2000, SUN, FLG_MOSEPH | FLG_SPEED | FLG_SIDEREAL).expect("lahiri sun");
    set_sid_mode(SIDM_FAGAN_BRADLEY, 0.0, 0.0).expect("fagan mode");
    let base_fagan = get_ayanamsa_ut(J2000).expect("fagan ayanamsha");
    let calc_fagan = calc_ut(J2000, SUN, FLG_MOSEPH | FLG_SPEED | FLG_SIDEREAL).expect("fagan sun");
    assert!((base_lahiri - base_fagan).abs() > 0.5, "modes must differ");
    assert_ne!(calc_lahiri, calc_fagan, "sidereal suns must differ");

    let mut builder_l = SessionBuilder::new();
    builder_l.sid_mode(SIDM_LAHIRI, 0.0, 0.0);
    let session_l = builder_l.build().expect("valid lahiri session");
    let mut builder_f = SessionBuilder::new();
    builder_f.sid_mode(SIDM_FAGAN_BRADLEY, 0.0, 0.0);
    let session_f = builder_f.build().expect("valid fagan session");

    let barrier = Arc::new(Barrier::new(4));
    let workers: Vec<_> = (0..4)
        .map(|id| {
            let barrier = Arc::clone(&barrier);
            let (session, ayan_base, calc_base) = if id % 2 == 0 {
                (session_l.clone(), base_lahiri, calc_lahiri.clone())
            } else {
                (session_f.clone(), base_fagan, calc_fagan.clone())
            };
            thread::spawn(move || {
                barrier.wait();
                for _ in 0..50 {
                    assert_eq!(
                        session.get_ayanamsa_ut(J2000).expect("session ayanamsha"),
                        ayan_base,
                        "worker {id} must see its own sidereal mode"
                    );
                    assert_eq!(
                        session
                            .calc_ut(J2000, SUN, FLG_MOSEPH | FLG_SPEED | FLG_SIDEREAL)
                            .expect("session sidereal sun"),
                        calc_base,
                        "worker {id} sidereal calc must see its own mode"
                    );
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("worker panicked");
    }
    restore_defaults();
}

#[test]
fn session_event_search_repeats_serial_and_side_effect_is_deterministic() {
    let _serial = serial();
    // Serial baseline: Rome sunrise through the free function.
    let serial_rome = rise_trans(
        2451544.5, SUN, None, FLG_MOSEPH, CALC_RISE, ROME.0, ROME.1, ROME.2, 1013.25, 15.0,
    )
    .expect("roman sunrise");

    // The session carries a *different* configured observer (Munich); the
    // search still runs with its own per-call observer (Rome).
    let mut builder = SessionBuilder::new();
    builder.topo(MUNICH.0, MUNICH.1, MUNICH.2);
    let session = builder.build().expect("valid session");
    let via_session = session
        .rise_trans(
            2451544.5, SUN, None, FLG_MOSEPH, CALC_RISE, ROME.0, ROME.1, ROME.2, 1013.25, 15.0,
        )
        .expect("session roman sunrise");
    assert_eq!(
        via_session, serial_rome,
        "session search must agree bitwise with the serial call"
    );

    // Deterministic side effect: after the session search the global
    // observer is the search's observer (Rome), exactly like the free
    // function. A following free topocentric calc with no intervening
    // set_topo therefore sees Rome.
    set_topo(SYDNEY.0, SYDNEY.1, SYDNEY.2).expect("sydney observer");
    session
        .rise_trans(
            2451544.5, SUN, None, FLG_MOSEPH, CALC_RISE, ROME.0, ROME.1, ROME.2, 1013.25, 15.0,
        )
        .expect("session search installs rome globally");
    let after_search = calc_ut(J2000, MOON, topo_flags()).expect("calc after search");
    set_topo(ROME.0, ROME.1, ROME.2).expect("rome observer");
    let expect_rome = calc_ut(J2000, MOON, topo_flags()).expect("roman moon");
    assert_eq!(
        after_search, expect_rome,
        "post-search global observer must be the search observer"
    );
    restore_defaults();
}

#[test]
fn session_builder_and_input_errors_preserve_state_and_close_recovers() {
    let _serial = serial();
    set_topo(MUNICH.0, MUNICH.1, MUNICH.2).expect("munich observer");
    let base = calc_ut(J2000, MOON, topo_flags()).expect("munich moon");

    // Invalid builders are rejected before any native call.
    let mut bad_topo = SessionBuilder::new();
    bad_topo.topo(f64::NAN, 41.9, 0.0);
    assert_eq!(
        bad_topo.build().expect_err("NaN topo").kind(),
        ErrorKind::InvalidInput
    );
    let mut bad_path = SessionBuilder::new();
    bad_path.ephe_path("bad\0path");
    assert_eq!(
        bad_path.build().expect_err("NUL path").kind(),
        ErrorKind::InvalidInput
    );
    let mut bad_lapse = SessionBuilder::new();
    bad_lapse.lapse_rate(f64::INFINITY);
    assert_eq!(
        bad_lapse.build().expect_err("infinite lapse").kind(),
        ErrorKind::InvalidInput
    );
    let mut bad_sid = SessionBuilder::new();
    bad_sid.sid_mode(SIDM_USER, f64::NAN, 0.0);
    assert_eq!(
        bad_sid.build().expect_err("NaN sidereal param").kind(),
        ErrorKind::InvalidInput
    );
    // Untouched native state still answers with the live configuration.
    assert_eq!(
        calc_ut(J2000, MOON, topo_flags()).expect("state preserved"),
        base,
        "rejected builders must not mutate native state"
    );

    // Invalid computation inputs are rejected pre-lock through sessions too.
    let session = SessionBuilder::new().build().expect("empty session");
    assert_eq!(
        session
            .calc_ut(f64::NAN, MOON, topo_flags())
            .expect_err("NaN jd")
            .kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        session
            .houses(f64::NAN, 48.14, 11.34, b'P')
            .expect_err("NaN houses jd")
            .kind(),
        ErrorKind::InvalidInput
    );

    // Explicit close drops caches; the next session call lazily
    // re-initializes and still observes its own configuration.
    let mut builder = SessionBuilder::new();
    builder.topo(MUNICH.0, MUNICH.1, MUNICH.2);
    let munich_session = builder.build().expect("munich session");
    close().expect("close");
    assert_eq!(
        munich_session
            .calc_ut(J2000, MOON, topo_flags())
            .expect("session calc after close"),
        base,
        "session must recover after close with its own config"
    );
    restore_defaults();
}

#[test]
fn session_dependent_rejection_keeps_applied_settings_but_preserves_history() {
    let _serial = serial();
    let mut builder = SessionBuilder::new();
    builder.topo(SYDNEY.0, SYDNEY.1, SYDNEY.2);
    let session = builder.build().expect("sydney session");
    let baseline = session
        .calc_ut(J2000, MOON, topo_flags())
        .expect("baseline");
    let files: Vec<_> = (0..=4)
        .map(|slot| session.get_current_file_data(slot).expect("snapshot"))
        .collect();
    set_topo(MUNICH.0, MUNICH.1, MUNICH.2).expect("competing observer");
    assert_ne!(
        calc_ut(J2000, MOON, topo_flags()).expect("munich"),
        baseline
    );
    assert_eq!(
        session
            .calc_ut(1e100, MOON, topo_flags())
            .expect_err("finite range")
            .kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        calc_ut(J2000, MOON, topo_flags()).expect("applied sydney"),
        baseline
    );
    for (slot, before) in files.into_iter().enumerate() {
        assert_eq!(
            session.get_current_file_data(slot as i32).expect("history"),
            before
        );
    }
    restore_defaults();
}

#[test]
fn session_and_free_calls_share_one_lock_without_hanging() {
    let _serial = serial();
    // Eight threads mixing free calls and session calls (each session
    // carries its own observer/mode) must all complete without
    // deadlocking. Assertions stay at success/finiteness plus one
    // per-thread self-consistency check; exact interleaving winners are
    // deliberately not asserted for the free pairs.
    let workers: Vec<_> = (0..8)
        .map(|id| {
            thread::spawn(move || {
                let lon = -180.0 + f64::from(id) * 45.0;
                let lat = 20.0 + f64::from(id) * 5.0;
                let mut builder = SessionBuilder::new();
                builder.topo(lon, lat, 100.0);
                builder.sid_mode(SIDM_LAHIRI, 0.0, 0.0);
                let session = builder.build().expect("worker session");
                for _ in 0..25 {
                    let positioned = session
                        .calc_ut(J2000, MOON, topo_flags())
                        .expect("worker session calc");
                    assert!(positioned.values.iter().all(|v| v.is_finite()));
                    // Same call twice in a row through the same session is
                    // bitwise stable (sequential reuse never trips the
                    // same-thread reentry guard).
                    assert_eq!(
                        session
                            .calc_ut(J2000, MOON, topo_flags())
                            .expect("worker session repeat"),
                        positioned,
                        "sequential session reuse must repeat bitwise"
                    );
                    let ayan = session.get_ayanamsa_ut(J2000).expect("worker ayanamsha");
                    assert!(ayan.is_finite());
                    let free = calc_ut(J2000, SUN, FLG_MOSEPH).expect("worker free calc");
                    assert!(free.values.iter().all(|v| v.is_finite()));
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("worker panicked");
    }
    restore_defaults();
}
