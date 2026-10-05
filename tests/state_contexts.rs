//! interleaved-contexts regression.
//!
//! Native configuration (data path, observer, sidereal mode, tidal
//! acceleration, user Delta-T override, lapse rate) is process-global, and
//! every public function serializes its own native sequence through one
//! process-wide lock. Most per-call observer/atmosphere blocks (`azalt`,
//! `azalt_rev`, `refrac`) truly leave global state alone — but the
//! rise/transit, heliacal and local-eclipse/occultation searches INSTALL
//! their per-call observer as the process-global observer (established by
//! probing the pinned build; see the side-effect notes on those
//! functions). They are therefore NOT configuration-independent.
//!
//! This file proves three things:
//!
//! 1. Sequential alternation is clean: switching a global context away and
//!    back reproduces the earlier result bitwise, and per-call searches
//!    repeat bitwise from their own observer argument.
//! 2. Concurrent mixed work (pure calls plus per-call observer work,
//!    including the topo-installing searches) shares the lock without
//!    deadlocking or corrupting state. The workers never READ the global
//!    observer, so their mutual topo clobbering is harmless to them.
//! 3. Concurrent global reconfiguration is serialized but *not* atomic:
//!    each native sequence is exclusive, while a `set_*` plus a later
//!    dependent computation remain two acquisitions. Callers that need
//!    atomic config-plus-computation must sequence those pairs themselves
//!    (one thread or an external mutex); the hammer section below pins the
//!    observable guarantee (no corruption, values always from a real
//!    configuration) without asserting which writer won.
//!
//! The two tests in this file MUST NOT overlap: the concurrent workers'
//! topo installs would otherwise land between the sequenced test's
//! `set_topo` and its dependent calculation (two lock acquisitions) and
//! break the bitwise restoration checks. A file-local serial mutex
//! enforces that; it nests outside the native lock (never the reverse),
//! so no deadlock is possible. No numeric reference vectors are stored:
//! assertions check bitwise reproducibility, expected difference, flag
//! provenance and error classification.

#[path = "support/isolated.rs"]
mod isolated;

use std::sync::{Mutex, MutexGuard};
use std::thread;

use swisseph_bindings::{
    CALC_RISE, Calendar, EQU2HOR, ErrorKind, FLG_MOSEPH, FLG_SPEED, FLG_SWIEPH, FLG_TOPOCTR,
    HELIACAL_RISING, MARS, MOON, SE_TIDAL_AUTOMATIC, SE_TIDAL_DE421, SE_TIDAL_DEFAULT,
    SIDM_FAGAN_BRADLEY, SIDM_LAHIRI, SUN, TRUE_TO_APP, azalt, calc_ut, close, deltat, fixstar_ut,
    get_ayanamsa_name, get_ayanamsa_ut, heliacal_ut, julday, refrac, rise_trans,
    set_delta_t_userdef, set_ephe_path, set_lapse_rate, set_sid_mode, set_tid_acc, set_topo,
    sol_eclipse_when_loc,
};

#[path = "data_dir.rs"]
mod test_data;

/// Serializes the two tests in this file. The concurrent workers install
/// a global observer on every rise/heliacal/local-eclipse call (probed
/// native behavior), which would otherwise land between the sequenced
/// test's `set_*` and its dependent computation and break bitwise
/// restoration. Held for the whole test body, always outside the native
/// lock, so no deadlock is possible.
static TEST_SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    TEST_SERIAL.lock().expect("state-contexts serial lock")
}

/// J2000.0 in Universal Time.
const J2000: f64 = 2451545.0;
/// Conventional standard-atmosphere lapse rate (K/m).
const LAPSE: f64 = 0.0065;
/// Munich observer (east deg, north deg, meters).
const MUNICH: (f64, f64, f64) = (11.34, 48.14, 520.0);
/// Sydney observer (east deg, north deg, meters).
const SYDNEY: (f64, f64, f64) = (151.21, -33.87, 50.0);
/// Rome observer for horizontal conversions.
const ROME: (f64, f64, f64) = (12.50, 41.90, 50.0);
/// Native atmosphere/observer defaults (zero entries select them).
const ATM: [f64; 4] = [0.0; 4];
const OBS: [f64; 6] = [0.0; 6];

/// Sequenced global-state matrix: every step depends on process-global
/// native configuration, so order matters and nothing here may move into a
/// standalone test.
#[test]
fn stateful_interleaved_contexts() {
    if !isolated::without_ephe_override("stateful_interleaved_contexts") {
        return;
    }
    let _serial = serial();
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP stateful_interleaved_contexts: no ephemeris data found; set SWISSEPH_EPHE_DIR \
             to a directory containing the .se1 files"
        );
        return;
    };
    set_ephe_path(Some(&data_dir)).expect("point at shipped data");

    // --- 1. Sidereal modes alternate without sticking ---
    set_sid_mode(SIDM_LAHIRI, 0.0, 0.0).expect("lahiri mode");
    let lahiri_a = get_ayanamsa_ut(J2000).expect("lahiri ayanamsha");
    set_sid_mode(SIDM_FAGAN_BRADLEY, 0.0, 0.0).expect("fagan mode");
    let fagan = get_ayanamsa_ut(J2000).expect("fagan ayanamsha");
    set_sid_mode(SIDM_LAHIRI, 0.0, 0.0).expect("lahiri again");
    let lahiri_b = get_ayanamsa_ut(J2000).expect("lahiri ayanamsha again");
    assert_eq!(lahiri_a, lahiri_b, "mode must restore bitwise");
    assert!(
        (lahiri_a - fagan).abs() > 0.5,
        "modes must differ: lahiri={lahiri_a} fagan={fagan}"
    );

    // --- 2. Topocentric observers alternate without sticking (Moshier,
    // file-free, so the path sections cannot disturb this result) ---
    set_topo(MUNICH.0, MUNICH.1, MUNICH.2).expect("munich observer");
    let moon_munich = calc_ut(J2000, MOON, FLG_MOSEPH | FLG_SPEED | FLG_TOPOCTR)
        .expect("topocentric moon from munich");
    set_topo(SYDNEY.0, SYDNEY.1, SYDNEY.2).expect("sydney observer");
    let moon_sydney = calc_ut(J2000, MOON, FLG_MOSEPH | FLG_SPEED | FLG_TOPOCTR)
        .expect("topocentric moon from sydney");
    set_topo(MUNICH.0, MUNICH.1, MUNICH.2).expect("munich again");
    let moon_munich_again = calc_ut(J2000, MOON, FLG_MOSEPH | FLG_SPEED | FLG_TOPOCTR)
        .expect("topocentric moon from munich again");
    assert_eq!(
        moon_munich, moon_munich_again,
        "observer must restore bitwise"
    );
    assert_ne!(moon_munich, moon_sydney, "observers must differ");

    // --- 3. Data-path alternation flips provenance visibly, then restores
    // bitwise (keep SE_EPHE_PATH out of the picture for determinism) ---
    let empty = std::env::temp_dir().join("swisseph-bindings-state-empty-ephe");
    std::fs::create_dir_all(&empty).expect("empty ephe dir");
    let mars_data = calc_ut(J2000, MARS, FLG_SWIEPH | FLG_SPEED).expect("mars via files");
    assert!(
        mars_data.returned_flags & FLG_SWIEPH != 0,
        "expected file-based source, got {:#X} ({})",
        mars_data.returned_flags,
        mars_data.diagnostic
    );
    set_ephe_path(Some(empty.to_str().expect("utf8 temp path"))).expect("empty data path");
    let mars_fallback = calc_ut(J2000, MARS, FLG_SWIEPH | FLG_SPEED).expect("fallback succeeds");
    assert!(
        mars_fallback.returned_flags & FLG_MOSEPH != 0
            && mars_fallback.returned_flags & FLG_SWIEPH == 0,
        "fallback must be reported as analytical, got {:#X}",
        mars_fallback.returned_flags
    );
    set_ephe_path(Some(&data_dir)).expect("restore shipped data");
    let mars_restored = calc_ut(J2000, MARS, FLG_SWIEPH | FLG_SPEED).expect("mars via files again");
    assert_eq!(mars_data, mars_restored, "path must restore bitwise");

    // --- 4. Tidal acceleration rescales pre-1955 Delta-T, then restores ---
    let jd_1900 = julday(1900, 1, 1, 12.0, Calendar::Gregorian);
    set_tid_acc(SE_TIDAL_AUTOMATIC).expect("automatic tidal mode");
    let dt_auto = deltat(jd_1900).expect("automatic delta-t");
    set_tid_acc(SE_TIDAL_DE421).expect("DE421 tidal value");
    let dt_de421 = deltat(jd_1900).expect("DE421 delta-t");
    assert_eq!(
        deltat(jd_1900).expect("repeat read"),
        dt_de421,
        "explicit mode must repeat bitwise"
    );
    assert!(
        (dt_de421 - dt_auto).abs() > 1e-9,
        "tidal acceleration must rescale historical Delta-T: {dt_auto} vs {dt_de421}"
    );
    assert_eq!(SE_TIDAL_DE421, -25.85, "constant shape guard");
    set_tid_acc(SE_TIDAL_AUTOMATIC).expect("restore automatic mode");
    assert_eq!(
        deltat(jd_1900).expect("restored delta-t"),
        dt_auto,
        "automatic mode must restore bitwise"
    );

    // --- 5. User Delta-T override pins exactly, then clears bitwise ---
    let dt_before = deltat(J2000).expect("computed delta-t");
    set_delta_t_userdef(Some(65.0 / 86400.0)).expect("pin delta-t");
    assert_eq!(
        deltat(J2000).expect("pinned delta-t"),
        65.0 / 86400.0,
        "override must pin exactly"
    );
    set_delta_t_userdef(None).expect("clear override");
    assert_eq!(
        deltat(J2000).expect("cleared delta-t"),
        dt_before,
        "clearing must restore bitwise"
    );

    // --- 6. Lapse-rate override is accepted, finite-only, and does not
    // disturb unrelated computations; restore follows ---
    let sun_plain = calc_ut(J2000, SUN, FLG_MOSEPH | FLG_SPEED).expect("moshier sun");
    set_lapse_rate(0.0100).expect("lapse override must succeed");
    assert_eq!(
        calc_ut(J2000, SUN, FLG_MOSEPH | FLG_SPEED).expect("sun under lapse override"),
        sun_plain,
        "lapse override must not disturb positions"
    );
    let bad_lapse = set_lapse_rate(f64::NAN).expect_err("non-finite lapse");
    assert_eq!(bad_lapse.kind(), ErrorKind::InvalidInput);
    set_lapse_rate(LAPSE).expect("lapse restore must succeed");

    // --- 7. Per-call observers repeat bitwise from their own argument.
    // (They still install the global observer as a side effect — see the
    // function docs — which is why this file serializes its tests; the
    // repeat calls below agree because each one carries its observer.)
    // Horizontal conversion with an explicit observer.
    let rome_h = azalt(
        J2000, EQU2HOR, ROME.0, ROME.1, ROME.2, 1013.25, 15.0, 200.0, 60.0,
    )
    .expect("azalt rome");
    let sydney_h = azalt(
        J2000, EQU2HOR, SYDNEY.0, SYDNEY.1, SYDNEY.2, 1013.25, 15.0, 200.0, 60.0,
    )
    .expect("azalt sydney");
    assert_eq!(
        azalt(
            J2000, EQU2HOR, ROME.0, ROME.1, ROME.2, 1013.25, 15.0, 200.0, 60.0
        )
        .expect("azalt rome again"),
        rome_h,
        "per-call observer must repeat bitwise"
    );
    assert_ne!(rome_h, sydney_h, "observers must differ");
    // Rise search with an explicit observer (Moshier, file-free).
    let rise_rome = rise_trans(
        2451544.5, SUN, None, FLG_MOSEPH, CALC_RISE, ROME.0, ROME.1, ROME.2, 1013.25, 15.0,
    )
    .expect("roman sunrise");
    let rise_sydney = rise_trans(
        2451544.5, SUN, None, FLG_MOSEPH, CALC_RISE, SYDNEY.0, SYDNEY.1, SYDNEY.2, 1013.25, 15.0,
    )
    .expect("sydney sunrise");
    assert_eq!(
        rise_trans(
            2451544.5, SUN, None, FLG_MOSEPH, CALC_RISE, ROME.0, ROME.1, ROME.2, 1013.25, 15.0,
        )
        .expect("roman sunrise again"),
        rise_rome,
        "rise observer must repeat bitwise"
    );
    assert_ne!(rise_rome, rise_sydney, "rise observers must differ");
    // Heliacal search with an explicit observer (Moshier, file-free).
    let helio_rome = heliacal_ut(
        J2000,
        ROME.0,
        ROME.1,
        ROME.2,
        ATM,
        OBS,
        "Venus",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect("roman heliacal venus");
    let helio_sydney = heliacal_ut(
        J2000,
        SYDNEY.0,
        SYDNEY.1,
        SYDNEY.2,
        ATM,
        OBS,
        "Venus",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect("sydney heliacal venus");
    assert_eq!(
        heliacal_ut(
            J2000,
            ROME.0,
            ROME.1,
            ROME.2,
            ATM,
            OBS,
            "Venus",
            HELIACAL_RISING,
            FLG_MOSEPH,
        )
        .expect("roman heliacal venus again"),
        helio_rome,
        "heliacal observer must repeat bitwise"
    );
    assert_ne!(
        helio_rome.begin(),
        helio_sydney.begin(),
        "heliacal observers must differ"
    );
    // Local-eclipse search with an explicit observer (file-based).
    let dallas = sol_eclipse_when_loc(2460310.5, FLG_SWIEPH, -96.8, 32.8, 0.0, false)
        .expect("dallas local eclipse");
    let london = sol_eclipse_when_loc(2460310.5, FLG_SWIEPH, 0.0, 51.5, 0.0, false)
        .expect("london local eclipse");
    assert_eq!(
        sol_eclipse_when_loc(2460310.5, FLG_SWIEPH, -96.8, 32.8, 0.0, false)
            .expect("dallas local eclipse again"),
        dallas,
        "local-eclipse observer must repeat bitwise"
    );
    assert_ne!(
        dallas.maximum(),
        london.maximum(),
        "local-eclipse observers must differ"
    );

    // --- 8. Rejected inputs never reach native code and corrupt nothing ---
    let nul = set_ephe_path(Some("bad\0path")).expect_err("NUL path");
    assert_eq!(nul.kind(), ErrorKind::InvalidInput);
    let long = set_ephe_path(Some(&"a".repeat(243))).expect_err("overlong path");
    assert_eq!(long.kind(), ErrorKind::InvalidInput);
    let neg_mode = get_ayanamsa_name(-1).expect_err("negative sidereal mode");
    assert_eq!(neg_mode.kind(), ErrorKind::InvalidInput);
    let unknown = fixstar_ut("NoSuchStarXYZ", J2000, FLG_SWIEPH).expect_err("unknown star");
    assert_eq!(unknown.kind(), ErrorKind::Native);
    // The shared state still answers after the whole error battery.
    set_sid_mode(SIDM_LAHIRI, 0.0, 0.0).expect("lahiri mode");
    assert_eq!(
        get_ayanamsa_ut(J2000).expect("ayanamsha after errors"),
        lahiri_a,
        "errors must not disturb live configuration"
    );

    // --- 9. Explicit close drops caches; re-pointing recovers (// reset semantics hold mid-matrix) ---
    close().expect("close");
    set_ephe_path(Some(&data_dir)).expect("re-point after close");
    let mars_after_close = calc_ut(J2000, MARS, FLG_SWIEPH | FLG_SPEED).expect("works after close");
    assert_eq!(mars_after_close, mars_data, "close must recover bitwise");

    // --- 10. Concurrent global reconfiguration is serialized, not atomic.
    // Writers alternate the sidereal mode while readers sample it: every
    // sample must be a finite value from one of the two real modes (both
    // inside 20-30 deg at J2000), never torn state. Which writer won a
    // given race is deliberately not asserted. ---
    set_sid_mode(SIDM_LAHIRI, 0.0, 0.0).expect("lahiri baseline");
    let workers: Vec<_> = (0..4)
        .map(|id| {
            thread::spawn(move || {
                for round in 0..50 {
                    let mode = if (round + id) % 2 == 0 {
                        SIDM_LAHIRI
                    } else {
                        SIDM_FAGAN_BRADLEY
                    };
                    set_sid_mode(mode, 0.0, 0.0).expect("hammer set");
                    let value = get_ayanamsa_ut(J2000).expect("hammer read");
                    assert!(
                        value.is_finite() && (20.0..30.0).contains(&value),
                        "hammer sample out of both modes' band: {value}"
                    );
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("hammer worker panicked");
    }

    // --- Restore process defaults for tidiness ---
    set_sid_mode(SIDM_FAGAN_BRADLEY, 0.0, 0.0).expect("restore default sidereal mode");
    set_tid_acc(SE_TIDAL_AUTOMATIC).expect("restore automatic tidal mode");
    set_delta_t_userdef(None).expect("clear delta-t override");
    set_lapse_rate(LAPSE).expect("restore lapse rate");
    set_topo(0.0, 0.0, 0.0).expect("restore default observer");
    set_ephe_path(None).expect("restore default path");
    assert_eq!(SE_TIDAL_DEFAULT, -25.8, "constant shape guard");
}

#[test]
fn concurrent_config_independent_calls_share_the_lock() {
    let _serial = serial();
    // Eight threads issuing only calls that never READ the global
    // observer (pure time math plus per-call observer/atmosphere work
    // with a per-thread observer) must all succeed without deadlocking.
    // The rise/heliacal searches below do install their observer globally
    // (probed native behavior), but no worker observes the global
    // observer, so their mutual clobbering is harmless here; the
    // serialization above keeps them away from the sequenced test's
    // global-observer reads. Assertions stay at success/finiteness.
    let workers: Vec<_> = (0..8)
        .map(|id| {
            thread::spawn(move || {
                let lon = -180.0 + f64::from(id) * 45.0;
                let lat = 20.0 + f64::from(id) * 5.0;
                assert_eq!(
                    julday(2000, 1, 1, 12.0, Calendar::Gregorian),
                    J2000,
                    "pure call from worker {id}"
                );
                let horizontal = azalt(J2000, EQU2HOR, lon, lat, 100.0, 1013.25, 15.0, 200.0, 60.0)
                    .expect("worker azalt");
                assert!(horizontal.azimuth.is_finite());
                assert!(
                    refrac(10.0, 1013.25, 15.0, TRUE_TO_APP)
                        .expect("worker refrac")
                        .is_finite()
                );
                let rise = rise_trans(
                    2451544.5, SUN, None, FLG_MOSEPH, CALC_RISE, lon, lat, 100.0, 1013.25, 15.0,
                )
                .expect("worker sunrise");
                match rise {
                    swisseph_bindings::RiseTransitOutcome::Event { time_ut, .. } => {
                        assert!(time_ut.is_finite());
                    }
                    swisseph_bindings::RiseTransitOutcome::Circumpolar { .. } => {}
                }
                let helio = heliacal_ut(
                    J2000,
                    lon,
                    lat,
                    100.0,
                    ATM,
                    OBS,
                    "Venus",
                    HELIACAL_RISING,
                    FLG_MOSEPH,
                )
                .expect("worker heliacal");
                assert!(helio.begin().is_finite());
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("worker panicked");
    }
}
