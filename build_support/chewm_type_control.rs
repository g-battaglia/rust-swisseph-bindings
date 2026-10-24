//! Specialize only the literal kind of the pinned lunar perturbation reader.
//!
//! Tables, macro branches and every numeric statement remain native source.
//! Four private copies retain the original row/angle loops and floating stages;
//! only their already literal kind becomes a local compile-time constant.

use super::c_source::{body_after, fingerprint, unique_offset};
use std::fs;
use std::path::{Path, PathBuf};

const SOURCE_FINGERPRINT: u64 = 0x5b6b69606284fdbb;
const PROTOTYPE: &str = "static void chewm(const short *pt, int nlines, int nangles, \n  \t\t\t\t     int typflg, double *ans );";
const DEFINITION: &str =
    "static void chewm(const short *pt, int nlines, int nangles, int typflg, double *ans )";

/// Table, row-count macro, literal kind and exact textual call-site count.
/// Counts include both native MOSH_MOON_200 branches; no branch is selected here.
const CALLS: [(&str, &str, u8, usize); 6] = [
    ("LR", "NLR", 1, 1),
    ("LRT", "NLRT", 1, 2),
    ("LRT2", "NLRT2", 2, 2),
    ("MB", "NMB", 3, 1),
    ("BT", "NBT", 4, 2),
    ("BT2", "NBT2", 4, 2),
];

/// Owned generator input declared to Cargo and the native-input manifest.
pub const INPUTS: [&str; 1] = ["build_support/chewm_type_control.rs"];

/// Write an otherwise identical native lunar source into Cargo OUT_DIR.
/// Complete source identity, literal call sites and inverse byte equality are
/// required; missing or changed input aborts before any C compilation.
pub fn generate(native: &Path, out: &Path) -> PathBuf {
    let original = fs::read_to_string(native.join("swemmoon.c")).expect("read lunar reader");
    let generated = transformed(&original).expect("reviewed literal lunar reader kinds");
    let path = out.join("swemmoon.c");
    fs::write(&path, generated).expect("write generated lunar reader");
    path
}

fn private_signature(original: &str, kind: u8) -> String {
    original
        .replacen("chewm(", &format!("chewm_type_{kind}("), 1)
        .replacen("int typflg,", "int supplied_typflg,", 1)
}

fn private_function(body: &str, kind: u8) -> String {
    // Keep the call's argument list unchanged. Proven literal arguments agree
    // with this constant; the unused supplied value is admitted explicitly.
    // The original body is one verbatim slice, including the low-k radius guard.
    format!(
        "{}\n{{\n  const int typflg = {kind};{body}  (void)supplied_typflg;\n}}",
        private_signature(DEFINITION, kind)
    )
}

fn call(table: &str, rows: &str, kind: u8) -> String {
    format!("chewm( {table}, {rows}, 4, {kind}, moonpol );")
}

fn replace_exact(source: &str, old: &str, new: &str, count: usize) -> Result<String, String> {
    if source.matches(old).count() != count {
        return Err("lunar reader declaration or literal calls differ".into());
    }
    Ok(source.replace(old, new))
}

fn transformed(source: &str) -> Result<String, String> {
    if fingerprint(source.as_bytes()) != SOURCE_FINGERPRINT {
        return Err("unreviewed lunar consumer or tables".into());
    }
    let body = body_after(source, DEFINITION)?;
    let original_function = format!("{DEFINITION}\n{{{body}}}");
    unique_offset(source, &original_function)?;
    let prototypes = (1..=4)
        .map(|kind| private_signature(PROTOTYPE, kind))
        .collect::<Vec<_>>()
        .join("\n");
    let functions = (1..=4)
        .map(|kind| private_function(body, kind))
        .collect::<Vec<_>>()
        .join("\n\n");
    let mut edits = vec![
        (PROTOTYPE.to_owned(), prototypes, 1),
        (original_function, functions, 1),
    ];
    for (table, rows, kind, count) in CALLS {
        let original = call(table, rows, kind);
        let typed = original.replacen("chewm(", &format!("chewm_type_{kind}("), 1);
        edits.push((original, typed, count));
    }
    let mut generated = source.to_owned();
    for (old, new, count) in &edits {
        generated = replace_exact(&generated, old, new, *count)?;
    }
    if generated.contains("chewm(") {
        return Err("unconsumed generic lunar reader".into());
    }
    // Undo only the declared edits. Equality covers all tables, integer/float
    // tokens, sscc code, callers, macro branches, comments and license text.
    let mut restored = generated.clone();
    for (old, new, count) in edits.iter().rev() {
        restored = replace_exact(&restored, new, old, *count)?;
    }
    if restored != source {
        return Err("lunar reader transformation is not byte-reversible".into());
    }
    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NATIVE: &str = include_str!("../swisseph/swemmoon.c");

    #[test]
    fn preserves_each_complete_numeric_body_and_the_closed_literal_calls() {
        let original = body_after(NATIVE, DEFINITION).unwrap();
        let generated = transformed(NATIVE).unwrap();
        for kind in 1..=4 {
            let signature = private_signature(DEFINITION, kind);
            let body = body_after(&generated, &signature).unwrap();
            assert_eq!(
                body.strip_prefix(&format!("\n  const int typflg = {kind};"))
                    .unwrap()
                    .strip_suffix("  (void)supplied_typflg;\n")
                    .unwrap(),
                original
            );
        }
        for (table, rows, kind, count) in CALLS {
            let original = call(table, rows, kind);
            assert_eq!(NATIVE.matches(&original).count(), count);
            let typed = original.replacen("chewm(", &format!("chewm_type_{kind}("), 1);
            assert_eq!(generated.matches(&typed).count(), count);
        }
        assert!(!generated.contains("chewm("));
    }

    #[test]
    fn keeps_both_macro_branches_and_the_native_low_word_radius_guard() {
        let generated = transformed(NATIVE).unwrap();
        for marker in ["#ifdef MOSH_MOON_200", "#define NMB 56", "#define NMB 77"] {
            assert_eq!(
                generated.matches(marker).count(),
                NATIVE.matches(marker).count()
            );
        }
        assert_eq!(
            generated
                .matches("if( k ) ans[2] += (10000.0 * j  + k) * cv;")
                .count(),
            4
        );
    }

    #[test]
    fn rejects_changed_tokens_calls_and_unreviewed_consumers() {
        for changed in [
            NATIVE.replacen(
                "chewm( LR, NLR, 4, 1, moonpol );",
                "chewm( LR, NLR, 4, 2, moonpol );",
                1,
            ),
            NATIVE.replacen("if( k ) ans[2]", "if( j ) ans[2]", 1),
            NATIVE.replacen("22639, 5858", "22639, 5859", 1),
            format!("{NATIVE}\nvoid *escape = chewm;"),
        ] {
            assert!(transformed(&changed).is_err());
        }
        assert!(replace_exact("one", "missing", "two", 1).is_err());
        assert!(replace_exact("one one", "one", "two", 1).is_err());
    }
}
