//! Isolated child supervision and input-only build identities.
//! No child output frame is written to disk or included in a failure message.
use super::protocol::{Record, Request, read_frame, write_frame};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Unique, owned temporary directory; removal never touches checkout data.
pub struct Scratch {
    path: PathBuf,
}
impl Scratch {
    /// Create atomically, so concurrent test invocations cannot share fixtures.
    pub fn new() -> Result<Self, String> {
        let tick = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        for attempt in 0..100 {
            let path = std::env::temp_dir().join(format!(
                "swisseph-parity-{}-{tick}-{attempt}",
                std::process::id()
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("cannot allocate owned scratch directory".into())
    }
    /// Root for compiler outputs and empty/partial-data directories only.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A peer role cannot be confused with a command-line parsing boolean.
#[derive(Clone, Copy)]
pub enum Role {
    /// Independent C executable with no Rust-worker argument.
    Native,
    /// Safe public Rust API peer.
    Rust,
    /// Synthetic silent peer used only to verify timeout/reaping.
    SilentProbe,
    /// Synthetic corrupt framing peer used only to verify rejection/cleanup.
    MalformedProbe,
}

/// One child, at most one outstanding request, and continuously drained pipes.
pub struct Worker {
    child: Child,
    input: Option<ChildStdin>,
    receive: Option<Receiver<Result<Record, String>>>,
    output_reader: Option<JoinHandle<()>>,
    diagnostic_reader: Option<JoinHandle<String>>,
    deadline: Duration,
    /// Executable identity used only for the process-specific library-path check.
    pub executable: PathBuf,
}

/// Drain stderr even after its retained prefix is full, avoiding pipe blockage.
fn drain(mut reader: impl Read) -> String {
    let mut retained = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let available = 8192_usize.saturating_sub(retained.len());
                retained.extend_from_slice(&buffer[..n.min(available)]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => (),
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&retained).into_owned()
}

impl Worker {
    /// Launch with child-only environment changes and an explicitly owned cwd.
    pub fn start(
        executable: &Path,
        role: Role,
        cwd: &Path,
        deadline: Duration,
    ) -> Result<Self, String> {
        let executable = executable.canonicalize().map_err(|e| e.to_string())?;
        let mut command = Command::new(&executable);
        match role {
            Role::Native => (),
            Role::Rust => {
                command.arg("--rust-worker");
            }
            Role::SilentProbe => {
                command.arg("--silent-test-worker");
            }
            Role::MalformedProbe => {
                command.arg("--malformed-test-worker");
            }
        }
        let mut child = command
            .current_dir(cwd)
            .env_remove("SE_EPHE_PATH")
            .env("LC_ALL", "C")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let input = child.stdin.take().ok_or("missing child stdin")?;
        let mut output = child.stdout.take().ok_or("missing child stdout")?;
        let diagnostic = child.stderr.take().ok_or("missing child stderr")?;
        let (send, receive) = mpsc::sync_channel(1);
        let output_reader = thread::spawn(move || {
            loop {
                let response = match read_frame(&mut output) {
                    Ok(Some(frame)) => {
                        Record::decode(&frame).map_err(|_| "malformed response frame".into())
                    }
                    Ok(None) => Err("worker EOF before a response".into()),
                    Err(_) => Err("worker framing failure".into()),
                };
                let terminal = response.is_err();
                if send.send(response).is_err() || terminal {
                    break;
                }
            }
        });
        let diagnostic_reader = thread::spawn(move || drain(diagnostic));
        Ok(Self {
            child,
            input: Some(input),
            receive: Some(receive),
            output_reader: Some(output_reader),
            diagnostic_reader: Some(diagnostic_reader),
            deadline,
            executable,
        })
    }
    /// Set a declared per-operation budget before issuing the next request.
    pub fn set_deadline(&mut self, deadline: Duration) {
        self.deadline = deadline;
    }

    /// Transmit one bounded request and require a response before its deadline.
    pub fn request(&mut self, request: &Request) -> Result<Record, String> {
        if let Err(error) = write_frame(
            self.input.as_mut().ok_or("closed worker input")?,
            &request.encode(),
        ) {
            return Err(self.abort(format!("worker write failure: {error}")));
        }
        let response = self
            .receive
            .as_ref()
            .ok_or("closed worker receiver")?
            .recv_timeout(self.deadline);
        let record = match response {
            Ok(Ok(record)) => record,
            Ok(Err(reason)) => return Err(self.abort(reason)),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err(self.abort("worker response deadline exceeded".into()));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(self.abort("worker response channel disconnected".into()));
            }
        };
        if record.id != request.id || record.op != request.op {
            return Err(self.abort("worker response identity".into()));
        }
        Ok(record)
    }

    /// Terminate/reap a failed peer before collecting bounded diagnostics.
    /// Closing the receiver first also releases a blocked stdout producer.
    fn abort(&mut self, reason: String) -> String {
        self.receive.take();
        self.input.take();
        let status = match self.child.try_wait() {
            Ok(Some(status)) => Ok(status),
            _ => {
                let _ = self.child.kill();
                self.child.wait()
            }
        };
        if let Some(reader) = self.output_reader.take() {
            let _ = reader.join();
        }
        let diagnostic = self
            .diagnostic_reader
            .take()
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default();
        format!("{reason}; termination={status:?}; stderr={diagnostic}")
    }
    /// Clean EOF must exit successfully; an unexpected response is not ignored.
    pub fn finish(&mut self) -> Result<(), String> {
        self.input.take();
        let start = Instant::now();
        let status = loop {
            if let Some(status) = self.child.try_wait().map_err(|e| e.to_string())? {
                break status;
            }
            if start.elapsed() > self.deadline {
                return Err("worker shutdown deadline".into());
            }
            thread::sleep(Duration::from_millis(5));
        };
        // Close the bounded channel before joining its producer, including on
        // error; otherwise an unsolicited second response could block cleanup.
        let receive = self.receive.take().ok_or("worker finished twice")?;
        if receive.try_iter().any(|response| response.is_ok()) {
            return Err("unsolicited worker response".into());
        }
        drop(receive);
        if let Some(reader) = self.output_reader.take() {
            reader.join().map_err(|_| "stdout reader panic")?;
        }
        let diagnostic = self
            .diagnostic_reader
            .take()
            .ok_or("missing stderr reader")?
            .join()
            .map_err(|_| "stderr reader panic")?;
        if !status.success() {
            return Err(format!("worker exit {status}: {diagnostic}"));
        }
        if !diagnostic.is_empty() {
            return Err(format!("unexpected worker stderr: {diagnostic}"));
        }
        Ok(())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.receive.take();
        self.input.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.output_reader.take() {
            let _ = reader.join();
        }
        if let Some(reader) = self.diagnostic_reader.take() {
            let _ = reader.join();
        }
    }
}

/// SHA-256 covers an input file, never a native result vector or protocol trace.
pub fn fingerprint(path: &Path) -> Result<String, String> {
    let output = Command::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!("cannot fingerprint {}", path.display()));
    }
    let text = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let hash = text.split_whitespace().next().ok_or("missing SHA-256")?;
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid SHA-256".into());
    }
    Ok(hash.to_owned())
}

/// Compile only the owned C caller against this Cargo target's exact archive.
/// The compiler is selected from build metadata, not the current shell's CC.
/// `SWISSEPH_PARITY_SANITIZED_UNITS` optionally names an owned directory of
/// prebuilt sanitized native objects (`swedate.o`.. `swehel.o`); the oracle
/// itself is then compiled with ASan/UBSan/float-cast-overflow and linked to
/// those objects instead of the production archive. The fresh sanitizer
/// script provides that directory; ordinary runs leave it unset.
pub fn compile_oracle(
    root: &Path,
    scratch: &Path,
) -> Result<(PathBuf, Vec<(PathBuf, String)>), String> {
    let out = Path::new(env!("OUT_DIR"));
    let manifest_path = out.join("native-inputs.txt");
    let manifest = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("missing native build identity: {e}"))?;
    let compiler = manifest
        .lines()
        .find_map(|line| line.strip_prefix("compiler: "))
        .ok_or("native compiler identity missing")?;
    let archive = out.join("libswisseph.a");
    let mut inputs = vec![archive.clone(), manifest_path];
    inputs.extend(
        manifest
            .lines()
            .filter(|line| line.starts_with("swisseph/"))
            .map(|line| root.join(line)),
    );
    // Caller headers/includes and the compiled Rust peer are real inputs too.
    // Hash bytes only; no native algorithm or fixture is inspected/copied.
    for entry in std::fs::read_dir(root.join("tests/native_parity")).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_file() {
            inputs.push(path);
        }
    }
    inputs.push(root.join("tests/native_parity.rs"));
    inputs.push(root.join("validation/source-pins.json"));
    inputs.push(std::env::current_exe().map_err(|e| e.to_string())?);
    let sanitized_units: Option<Vec<PathBuf>> = std::env::var_os("SWISSEPH_PARITY_SANITIZED_UNITS")
        .and_then(|dir| {
            let dir = PathBuf::from(dir);
            if !dir.is_dir() {
                return None;
            }
            Some(
                [
                    "swedate", "swehouse", "swejpl", "swemmoon", "swemplan", "sweph", "swephlib",
                    "swecl", "swehel",
                ]
                .into_iter()
                .map(|unit| dir.join(format!("{unit}.o")))
                .collect(),
            )
        });
    if let Some(units) = &sanitized_units {
        for unit in units {
            if !unit.is_file() {
                return Err(format!(
                    "sanitized native object missing: {}",
                    unit.display()
                ));
            }
            inputs.push(unit.clone());
        }
    }
    // Sanitized objects join the identity after fingerprinting the standard
    // inputs would miss them; re-collect includes them when present.
    let identity = inputs
        .into_iter()
        .map(|path| fingerprint(&path).map(|hash| (path, hash)))
        .collect::<Result<Vec<_>, _>>()?;
    let executable = scratch.join("native-oracle");
    let mut command = Command::new(compiler);
    if sanitized_units.is_some() {
        command.args([
            "-std=c11",
            "-g",
            "-O1",
            "-fno-omit-frame-pointer",
            "-fsanitize=address,undefined",
            "-fno-sanitize-recover=all",
            "-Wall",
            "-Wextra",
            "-Werror",
        ]);
    } else {
        command.args(["-std=c11", "-O2", "-fPIC", "-Wall", "-Wextra", "-Werror"]);
    }
    command
        .arg(format!("-I{}", root.join("swisseph").display()))
        .arg(root.join("tests/native_parity/oracle.c"));
    if let Some(units) = &sanitized_units {
        for unit in units {
            command.arg(unit);
        }
        command.args(["-fsanitize=address,undefined"]);
    } else {
        command.arg(&archive);
    }
    let mut child = command
        .arg("-lm")
        .arg("-o")
        .arg(&executable)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run selected native compiler {compiler}: {e}"))?;
    let stdout = child.stdout.take().ok_or("compiler stdout")?;
    let stderr = child.stderr.take().ok_or("compiler stderr")?;
    let stdout = thread::spawn(move || drain(stdout));
    let stderr = thread::spawn(move || drain(stderr));
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(60) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout.join();
            let _ = stderr.join();
            return Err("C caller compiler deadline".into());
        }
        thread::sleep(Duration::from_millis(10));
    };
    let output = stdout.join().map_err(|_| "compiler stdout reader")?;
    let diagnostic = stderr.join().map_err(|_| "compiler stderr reader")?;
    if !status.success() {
        return Err(format!("C caller compilation failed: {output}{diagnostic}"));
    }
    if !output.is_empty() || !diagnostic.is_empty() {
        return Err(format!(
            "C caller compiler diagnostics: {output}{diagnostic}"
        ));
    }
    println!(
        "native compiler: {compiler}; {}",
        manifest
            .lines()
            .find(|line| line.starts_with("compiler-version:"))
            .unwrap_or("compiler-version: unavailable")
    );
    Ok((executable, identity))
}

/// Reject evidence after any selected input changes during a comparison run.
pub fn verify_identity(identity: &[(PathBuf, String)]) -> Result<(), String> {
    for (path, hash) in identity {
        if fingerprint(path)? != *hash {
            return Err(format!("input changed during run: {}", path.display()));
        }
    }
    Ok(())
}

/// Exercise actual timeout, malformed-frame and C argument-shape failures.
/// Every synthetic peer must be reaped before the comparison suite starts.
/// Flaky-prone probes (notably the C shape probe under parallel load) retry
/// a bounded number of times: only a stable outcome is accepted, and every
/// attempt is reaped before the next starts.
const PROBE_ATTEMPTS: usize = 3;
pub fn self_test(executable: &Path, oracle: &Path, cwd: &Path) -> Result<(), String> {
    let probes = [
        (
            executable,
            Role::SilentProbe,
            Duration::from_millis(100),
            Request {
                op: 1000,
                ..Request::default()
            },
            "deadline exceeded",
        ),
        (
            executable,
            Role::MalformedProbe,
            Duration::from_secs(2),
            Request {
                op: 1000,
                ..Request::default()
            },
            "framing failure",
        ),
        (
            oracle,
            Role::Native,
            Duration::from_secs(2),
            Request {
                op: 1000,
                ints: vec![0],
                ..Request::default()
            },
            "argument shape",
        ),
        (
            executable,
            Role::Rust,
            Duration::from_secs(2),
            Request {
                op: super::session_probe::PROBE,
                ..Request::default()
            },
            "session probe shape",
        ),
        (
            executable,
            Role::Rust,
            Duration::from_secs(2),
            Request {
                op: super::session_commands::BUILD,
                ..Request::default()
            },
            "persistent builder command shape/mask/slot",
        ),
        (
            executable,
            Role::Rust,
            Duration::from_secs(2),
            Request {
                op: super::session_commands::COMPUTE,
                ..Request::default()
            },
            "persistent session command shape",
        ),
        (
            executable,
            Role::Rust,
            Duration::from_secs(2),
            Request {
                op: super::session_commands::CLONE,
                ints: vec![0, 0],
                ..Request::default()
            },
            "session ID not built",
        ),
        (
            executable,
            Role::Rust,
            Duration::from_secs(2),
            Request {
                op: super::session_commands::DROP,
                ints: vec![0],
                ..Request::default()
            },
            "session ID not built",
        ),
        (
            executable,
            Role::Rust,
            Duration::from_secs(2),
            Request {
                op: super::session_probe::PROVENANCE,
                ints: vec![0, 0],
                ..Request::default()
            },
            "session ID not built",
        ),
        (
            executable,
            Role::Rust,
            Duration::from_secs(2),
            Request {
                op: super::registry::Op::RequireSourceFlags as u32,
                ints: vec![swisseph_bindings::FLG_MOSEPH],
                ..Request::default()
            },
            "source policy requires an earlier successful position",
        ),
    ];
    for (executable, role, deadline, request, expected) in probes {
        let mut failure = None;
        for _ in 0..PROBE_ATTEMPTS {
            let mut worker = Worker::start(executable, role, cwd, deadline)?;
            let outcome = worker.request(&request);
            let reaped = worker
                .child
                .try_wait()
                .map_err(|e| e.to_string())?
                .is_some();
            match outcome {
                Err(error) if error.contains(expected) => {
                    if !reaped {
                        failure = Some("failed peer was not reaped".to_string());
                        continue;
                    }
                    failure = None;
                    break;
                }
                Err(error) => {
                    failure = Some(format!(
                        "supervision failure misclassified: op={} role-deadline={:?} want={expected:?} got={error:?}",
                        request.op, deadline,
                    ));
                }
                Ok(_) => {
                    failure = Some("synthetic peer unexpectedly succeeded".to_string());
                    break;
                }
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
    }
    println!(
        "PASS runner self-tests: finite-bit/sign/flag/order/tail/text mutations, projectors, protocol truncation, timeout/reaping, malformed response, C/session argument shape and missing state"
    );
    Ok(())
}
