//! Build script: compile the pinned, unmodified native Swiss Ephemeris
//! library and link it into this crate.
//!
//! The submodule `swisseph/` must be checked out at the commit recorded in
//! `validation/source-pins.json`. This script never updates the submodule;
//! when its sources are absent the build fails with a message that names
//! the missing prerequisite instead of an obscure compiler error.
//!
//! Library translation units follow the upstream `Makefile` (`SWEOBJ`):
//! command line tools (`swetest`, `swevents`, `swemini`, `obama`), test
//! oracles and unrelated programs are excluded. All compiler outputs live
//! under Cargo `OUT_DIR`; nothing is written into the submodule.

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
    let mut objects = Vec::with_capacity(SOURCES.len());
    for name in SOURCES {
        let object = out.join(format!("{name}.o"));
        let status = Command::new(&cc)
            .arg("-O2")
            .arg("-fPIC")
            .arg(format!("-I{}", native.display()))
            .arg("-c")
            .arg(native.join(name))
            .arg("-o")
            .arg(&object)
            .status();
        match status {
            Ok(status) if status.success() => objects.push(object),
            Ok(status) => {
                panic!(
                    "compiling swisseph/{name} failed (exit {status}); see compiler output above"
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
    write_input_manifest(&out, &cc);
}

/// Record which native inputs produced the linked archive without touching
/// the submodule checkout.
///
/// The manifest names the compiler, the fixed compile options and every
/// tracked native input, plus a best-effort compiler version line. It
/// never shells out to Git, so it also works when built from an extracted
/// package without version-control metadata.
fn write_input_manifest(out: &Path, cc: &str) {
    let manifest = out.join("native-inputs.txt");
    let mut content = String::from("pinned native inputs (see validation/source-pins.json):\n");
    for name in SOURCES.iter().chain(HEADERS.iter()) {
        content.push_str(&format!("swisseph/{name}\n"));
    }
    content.push_str(&format!("compiler: {cc}\n"));
    content.push_str("cflags: -O2 -fPIC\n");
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
