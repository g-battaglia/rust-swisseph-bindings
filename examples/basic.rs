//! Example: the binding route to positions, houses and
//! event searches with explicit configuration and error handling. This
//! example calls the binding crate directly (no adapter, no second
//! engine).
//!
//! Run with `cargo run --example basic`. Data-independent steps (version,
//! calendar, the analytical Sun, input-error handling,
//! houses with speeds, the atomic [`Session`](swisseph_bindings::Session)
//! route, the analytical occultations) always run. File-based steps (the
//! Mars position, the Dallas eclipse) need ephemeris data: set
//! `SWISSEPH_EPHE_DIR` to a directory holding the `.se1` files, or run
//! from a checkout containing `swisseph/ephe`. Without data those steps
//! print an actionable note and are skipped; nothing silently falls back
//! to lower precision.
//! The fallback demonstration is skipped when a nonempty `SE_EPHE_PATH`
//! overrides the empty directory needed to force analytical fallback.
//!
//! Native gaps worth knowing (all documented on the functions
//! themselves): the Moon is rejected on the occultation body path (its
//! native self-occultation search does not terminate); a one-try search
//! that finds nothing reports `occult_type == 0` with a continuation
//! epoch rather than failing; polar latitudes can fail natively for
//! some house systems; fixed-star lookups need the catalog file;
//! heliocentric Sun crossings and arcus-visionis heliacal search flags are
//! rejected before native access because the pinned searches cannot run safely.

use swisseph_bindings::{
    Calendar, ErrorKind, FLG_MOSEPH, FLG_SPEED, FLG_SWIEPH, FLG_TOPOCTR, MARS, MOON,
    OccultSearchOptions, SUN, SessionBuilder, VENUS, calc_ut, get_planet_name, houses_ex2, julday,
    lun_occult_when_glob, lun_occult_when_glob_with_options, require_source_flags, revjul,
    set_ephe_path, sol_eclipse_when_loc, version,
};

/// Resolve the external ephemeris data directory: `SWISSEPH_EPHE_DIR`
/// when set, otherwise the checkout's `swisseph/ephe` when present.
fn ephemeris_dir() -> Option<String> {
    if let Some(dir) = std::env::var_os("SWISSEPH_EPHE_DIR") {
        let path = std::path::PathBuf::from(&dir);
        if path.is_dir() {
            return path.to_str().map(str::to_owned);
        }
        eprintln!(
            "SWISSEPH_EPHE_DIR points at {dir:?}, which is not a directory; \
             file-based steps are skipped"
        );
        return None;
    }
    let candidate = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("swisseph/ephe");
    if candidate.is_dir() {
        candidate.to_str().map(str::to_owned)
    } else {
        None
    }
}

fn main() -> Result<(), swisseph_bindings::Error> {
    // 1. Native identity: the linked C library, not a Rust port.
    println!("native version: {}", version()?);

    // 2. Calendar conversion around J2000.0.
    let jd = julday(2000, 1, 1, 12.0, Calendar::Gregorian);
    println!("J2000.0 Julian Day: {jd}");
    let date = revjul(jd, Calendar::Gregorian)?;
    println!(
        "back to calendar: {}-{:02}-{:02} {:05.2}h",
        date.year, date.month, date.day, date.hour
    );

    // 3. File-based steps with an explicit data path.
    match ephemeris_dir() {
        Some(data_dir) => {
            set_ephe_path(Some(&data_dir))?;
            // All six components: longitude/latitude/distance plus daily
            // speeds (components 3-5 exist because FLG_SPEED was asked
            // for). `returned_flags` names the engine that actually
            // served the request; `require_source_flags` turns a silent
            // fallback into an explicit error instead.
            let mars = calc_ut(jd, MARS, FLG_SWIEPH | FLG_SPEED)?;
            require_source_flags(&mars, FLG_SWIEPH)?;
            println!(
                "{} longitude: {:.6}°, latitude: {:.6}°, distance: {:.6} AU (flags {:#X})",
                get_planet_name(MARS)?,
                mars.longitude(),
                mars.latitude(),
                mars.distance(),
                mars.returned_flags
            );
            println!(
                "Mars speeds: {:+.6}°/day lon, {:+.6}°/day lat, {:+.6} AU/day; diagnostic: {:?}",
                mars.values[3], mars.values[4], mars.values[5], mars.diagnostic
            );

            // Next solar eclipse visible from Dallas after 2024-01-01
            // (file-based, using the same data path configured above).
            let dallas = sol_eclipse_when_loc(2460310.5, FLG_SWIEPH, -96.8, 32.8, 0.0, false)?;
            println!(
                "Dallas eclipse maximum JD: {:.5} (contacts {:.5}..{:.5})",
                dallas.maximum(),
                dallas.first_contact(),
                dallas.fourth_contact()
            );
            set_ephe_path(None)?;
        }
        None => println!(
            "no ephemeris data found: set SWISSEPH_EPHE_DIR to a directory with the \
             .se1 files to run the file-based Mars position and Dallas eclipse; \
             continuing with the data-independent steps"
        ),
    }

    // 4. The Moshier model needs no data files; the returned flags say
    // which source served the request.
    let sun = calc_ut(jd, SUN, FLG_MOSEPH | FLG_SPEED)?;
    println!(
        "Sun (analytical) longitude: {:.6}° (flags {:#X})",
        sun.longitude(),
        sun.returned_flags
    );

    // 5. With an empty data directory a file-based request succeeds
    // analytically and reports fallback. The native environment override
    // takes precedence over that directory, so report a skip when set.
    if std::env::var_os("SE_EPHE_PATH").is_some_and(|path| !path.is_empty()) {
        println!("SKIP fallback demo: SE_EPHE_PATH overrides the empty ephemeris directory");
    } else {
        let empty = std::env::temp_dir().join(format!(
            "swisseph-bindings-basic-empty-{}",
            std::process::id()
        ));
        std::fs::create_dir(&empty).expect("create empty ephe dir");
        set_ephe_path(Some(empty.to_str().expect("utf8 temp path")))?;
        let fallback = calc_ut(jd, SUN, FLG_SWIEPH | FLG_SPEED)?;
        assert!(
            fallback.returned_flags & FLG_MOSEPH != 0 && fallback.returned_flags & FLG_SWIEPH == 0,
            "fallback must be reported as analytical"
        );
        println!(
            "fallback demo: requested SWIEPH, served flags {:#X}; diagnostic: {:?}",
            fallback.returned_flags, fallback.diagnostic
        );
        set_ephe_path(None)?;
        std::fs::remove_dir(empty).expect("remove empty ephe dir");
    }

    // 6. Errors are classified: invalid Rust-side input never reaches
    // native code, and native failures keep their diagnostic.
    match revjul(f64::NAN, Calendar::Gregorian) {
        Err(err) if err.kind() == ErrorKind::InvalidInput => {
            println!("input-error demo: rejected as InvalidInput ({err})");
        }
        other => panic!("expected an InvalidInput rejection, got {other:?}"),
    }
    // Finiteness alone does not guarantee representable native calendar casts.
    let too_large = calc_ut(1e100, MOON, FLG_SWIEPH).expect_err("calendar arithmetic bound");
    assert_eq!(too_large.kind(), ErrorKind::InvalidInput);

    // 7. Houses with speeds (London Placidus): cusps, angles and their
    // daily rates come from one call.
    let london = houses_ex2(jd, 0, 51.5, -0.12, b'P')?;
    println!(
        "London Placidus ASC: {:.6}° ({:+.6}°/day), MC: {:.6}°, cusp 10: {:.6}°",
        london.ascendant(),
        london.angle_speeds[0],
        london.mc(),
        london.cusps[9]
    );

    // 8. The concurrency route: a Session applies an owned configuration
    // (here the London observer) and runs one dependent computation in a
    // single acquisition of the process-wide native lock. Free `set_*`
    // plus compute pairs are two acquisitions and therefore not atomic
    // under concurrent reconfiguration.
    let mut builder = SessionBuilder::new();
    builder.topo(-0.12, 51.5, 0.0);
    let session = builder.build()?;
    let moon = session.calc_ut(jd, MOON, FLG_MOSEPH | FLG_TOPOCTR | FLG_SPEED)?;
    println!(
        "Moon topocentric via Session: {:.6}° lon, {:+.6}°/day (flags {:#X})",
        moon.longitude(),
        moon.values[3],
        moon.returned_flags
    );

    // 9. Occultations: the unbounded search runs to an event, while the
    // one-try options examine only the nearest conjunction — a miss is
    // type 0 with a continuation epoch, not a failure.
    let occult = lun_occult_when_glob(2451545.0, VENUS, None, FLG_MOSEPH, 0, false)?;
    println!(
        "Venus occultation maximum JD: {:.5} ({:.5}..{:.5})",
        occult.maximum(),
        occult.begin(),
        occult.end()
    );
    let single = lun_occult_when_glob_with_options(
        2451545.0,
        VENUS,
        None,
        FLG_MOSEPH,
        0,
        OccultSearchOptions {
            backward: false,
            one_try: true,
        },
    )?;
    if single.occult_type == 0 {
        println!(
            "one-try near J2000: no occultation at that conjunction; next try at {:.5}",
            single.times[0]
        );
    }
    Ok(())
}
