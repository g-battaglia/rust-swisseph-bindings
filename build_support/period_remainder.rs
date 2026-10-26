//! Apply a bounded exact remainder only to the two pinned angle normalizers.

use super::c_source::{body_after, fingerprint, unique_offset};
use std::{fs, path::Path};

const HELPER: &str = "src/native/exact_period_remainder.h";
const SOURCE_FINGERPRINT: u64 = 0xe642_4aa4_7d15_66b1;
const HELPER_FINGERPRINT: u64 = 0xb27ace9b97ff276c;

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

/// Check the actual native C macros using the same compile-only ABI probe.
/// Cross compilation never executes a target program or guesses a header value.
pub fn period_admission() -> String {
    let mut guard = String::from("#include \"swephexp.h\"\n#include \"sweph.h\"\n");
    for (expression, bits) in periods() {
        // Derivation also checks that the fixed period's reciprocal is admitted.
        reciprocal_bits(bits).expect("reviewed positive period and reciprocal binade");
        guard.push_str(&format!(
            "_Static_assert((double)({expression}) == {}, \"native period differs from reciprocal proof\");\n",
            c_hex(bits)
        ));
    }
    guard
}

fn periods() -> [(&'static str, u64); 2] {
    [
        ("360.0", 360.0_f64.to_bits()),
        ("TWOPI", (2.0 * std::f64::consts::PI).to_bits()),
    ]
}

fn round_ratio_even(numerator: u128, denominator: u128) -> Result<u128, String> {
    if denominator == 0 {
        return Err("zero reciprocal denominator".into());
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let above_half = remainder > denominator - remainder;
    let tie_odd = remainder == denominator - remainder && quotient & 1 != 0;
    quotient
        .checked_add(u128::from(above_half || tie_odd))
        .ok_or("rounded quotient overflow".into())
}

fn encode(significand: u128, exponent: i32) -> Result<u64, String> {
    if !((1_u128 << 52)..(1_u128 << 53)).contains(&significand)
        || !(-1022..=1023).contains(&exponent)
    {
        return Err("reciprocal leaves the reviewed normal binade".into());
    }
    Ok(((exponent + 1023) as u64) << 52 | (significand as u64 & 0x000f_ffff_ffff_ffff))
}

fn reciprocal_bits(period: u64) -> Result<u64, String> {
    if !periods().iter().any(|(_, bits)| *bits == period) {
        return Err("only the two admitted native periods are supported".into());
    }
    let significand = u128::from((period & 0x000f_ffff_ffff_ffff) | (1_u64 << 52));
    let exponent = ((period >> 52) & 0x7ff) as i32 - 1023 - 52;
    // 2^105/P is the reciprocal significand in this same 53-bit binade.
    // RN-even plus one is exactly nextUp(RN-even(1/period)), without floats.
    let rounded = round_ratio_even(1_u128 << 105, significand)?;
    let upward = rounded
        .checked_add(1)
        .ok_or("reciprocal successor overflow")?;
    let product = significand * upward;
    let excess = product
        .checked_sub(1_u128 << 105)
        .ok_or("reciprocal is below exact")?;
    if excess == 0 || excess >= 3_u128 << 52 {
        return Err("reciprocal exceeds the strict rational error bound".into());
    }
    encode(upward, -53 - exponent)
}

fn c_hex(bits: u64) -> String {
    let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023;
    format!("0x1.{:013x}p{exponent:+}", bits & 0x000f_ffff_ffff_ffff)
}

fn transformed(source: &str, helper: &str) -> Result<String, String> {
    if fingerprint(source.as_bytes()) != SOURCE_FINGERPRINT
        || fingerprint(helper.as_bytes()) != HELPER_FINGERPRINT
    {
        return Err("unreviewed normalizer consumer or exact remainder helper".into());
    }
    let mut generated = source.to_owned();
    for ((name, period), (_, bits)) in [("swe_degnorm", "360.0"), ("swe_radnorm", "TWOPI")]
        .into_iter()
        .zip(periods())
    {
        let body = body_after(&generated, &format!("double CALL_CONV {name}(double x)"))?;
        let original = format!("fmod(x, {period})");
        let offset = unique_offset(&generated, body)? + unique_offset(body, &original)?;
        generated.replace_range(
            offset..offset + original.len(),
            &format!(
                "swe_exact_period_remainder(x, {period}, {})",
                c_hex(reciprocal_bits(bits)?)
            ),
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
    fn rounds_integer_ratios_with_even_ties_and_checks_the_binade() {
        for (numerator, denominator, expected) in
            [(5, 2, 2), (7, 2, 4), (4, 3, 1), (5, 3, 2), (8, 2, 4)]
        {
            assert_eq!(round_ratio_even(numerator, denominator).unwrap(), expected);
        }
        assert!(round_ratio_even(1, 0).is_err());
        assert_eq!(encode(1_u128 << 52, 0).unwrap(), 1.0_f64.to_bits());
        for significand in [(1_u128 << 52) - 1, 1_u128 << 53] {
            assert!(encode(significand, 0).is_err());
        }
        assert!(encode(1_u128 << 52, -1023).is_err());
        assert!(encode(1_u128 << 52, 1024).is_err());
    }

    #[test]
    fn derives_only_fixed_periods_with_strict_rational_error_bounds() {
        for (_, period) in periods() {
            let p = u128::from((period & 0x000f_ffff_ffff_ffff) | (1_u64 << 52));
            let reciprocal = reciprocal_bits(period).unwrap();
            let upward = u128::from((reciprocal & 0x000f_ffff_ffff_ffff) | (1_u64 << 52));
            let rounded = upward - 1;
            let numerator = 1_u128 << 105;
            // The predecessor is nearest-even; no floating-point division is
            // used in either the derivation or this independent distance test.
            let distance = numerator.abs_diff(p * rounded);
            assert!(2 * distance <= p);
            if 2 * distance == p {
                assert_eq!(rounded & 1, 0);
            }
            assert!(distance <= numerator.abs_diff(p * (rounded - 1)));
            assert!(distance <= numerator.abs_diff(p * (rounded + 1)));
            let excess = p * upward - numerator;
            assert!(excess > 0 && excess < 3_u128 << 52);
            // t<=2^50 gives <3/8 excess; final directed rounding adds <=1/4.
            assert!(excess * (1_u128 << 50) < 3_u128 << 102);
        }
        for unsupported in [
            0,
            1,
            1.0_f64.to_bits(),
            (-360.0_f64).to_bits(),
            f64::INFINITY.to_bits(),
            f64::NAN.to_bits(),
        ] {
            assert!(reciprocal_bits(unsupported).is_err());
        }
    }

    #[test]
    fn native_macro_admission_is_compile_only_and_rejects_a_changed_period() {
        use std::process::Command;
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let directory = std::env::temp_dir().join(format!("native-period-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let source = directory.join("admission.c");
        let compiler = std::env::var("CC").unwrap_or_else(|_| "cc".into());
        for changed in [false, true] {
            let prefix = if changed {
                "#include \"swephexp.h\"\n#include \"sweph.h\"\n#undef TWOPI\n#define TWOPI (2.0 * PI + 1.0)\n"
            } else {
                ""
            };
            fs::write(&source, format!("{prefix}{}", period_admission())).unwrap();
            let result = Command::new(&compiler)
                .arg("-fsyntax-only")
                .arg(format!("-I{}", root.join("swisseph").display()))
                .arg(&source)
                .output()
                .unwrap();
            assert_eq!(result.status.success(), !changed);
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn changes_only_the_two_period_remainder_calls() {
        let original = include_str!("../swisseph/swephlib.c");
        let helper = include_str!("../src/native/exact_period_remainder.h");
        let generated = transformed(original, helper).unwrap();
        assert_eq!(generated.matches("swe_exact_period_remainder(").count(), 2);
        let mut restored = generated.replace("#include \"exact_period_remainder.h\"\n", "");
        for (period, bits) in periods() {
            let call = format!(
                "swe_exact_period_remainder(x, {period}, {})",
                c_hex(reciprocal_bits(bits).unwrap())
            );
            restored = restored.replace(&call, &format!("fmod(x, {period})"));
        }
        assert_eq!(restored, original);
    }

    #[test]
    fn rejects_unreviewed_source_or_helper_even_outside_the_selected_body() {
        let source = include_str!("../swisseph/swephlib.c");
        let helper = include_str!("../src/native/exact_period_remainder.h");
        assert!(transformed(&format!("{source}\nvoid *new_consumer;"), helper).is_err());
        assert!(transformed(source, &helper.replace("fma(-q, period, a)", "a-q*period")).is_err());
    }
}
