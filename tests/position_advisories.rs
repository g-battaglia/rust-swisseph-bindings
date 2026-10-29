//! The native position API has diagnostic text, not structured advisory events.
use swisseph_bindings::{Advisory, FLG_MOSEPH, FLG_SPEED, MOON, Position, SessionBuilder};

#[test]
fn successful_native_positions_do_not_invent_structured_advisories() {
    let session = SessionBuilder::new()
        .build()
        .expect("native session configuration");
    let position = session
        .calc_ut(2451545.0, MOON, FLG_MOSEPH | FLG_SPEED)
        .expect("data-independent native Moon calculation");
    let Position {
        values,
        returned_flags,
        diagnostic,
        warnings,
        messages,
    } = position;
    let _: Vec<Advisory> = warnings.clone();
    assert!(
        warnings.is_empty(),
        "do not classify diagnostic prose as events"
    );
    assert!(
        messages.is_empty(),
        "native entry has no separate message channel"
    );
    assert!(values.iter().all(|value| value.is_finite()));
    assert_ne!(returned_flags & FLG_MOSEPH, 0);
    // Diagnostic ownership/text is unchanged, independent of an empty advisory
    // vector; do not assert that a successful engine call must be warning-free.
    let _: String = diagnostic;
}
