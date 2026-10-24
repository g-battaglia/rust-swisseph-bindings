//! Compare the native filename builder and integer suffix with original C.
use std::{fs, path::Path, process::Command, time::SystemTime};

#[allow(dead_code)]
#[path = "../build_support/c_source.rs"]
mod c_source;
#[allow(dead_code)]
#[path = "../build_support/file_name.rs"]
mod file_name;

#[test]
fn native_filename_suffix_and_date_branches_match_original_c() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let unique = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("file-name-{}-{unique}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let source = fs::read_to_string(root.join("swisseph/swephlib.c")).unwrap();
    let signature = "void swi_gen_filename(double tjd, int ipli, char *fname)";
    let body = c_source::body_after(&source, signature).unwrap();
    let notices = &source[..source.find("#include").unwrap()];
    let oracle = directory.join("oracle.c");
    fs::write(
        &oracle,
        format!(
            r#"{notices}
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "swephexp.h"
#include "sweph.h"
#include "swephlib.h"
void oracle_swi_gen_filename(double tjd, int ipli, char *fname)
{{{body}}}
"#
        ),
    )
    .unwrap();
    let executable = directory.join("control");
    let compiler = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let compiled = Command::new(compiler)
        .args(["-O3", "-fno-fast-math"])
        .arg(format!("-I{}", root.join("swisseph").display()))
        .arg(format!("-I{}", root.join("src/native").display()))
        .arg(root.join("tests/file_name_suffix.c"))
        .arg(&oracle)
        .arg(Path::new(env!("OUT_DIR")).join("libswisseph.a"))
        .args(["-lm", "-o"])
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let result = Command::new(&executable).output().unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    println!("{}", String::from_utf8_lossy(&result.stdout));
    fs::remove_dir_all(directory).unwrap();
}
