//! Apply one read-only file-open substitution to the pinned native consumer.

use super::c_source::{body_after, fingerprint, unique_offset};
use std::{fs, path::Path};

const HELPER: &str = "src/native/readonly_file_open.h";
const SIGNATURE: &str = "FILE *swi_fopen(int ifno, char *fname, char *ephepath, char *serr)";
const SOURCE_FINGERPRINT: u64 = 0x811a_3282_bb39_3790;
const CATALOG_OVERLAY_FINGERPRINT: u64 = 0x9c35_ac2d_2d35_c3d8;
const CONSUMER_FINGERPRINT: u64 = 0x99390016816f5feb;
const HELPER_FINGERPRINT: u64 = 0xef97532d0974fa3e;

/// Inputs declared to Cargo and the linked native-input manifest.
pub const INPUTS: [&str; 2] = ["build_support/file_open.rs", HELPER];

/// Compose after the reviewed catalog overlay. Whole-source, whole-consumer
/// and helper identities prevent changes to another fopen or path policy.
pub fn apply(native: &Path, generated: &Path) {
    let original = fs::read_to_string(native.join("sweph.c")).expect("read native file consumer");
    let catalog = fs::read_to_string(generated).expect("read preceding catalog overlay");
    let helper = fs::read_to_string(HELPER).expect("read read-only file helper");
    let transformed =
        transformed(&original, &catalog, &helper).expect("reviewed file-open overlay");
    fs::write(generated, transformed).expect("write generated file-open consumer");
}

fn transformed(original: &str, catalog: &str, helper: &str) -> Result<String, String> {
    if fingerprint(original.as_bytes()) != SOURCE_FINGERPRINT
        || fingerprint(catalog.as_bytes()) != CATALOG_OVERLAY_FINGERPRINT
        || fingerprint(helper.as_bytes()) != HELPER_FINGERPRINT
    {
        return Err("unreviewed native source, preceding overlay or file helper".into());
    }
    let body = body_after(original, SIGNATURE)?;
    let start = unique_offset(original, SIGNATURE)?;
    let end = unique_offset(original, body)? + body.len() + 1;
    if fingerprint(
        original
            .as_bytes()
            .get(start..end)
            .ok_or("consumer boundary")?,
    ) != CONSUMER_FINGERPRINT
        || body_after(catalog, SIGNATURE)? != body
    {
        return Err("unreviewed complete file-open consumer".into());
    }
    let original_call = "fopen(fnamp, BFILE_R_ACCESS)";
    let offset = unique_offset(catalog, body)? + unique_offset(body, original_call)?;
    let mut generated = catalog.to_owned();
    generated.replace_range(
        offset..offset + original_call.len(),
        "swe_open_readonly_file(fnamp)",
    );
    let include = unique_offset(&generated, SIGNATURE)?;
    generated.insert_str(include, "#include \"readonly_file_open.h\"\n");
    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> String {
        let source = include_str!("../swisseph/sweph.c");
        let old = "    while ((sp = strchr(fstar, ' ')) != NULL)\n      swi_strcpy(sp, sp+1);";
        format!(
            "#include \"fixed_star_ascii_space_compaction.h\"\n{}",
            source.replacen(old, "    swe_compact_fixed_star_ascii_spaces(fstar);", 1)
        )
    }

    #[test]
    fn replaces_only_the_one_pinned_read_only_call() {
        let source = include_str!("../swisseph/sweph.c");
        let helper = include_str!("../src/native/readonly_file_open.h");
        let prior = catalog();
        let generated = transformed(source, &prior, helper).unwrap();
        assert_eq!(generated.matches("swe_open_readonly_file(").count(), 1);
        assert_eq!(
            generated
                .replace(
                    "swe_open_readonly_file(fnamp)",
                    "fopen(fnamp, BFILE_R_ACCESS)"
                )
                .replace("#include \"readonly_file_open.h\"\n", ""),
            prior
        );
    }

    #[test]
    fn rejects_unreviewed_consumer_helper_or_preceding_overlay() {
        let source = include_str!("../swisseph/sweph.c");
        let helper = include_str!("../src/native/readonly_file_open.h");
        let prior = catalog();
        assert!(transformed(&format!("{source}\n"), &prior, helper).is_err());
        assert!(transformed(source, &format!("{prior}\n"), helper).is_err());
        assert!(transformed(source, &prior, &helper.replace("O_RDONLY", "O_RDWR")).is_err());
    }
}
