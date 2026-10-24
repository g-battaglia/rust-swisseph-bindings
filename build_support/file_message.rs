//! Replace only the pinned missing-file formatter with bounded byte copying.
//!
//! Search order, file observations, native filename storage and the caller's
//! final diagnostic copy stay in the original consumer. No numerical C changes.

use super::c_source::{fingerprint, unique_offset};
use std::{fs, path::Path};

const HELPER: &str = "src/native/missing_file_message.h";
const SIGNATURE: &str = "FILE *swi_fopen(int ifno, char *fname, char *ephepath, char *serr)";
const SOURCE_FINGERPRINT: u64 = 0x811a_3282_bb39_3790;
const PRECEDING_FINGERPRINT: u64 = 0x990f_8fa6_425c_1ff5;
const HELPER_FINGERPRINT: u64 = 0x1413_91b5_a079_0267;
const ORIGINAL: &str = "  sprintf(s, \"SwissEph file '%s' not found in PATH '%s'\", fname, ephepath);\n  s[AS_MAXCH-1] = '\\0';";
const REPLACEMENT: &str = "  swe_missing_file_message(s, fname, ephepath);";
const INCLUDE: &str = "#include \"missing_file_message.h\"\n";

/// Owned generator/helper inputs declared to Cargo and native build identity.
pub const INPUTS: [&str; 2] = ["build_support/file_message.rs", HELPER];

/// Compose after the reviewed catalog and read-only-open overlays.
/// Whole-source/helper identities and a full byte inverse reject additional
/// substitutions or preceding changes. The dependency checkout stays read-only.
pub fn apply(native: &Path, generated: &Path) {
    let original = fs::read_to_string(native.join("sweph.c")).expect("read native file consumer");
    let preceding = fs::read_to_string(generated).expect("read preceding file overlays");
    let helper = fs::read_to_string(HELPER).expect("read bounded diagnostic helper");
    let output = transformed(&original, &preceding, &helper)
        .expect("reviewed missing-file diagnostic construction");
    fs::write(generated, output).expect("write exact native diagnostic consumer");
}

fn transformed(original: &str, preceding: &str, helper: &str) -> Result<String, String> {
    if fingerprint(original.as_bytes()) != SOURCE_FINGERPRINT
        || fingerprint(preceding.as_bytes()) != PRECEDING_FINGERPRINT
        || fingerprint(helper.as_bytes()) != HELPER_FINGERPRINT
    {
        return Err("unreviewed file consumer, preceding overlays or diagnostic helper".into());
    }
    unique_offset(original, ORIGINAL)?;
    unique_offset(preceding, ORIGINAL)?;
    let mut output = preceding.replacen(ORIGINAL, REPLACEMENT, 1);
    let include = unique_offset(&output, SIGNATURE)?;
    output.insert_str(include, INCLUDE);
    let restored = output
        .replacen(INCLUDE, "", 1)
        .replacen(REPLACEMENT, ORIGINAL, 1);
    if restored != preceding {
        return Err("diagnostic construction is not byte-reversible".into());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preceding() -> String {
        let original = include_str!("../swisseph/sweph.c");
        // Independently recover the two preceding, already-qualified edits.
        let old = "    while ((sp = strchr(fstar, ' ')) != NULL)\n      swi_strcpy(sp, sp+1);";
        let catalog = format!(
            "#include \"fixed_star_ascii_space_compaction.h\"\n{}",
            original.replacen(old, "    swe_compact_fixed_star_ascii_spaces(fstar);", 1)
        );
        catalog
            .replacen(
                "fopen(fnamp, BFILE_R_ACCESS)",
                "swe_open_readonly_file(fnamp)",
                1,
            )
            .replacen(
                SIGNATURE,
                &format!("#include \"readonly_file_open.h\"\n{SIGNATURE}"),
                1,
            )
    }

    #[test]
    fn preserves_every_byte_except_the_one_formatter_and_include() {
        let original = include_str!("../swisseph/sweph.c");
        let helper = include_str!("../src/native/missing_file_message.h");
        let prior = preceding();
        let output = transformed(original, &prior, helper).unwrap();
        assert_eq!(output.matches(REPLACEMENT).count(), 1);
        assert_eq!(output.matches(INCLUDE).count(), 1);
        assert_eq!(
            output
                .replacen(INCLUDE, "", 1)
                .replacen(REPLACEMENT, ORIGINAL, 1),
            prior
        );
        assert!(output.contains("  if (serr != NULL)\n    strcpy(serr, s);"));
        assert!(output.contains("swe_open_readonly_file(fnamp)"));
    }

    #[test]
    fn rejects_unreviewed_sources_helper_or_composition() {
        let original = include_str!("../swisseph/sweph.c");
        let helper = include_str!("../src/native/missing_file_message.h");
        let prior = preceding();
        assert!(transformed(&format!("{original}\n"), &prior, helper).is_err());
        assert!(transformed(original, &format!("{prior}\n"), helper).is_err());
        assert!(transformed(original, &prior, &helper.replace("PATH", "path")).is_err());
        assert!(transformed(original, &prior.replace("AS_MAXCH-1", "AS_MAXCH-2"), helper).is_err());
    }
}
