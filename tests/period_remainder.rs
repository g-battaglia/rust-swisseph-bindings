//! Independent libc and integer controls for the bounded exact C remainder.

use std::{fs, process::Command, time::SystemTime};

#[allow(dead_code)]
#[path = "../build_support/c_source.rs"]
mod c_source;

#[test]
fn exact_period_remainder_matches_libc_and_integer_oracles_in_all_rounding_modes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let unique = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("period-remainder-{}-{unique}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let source = fs::read_to_string(root.join("swisseph/swephlib.c")).unwrap();
    let begin = source
        .find("double CALL_CONV swe_degnorm(double x)")
        .unwrap();
    let end = source[begin..]
        .find("double CALL_CONV swe_deg_midp(")
        .unwrap()
        + begin;
    let normalizers = source[begin..end]
        .replace("swe_degnorm", "oracle_degnorm")
        .replace("swe_radnorm", "oracle_radnorm");
    let oracle = directory.join("oracle.c");
    let oracle_source = format!(
        r#"#pragma STDC FENV_ACCESS ON
#include "swephexp.h"
#include "sweph.h"

{normalizers}
double oracle_fmod(double x, double period)
{{
    return fmod(x, period);
}}
"#
    );
    fs::write(&oracle, oracle_source).unwrap();
    let generated =
        fs::read_to_string(std::path::Path::new(env!("OUT_DIR")).join("swephlib.c")).unwrap();
    let mut control = fs::read_to_string(root.join("tests/period_remainder.c")).unwrap();
    for (name, period, placeholder) in [
        ("swe_degnorm", "360.0", "@DEGREE_RECIPROCAL@"),
        ("swe_radnorm", "TWOPI", "@RADIAN_RECIPROCAL@"),
    ] {
        let body = c_source::body_after(&generated, &format!("double CALL_CONV {name}(double x)"))
            .unwrap();
        let call = format!("swe_exact_period_remainder(x, {period}, ");
        let literal = body.split_once(&call).unwrap().1.split_once(')').unwrap().0;
        control = control.replace(placeholder, literal);
    }
    let control_source = directory.join("control.c");
    fs::write(&control_source, control).unwrap();
    // Exercise the portable admission on this host as well as the selected ABI
    // path. Both binaries retain the same libc/integer oracles and input grid.
    for (name, scalar) in [("native", false), ("scalar", true)] {
        let executable = directory.join(name);
        let mut compiler = Command::new("cc");
        compiler.args([
            "-O3",
            "-frounding-math",
            "-fno-builtin-fmod",
            "-fno-fast-math",
        ]);
        if scalar {
            compiler.arg("-DSWE_EXACT_PERIOD_FORCE_SCALAR_ADMISSION=1");
        }
        let compiled = compiler
            .arg(format!("-I{}", root.join("swisseph").display()))
            .arg(format!("-I{}", root.join("src/native").display()))
            .arg(&control_source)
            .arg(&oracle)
            .arg("-lm")
            .arg("-o")
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
        println!("{name}: {}", String::from_utf8_lossy(&result.stdout));
    }
    fs::remove_dir_all(directory).unwrap();
}
