//! Apply a bounded exact remainder only to the two pinned angle normalizers.

use super::c_source::{body_after, fingerprint, unique_offset};
use std::{fs, path::Path};

const HELPER: &str = "src/native/exact_period_remainder.h";
const SOURCE_FINGERPRINT: u64 = 0xe642_4aa4_7d15_66b1;
const HELPER_FINGERPRINT: u64 = 0x18b7_cb73_471d_ef16;

/// Complete declared inputs for Cargo and the linked native-input manifest.
pub const INPUTS: [&str; 2] = ["build_support/period_remainder.rs", HELPER];

/// Transform an otherwise unchanged generated swephlib.c. Complete reviewed
/// consumer/helper identities prevent accidental expansion to other fmod uses.
pub fn apply(native: &Path, generated: &Path) {
    let source = fs::read_to_string(native.join("swephlib.c")).expect("read angle normalizers");
    let helper = fs::read_to_string(HELPER).expect("read exact period remainder");
    assert_eq!(
        fs::read_to_string(generated).unwrap(),
        source,
        "unreviewed preceding source overlay"
    );
    fs::write(
        generated,
        transformed(&source, &helper).expect("reviewed exact normalizer remainder"),
    )
    .expect("write generated normalizers");
}

fn transformed(source: &str, helper: &str) -> Result<String, String> {
    if fingerprint(source.as_bytes()) != SOURCE_FINGERPRINT
        || fingerprint(helper.as_bytes()) != HELPER_FINGERPRINT
    {
        return Err("unreviewed normalizer consumer or exact remainder helper".into());
    }
    let mut generated = source.to_owned();
    for (name, period) in [("swe_degnorm", "360.0"), ("swe_radnorm", "TWOPI")] {
        let body = body_after(&generated, &format!("double CALL_CONV {name}(double x)"))?;
        let original = format!("fmod(x, {period})");
        let offset = unique_offset(&generated, body)? + unique_offset(body, &original)?;
        generated.replace_range(
            offset..offset + original.len(),
            &format!("swe_exact_period_remainder(x, {period})"),
        );
    }
    let include = unique_offset(&generated, "#include <string.h>")?;
    generated.insert_str(include, "#include \"exact_period_remainder.h\"\n");
    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_only_the_two_period_remainder_calls() {
        let original = include_str!("../swisseph/swephlib.c");
        let helper = include_str!("../src/native/exact_period_remainder.h");
        let generated = transformed(original, helper).unwrap();
        assert_eq!(generated.matches("swe_exact_period_remainder(").count(), 2);
        assert_eq!(
            generated
                .replace("swe_exact_period_remainder(", "fmod(")
                .replace("#include \"exact_period_remainder.h\"\n", ""),
            original
        );
    }

    #[test]
    fn rejects_unreviewed_source_or_helper_even_outside_the_selected_body() {
        let source = include_str!("../swisseph/swephlib.c");
        let helper = include_str!("../src/native/exact_period_remainder.h");
        assert!(transformed(&format!("{source}\nvoid *new_consumer;"), helper).is_err());
        assert!(transformed(source, &helper.replace("fma(-q, period, a)", "a-q*period")).is_err());
    }
}
