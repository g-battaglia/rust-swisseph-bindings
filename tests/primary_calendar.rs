//! Typed primary calendar transport against a separately compiled C caller.
//! Native bytes remain in memory; no expected numerical vectors are stored.
use std::path::Path;
use std::process::Command;
use swisseph_bindings as eph;

#[test]
fn primary_calendar_preserves_complete_native_results() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Path::new(env!("OUT_DIR"));
    let identity = std::fs::read_to_string(out.join("native-inputs.txt")).unwrap();
    let compiler = identity
        .lines()
        .find_map(|line| line.strip_prefix("compiler: "))
        .unwrap();
    let oracle = out.join("primary-calendar-oracle");
    assert!(
        Command::new(compiler)
            .args(["-O2", "-Wall", "-Wextra", "-Werror", "-I"])
            .arg(root.join("swisseph"))
            .arg(root.join("tests/primary_calendar_oracle.c"))
            .arg(out.join("libswisseph.a"))
            .args(["-lm", "-o"])
            .arg(&oracle)
            .status()
            .unwrap()
            .success()
    );
    let output = Command::new(oracle)
        .env_remove("SE_EPHE_PATH")
        .output()
        .unwrap();
    assert!(output.status.success());
    const RECORD: usize = 68;
    assert_eq!(output.stdout.len(), 5184 * RECORD);
    let (records, trailing) = output.stdout.as_chunks::<RECORD>();
    assert!(trailing.is_empty());
    for record in records {
        let integer = |start| i32::from_ne_bytes(record[start..start + 4].try_into().unwrap());
        let number = |start| f64::from_ne_bytes(record[start..start + 8].try_into().unwrap());
        let (year, month, day) = (integer(0), integer(4), integer(8));
        let calendar = match integer(12) {
            0 => eph::Calendar::Julian,
            1 => eph::Calendar::Gregorian,
            other => panic!("unexpected C calendar {other}"),
        };
        let hour = number(16);
        let julian = eph::julday(year, month, day, hour, calendar).unwrap();
        assert_eq!(julian.value.to_bits(), number(24).to_bits());
        let checked = eph::date_conversion(year, month, day, hour, calendar).unwrap();
        assert_eq!(checked.valid, integer(40) >= 0);
        assert_eq!(checked.status, Some(integer(40)));
        assert!(checked.diagnostic.is_none(), "no native diagnostic channel");
        assert_eq!(checked.julian_day.to_bits(), number(32).to_bits());
        assert_eq!(
            (checked.date.year, checked.date.month, checked.date.day),
            (integer(44), integer(48), integer(52))
        );
        assert_eq!(checked.date.hour.to_bits(), number(56).to_bits());
        let reverse = eph::revjul(checked.julian_day, calendar).unwrap();
        assert_eq!(
            (reverse.year, reverse.month, reverse.day),
            (integer(44), integer(48), integer(52))
        );
        assert_eq!(reverse.hour.to_bits(), number(56).to_bits());
        assert_eq!(eph::day_of_week(julian.value).unwrap().index, integer(64));
    }
}

#[test]
fn primary_finite_admission_does_not_change_legacy_signatures_or_results() {
    for hour in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = eph::julday(2000, 1, 1, hour, eph::Calendar::Gregorian).unwrap_err();
        assert_eq!(error.kind(), eph::ErrorKind::InvalidInput);
        assert!(error.native_status().is_none());
        assert!(error.native_diagnostic().is_none());
        assert!(eph::date_conversion(2000, 1, 1, hour, eph::Calendar::Gregorian).is_err());
        assert!(!eph::native::julday(2000, 1, 1, hour, eph::Calendar::Gregorian).is_finite());
        assert!(eph::day_of_week(hour).is_err());
    }
    for calendar in [eph::Calendar::Julian, eph::Calendar::Gregorian] {
        // A valid forward conversion need not be admitted for inverse calendar
        // decomposition: retain the native i32-year domain, not a guessed date.
        let large = eph::julday(i32::MAX, 12, 31, f64::MAX, calendar).unwrap();
        assert!(large.value.is_finite());
        assert!(eph::date_conversion(i32::MAX, 12, 31, f64::MAX, calendar).is_err());
    }
    assert!(eph::native::date_conversion(2000, 2, 30, 12.0, eph::Calendar::Gregorian).is_err());
    let outcome = eph::date_conversion(2000, 2, 30, 12.0, eph::Calendar::Gregorian).unwrap();
    assert!(!outcome.valid);
    assert!(outcome.status.is_some_and(|status| status < 0));
}
