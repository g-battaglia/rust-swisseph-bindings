//! Serial builder/reset/inheritance/ownership scripts with independent C state.
//! Native results/snapshots live only in memory. A rejected builder is never a
//! C setter request; subsequent genuine comparisons check state preservation.
use super::cases::step;
use super::process::Worker;
use super::protocol::{Record, Request};
use super::registry::Op;
use super::session_commands::{
    self as command, ALL, DELTA, JPL, LAPSE, PATH, Recipe, SIDEREAL, TIDAL, TOPO,
};
use swisseph_bindings::*;

/// Executed state-script evidence, distinct from declaration/full acceptance.
#[derive(Default)]
pub struct Summary {
    /// Matched session computation attempts, including classified native errors.
    pub computations: usize,
    /// Successfully built actual public sessions (new/Default/cloned builder).
    pub builders: usize,
    /// Rust-only invalid builder recipes; no C configuration dispatch.
    pub rejections: usize,
    /// Native-error outcomes whose public classification/diagnostic matched.
    pub native_errors: usize,
    /// Matched explicit no-event variants, rather than successful event times.
    pub absences: usize,
    /// Every compared computation/snapshot/status/rejection field.
    pub fields: usize,
}

/// Owned script state and bounded worker references; no raw FFI is reachable.
struct Script<'a> {
    /// Independent C worker.
    c: &'a mut Worker,
    /// Safe Rust worker with persistent public sessions.
    rust: &'a mut Worker,
    /// Monotonic private-script request ID, outside ordinary recipe IDs.
    id: u32,
    /// Current input-only recipe identifier used in every failure.
    key: &'static str,
    /// Requested ephemeris flags (actual returned sources are compared).
    source: i32,
    /// Successful execution accounting, never expected numerical output.
    summary: Summary,
}

/// Distinct fully explicit owned input recipe, with no computed result values.
fn explicit(which: usize, path: &str) -> Recipe {
    let input = super::session_probe::configuration(which);
    Recipe {
        mask: ALL,
        mode: input.sidereal,
        numbers: [
            input.topo[0],
            input.topo[1],
            input.topo[2],
            input.reference[0],
            input.reference[1],
            input.tidal,
            input.delta_t,
            input.lapse,
        ],
        path: Some(path.into()),
        jpl: Some("__parity_missing_jpl__.eph".into()),
        ..Recipe::default()
    }
}

/// Translate selected input knobs to independent public C/free API requests.
/// Unselected knobs emit no setter; None and explicit empty remain distinct.
fn writes(recipe: &Recipe) -> Vec<Request> {
    use Op::*;
    let mut result = Vec::new();
    if recipe.mask & PATH != 0 {
        result.push(step(SetEphePath, &[], &[], &[recipe.path.as_deref()]).request);
    }
    if recipe.mask & JPL != 0 {
        result.push(
            step(
                SetJplFile,
                &[],
                &[],
                &[Some(recipe.jpl.as_deref().unwrap_or(""))],
            )
            .request,
        );
    }
    if recipe.mask & TOPO != 0 {
        result.push(step(SetTopo, &[], &recipe.numbers[..3], &[]).request);
    }
    if recipe.mask & SIDEREAL != 0 {
        result.push(step(SetSidMode, &[recipe.mode], &recipe.numbers[3..5], &[]).request);
    }
    if recipe.mask & TIDAL != 0 {
        result.push(step(SetTidAcc, &[], &[recipe.numbers[5]], &[]).request);
    }
    if recipe.mask & DELTA != 0 {
        result.push(
            step(
                SetDeltaTUserdef,
                &[i32::from(!recipe.automatic_delta)],
                &[recipe.numbers[6]],
                &[],
            )
            .request,
        );
    }
    if recipe.mask & LAPSE != 0 {
        result.push(step(SetLapseRate, &[], &[recipe.numbers[7]], &[]).request);
    }
    result
}

impl Script<'_> {
    /// Context contains only recipe/operation identifiers and mismatch category.
    fn context(&self, operation: &str, failure: String) -> String {
        format!("case={} operation={operation}: {failure}", self.key)
    }
    /// Give each request a unique ID before either worker can see it.
    fn identified(&mut self, mut r: Request) -> Request {
        r.id = self.id;
        self.id += 1;
        r
    }
    /// Ordinary free C/Rust operation and exact public comparison. Returns only
    /// an ephemeral native record for later ownership/provenance comparison.
    fn both(&mut self, request: Request) -> Result<Record, String> {
        let op = Op::from_wire(request.op).ok_or("state script operation")?;
        let request = self.identified(request);
        let native = self
            .c
            .request(&request)
            .map_err(|e| self.context(op.entry().name, e))?;
        let rust = self
            .rust
            .request(&request)
            .map_err(|e| self.context(op.entry().name, e))?;
        self.summary.fields += super::compare::operation(op, &request, Some(&native), &rust)
            .map_err(|e| self.context(op.entry().name, e))?;
        Ok(native)
    }
    /// Apply a declared configuration to both processes, or only C before a
    /// configured Session operation. Never apply hazardous builder inputs here.
    fn install(&mut self, recipe: &Recipe, both: bool) -> Result<(), String> {
        for request in writes(recipe) {
            if both {
                self.both(request)?;
            } else {
                let request = self.identified(request);
                let result = self.c.request(&request)?;
                if result.code != 0
                    || !result.ints.is_empty()
                    || !result.floats.is_empty()
                    || !result.texts.is_empty()
                {
                    return Err(self.context("C configuration", "setter result shape".into()));
                }
            }
        }
        Ok(())
    }
    /// Build through the safe public API only. Invalid recipes never reach C.
    fn build(&mut self, slot: i32, recipe: &Recipe, rejected: bool) -> Result<(), String> {
        let request = self.identified(recipe.request(slot));
        let record = self
            .rust
            .request(&request)
            .map_err(|e| self.context("SessionBuilder::build", e))?;
        if rejected {
            self.summary.fields += super::compare::compare(Op::Calc, None, &record, true)
                .map_err(|e| self.context("SessionBuilder::build rejection", e))?;
            if record.texts[0].is_empty() {
                return Err("empty builder rejection diagnostic".into());
            }
            self.summary.rejections += 1;
        } else {
            self.summary.fields += super::compare::same(&Record::default(), &record, false)
                .map_err(|e| self.context("SessionBuilder::build", e))?;
            self.summary.builders += 1;
        }
        Ok(())
    }
    /// Status-only clone/drop control; worker shape validation precedes lookup.
    fn control(&mut self, op: u32, ints: &[i32]) -> Result<(), String> {
        let request = self.identified(Request {
            op,
            ints: ints.to_vec(),
            ..Request::default()
        });
        let record = self.rust.request(&request)?;
        self.summary.fields += super::compare::same(&Record::default(), &record, false)?;
        Ok(())
    }
    /// Observe both real globals without restoring/rewriting any configuration.
    /// The Moon position senses observer/sidereal/path/Delta-T; scalar probes
    /// detect builder preflight tide/override mutations. Both sides may lazily
    /// select their source, identically; this is not a promised state snapshot.
    fn globals(&mut self) -> Result<(), String> {
        use Op::*;
        for request in [
            step(GetTidAcc, &[], &[], &[]).request,
            super::session_driver::input(Deltat, self.source),
            super::session_driver::input(GetAyanamsaUt, self.source),
            super::session_driver::input(CalcUt, self.source),
            step(Sidtime, &[], &[2451545.0], &[]).request,
        ] {
            self.both(request)?;
        }
        for slot in 0..5 {
            self.both(step(GetCurrentFileData, &[slot], &[], &[]).request)?;
        }
        Ok(())
    }
    /// Capture every native file slot before configuration or any other C call
    /// can replace it; records are never written or printed.
    fn capture(&mut self) -> Result<Vec<Record>, String> {
        let mut records = Vec::new();
        for slot in 0..5 {
            let request = self.identified(step(Op::GetCurrentFileData, &[slot], &[], &[]).request);
            records.push(self.c.request(&request)?);
        }
        Ok(records)
    }
    /// Compare an actual retained history with its in-memory C capture. These
    /// queries must not reapply config or overwrite any current native state.
    fn history(&mut self, slot: i32, expected: &[Record]) -> Result<(), String> {
        if expected.len() != 5 {
            return Err("state history width".into());
        }
        for (index, native) in expected.iter().enumerate() {
            let request = self.identified(Request {
                op: super::session_probe::PROVENANCE,
                ints: vec![slot, index as i32],
                ..Request::default()
            });
            let record = self.rust.request(&request)?;
            self.summary.fields += super::compare::same(native, &record, false).map_err(|e| {
                self.context(
                    "Session::get_current_file_data",
                    format!("owner={slot} slot={index}: {e}"),
                )
            })?;
        }
        Ok(())
    }
    /// None before any computation is the public contract, not an ephemeris
    /// golden value. Separately built sessions have independent histories.
    fn fresh(&mut self, slot: i32) -> Result<(), String> {
        let none = Record {
            ints: vec![0],
            ..Record::default()
        };
        self.history(slot, &vec![none; 5])
    }
    /// Compare one actual Session operation to C and immediately check all five
    /// captured slots, including after classified Native failure.
    fn compute(
        &mut self,
        slot: i32,
        recipe: Option<&Recipe>,
        input: Request,
    ) -> Result<Vec<Record>, String> {
        if let Some(recipe) = recipe {
            self.install(recipe, false)?;
        }
        let op = Op::from_wire(input.op).ok_or("state session operation")?;
        let input = self.identified(input);
        let native = self.c.request(&input)?;
        let history = self.capture()?;
        let mut wrapped = input.clone();
        wrapped.op = command::COMPUTE;
        wrapped.ints.splice(0..0, [slot, op as i32]);
        let mut record = self.rust.request(&wrapped)?;
        // Worker supervision already verified the real wrapper ID/op; restore
        // only the logical operation for the independent public comparator.
        record.op = op as u32;
        self.summary.fields += super::compare::operation(op, &input, Some(&native), &record)
            .map_err(|e| self.context(op.entry().name, e))?;
        self.summary.computations += 1;
        if record.code == super::rust_worker::NATIVE_ERROR {
            self.summary.native_errors += 1;
        }
        if record.code == -2 {
            self.summary.absences += 1;
        }
        self.history(slot, &history)?;
        Ok(history)
    }
}

/// Run inherited-at-call-time, last-writer/no-build-side-effect, resets, partial
/// knobs, cloned/shared versus separate histories, native failure, drop and
/// Rust-only builder-boundary recipes. Fully inherited sessions are serial only.
pub fn run(
    c: &mut Worker,
    rust: &mut Worker,
    paths: [&str; 2],
    source: i32,
) -> Result<Summary, String> {
    use Op::*;
    let mut s = Script {
        c,
        rust,
        id: 100000,
        key: "SESSION_INHERITS_AT_CALL_TIME",
        source,
        summary: Summary::default(),
    };
    let a = explicit(0, paths[0]);
    let b = explicit(1, paths[1]);
    s.install(&a, true)?;
    s.build(0, &Recipe::default(), false)?;
    s.fresh(0)?;
    for base in [&a, &b] {
        s.install(base, true)?;
        for op in [
            Calc,
            CalcUt,
            Houses,
            HousesEx,
            GetAyanamsaUt,
            Deltat,
            RiseTrans,
        ] {
            s.compute(0, None, super::session_driver::input(op, source))?;
        }
    }

    s.key = "SESSION_BUILD_NO_INSTALL_LAST_WRITER";
    s.install(&b, true)?;
    s.build(1, &a, false)?;
    s.fresh(1)?;
    s.globals()?;
    let first = s.compute(1, Some(&a), super::session_driver::input(CalcUt, source))?;
    s.globals()?;
    s.history(1, &first)?;
    let mut clone_builder = b.clone();
    clone_builder.style = 2;
    s.build(2, &clone_builder, false)?;
    s.fresh(2)?;
    s.globals()?;
    s.compute(2, Some(&b), super::session_driver::input(CalcUt, source))?;
    s.globals()?;

    s.key = "SESSION_PARTIAL_TOPO_INHERITS_OTHER_KNOBS";
    let partial = Recipe {
        mask: TOPO,
        numbers: b.numbers,
        style: 1,
        ..Recipe::default()
    };
    s.install(&a, true)?;
    s.build(3, &partial, false)?;
    s.fresh(3)?;
    for op in [CalcUt, HousesEx, Deltat, GetAyanamsaUt] {
        s.compute(3, Some(&partial), super::session_driver::input(op, source))?;
    }
    s.globals()?;

    s.key = "SESSION_LAPSE_RATE_SENSITIVITY_AND_ATMOSPHERE";
    // Lapse rate feeds refraction-affected rise/transit geometry: distinct
    // finite rates must compare exactly against C, and the lapse-sensitive
    // computation must differ from the zero-rate baseline (in-memory only).
    for lapse in [0.0, 0.0065, 0.01] {
        let rated = Recipe {
            mask: LAPSE,
            numbers: {
                let mut numbers = b.numbers;
                numbers[7] = lapse;
                numbers
            },
            style: 1,
            ..Recipe::default()
        };
        s.install(&b, true)?;
        s.build(9, &rated, false)?;
        s.fresh(9)?;
        s.compute(
            9,
            Some(&rated),
            step(
                RiseTrans,
                &[MARS, FLG_MOSEPH, CALC_RISE],
                &[2451545.0, 48.14, 11.34, 0.0, 1013.25, 15.0],
                &[None],
            )
            .request,
        )?;
        s.globals()?;
        s.control(command::DROP, &[9])?;
    }
    // Atmosphere triple sensitivity: pressure/temperature variations move
    // the rise geometry through the same lapse-rate path; compare exactly.
    for (pressure, temperature) in [(0.0, 15.0), (1013.25, 15.0), (900.0, -10.0)] {
        s.compute(
            1,
            Some(&a),
            step(
                RiseTrans,
                &[MARS, FLG_MOSEPH, CALC_RISE],
                &[2451545.0, 48.14, 11.34, 0.0, pressure, temperature],
                &[None],
            )
            .request,
        )?;
    }
    s.globals()?;

    s.key = "SESSION_CIRCUMPOLAR_AND_POLAR_HOUSE_NATIVE_ERRORS";
    s.compute(
        1,
        Some(&a),
        step(
            RiseTrans,
            &[SUN, FLG_MOSEPH, CALC_RISE],
            &[2451545.0, 2.35, 89.0, 0.0, 1013.25, 15.0],
            &[None],
        )
        .request,
    )?;
    for op in [Houses, HousesEx] {
        let ints = if op == Houses {
            vec![i32::from(b'P')]
        } else {
            vec![FLG_SIDEREAL, i32::from(b'P')]
        };
        s.compute(
            1,
            Some(&a),
            step(op, &ints, &[2451545.0, 89.0, 2.35], &[]).request,
        )?;
    }

    s.key = "SESSION_DEFAULT_CLEAR_CLOSE_RESET";
    // Use explicit Moshier for compiled-default-path probes: no unidentified
    // data/default JPL file is a hidden success prerequisite.
    for mask in [PATH, JPL, DELTA, PATH | JPL | DELTA] {
        let reset = Recipe {
            mask,
            automatic_delta: true,
            style: 1,
            ..Recipe::default()
        };
        s.install(&b, true)?;
        s.build(6, &reset, false)?;
        s.globals()?;
        for op in [Deltat, CalcUt, GetAyanamsaUt] {
            s.compute(
                6,
                Some(&reset),
                super::session_driver::input(op, FLG_MOSEPH),
            )?;
        }
        // Current-state readers do not select an unmanifested default source.
        s.both(step(GetTidAcc, &[], &[], &[]).request)?;
    }

    s.key = "SESSION_USER_UT_CLEAR_PREFLIGHT_RESTORES_OVERRIDE";
    let clear_user = Recipe {
        mask: SIDEREAL | DELTA,
        mode: SIDM_USER | 1024,
        automatic_delta: true,
        numbers: b.numbers,
        ..Recipe::default()
    };
    s.install(&a, true)?;
    s.build(8, &clear_user, false)?;
    s.globals()?;
    for op in [Deltat, GetAyanamsaUt, CalcUt] {
        s.compute(
            8,
            Some(&clear_user),
            super::session_driver::input(op, FLG_MOSEPH),
        )?;
    }

    s.key = "SESSION_CLONE_NATIVE_FAILURE_AND_OWNED_DROP_HISTORY";
    s.install(&a, true)?;
    let success = s.compute(1, Some(&a), super::session_driver::input(CalcUt, source))?;
    s.control(command::CLONE, &[4, 1])?;
    s.history(4, &success)?;
    s.build(5, &a, false)?;
    s.fresh(5)?;
    s.history(1, &success)?;
    let errors_before = s.summary.native_errors;
    let failure = s.compute(
        4,
        Some(&a),
        step(Calc, &[999, FLG_MOSEPH], &[2451545.0], &[]).request,
    )?;
    if s.summary.native_errors != errors_before + 1 {
        return Err("native-failure ownership recipe did not fail exactly once".into());
    }
    s.history(1, &failure)?;
    s.fresh(5)?;
    s.both(super::session_driver::input(CalcUt, source))?;
    s.history(1, &failure)?;
    s.history(4, &failure)?;
    // Dropping one clone must preserve its peer, other sessions and current
    // native files; C performs no corresponding native action.
    s.control(command::DROP, &[4])?;
    s.globals()?;
    s.history(1, &failure)?;
    s.both(step(Close, &[], &[], &[]).request)?;
    s.history(1, &failure)?;
    s.fresh(5)?;
    s.compute(1, Some(&a), super::session_driver::input(CalcUt, source))?;

    s.key = "SESSION_BUILDER_SAFE_REJECTIONS_PRESERVE_GLOBALS";
    s.install(&a, true)?;
    let saved = s.compute(1, Some(&a), super::session_driver::input(CalcUt, source))?;
    for (index, mask) in [
        (0, TOPO),
        (1, TOPO),
        (2, TOPO),
        (3, SIDEREAL),
        (4, SIDEREAL),
        (5, TIDAL),
        (6, DELTA),
        (7, LAPSE),
    ] {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut bad = a.clone();
            bad.mask = mask;
            bad.numbers[index] = invalid;
            s.build(1, &bad, true)?;
            s.globals()?;
            s.history(1, &saved)?;
        }
    }
    for (mask, text) in [
        (PATH, "bad\0path".into()),
        (JPL, "bad\0name".into()),
        (PATH, "x".repeat(MAX_EPHE_PATH_LEN + 1)),
        (JPL, "x".repeat(MAX_JPL_FILE_LEN + 1)),
        (PATH, "é".repeat(MAX_EPHE_PATH_LEN / 2 + 1)),
        (JPL, "é".repeat(MAX_JPL_FILE_LEN / 2 + 1)),
    ] {
        let mut bad = a.clone();
        bad.mask = mask;
        if mask == PATH {
            bad.path = Some(text);
        } else {
            bad.jpl = Some(text);
        }
        s.build(1, &bad, true)?;
        s.globals()?;
        s.history(1, &saved)?;
    }
    for (index, mask, value) in [
        (2, TOPO, 1e100),
        (2, TOPO, -1e100),
        (3, SIDEREAL, 1e100),
        (3, SIDEREAL, -1e100),
    ] {
        let mut bad = a.clone();
        bad.mask = mask;
        bad.mode = SIDM_USER;
        bad.numbers[index] = value;
        s.build(1, &bad, true)?;
        s.globals()?;
        s.history(1, &saved)?;
    }
    s.key = "SESSION_USER_UT_UNREPRESENTABLE_SHIFT_PRESERVES_OVERRIDE";
    for shift in [f64::MAX, -f64::MAX] {
        let mut bad = a.clone();
        bad.mask = SIDEREAL | DELTA;
        bad.mode = SIDM_USER | 1024;
        bad.numbers[3] = 2451545.0;
        bad.numbers[6] = shift;
        s.build(1, &bad, true)?;
        s.globals()?;
        s.history(1, &saved)?;
    }
    // Boundary-length strings are safe builder-only inputs, not C probes of
    // missing files. Retained recipes own their bytes after this frame is gone.
    s.key = "SESSION_BUILDER_BYTE_LENGTH_BOUNDARIES";
    for (mask, text) in [
        (PATH, "x".repeat(MAX_EPHE_PATH_LEN)),
        (JPL, "x".repeat(MAX_JPL_FILE_LEN)),
        (PATH, "é".repeat(MAX_EPHE_PATH_LEN / 2)),
        (JPL, format!("{}x", "é".repeat(MAX_JPL_FILE_LEN / 2))),
    ] {
        let mut boundary = Recipe {
            mask,
            ..Recipe::default()
        };
        if mask == PATH {
            boundary.path = Some(text);
        } else {
            boundary.jpl = Some(text);
        }
        s.build(7, &boundary, false)?;
        s.fresh(7)?;
        s.globals()?;
        s.control(command::DROP, &[7])?;
    }
    if s.summary.native_errors != 3 || s.summary.absences != 1 {
        return Err(
            "session state recipes did not exercise polar/body-error and circumpolar branches"
                .into(),
        );
    }
    Ok(s.summary)
}
