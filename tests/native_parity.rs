//! Native differential validation of the safe public binding API.
//!
//! A separate public-header C caller links this Cargo target's exact archive.
//! See `validation/native-parity.md` for scope, equality rules and limitations.
//! Protocol frames and native numerical values remain in memory only.
#[path = "native_parity/cases.rs"]
mod cases;
#[path = "native_parity/cold_driver.rs"]
mod cold_driver;
#[path = "native_parity/compare.rs"]
mod compare;
#[path = "native_parity/constants.rs"]
mod constants;
#[path = "native_parity/crossing_cases.rs"]
mod crossing_cases;
#[path = "native_parity/crossing_worker.rs"]
mod crossing_worker;
#[path = "native_parity/eclipse_worker.rs"]
mod eclipse_worker;
#[path = "native_parity/event_cases.rs"]
mod event_cases;
#[path = "native_parity/event_pack.rs"]
mod event_pack;
#[path = "native_parity/heliacal_worker.rs"]
mod heliacal_worker;
#[path = "native_parity/matrix_cases.rs"]
mod matrix_cases;
#[path = "native_parity/physical_cases.rs"]
mod physical_cases;
#[path = "native_parity/physical_worker.rs"]
mod physical_worker;
#[path = "native_parity/process.rs"]
mod process;
#[path = "native_parity/protocol.rs"]
mod protocol;
#[path = "native_parity/registry.rs"]
mod registry;
#[path = "native_parity/rust_worker.rs"]
mod rust_worker;
#[path = "native_parity/schema.rs"]
mod schema;
#[path = "native_parity/session_commands.rs"]
mod session_commands;
#[path = "native_parity/session_driver.rs"]
mod session_driver;
#[path = "native_parity/session_probe.rs"]
mod session_probe;
#[path = "native_parity/session_rejection.rs"]
mod session_rejection;
#[path = "native_parity/session_state_driver.rs"]
mod session_state_driver;
#[path = "native_parity/source_cases.rs"]
mod source_cases;

use cases::Expectation;
use process::{Role, Scratch, Worker};
use protocol::{Record, Request};
use registry::Op;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

/// Implemented profiles; a pilot is never relabeled as full acceptance.
#[derive(Default, Clone, Copy)]
enum Profile {
    /// Default file-independent validation.
    #[default]
    Smoke,
    /// Bounded selected-data pilot used during family integration.
    DataPilot,
    /// Cost/branch integration of the extended matrix, not full acceptance.
    MatrixPilot,
    /// Complete local Swiss/catalog validation.
    Full,
    /// Replayable extended soak over valid dates/bodies/flags/observers.
    Sweep,
}
impl Profile {
    /// Explicit report label; extended work is never relabeled as full.
    fn label(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::DataPilot => "data pilot",
            Self::MatrixPilot => "matrix pilot",
            Self::Full => "full",
            Self::Sweep => "sweep",
        }
    }
}

/// A command-line filter cannot silently become a complete acceptance claim.
#[derive(Default)]
struct Options {
    /// Optional family filter, applied to stable manifest families.
    family: Option<String>,
    /// Optional exact input-recipe key.
    case: Option<String>,
    /// List input keys without compiling or launching a native caller.
    list: bool,
    /// Explicitly selected profile and its data prerequisites.
    profile: Profile,
    /// Optional bounded generated prefix for measuring the extended matrix.
    samples: Option<usize>,
    /// Make the optional JPL-success capability mandatory when selected.
    require_jpl: bool,
}

/// Parse only documented runner flags; standard display flags are harmless.
fn options() -> Result<Options, String> {
    let mut result = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--family" => result.family = Some(args.next().ok_or("missing family")?),
            "--case" => result.case = Some(args.next().ok_or("missing case")?),
            "--list" => result.list = true,
            "--nocapture" | "--quiet" | "-q" => (),
            "--test-threads=1" => (),
            "--samples" => {
                result.samples = Some(
                    args.next()
                        .ok_or("missing sample count")?
                        .parse()
                        .map_err(|_| "invalid sample count")?,
                )
            }
            "--require-jpl" => result.require_jpl = true,
            "--profile" => {
                result.profile =
                    match args.next().as_deref() {
                        Some("smoke") => Profile::Smoke,
                        Some("catalog-pilot" | "data-pilot") => Profile::DataPilot,
                        Some("matrix-pilot") => Profile::MatrixPilot,
                        Some("full") => Profile::Full,
                        Some("sweep") => Profile::Sweep,
                        _ => return Err(
                            "implemented profiles: smoke, data-pilot, matrix-pilot, full, sweep"
                                .into(),
                        ),
                    };
            }
            _ => return Err(format!("unknown native-parity argument {arg}")),
        }
    }
    if result.samples.is_some()
        && !matches!(
            result.profile,
            Profile::MatrixPilot | Profile::Full | Profile::Sweep
        )
    {
        return Err("--samples requires --profile matrix-pilot, full or sweep".into());
    }
    if result
        .samples
        .is_some_and(|count| !(1..=matrix_cases::POSITION_CASES).contains(&count))
    {
        return Err("matrix sample count must be 1..=32000".into());
    }
    Ok(result)
}

/// Counts describe executed checks, not declaration inventory or pending work.
#[derive(Default)]
struct Counts {
    success: usize,
    native_error: usize,
    absent: usize,
    rejected: usize,
    fields: usize,
}

/// Process-specific path validation is the sole text exception. The expected
/// bytes come from each known executable pathname, not the other worker.
fn executable_path(record: &Record, executable: &Path) -> Result<usize, String> {
    use std::os::unix::ffi::OsStrExt;
    let bytes = executable.as_os_str().as_bytes();
    let expected = String::from_utf8_lossy(&bytes[..bytes.len().min(256)]);
    if record.code != 0
        || !record.ints.is_empty()
        || !record.floats.is_empty()
        || record.texts.len() != 1
    {
        return Err("executable-path result layout".into());
    }
    if String::from_utf8_lossy(&record.texts[0]) != expected {
        return Err("process-specific executable pathname/truncation".into());
    }
    Ok(2)
}

/// Compile the caller, compare owned recipes, verify stable input identities and
/// close both processes. Any mismatch identifies only the input key and field.
fn driver() -> Result<(), String> {
    let options = options()?;
    compare::self_test()?;
    let scratch = Scratch::new()?;
    let empty = scratch.path().join("empty-data");
    std::fs::create_dir(&empty).map_err(|e| e.to_string())?;
    let empty_text = empty.to_str().ok_or("scratch path encoding")?;
    let mut manifest = cases::smoke(empty_text);
    let mut data_identity = Vec::new();
    let alternative_empty = scratch.path().join("alternate-empty");
    std::fs::create_dir(&alternative_empty).map_err(|e| e.to_string())?;
    let mut session_paths = [
        empty_text.to_string(),
        alternative_empty
            .to_str()
            .ok_or("alternate path encoding")?
            .to_string(),
    ];
    if !matches!(options.profile, Profile::Smoke) {
        let selected = std::env::var("SWISSEPH_EPHE_DIR")
            .map_err(|_| "selected-data profiles require an explicit SWISSEPH_EPHE_DIR")?;
        let path = Path::new(&selected)
            .canonicalize()
            .map_err(|e| format!("selected data directory: {e}"))?;
        for name in [
            "sepl_18.se1",
            "semo_18.se1",
            "seas_18.se1",
            "sefstars.txt",
            "seorbel.txt",
            "swe_deltat.txt",
            "seleapsec.txt",
        ] {
            let file = path.join(name);
            if !file.is_file() {
                if matches!(
                    name,
                    "sepl_18.se1" | "semo_18.se1" | "seas_18.se1" | "sefstars.txt"
                ) {
                    return Err(format!("required selected data file missing: {name}"));
                }
                continue;
            }
            if file.metadata().map_err(|e| e.to_string())?.len() == 0 {
                return Err(format!("empty selected data file: {name}"));
            }
            data_identity.push((file.clone(), process::fingerprint(&file)?));
        }
        let selected = path.to_str().ok_or("selected path is not UTF-8")?;
        session_paths[0] = selected.to_string();
        manifest.extend(physical_cases::catalog(selected));
        let planet_only = scratch.path().join("planet-only");
        let moon_only = scratch.path().join("moon-only");
        for (directory, file) in [(&planet_only, "sepl_18.se1"), (&moon_only, "semo_18.se1")] {
            std::fs::create_dir(directory).map_err(|e| e.to_string())?;
            let copy = directory.join(file);
            std::fs::copy(path.join(file), &copy).map_err(|e| e.to_string())?;
            data_identity.push((copy.clone(), process::fingerprint(&copy)?));
        }
        let paths = [
            selected,
            empty_text,
            planet_only.to_str().ok_or("partial path encoding")?,
            moon_only.to_str().ok_or("partial path encoding")?,
        ];
        manifest.extend(source_cases::pilot(
            paths[0],
            paths[1],
            paths[2],
            paths[3],
            options.require_jpl,
        )?);
        if matches!(options.profile, Profile::MatrixPilot) {
            matrix_cases::self_test()?;
            manifest.extend(matrix_cases::positions(
                paths,
                options.samples.unwrap_or(matrix_cases::POSITION_CASES),
            ));
            manifest.extend(matrix_cases::sidereal(selected));
            manifest.extend(matrix_cases::houses(selected));
        }
        if matches!(options.profile, Profile::Full) {
            matrix_cases::self_test()?;
            // Full = matrix v2 in full (no --samples prefix) + JPL block.
            manifest.extend(matrix_cases::positions(paths, matrix_cases::POSITION_CASES));
            manifest.extend(matrix_cases::sidereal(selected));
            manifest.extend(matrix_cases::houses(selected));
        }
        if matches!(options.profile, Profile::Sweep) {
            matrix_cases::self_test()?;
            manifest.extend(matrix_cases::sweep(
                paths,
                options.samples.unwrap_or(matrix_cases::SWEEP_CASES),
            ));
        }
    }
    let cases = manifest
        .into_iter()
        .filter(|case| {
            options
                .family
                .as_ref()
                .is_none_or(|family| family == case.family)
                && options.case.as_ref().is_none_or(|key| key == &case.key)
        })
        .collect::<Vec<_>>();
    if cases.is_empty() {
        return Err("filter selected no native-parity cases".into());
    }
    if options.list {
        for case in &cases {
            println!("{}: {}", case.family, case.key);
        }
        return Ok(());
    }
    for case in &cases {
        for step in &case.steps {
            schema::validate(step.op, &step.request).map_err(|e| format!("{}: {e}", case.key))?;
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (oracle, mut identity) = process::compile_oracle(root, scratch.path())?;
    identity.extend(data_identity);
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let deadline = if std::env::var_os("SWISSEPH_PARITY_SANITIZED").is_some() {
        Duration::from_secs(30)
    } else {
        Duration::from_secs(10)
    };
    process::self_test(&executable, &oracle, &empty)?;
    let mut c = Worker::start(&oracle, Role::Native, &empty, deadline)?;
    let mut rust = Worker::start(&executable, Role::Rust, &empty, deadline)?;
    let handshake = Request {
        op: 1000,
        ..Request::default()
    };
    compare::same(&c.request(&handshake)?, &rust.request(&handshake)?, false)
        .map_err(|e| format!("ABI/version handshake: {e}"))?;
    let header = Request {
        id: 1,
        op: 1001,
        ..Request::default()
    };
    let constant_fields = compare::same(&c.request(&header)?, &rust.request(&header)?, false)
        .map_err(|e| format!("compiled-header constants: {e}"))?;
    let setup = Request {
        id: 2,
        op: Op::SetEphePath as u32,
        texts: vec![Some(empty_text.as_bytes().to_vec())],
        ..Request::default()
    };
    compare::compare(
        Op::SetEphePath,
        Some(&c.request(&setup)?),
        &rust.request(&setup)?,
        false,
    )?;
    let start = Instant::now();
    let mut counts: BTreeMap<u32, Counts> = BTreeMap::new();
    let mut native_symbols = std::collections::BTreeSet::new();
    let mut id = 3_u32;
    let mut last_native_position = None;
    for case in &cases {
        for (step_index, step) in case.steps.iter().enumerate() {
            let total_budget = if matches!(
                options.profile,
                Profile::MatrixPilot | Profile::Full | Profile::Sweep
            ) {
                300
            } else {
                180
            };
            if start.elapsed() > Duration::from_secs(total_budget) {
                return Err("selected profile total deadline".into());
            }
            let mut request = step.request.clone();
            request.id = id;
            id += 1;
            let context = |error: String| {
                format!(
                    "case={} step={step_index} operation={}: {error}",
                    case.key,
                    step.op.entry().name
                )
            };
            // Searches have an explicitly larger budget, never a tolerance or
            // a substitute for an impossible/unsafe native request.
            let budget = if step.op.entry().family == "events" {
                if std::env::var_os("SWISSEPH_PARITY_SANITIZED").is_some() {
                    Duration::from_secs(120)
                } else {
                    Duration::from_secs(60)
                }
            } else {
                deadline
            };
            c.set_deadline(budget);
            rust.set_deadline(budget);
            // Rejections are deliberately not C requests. Later normal steps
            // check the native configuration that the contract must preserve.
            let native = match step.expectation {
                Expectation::Rejected | Expectation::SourcePolicy { .. } => None,
                _ => Some(c.request(&request).map_err(context)?),
            };
            if let Some(native) = &native {
                let extended = matches!(step.op, Op::Fixstar2 | Op::Fixstar2Ut | Op::Fixstar2Mag);
                if extended && native.code < 0 && native.ints == [Op::FixstarMag as i32] {
                    native_symbols.insert("swe_fixstar_mag");
                } else if !step.op.entry().native.is_empty() {
                    native_symbols.insert(step.op.entry().native);
                }
            }
            let record = rust.request(&request).map_err(context)?;
            let fields = match step.expectation {
                Expectation::ExecutablePath => {
                    executable_path(native.as_ref().ok_or("missing native path")?, &c.executable)?
                        + executable_path(&record, &rust.executable)?
                }
                Expectation::Native => {
                    compare::operation(step.op, &request, native.as_ref(), &record)
                        .map_err(context)?
                }
                Expectation::Rejected => {
                    compare::compare(step.op, None, &record, true).map_err(context)?
                }
                Expectation::SourcePolicy {
                    accepted,
                    primary,
                    moshier_component,
                } => compare::source_policy(
                    last_native_position
                        .as_ref()
                        .ok_or("source policy requires a preceding checked position")?,
                    &record,
                    accepted,
                    primary,
                    moshier_component,
                )
                .map_err(context)?,
            };
            if matches!(step.op, Op::Calc | Op::CalcUt | Op::CalcPctr)
                && let Some(native) = native.filter(|value| value.code >= 0)
            {
                last_native_position = Some(native);
            }
            let count = counts.entry(step.op as u32).or_default();
            count.fields += fields;
            let no_event = record.code == 0
                && matches!(
                    step.op,
                    Op::SolEclipseWhenGlob
                        | Op::LunEclipseWhen
                        | Op::SolEclipseWhere
                        | Op::SolEclipseHow
                        | Op::LunEclipseHow
                        | Op::SolEclipseWhenLoc
                        | Op::LunEclipseWhenLoc
                        | Op::LunOccultWhenGlob
                        | Op::LunOccultWhenGlobWithOptions
                        | Op::LunOccultWhenLoc
                        | Op::LunOccultWhenLocWithOptions
                        | Op::LunOccultWhere
                );
            match record.code {
                rust_worker::NATIVE_ERROR => count.native_error += 1,
                rust_worker::INVALID_INPUT => count.rejected += 1,
                -2 => count.absent += 1,
                _ if no_event => count.absent += 1,
                _ => count.success += 1,
            }
        }
    }
    if (options.family.is_none() && options.case.is_none())
        || (options.family.as_deref() == Some("sessions") && options.case.is_none())
    {
        let session_budget = if std::env::var_os("SWISSEPH_PARITY_SANITIZED").is_some() {
            120
        } else {
            60
        };
        c.set_deadline(Duration::from_secs(session_budget));
        rust.set_deadline(Duration::from_secs(session_budget));
        let source = if matches!(options.profile, Profile::Smoke) {
            swisseph_bindings::FLG_MOSEPH
        } else {
            swisseph_bindings::FLG_SWIEPH
        };
        let session = session_driver::run(
            &mut c,
            &mut rust,
            [&session_paths[0], &session_paths[1]],
            source,
        )?;
        println!(
            "PASS Session: all seven computation methods against independent C A/B baselines; 4/8-thread contention, competing free setters/close, clone histories and five-slot post-close snapshots; results={} rejections={} fields={}",
            session.computations, session.rejections, session.fields
        );
        let state = session_state_driver::run(
            &mut c,
            &mut rust,
            [&session_paths[0], &session_paths[1]],
            source,
        )?;
        println!(
            "PASS Session state: inherits/partial/reset/clear/clone-drop/lapse-atmosphere/native errors; computations={} builders={} rejections={} native_errors={} absences={} fields={}",
            state.computations,
            state.builders,
            state.rejections,
            state.native_errors,
            state.absences,
            state.fields
        );
        let (scripts, fields) =
            cold_driver::run(&oracle, &executable, &empty, empty_text, deadline)?;
        println!(
            "PASS Cold/reset: fresh defaults, first tidal/Delta-T writes, close/clear/automatic selection; scripts={scripts} fields={fields}"
        );
    }
    c.finish()?;
    rust.finish()?;
    process::verify_identity(&identity)?;
    println!(
        "input archive: {} sha256={}",
        identity[0].0.display(),
        identity[0].1
    );
    println!(
        "environment: child SE_EPHE_PATH unset; LC_ALL=C; explicit owned cwd/path; profile={}",
        options.profile.label()
    );
    if matches!(options.profile, Profile::MatrixPilot) {
        println!(
            "matrix input: recipe-version={} seed={:016x} generated-position-cap={} selected-prefix={}",
            matrix_cases::VERSION,
            matrix_cases::SEED,
            matrix_cases::POSITION_CASES,
            options.samples.unwrap_or(matrix_cases::POSITION_CASES)
        );
    }
    if matches!(options.profile, Profile::Sweep) {
        println!(
            "sweep input: recipe-version={} seed={:016x} generated-position-cap={} selected-prefix={}",
            matrix_cases::SWEEP_VERSION,
            matrix_cases::SWEEP_SEED,
            matrix_cases::SWEEP_CASES,
            options.samples.unwrap_or(matrix_cases::SWEEP_CASES)
        );
    }
    if !matches!(options.profile, Profile::Smoke) {
        for (path, hash) in &identity {
            if path
                .extension()
                .is_some_and(|ext| ext == "se1" || ext == "txt")
                && !path.starts_with(env!("OUT_DIR"))
            {
                println!("data input: {} sha256={hash}", path.display());
            }
        }
    }
    let mut total = Counts::default();
    for (id, count) in &counts {
        let op = Op::from_wire(*id).ok_or("coverage operation identity")?;
        println!(
            "CHECK {} [{}]: success={} native_error={} absent={} rejected={} fields={}",
            op.entry().name,
            op.entry().family,
            count.success,
            count.native_error,
            count.absent,
            count.rejected,
            count.fields
        );
        total.success += count.success;
        total.native_error += count.native_error;
        total.absent += count.absent;
        total.rejected += count.rejected;
        total.fields += count.fields;
    }
    println!(
        "PASS {}{}: {} cases; {} public operations/105; {} compared native symbols/101; {} compared/rejection fields + {} constant/status fields; successes={} native_errors={} absences={} rejections={}; {:.2}s comparison time",
        options.profile.label(),
        if options.family.is_some() || options.case.is_some() || options.samples.is_some() {
            " (FILTERED)"
        } else {
            ""
        },
        cases.len(),
        counts.len(),
        native_symbols.len(),
        total.fields,
        constant_fields,
        total.success,
        total.native_error,
        total.absent,
        total.rejected,
        start.elapsed().as_secs_f64()
    );
    println!(
        "SCOPE: this result covers only the selected profile and inputs; native sanitizer and extracted-package checks are separate commands."
    );
    Ok(())
}

/// Emit a synthetic invalid frame, then remain alive so cancellation/reaping
/// is tested independently of EOF or an already-exited child.
fn malformed_probe() -> Result<(), String> {
    use std::io::Write;
    let mut output = std::io::stdout().lock();
    output
        .write_all(&[0; 4])
        .and_then(|()| output.flush())
        .map_err(|e| e.to_string())?;
    std::thread::sleep(Duration::from_secs(60));
    Ok(())
}

/// Native vectors never enter panic/debug output; the main boundary reports
/// only classified, input-keyed failures and a nonzero exit status.
fn main() -> std::process::ExitCode {
    let result = match std::env::args().nth(1).as_deref() {
        Some("--rust-worker") => rust_worker::run(),
        // Synthetic worker modes test supervision without any native call.
        Some("--silent-test-worker") => {
            std::thread::sleep(Duration::from_secs(60));
            Ok(())
        }
        Some("--malformed-test-worker") => malformed_probe(),
        _ => driver(),
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("native parity FAIL: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
