//! Integration checks: the full body-number domain  and the
//! strict-source helper.
//!
//! Native configuration is process-global, so every path-dependent step
//! runs inside the single sequenced `stateful_bodies_and_source` test.
//! Standalone tests use the Moshier model or pure-Rust checks only, which
//! depend on no shared configuration. Assertions are structural (ranges,
//! flag provenance, error classification, same-engine consistency), never
//! stored reference vectors.

use swisseph_bindings::{
    AST_OFFSET, CERES, CUPIDO, ECL_NUT, ErrorKind, FLG_JPLEPH, FLG_MOSEPH, FLG_SPEED, FLG_SWIEPH,
    Position, SUN, calc_ut, get_planet_name, require_source_flags, set_ephe_path,
};

#[path = "data_dir.rs"]
mod test_data;

/// J2000.0 as a Universal-Time label.
const J2000_UT: f64 = 2451545.0;

#[test]
fn fictitious_bodies_need_no_data_files() {
    // Cupido (a Hamburg-school body) is computed from built-in elements:
    // it succeeds with the file-free Moshier selection anywhere.
    let cupido = calc_ut(J2000_UT, CUPIDO, FLG_MOSEPH | FLG_SPEED).expect("cupido moshier");
    assert!((0.0..360.0).contains(&cupido.longitude()));
    assert!(cupido.values.iter().all(|v| v.is_finite()));
    assert!(
        cupido.returned_flags & FLG_MOSEPH != 0,
        "source bit must echo the request, got {:#X}",
        cupido.returned_flags
    );
    // A strict Moshier expectation is satisfied by construction here.
    require_source_flags(&cupido, FLG_MOSEPH | FLG_SPEED).expect("moshier source matches");
}

#[test]
fn nutation_pseudo_body_is_structural() {
    // ECL_NUT returns nutation/obliquity data, not a planet: six finite
    // components with no failure mode.
    let nut = calc_ut(J2000_UT, ECL_NUT, FLG_MOSEPH | FLG_SPEED).expect("ecl_nut");
    assert!(nut.values.iter().all(|v| v.is_finite()));
}

#[test]
fn strict_helper_accepts_match_and_default_chain() {
    let moshier = Position {
        values: [0.0; 6],
        returned_flags: FLG_MOSEPH | FLG_SPEED,
        diagnostic: String::new(),
    };
    require_source_flags(&moshier, FLG_MOSEPH | FLG_SPEED).expect("matching source");
    // No source bit requested means the native default chain: nothing
    // strict to enforce.
    require_source_flags(&moshier, FLG_SPEED).expect("default chain");
    require_source_flags(&moshier, 0).expect("bare default");
    // Combined request bits accept any requested source actually used.
    require_source_flags(&moshier, FLG_SWIEPH | FLG_MOSEPH).expect("either source");
}

#[test]
fn strict_helper_rejects_silent_fallback() {
    let moshier = Position {
        values: [0.0; 6],
        returned_flags: FLG_MOSEPH | FLG_SPEED,
        diagnostic: "using Moshier eph.; ".to_string(),
    };
    let err = require_source_flags(&moshier, FLG_SWIEPH | FLG_SPEED)
        .expect_err("swiss request served by moshier must fail strict");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(
        err.message().contains("using Moshier eph.; "),
        "native diagnostic must be kept, got {}",
        err.message()
    );
    let jpl = Position {
        values: [0.0; 6],
        returned_flags: FLG_MOSEPH | FLG_SPEED,
        diagnostic: String::new(),
    };
    let err = require_source_flags(&jpl, FLG_JPLEPH | FLG_SPEED)
        .expect_err("jpl request served by moshier must fail strict");
    assert_eq!(err.kind(), ErrorKind::Native);
}

/// Sequenced stateful checks: body-number domain with shipped data, name
/// lookups, missing-element errors and end-to-end strict-source gating.
/// Order matters because native state is process-global; the default
/// search path is restored before the test ends.
#[test]
fn stateful_bodies_and_source() {
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP stateful_bodies_and_source: no ephemeris data found; set SWISSEPH_EPHE_DIR \
             to a directory containing the .se1 files"
        );
        return;
    };
    set_ephe_path(Some(&data_dir)).expect("point at shipped data");

    // 1. Fictitious bodies are computed from built-in elements under
    // either source bit: values agree to well below an arcsecond while
    // the returned source bit echoes the request. (The last-ulp-scale
    // split is native correction-chain behavior for different source
    // selections, confirmed per-flag against the C caller in ;
    // it is not a binding conversion error, so bitwise equality across
    // source bits is not asserted.)
    let cupido_sw = calc_ut(J2000_UT, CUPIDO, FLG_SWIEPH | FLG_SPEED).expect("cupido swiss");
    let cupido_mo = calc_ut(J2000_UT, CUPIDO, FLG_MOSEPH | FLG_SPEED).expect("cupido moshier");
    for (index, (sw, mo)) in cupido_sw
        .values
        .iter()
        .zip(cupido_mo.values.iter())
        .enumerate()
    {
        // Angles and daily rates in degrees: tight absolute band.
        // Distance in AU (~42 here): relative band of the same order.
        let allowed = if index == 2 {
            1e-6 * sw.abs().max(1.0)
        } else {
            1e-6
        };
        assert!(
            (sw - mo).abs() < allowed,
            "fictitious component {index} must agree across source bits: {sw} vs {mo}"
        );
    }
    assert!(cupido_sw.returned_flags & FLG_SWIEPH != 0);
    assert!(cupido_mo.returned_flags & FLG_MOSEPH != 0);
    require_source_flags(&cupido_sw, FLG_SWIEPH | FLG_SPEED).expect("swiss echo");

    // 2. Numbered asteroids alias the dedicated ids where both exist:
    // AST_OFFSET + 1 computes exactly like CERES through the same engine.
    let ast1 = calc_ut(J2000_UT, AST_OFFSET + 1, FLG_SWIEPH | FLG_SPEED).expect("asteroid 1");
    let ceres = calc_ut(J2000_UT, CERES, FLG_SWIEPH | FLG_SPEED).expect("ceres");
    assert_eq!(ast1.values, ceres.values, "AST_OFFSET+1 must equal CERES");
    assert_eq!(ast1.returned_flags, ceres.returned_flags);
    assert_eq!(
        get_planet_name(AST_OFFSET + 1).expect("asteroid name"),
        get_planet_name(CERES).expect("ceres name")
    );

    // 3. Names preserve the native verdict verbatim: known fictitious
    // bodies resolve, while out-of-range numbers keep their descriptive
    // native text as `Ok` data instead of failing.
    assert_eq!(get_planet_name(CUPIDO).expect("cupido name"), "Cupido");
    let unknown = get_planet_name(999).expect("unknown name stays verbatim");
    assert!(
        unknown.contains("not found"),
        "native verdict must be preserved, got {unknown:?}"
    );

    // 4. Body numbers with no built-in or file elements fail natively
    // with the engine diagnostic kept, never as invented data.
    let err = calc_ut(J2000_UT, 999, FLG_SWIEPH | FLG_SPEED).expect_err("body 999 must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(!err.message().is_empty(), "native diagnostic must be kept");

    // 5. Strict gating end to end: file-based Sun passes, the visible
    // empty-dir fallback is rejected, and an honest Moshier request
    // still passes there.
    let sun = calc_ut(J2000_UT, SUN, FLG_SWIEPH | FLG_SPEED).expect("swiss sun");
    assert!(sun.returned_flags & FLG_SWIEPH != 0);
    require_source_flags(&sun, FLG_SWIEPH | FLG_SPEED).expect("file-based sun is strict-ok");
    let empty = std::env::temp_dir().join("swisseph-bindings-empty-ephe5");
    std::fs::create_dir_all(&empty).expect("empty ephe dir");
    set_ephe_path(Some(empty.to_str().expect("utf8 temp path"))).expect("empty data path");
    let fallback = calc_ut(J2000_UT, SUN, FLG_SWIEPH | FLG_SPEED).expect("fallback succeeds");
    assert!(fallback.returned_flags & FLG_MOSEPH != 0);
    let err = require_source_flags(&fallback, FLG_SWIEPH | FLG_SPEED)
        .expect_err("fallback must fail strict");
    assert_eq!(err.kind(), ErrorKind::Native);
    let honest = calc_ut(J2000_UT, SUN, FLG_MOSEPH | FLG_SPEED).expect("honest moshier");
    require_source_flags(&honest, FLG_MOSEPH | FLG_SPEED).expect("honest moshier is strict-ok");

    // 6. Restore the default search path for a clean handoff.
    set_ephe_path(None).expect("restore default path");
}
