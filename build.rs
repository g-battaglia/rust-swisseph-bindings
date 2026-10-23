//! Build script: compile the pinned native Swiss Ephemeris
//! library and link it into this crate.
//!
//! The submodule `swisseph/` must be checked out at the commit recorded in
//! `validation/source-pins.json`. This script never updates the submodule;
//! when its sources are absent the build fails with a message that names
//! the missing prerequisite instead of an obscure compiler error.
//!
//! Engine translation units follow the upstream `Makefile` (`SWEOBJ`):
//! command line tools (`swetest`, `swevents`, `swemini`, `obama`), test
//! oracles and unrelated programs are excluded. All compiler outputs live
//! under Cargo `OUT_DIR`; nothing is written into the submodule. The separately
//! attributed 2.10.03 degree splitter is an explicit versioned compatibility
//! unit under `src/native/`, without changing any pinned engine source.
//! One declared build overlay compacts ASCII spaces in a catalog name in one
//! pass. Only the generated OUT_DIR copy changes; lookup order and numerical
//! calculations remain the pinned implementation.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Native translation units selected from the upstream build metadata
/// (`SWEOBJ` in `swisseph/Makefile` at the pinned commit).
const SOURCES: &[&str] = &[
    "swedate.c",
    "swehouse.c",
    "swejpl.c",
    "swemmoon.c",
    "swemplan.c",
    "sweph.c",
    "swephlib.c",
    "swecl.c",
    "swehel.c",
];

/// Headers that affect the compiled library; listed so Cargo rebuilds when
/// any of them changes.
///
/// This is the exact union reported by `cc -MM -I swisseph` over every
/// entry of `SOURCES` at the pinned commit (notably `swenut2000a.h`,
/// included by `swephlib.c`). `swedll.h` is intentionally absent: none of
/// the nine selected translation units includes it under the flags used
/// below. Re-verify with `cc -MM` if the unit set or flags change.
const HEADERS: &[&str] = &[
    "swephexp.h",
    "sweph.h",
    "swephlib.h",
    "swehouse.h",
    "swejpl.h",
    "sweodef.h",
    "swemptab.h",
    "swenut2000a.h",
];

/// Versioned upstream utility kept separately from the current engine.
const COMPAT_SOURCE: &str = "src/native/split_deg_2_10_03.c";

/// Lossless string operation compiled into the generated catalog reader.
const CATALOG_COMPACTION: &str = "src/native/fixed_star_ascii_space_compaction.h";

/// Exact pinned fragment replaced once, after the existing name truncation.
const CATALOG_SPACE_LOOP: &str =
    "    while ((sp = strchr(fstar, ' ')) != NULL)\n      swi_strcpy(sp, sp+1);";

/// C probe asserting the ABI widths the Rust declarations rely on.
///
/// `int32` is `int` (32 bit) outside 16-bit targets and `double` is IEEE
/// binary64; the probe turns a platform mismatch into a build failure here
/// instead of silent FFI corruption later.
const ABI_PROBE: &str = r#"
#include "sweodef.h"
#if !defined(_SWEODEF_INCLUDED)
#error "sweodef.h did not provide the expected ABI types"
#endif
_Static_assert(sizeof(int32) == 4, "native int32 must be 32 bit");
_Static_assert(sizeof(double) == 8, "native double must be 64 bit");
_Static_assert(sizeof(char) == 1, "native char must be 8 bit");
int main(void) { return 0; }
"#;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let native = manifest.join("swisseph");
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    // Rebuild when any compiled input changes. The submodule directory
    // itself is tracked file by file so unrelated submodule files
    // (documents, data, tools) do not trigger rebuilds.
    // `CC` is the only compiler environment variable consumed below, so
    // a changed compiler must rerun this script instead of silently
    // reusing the cached archive.
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-changed={COMPAT_SOURCE}");
    println!("cargo:rerun-if-changed={CATALOG_COMPACTION}");
    for name in SOURCES.iter().chain(HEADERS.iter()) {
        println!("cargo:rerun-if-changed=swisseph/{name}");
    }

    for name in SOURCES {
        let path = native.join(name);
        if !path.is_file() {
            panic!(
                "missing native source swisseph/{name} at the pinned commit; \
                 initialize the submodule (`git submodule update --init --checkout -- swisseph`) \
                 without changing its pin (see validation/source-pins.json)"
            );
        }
    }

    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_string());
    // Use ordinary full optimization for a release build. This never enables
    // fast-math, reassociation, reduced precision or host-specific instructions.
    let optimization = if env::var("OPT_LEVEL").as_deref() == Ok("3") {
        "-O3"
    } else {
        "-O2"
    };
    // The Darwin linker consumes the compiler's native bitcode archive and
    // optimizes across the C translation units. Keep ordinary floating-point
    // semantics and portable target instructions; development builds and
    // other linkers retain ordinary native objects.
    let native_lto = optimization == "-O3"
        && env::var("TARGET").is_ok_and(|target| target.ends_with("-apple-darwin"));

    // 1. ABI probe: verify native integer widths before compiling.
    let probe_path = out.join("abi_probe.c");
    fs::write(&probe_path, ABI_PROBE).expect("write ABI probe");
    let probe_status = Command::new(&cc)
        .arg("-fsyntax-only")
        .arg(format!("-I{}", native.display()))
        .arg(&probe_path)
        .status();
    match probe_status {
        Ok(status) if status.success() => {}
        Ok(status) => panic!(
            "native ABI probe failed (exit {status}); refusing to guess integer widths \
             for the pinned declarations in swisseph/sweodef.h"
        ),
        Err(err) => panic!(
            "cannot run C compiler `{cc}` for the native ABI probe: {err}; \
             install a C toolchain (no sudo/container setup is provided here)"
        ),
    }

    // 2. Compile each translation unit into OUT_DIR.
    let mut objects = Vec::with_capacity(SOURCES.len() + 1);
    let catalog_input = compacted_catalog_source(&native, &out);
    let inputs = SOURCES
        .iter()
        .map(|name| {
            if *name == "sweph.c" {
                catalog_input.clone()
            } else {
                native.join(name)
            }
        })
        .chain(std::iter::once(manifest.join(COMPAT_SOURCE)));
    for input in inputs {
        let name = input
            .file_name()
            .expect("native source filename")
            .to_string_lossy();
        let object = out.join(format!("{name}.o"));
        let mut compiler = Command::new(&cc);
        compiler.arg(optimization).arg("-fPIC");
        if native_lto {
            compiler.arg("-flto");
        }
        let status = compiler
            .arg(format!("-I{}", native.display()))
            .arg(format!("-I{}", manifest.join("src/native").display()))
            .arg("-c")
            .arg(&input)
            .arg("-o")
            .arg(&object)
            .status();
        match status {
            Ok(status) if status.success() => objects.push(object),
            Ok(status) => {
                panic!(
                    "compiling {} failed (exit {status}); see compiler output above",
                    input.display()
                )
            }
            Err(err) => panic!("cannot run C compiler `{cc}`: {err}"),
        }
    }

    // 3. Archive a static library and link it.
    let archive = out.join("libswisseph.a");
    let mut ar = Command::new("ar");
    ar.arg("crus").arg(&archive);
    for object in &objects {
        ar.arg(object);
    }
    match ar.status() {
        Ok(status) if status.success() => {}
        Ok(status) => panic!("archiving libswisseph.a failed (exit {status})"),
        Err(err) => panic!("cannot run `ar` to archive the native objects: {err}"),
    }

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=swisseph");
    write_input_manifest(&out, &cc, optimization, native_lto);
}

/// Generate exactly one lossless catalog-string substitution in OUT_DIR.
/// A changed or missing pinned fragment fails closed before compilation;
/// the dependency checkout and every numerical statement remain untouched.
fn compacted_catalog_source(native: &Path, out: &Path) -> PathBuf {
    let original = fs::read_to_string(native.join("sweph.c")).expect("read pinned catalog source");
    assert_eq!(
        original.matches(CATALOG_SPACE_LOOP).count(),
        1,
        "pinned fixed-star space loop differs; refusing an unreviewed transformation"
    );
    let transformed = original.replacen(
        CATALOG_SPACE_LOOP,
        "    swe_compact_fixed_star_ascii_spaces(fstar);",
        1,
    );
    let generated = out.join("sweph.c");
    fs::write(
        &generated,
        format!("#include \"fixed_star_ascii_space_compaction.h\"\n{transformed}"),
    )
    .expect("write generated catalog source");
    generated
}

/// Record which native inputs produced the linked archive without touching
/// the submodule checkout.
///
/// The manifest names the compiler, the fixed compile options and every
/// tracked native input, plus a best-effort compiler version line. It
/// never shells out to Git, so it also works when built from an extracted
/// package without version-control metadata.
fn write_input_manifest(out: &Path, cc: &str, optimization: &str, native_lto: bool) {
    let manifest = out.join("native-inputs.txt");
    let mut content = String::from("pinned native inputs (see validation/source-pins.json):\n");
    for name in SOURCES.iter().chain(HEADERS.iter()) {
        content.push_str(&format!("swisseph/{name}\n"));
    }
    content.push_str(&format!("{COMPAT_SOURCE}\n"));
    content.push_str(&format!("{CATALOG_COMPACTION}\n"));
    content
        .push_str("native-overlay: fixed-star ASCII-space compaction; generated OUT_DIR/sweph.c\n");
    content.push_str(&format!("compiler: {cc}\n"));
    let lto_flag = if native_lto { " -flto" } else { "" };
    content.push_str(&format!("cflags: {optimization} -fPIC{lto_flag}\n"));
    let version_line = Command::new(cc)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .map(str::to_owned)
            } else {
                None
            }
        })
        .unwrap_or_else(|| "<compiler version unavailable>".to_string());
    content.push_str(&format!("compiler-version: {version_line}\n"));
    fs::write(manifest, content).expect("write native input manifest");
}
