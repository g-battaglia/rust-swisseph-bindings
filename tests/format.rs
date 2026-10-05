//! checks: centisecond helpers, formatted angle/time strings,
//! degree splitting, file-identity queries and the remaining constants.
//!
//! The pure helpers need no ephemeris files and no process-global
//! configuration, so they run as standalone tests. The file-identity
//! query depends on which data files the engine has opened, so it runs
//! inside the single sequenced `stateful_file_data` test together with
//! its path setup, file-based computation and restore steps. No numeric
//! reference vectors are stored here: assertions check probed native
//! contracts (ranges, layouts, error classification) without persisting
//! engine output beyond exact small integers and short format strings.

#[path = "support/isolated.rs"]
mod isolated;

use swisseph_bindings::{
    ACRONYCHAL_RISING, ACRONYCHAL_SETTING, ARMC, ASC, ASTNAMFILE, AUNIT_TO_KM, AUNIT_TO_LIGHTYEAR,
    AUNIT_TO_PARSEC, COASC1, COASC2, COMET_OFFSET, COSMICAL_SETTING, DE_NUMBER, EQUASC, ErrorKind,
    FICT_MAX, FICT_OFFSET_1, FICTFILE, FLG_CENTER_BODY, FLG_SWIEPH, MARS, MC, SPLIT_DEG_KEEP_DEG,
    SPLIT_DEG_KEEP_SIGN, SPLIT_DEG_NAKSHATRA, SPLIT_DEG_ROUND_DEG, SPLIT_DEG_ROUND_MIN,
    SPLIT_DEG_ROUND_SEC, SPLIT_DEG_ZODIACAL, VERTEX, calc_ut, cs2degstr, cs2lonlatstr, cs2timestr,
    csnorm, csroundsec, difcs2n, difcsn, get_current_file_data, set_ephe_path, split_deg,
};

#[path = "data_dir.rs"]
mod test_data;

#[test]
fn centisecond_normalization_ranges() {
    assert_eq!(csnorm(0), 0);
    assert_eq!(csnorm(360000), 360000);
    assert_eq!(csnorm(-360000), 129240000);
    assert_eq!(csnorm(129600000), 0);
    assert_eq!(csnorm(129960000), 360000);
    assert_eq!(csnorm(-1), 129599999);
}

#[test]
fn centisecond_distance_ranges() {
    // [0, 360°) for `difcsn`, ±180° for `difcs2n`: the pair below
    // separates the two contracts (values in centiseconds).
    assert_eq!(difcsn(720000, 360000).expect("difcsn"), 360000);
    assert_eq!(difcsn(360000, 720000).expect("difcsn wrap"), 129240000);
    assert_eq!(difcsn(0, 0).expect("difcsn zero"), 0);
    assert_eq!(difcs2n(360000, 720000).expect("difcs2n"), -360000);
    assert_eq!(difcs2n(129240000, 360000).expect("difcs2n"), -720000);
    // An exact half-turn reports −180° (the native convention).
    assert_eq!(difcs2n(64800000, 0).expect("half turn"), -64800000);
    assert_eq!(difcs2n(0, 64800000).expect("half turn"), -64800000);
}

#[test]
fn csroundsec_rounds_to_whole_arcseconds() {
    assert_eq!(csroundsec(150).expect("round"), 200);
    assert_eq!(csroundsec(149).expect("round"), 100);
    assert_eq!(csroundsec(50).expect("round"), 100);
    assert_eq!(csroundsec(100).expect("round"), 100);
    assert_eq!(csroundsec(0).expect("round"), 0);
    assert_eq!(csroundsec(-150).expect("round"), -100);
    assert_eq!(csroundsec(-50).expect("round"), 0);
    // Probed engine rule: just below a 30° sector boundary the value
    // rounds down instead of crossing into the next sector.
    assert_eq!(csroundsec(10799950).expect("sector rule"), 10799900);
}

#[test]
fn cs2timestr_formats_and_suppresses() {
    assert_eq!(cs2timestr(0, b':', false).expect("midnight"), "00:00:00");
    assert_eq!(cs2timestr(360000, b':', false).expect("1h"), "01:00:00");
    assert_eq!(
        cs2timestr(4526050, b':', false).expect("12:34:21"),
        "12:34:21"
    );
    assert_eq!(
        cs2timestr(4526050, b'-', false).expect("separator"),
        "12-34-21"
    );
    // A zero seconds field is dropped when suppression is requested.
    assert_eq!(
        cs2timestr(4524000, b':', true).expect("suppress zero"),
        "12:34"
    );
    assert_eq!(cs2timestr(0, b':', true).expect("suppress zero"), "00:00");
    // Nonzero seconds survive suppression.
    assert_eq!(
        cs2timestr(4526050, b':', true).expect("keep nonzero"),
        "12:34:21"
    );
    // A rounding carry out of the day wraps back to midnight.
    assert_eq!(
        cs2timestr(8639999, b':', false).expect("carry wrap"),
        "00:00:00"
    );
    // A NUL separator would truncate the output: rejected pre-call.
    let err = cs2timestr(0, 0, false).expect_err("NUL separator");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
}

#[test]
fn cs2degstr_whole_seconds_with_degree_sign() {
    assert_eq!(cs2degstr(0).expect("zero"), " 0°00'00");
    assert_eq!(cs2degstr(360000).expect("1°"), " 1°00'00");
    assert_eq!(cs2degstr(3723456).expect("10°20'34.56\""), "10°20'34");
    // Seconds are truncated, not rounded: 34.99" still reads 34...
    assert_eq!(cs2degstr(3723499).expect("truncate"), "10°20'34");
    //... while 35.50" reads 35.
    assert_eq!(cs2degstr(3723550).expect("next second"), "10°20'35");
    assert_eq!(cs2degstr(-360000).expect("negative"), "-1°00'00");
}

#[test]
fn cs2lonlatstr_selects_direction() {
    assert_eq!(cs2lonlatstr(360000, b'N', b'S').expect("north"), "1N00");
    assert_eq!(cs2lonlatstr(-360000, b'N', b'S').expect("south"), "1S00");
    assert_eq!(
        cs2lonlatstr(12345678, b'E', b'W').expect("seconds"),
        "34E17'37"
    );
    // Zero takes the positive branch; a sub-arcsecond negative still
    // takes the negative branch.
    assert_eq!(cs2lonlatstr(0, b'N', b'S').expect("zero"), "0N00");
    assert_eq!(cs2lonlatstr(-1, b'N', b'S').expect("tiny"), "0S00");
    let err = cs2lonlatstr(0, 0, b'S').expect_err("NUL direction");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    let err = cs2lonlatstr(0, b'N', 0).expect_err("NUL direction");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
}

#[test]
fn split_deg_plain_and_zodiacal() {
    let plain = split_deg(45.5, 0).expect("plain split");
    assert_eq!(
        (plain.deg, plain.min, plain.sec, plain.secfr, plain.sign),
        (45, 30, 0, 0.0, 1)
    );
    let zodiac = split_deg(45.5, SPLIT_DEG_ZODIACAL).expect("zodiacal");
    assert_eq!(
        (zodiac.deg, zodiac.min, zodiac.sec, zodiac.sign),
        (15, 30, 0, 1)
    );
    // Negative inputs split their absolute value with sign −1...
    let neg = split_deg(-30.5, 0).expect("negative");
    assert_eq!((neg.deg, neg.min, neg.sec, neg.sign), (30, 30, 0, -1));
    //... except in the zodiacal form, which reports the segment index.
    let neg_zod = split_deg(-30.5, SPLIT_DEG_ZODIACAL).expect("negative zodiacal");
    assert_eq!(
        (neg_zod.deg, neg_zod.min, neg_zod.sec, neg_zod.sign),
        (0, 30, 0, 1)
    );
    // A full turn reports segment 0, not 12.
    let full = split_deg(360.0, SPLIT_DEG_ZODIACAL).expect("full turn");
    assert_eq!(full.sign, 0);
    assert_eq!((full.deg, full.min, full.sec), (0, 0, 0));
    // Non-finite inputs never reach the native decomposition.
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let err = split_deg(bad, 0).expect_err("non-finite must be rejected");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
}

#[test]
fn split_deg_rounding_and_nakshatra() {
    // Rounding to degrees carries 45.5° up to 46°.
    let rounded = split_deg(45.5, SPLIT_DEG_ROUND_DEG).expect("round deg");
    assert_eq!((rounded.deg, rounded.min, rounded.sec), (46, 0, 0));
    // Rounding to minutes promotes 13°19'59.99" to 13°20'.
    let to_min = split_deg(13.3333333333, SPLIT_DEG_ROUND_MIN).expect("round min");
    assert_eq!((to_min.deg, to_min.min, to_min.sec), (13, 20, 0));
    // The keep-degree guard suppresses a carry into the next degree:
    // 13.9999° rounds to 14°00' but stays at 13°59' when guarded.
    let unguarded = split_deg(13.9999, SPLIT_DEG_ROUND_MIN).expect("round up");
    assert_eq!((unguarded.deg, unguarded.min, unguarded.sec), (14, 0, 0));
    let kept = split_deg(13.9999, SPLIT_DEG_ROUND_MIN | SPLIT_DEG_KEEP_DEG).expect("keep deg");
    assert_eq!((kept.deg, kept.min, kept.sec), (13, 59, 0));
    // The keep-sign guard suppresses a carry across a 30° boundary:
    // 29.999999° rounds to 30°00' but stays at 29°59'59" when guarded.
    let cross = split_deg(29.999999, SPLIT_DEG_ROUND_SEC).expect("cross sign");
    assert_eq!((cross.deg, cross.min, cross.sec), (30, 0, 0));
    let held = split_deg(29.999999, SPLIT_DEG_ROUND_SEC | SPLIT_DEG_KEEP_SIGN).expect("keep sign");
    assert_eq!((held.deg, held.min, held.sec), (29, 59, 59));
    // Nakshatra mode: 45.5° sits in segment 3 at 5°30' within it.
    let nak = split_deg(45.5, SPLIT_DEG_NAKSHATRA).expect("nakshatra");
    assert_eq!((nak.deg, nak.min, nak.sec, nak.sign), (5, 30, 0, 3));
    assert!(
        nak.secfr.abs() < 1e-6,
        "nakshatra residue must be sub-arcsecond, got {}",
        nak.secfr
    );
    // Negative inputs fall back to the signed split in nakshatra mode.
    let neg_nak = split_deg(-30.5, SPLIT_DEG_NAKSHATRA).expect("negative nakshatra");
    assert_eq!((neg_nak.deg, neg_nak.min, neg_nak.sign), (30, 30, -1));
    // The keep-sign guard is accepted engine-defined (no assertion on
    // the guarded value beyond finiteness of the fraction).
    let guarded = split_deg(45.5, SPLIT_DEG_ROUND_SEC | SPLIT_DEG_KEEP_SIGN).expect("keep sign");
    assert!(guarded.secfr.is_finite());
}

/// Sequenced stateful test: file-identity reporting follows the files
/// the engine has actually opened.
#[test]
fn stateful_file_data() {
    if !isolated::without_ephe_override("stateful_file_data") {
        return;
    }
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP stateful_file_data: no ephemeris data found; set SWISSEPH_EPHE_DIR \
             to a directory containing the .se1 files"
        );
        return;
    };
    // Keep SE_EPHE_PATH out of the picture for a deterministic outcome.

    set_ephe_path(Some(&data_dir)).expect("point at shipped data");

    // A file-based planetary computation opens the planet and Moon files.
    let mars = calc_ut(2451545.0, MARS, FLG_SWIEPH).expect("file-based Mars");
    assert!(
        mars.returned_flags & FLG_SWIEPH != 0,
        "Mars must be file-based, flags {:#x}",
        mars.returned_flags
    );

    let planets = get_current_file_data(0)
        .expect("slot 0 query")
        .expect("planet file must be open");
    assert!(
        planets.path.ends_with("sepl_18.se1"),
        "unexpected planet file: {}",
        planets.path
    );
    assert!(
        planets.tfstart < 2451545.0 && 2451545.0 < planets.tfend,
        "J2000 must sit inside [{}, {}]",
        planets.tfstart,
        planets.tfend
    );
    assert_eq!(planets.denum, 441);
    assert!(
        planets.tfstart < planets.tfend,
        "file range must be ordered"
    );

    let moon = get_current_file_data(1)
        .expect("slot 1 query")
        .expect("Moon file must be open");
    assert!(
        moon.path.ends_with("semo_18.se1"),
        "unexpected Moon file: {}",
        moon.path
    );
    assert_eq!(moon.denum, 441);

    // Slots the engine never fills report absence as data, not an error.
    for empty in [2, 3, 4, 5, -1] {
        assert_eq!(
            get_current_file_data(empty).expect("empty-slot query"),
            None,
            "slot {empty} must report no file"
        );
    }

    // A repeat query is stable while the configuration is untouched.
    assert_eq!(
        get_current_file_data(0).expect("repeat query"),
        Some(planets),
        "repeat file-identity query must agree"
    );

    set_ephe_path(None).expect("restore default path");
}

#[test]
fn utility_constant_values() {
    assert_eq!(
        (
            SPLIT_DEG_ROUND_SEC,
            SPLIT_DEG_ROUND_MIN,
            SPLIT_DEG_ROUND_DEG,
            SPLIT_DEG_ZODIACAL,
            SPLIT_DEG_NAKSHATRA,
            SPLIT_DEG_KEEP_SIGN,
            SPLIT_DEG_KEEP_DEG
        ),
        (1, 2, 4, 8, 1024, 16, 32)
    );
    assert_eq!(
        (ACRONYCHAL_RISING, ACRONYCHAL_SETTING, COSMICAL_SETTING),
        (5, 6, 6)
    );
    assert_eq!(AUNIT_TO_KM, 149597870.700);
    assert!((AUNIT_TO_LIGHTYEAR - 1.0 / 63241.07708427).abs() == 0.0);
    assert!((AUNIT_TO_PARSEC - 1.0 / 206264.8062471).abs() == 0.0);
    assert_eq!(
        (COMET_OFFSET, FICT_MAX, FICT_OFFSET_1, DE_NUMBER),
        (1000, 999, 39, 431)
    );
    assert_eq!(FLG_CENTER_BODY, 1024 * 1024);
    assert_eq!(
        (ASC, MC, ARMC, VERTEX, EQUASC, COASC1, COASC2),
        (0, 1, 2, 3, 4, 5, 6)
    );
    assert_eq!((ASTNAMFILE, FICTFILE), ("seasnam.txt", "seorbel.txt"));
}
