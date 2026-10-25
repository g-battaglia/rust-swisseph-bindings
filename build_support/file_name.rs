//! Replace only the pinned filename suffix formatter with decimal byte output.
//!
//! Calendar selection, century arithmetic, prefix and special-body branches
//! stay native. Whole-file identities and a complete inverse constrain the edit.

use super::c_source::{fingerprint, unique_offset};
use std::{fs, path::Path};

const HELPER: &str = "src/native/file_name_suffix.h";
const SIGNATURE: &str = "void swi_gen_filename(double tjd, int ipli, char *fname)";
const SOURCE_FINGERPRINT: u64 = 0xe642_4aa4_7d15_66b1;
const PRECEDING_FINGERPRINT: u64 = 0x2e8a_8b17_e026_807e;
const HELPER_FINGERPRINT: u64 = 0x5d46_8d14_8e25_ec0d;
const ORIGINAL: &str = "  sprintf(fname + strlen(fname), \"%02d.%s\", icty, SE_FILE_SUFFIX);";
const REPLACEMENT: &str = "  swe_append_file_century(fname + strlen(fname), (unsigned int) icty);";
const INCLUDE: &str = "#include \"file_name_suffix.h\"\n";

/// Owned generator and helper inputs declared to Cargo and native identity.
pub const INPUTS: [&str; 2] = ["build_support/file_name.rs", HELPER];

/// Compose after the reviewed nutation, normalizer and precession overlays.
/// Unreviewed source/helper bytes or composition abort the build before C
/// compilation. Only the generated output is written; vendor files stay intact.
pub fn apply(native: &Path, generated: &Path) {
    let original = fs::read_to_string(native.join("swephlib.c")).expect("read filename consumer");
    let preceding = fs::read_to_string(generated).expect("read preceding numerical overlays");
    let helper = fs::read_to_string(HELPER).expect("read decimal filename helper");
    let output = transformed(&original, &preceding, &helper)
        .expect("reviewed native filename suffix construction");
    fs::write(generated, output).expect("write native filename consumer");
}

fn transformed(original: &str, preceding: &str, helper: &str) -> Result<String, String> {
    if fingerprint(original.as_bytes()) != SOURCE_FINGERPRINT
        || fingerprint(preceding.as_bytes()) != PRECEDING_FINGERPRINT
        || fingerprint(helper.as_bytes()) != HELPER_FINGERPRINT
    {
        return Err("unreviewed native filename source, composition or helper".into());
    }
    unique_offset(original, ORIGINAL)?;
    unique_offset(preceding, ORIGINAL)?;
    let mut output = preceding.replacen(ORIGINAL, REPLACEMENT, 1);
    let offset = unique_offset(&output, SIGNATURE)?;
    output.insert_str(offset, INCLUDE);
    let restored = output
        .replacen(INCLUDE, "", 1)
        .replacen(REPLACEMENT, ORIGINAL, 1);
    if restored != preceding {
        return Err("filename suffix construction is not byte-reversible".into());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preceding() -> String {
        // Cargo supplies the actual generated unit. Its complete inverse must
        // match the independently pinned preceding fingerprint before any test
        // may treat it as the qualified input.
        super::super::precession_frame::restore_lib(include_str!(concat!(
            env!("OUT_DIR"),
            "/swephlib.c"
        )))
        .expect("restore final local precession preparation")
        .replacen(INCLUDE, "", 1)
        .replacen(REPLACEMENT, ORIGINAL, 1)
    }

    #[test]
    fn replaces_only_the_final_pinned_filename_formatter() {
        let original = include_str!("../swisseph/swephlib.c");
        let helper = include_str!("../src/native/file_name_suffix.h");
        let prior = preceding();
        assert_eq!(fingerprint(prior.as_bytes()), PRECEDING_FINGERPRINT);
        let output = transformed(original, &prior, helper).unwrap();
        assert_eq!(output.matches(INCLUDE).count(), 1);
        assert_eq!(output.matches(REPLACEMENT).count(), 1);
        assert_eq!(
            output
                .replacen(INCLUDE, "", 1)
                .replacen(REPLACEMENT, ORIGINAL, 1),
            prior
        );
    }

    #[test]
    fn rejects_unreviewed_native_composition_or_helper() {
        let original = include_str!("../swisseph/swephlib.c");
        let helper = include_str!("../src/native/file_name_suffix.h");
        let prior = preceding();
        assert!(transformed(&format!("{original}\n"), &prior, helper).is_err());
        assert!(transformed(original, &format!("{prior}\n"), helper).is_err());
        assert!(transformed(original, &prior, &helper.replace("10u", "11u")).is_err());
        assert!(transformed(original, &prior.replace("%02d", "%03d"), helper).is_err());
    }
}
