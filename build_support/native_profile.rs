//! Explicit, optional source-instrumentation policy for native C only.
//!
//! Ordinary builds do not inspect a compiler, runtime or profile here. Diagnostic
//! generation and packaged-profile use have no implicit fallback. Content IDs use
//! the same labelled FNV-1a checksum as native overlay provenance; qualification
//! additionally freezes the complete bundle with SHA-256 outside this build.
use super::c_source::fingerprint;
use std::{
    collections::BTreeMap,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

const BUNDLE: &str = "native-profiles/source-v1";
const MODE: &str = "SWISSEPH_NATIVE_PROFILE_MODE";
const RAW: &str = "SWISSEPH_NATIVE_PROFILE_RAW";
const RUNTIME: &str = "SWISSEPH_NATIVE_PROFILE_RUNTIME";
const TARGET: &str = "aarch64-apple-darwin";
const RUNTIME_NAME: &str = "libclang_rt.profile_osx.a";
const PROFILE_FILES: [&str; 5] = [
    "descriptor.txt",
    "counts.profdata",
    "audit.txt",
    "training-inputs.json",
    "training-configuration.json",
];

enum Mode {
    Off,
    Generate { raw: PathBuf, runtime: PathBuf },
    Use { directory: PathBuf },
}

/// One declared native build policy; it never trains or merges a profile.
pub(super) struct NativeProfile {
    mode: Mode,
    context: BTreeMap<String, String>,
    inputs: BTreeMap<String, String>,
}

fn id(path: &Path) -> String {
    format!(
        "{:016x}",
        fingerprint(&fs::read(path).expect("read declared profile input"))
    )
}
fn text_id(text: &str) -> String {
    format!("{:016x}", fingerprint(text.as_bytes()))
}
fn output(compiler: &Path, argument: &str) -> String {
    let output = Command::new(compiler)
        .arg(argument)
        .output()
        .expect("query declared Clang");
    assert!(
        output.status.success() && output.stderr.is_empty(),
        "Clang identity query failed"
    );
    String::from_utf8(output.stdout).expect("Clang identity is UTF-8")
}
fn absolute_file(variable: &str) -> PathBuf {
    let value =
        PathBuf::from(env::var_os(variable).expect("missing explicit native profile input"));
    assert!(
        value.is_absolute() && value.is_file(),
        "profile input must be an absolute file"
    );
    value
        .canonicalize()
        .expect("resolve declared profile input")
}
fn table_id(table: &BTreeMap<String, String>) -> String {
    text_id(
        &table
            .iter()
            .map(|(key, value)| format!("{key}={value}\n"))
            .collect::<String>(),
    )
}
fn descriptor(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut result = BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or("descriptor line needs key=value")?;
        if key.is_empty()
            || value.is_empty()
            || key.trim() != key
            || value.trim() != value
            || result.insert(key.to_owned(), value.to_owned()).is_some()
        {
            return Err("empty, padded or duplicate profile descriptor field".into());
        }
    }
    Ok(result)
}

/// Only an absent variable selects the ordinary default. A present non-Unicode
/// value is an invalid explicit policy, rather than permission to disable it.
fn selected_mode(value: Result<String, env::VarError>) -> Result<String, &'static str> {
    match value {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok("off".into()),
        Err(env::VarError::NotUnicode(_)) => Err("native profile mode must be Unicode"),
    }
}

/// Admit the archive envelope used by the declared native arm64 runtime.
/// Darwin FAT32 fields are big-endian (mach-o/fat.h). Other universal formats
/// need an explicit review; this is not an archive-member or object validator.
fn runtime_archive(bytes: &[u8]) -> bool {
    const ARCHIVE: &[u8] = b"!<arch>\n";
    const ARM64: u32 = 0x0100_000c;
    const ARM64_ALL: u32 = 0;
    const MAX_ARCHITECTURES: usize = 16;
    if bytes.starts_with(ARCHIVE) {
        return true;
    }
    if !bytes.starts_with(&[0xca, 0xfe, 0xba, 0xbe]) || bytes.len() < 8 {
        return false;
    }
    let count = u32::from_be_bytes(bytes[4..8].try_into().expect("FAT32 count")) as usize;
    if count == 0 || count > MAX_ARCHITECTURES {
        return false;
    }
    let table_end = 8 + count * 20;
    let Some(table) = bytes.get(8..table_end) else {
        return false;
    };
    let mut found = false;
    for entry in table.as_chunks::<20>().0 {
        let word =
            |offset| u32::from_be_bytes(entry[offset..offset + 4].try_into().expect("FAT32 field"));
        let (offset, size, alignment) = (word(8) as usize, word(12) as usize, word(16));
        let Some(end) = offset.checked_add(size) else {
            return false;
        };
        if offset < table_end
            || end > bytes.len()
            || alignment > 31
            || offset % (1_usize << alignment) != 0
        {
            return false;
        }
        if word(0) == ARM64 && word(4) == ARM64_ALL {
            if found || !bytes[offset..end].starts_with(ARCHIVE) {
                return false;
            }
            found = true;
        }
    }
    found
}

impl NativeProfile {
    /// Register every explicit policy input, including the ordinary default.
    pub(super) fn register() {
        println!("cargo:rerun-if-changed=build_support/native_profile.rs");
        for variable in [
            MODE,
            RAW,
            RUNTIME,
            "SDKROOT",
            "CFLAGS",
            "CPPFLAGS",
            "LLVM_PROFILE_FILE",
        ] {
            println!("cargo:rerun-if-env-changed={variable}");
        }
    }

    /// Admit a policy before any native compilation. Unsupported opt-ins fail.
    pub(super) fn load(manifest: &Path, compiler: &str, optimization: &str, lto: bool) -> Self {
        let selected = selected_mode(env::var(MODE)).expect("invalid native profile mode");
        let mut policy = Self {
            mode: Mode::Off,
            context: BTreeMap::new(),
            inputs: BTreeMap::new(),
        };
        if selected == "off" {
            return policy;
        }
        assert!(
            selected == "generate" || selected == "use",
            "unknown native profile mode"
        );
        assert!(
            env::var("HOST").as_deref() == Ok(TARGET)
                && env::var("TARGET").as_deref() == Ok(TARGET),
            "profile policy supports native Darwin arm64 only"
        );
        assert!(
            optimization == "-O3" && lto,
            "profile policy requires ordinary release/native LTO"
        );
        for variable in ["CFLAGS", "CPPFLAGS", "LLVM_PROFILE_FILE"] {
            assert!(
                env::var_os(variable).is_none_or(|value| value.is_empty()),
                "profile builds reject implicit flags or inherited profile output"
            );
        }
        let compiler = Path::new(compiler);
        assert!(
            compiler.is_absolute(),
            "profile policy requires an explicit absolute CC, not a wrapper"
        );
        let compiler = compiler.canonicalize().expect("resolve declared Clang");
        let bytes = fs::read(&compiler).expect("read declared Clang");
        assert!(
            bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe])
                || bytes.starts_with(&[0xca, 0xfe, 0xba, 0xbe])
                || bytes.starts_with(&[0xca, 0xfe, 0xba, 0xbf]),
            "CC must be a native Mach-O compiler"
        );
        let version = output(&compiler, "--version");
        assert!(
            version.starts_with("Apple clang version 17."),
            "unadmitted Clang profile format"
        );
        let root = PathBuf::from(
            env::var_os("SDKROOT").expect("profile policy requires explicit SDKROOT"),
        );
        assert!(
            root.is_absolute() && root.is_dir(),
            "SDKROOT must be an absolute installed SDK"
        );
        let settings = root.join("SDKSettings.json");
        policy.context = BTreeMap::from([
            ("schema".into(), "swiss-native-clang-source-v1".into()),
            ("scheme".into(), "fprofile-instr".into()),
            ("target".into(), TARGET.into()),
            ("compiler-fnv1a64".into(), id(&compiler)),
            ("compiler-version-fnv1a64".into(), text_id(&version)),
            ("sdk-settings-fnv1a64".into(), id(&settings)),
            ("cflags".into(), "-O3 -fPIC -flto".into()),
        ]);
        policy.mode = if selected == "generate" {
            let raw = PathBuf::from(
                env::var_os(RAW).expect("generation requires an explicit external raw pattern"),
            );
            let raw_text = raw.to_str().expect("profile pattern must be UTF-8");
            assert!(
                raw.is_absolute()
                    && raw.parent().is_some_and(Path::is_dir)
                    && !raw.starts_with(manifest)
                    && raw_text.contains("%p")
                    && raw_text.contains("%m")
                    && raw_text.ends_with(".profraw"),
                "use an external existing directory and %p-%m.profraw"
            );
            let runtime = absolute_file(RUNTIME);
            assert!(
                runtime.file_name().is_some_and(|name| name == RUNTIME_NAME)
                    && runtime_archive(&fs::read(&runtime).expect("read profile runtime")),
                "generation requires the declared Clang static profile runtime"
            );
            let located = PathBuf::from(
                output(&compiler, &format!("-print-file-name={RUNTIME_NAME}")).trim(),
            );
            assert!(
                located.canonicalize().expect("resolve Clang runtime") == runtime,
                "declared profile runtime is not this Clang's runtime"
            );
            Mode::Generate { raw, runtime }
        } else {
            let directory = manifest.join(BUNDLE);
            for name in PROFILE_FILES {
                let path = directory.join(name);
                assert!(
                    path.is_file() && !path.is_symlink(),
                    "packaged native profile bundle needs ordinary files"
                );
                println!("cargo:rerun-if-changed={}", path.display());
            }
            Mode::Use { directory }
        };
        policy
    }

    /// Bind the actual C graph, toolchain and flags before the first compile.
    /// Cargo may normalize its package manifest without changing that graph;
    /// effective manifest provenance is recorded separately, never aliased to
    /// Cargo.toml.orig. Stable labels omit checkout/OUT_DIR prefixes; profile
    /// name matching remains a separate mandatory qualification.
    pub(super) fn bind(
        &mut self,
        manifest: &Path,
        compiled: &[PathBuf],
        headers: &[PathBuf],
        generated: &[PathBuf],
    ) {
        if matches!(self.mode, Mode::Off) {
            return;
        }
        let mut inputs = BTreeMap::new();
        for path in compiled.iter().chain(headers) {
            let key = if generated.iter().any(|generated| generated == path) {
                format!(
                    "generated:{}",
                    path.file_name().expect("input name").to_string_lossy()
                )
            } else {
                path.strip_prefix(manifest)
                    .expect("native input outside declared roots")
                    .to_string_lossy()
                    .into_owned()
            };
            assert!(
                inputs.insert(key, id(path)).is_none(),
                "ambiguous native profile input label"
            );
        }
        for name in ["build.rs", "validation/source-pins.json"] {
            inputs.insert(name.into(), id(&manifest.join(name)));
        }
        for directory in ["build_support", "src/native"] {
            for entry in fs::read_dir(manifest.join(directory))
                .expect("read declared owned source directory")
            {
                let path = entry.expect("read owned source entry").path();
                assert!(
                    path.is_file(),
                    "profile source closure needs explicit review for nested owned inputs"
                );
                let name = path
                    .strip_prefix(manifest)
                    .expect("owned input")
                    .to_string_lossy()
                    .into_owned();
                inputs.insert(name, id(&path));
            }
        }
        let generated: BTreeMap<_, _> = generated
            .iter()
            .map(|path| {
                (
                    path.file_name()
                        .expect("generated name")
                        .to_string_lossy()
                        .into_owned(),
                    id(path),
                )
            })
            .collect();
        self.context
            .insert("inputs-fnv1a64".into(), table_id(&inputs));
        self.context
            .insert("generated-fnv1a64".into(), table_id(&generated));
        self.inputs = inputs;
        if let Mode::Use { directory } = &self.mode {
            let actual = descriptor(
                &fs::read_to_string(directory.join("descriptor.txt"))
                    .expect("read packaged descriptor"),
            )
            .expect("strict native profile descriptor");
            let mut expected = self.context.clone();
            expected.insert(
                "profile-fnv1a64".into(),
                id(&directory.join("counts.profdata")),
            );
            expected.insert("audit-fnv1a64".into(), id(&directory.join("audit.txt")));
            expected.insert(
                "training-inputs-fnv1a64".into(),
                id(&directory.join("training-inputs.json")),
            );
            expected.insert(
                "training-configuration-fnv1a64".into(),
                id(&directory.join("training-configuration.json")),
            );
            expected.insert(
                "record-admission".into(),
                "counts-only-external-link-and-paths-v1".into(),
            );
            for key in ["merge-tool-fnv1a64", "training-runtime-fnv1a64"] {
                let value = actual.get(key).expect("missing training provenance");
                assert!(
                    value.len() == 16 && value.bytes().all(|byte| byte.is_ascii_hexdigit()),
                    "invalid provenance checksum"
                );
                expected.insert(key.into(), value.clone());
            }
            assert!(
                actual == expected,
                "packaged profile does not match this exact source/toolchain policy"
            );
        }
    }

    /// Add only the declared source-instrumentation flag and strict use warnings.
    pub(super) fn compile_flags(&self, compiler: &mut Command) {
        match &self.mode {
            Mode::Off => {}
            Mode::Generate { raw, .. } => {
                compiler.arg(format!("-fprofile-instr-generate={}", raw.display()));
            }
            Mode::Use { directory } => {
                compiler.arg(format!(
                    "-fprofile-instr-use={}",
                    directory.join("counts.profdata").display()
                ));
                compiler.args([
                    "-Werror=profile-instr-missing",
                    "-Werror=profile-instr-out-of-date",
                ]);
            }
        }
    }

    /// Declare the runtime needed by the final Rust diagnostic link, not just C
    /// object compilation. Actual Cargo/linker/runtime qualification is required.
    pub(super) fn link_runtime(&self) {
        if let Mode::Generate { runtime, .. } = &self.mode {
            println!(
                "cargo:rustc-link-search=native={}",
                runtime.parent().expect("runtime parent").display()
            );
            println!("cargo:rustc-link-lib=static:+whole-archive=clang_rt.profile_osx");
        }
    }

    /// Record policy resources and the effective package manifest separately.
    /// The manifest checksum is provenance, not a C-context descriptor field;
    /// external package qualification binds its complete bytes with SHA-256.
    pub(super) fn record(&self, out: &Path, manifest: &Path) {
        let mode = match self.mode {
            Mode::Off => "off",
            Mode::Generate { .. } => "generate",
            Mode::Use { .. } => "use",
        };
        let mut text = format!("native-source-profile-mode: {mode}\n");
        if !matches!(self.mode, Mode::Off) {
            text.push_str(&format!(
                "effective-cargo-manifest-fnv1a64: {}\n",
                id(&manifest.join("Cargo.toml"))
            ));
        }
        for (key, value) in &self.context {
            text.push_str(&format!("{key}={value}\n"));
        }
        for (key, value) in &self.inputs {
            text.push_str(&format!("input-fnv1a64: {key} {value}\n"));
        }
        match &self.mode {
            Mode::Generate { raw, runtime } => {
                text.push_str(&format!(
                    "raw-pattern: {}\ntraining-runtime: {} {}\n",
                    raw.display(),
                    runtime.display(),
                    id(runtime)
                ));
            }
            Mode::Use { directory } => {
                for name in PROFILE_FILES {
                    text.push_str(&format!(
                        "profile-input-fnv1a64: {BUNDLE}/{name} {}\n",
                        id(&directory.join(name))
                    ));
                }
            }
            Mode::Off => {}
        }
        fs::write(out.join("native-profile-inputs.txt"), text)
            .expect("write native profile provenance");
        let mut ordinary = fs::OpenOptions::new()
            .append(true)
            .open(out.join("native-inputs.txt"))
            .expect("open native provenance");
        writeln!(ordinary, "build_support/native_profile.rs\nnative-profile-inputs.txt\nnative-source-profile-mode: {mode}").expect("append profile policy");
    }
}

#[cfg(test)]
mod tests {
    use super::{descriptor, runtime_archive, selected_mode};
    #[test]
    fn descriptor_rejects_ambiguous_or_padded_fields() {
        for source in [
            "scheme=x\nscheme=y\n",
            "scheme=\n",
            "=x\n",
            " scheme=x\n",
            "scheme=x \n",
            "scheme\n",
        ] {
            assert!(descriptor(source).is_err());
        }
    }
    #[test]
    fn descriptor_preserves_complete_fields_for_exact_context_comparison() {
        let actual = descriptor("schema=v1\nprofile-fnv1a64=0123456789abcdef\n").unwrap();
        assert_eq!(actual.len(), 2);
        assert_eq!(actual["schema"], "v1");
        assert_eq!(actual["profile-fnv1a64"], "0123456789abcdef");
    }

    // Structural fixture only: four FAT32 entries, each with a tiny archive
    // envelope. No installed runtime bytes or compiled objects are copied.
    fn universal_runtime() -> Vec<u8> {
        let mut bytes = vec![0; 120];
        bytes[..4].copy_from_slice(&0xcafe_babe_u32.to_be_bytes());
        bytes[4..8].copy_from_slice(&4_u32.to_be_bytes());
        for (index, (cpu, subtype)) in [(7_u32, 3_u32), (0x0100_0007, 3), (12, 9), (0x0100_000c, 0)]
            .into_iter()
            .enumerate()
        {
            let entry = 8 + index * 20;
            let offset = 88 + index * 8;
            for (field, value) in [cpu, subtype, offset as u32, 8, 3].into_iter().enumerate() {
                bytes[entry + field * 4..entry + field * 4 + 4]
                    .copy_from_slice(&value.to_be_bytes());
            }
            bytes[offset..offset + 8].copy_from_slice(b"!<arch>\n");
        }
        bytes
    }

    #[test]
    fn runtime_accepts_thin_and_big_endian_arm64_all_universal_archives() {
        assert!(runtime_archive(b"!<arch>\n"));
        assert!(runtime_archive(&universal_runtime()));
    }

    #[test]
    fn runtime_rejects_truncated_wrong_architecture_or_nonarchive_slices() {
        let valid = universal_runtime();
        for length in 0..valid.len() {
            assert!(!runtime_archive(&valid[..length]));
        }
        // Exact field mutations: unsupported magic/count, wrong target/subtype,
        // overlapping/out-of-range slice, invalid alignment, nonarchive bytes.
        for (offset, replacement) in [
            (0, 0xcafe_babf_u32),
            (4, 0),
            (4, 17),
            (68, 12),
            (72, 2),
            (76, 0),
            (76, u32::MAX),
            (80, u32::MAX),
            (84, 32),
            (84, 5),
            (112, 0),
        ] {
            let mut changed = valid.clone();
            changed[offset..offset + 4].copy_from_slice(&replacement.to_be_bytes());
            assert!(!runtime_archive(&changed));
        }
        let mut duplicate = valid;
        duplicate[8..12].copy_from_slice(&0x0100_000c_u32.to_be_bytes());
        duplicate[12..16].copy_from_slice(&0_u32.to_be_bytes());
        assert!(!runtime_archive(&duplicate));
    }

    #[test]
    fn only_missing_mode_defaults_to_off() {
        assert_eq!(
            selected_mode(Err(std::env::VarError::NotPresent)),
            Ok("off".into())
        );
        for mode in ["off", "generate", "use", "unknown", ""] {
            assert_eq!(selected_mode(Ok(mode.into())), Ok(mode.into()));
        }
    }

    #[cfg(unix)]
    #[test]
    fn present_nonunicode_mode_is_rejected_without_mutating_environment() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};
        let invalid = OsString::from_vec(vec![0xff]);
        assert!(selected_mode(Err(std::env::VarError::NotUnicode(invalid))).is_err());
    }
}
