//! Store exact, immutable integer nutation coefficients as native doubles.
//!
//! Every original use promotes these integers to double before arithmetic.
//! Moving that exact conversion to static initialization removes repeated
//! conversion instructions, without changing any expression or table token.

use super::c_source::{body_after, fingerprint, integers, unique_offset, without_comments};
use std::fs;
use std::path::{Path, PathBuf};

const CONSUMER: &str = "static int calc_nutation_iau2000ab(double J, double *nutlo)";
const CONSUMER_FINGERPRINT: u64 = 0xf4f2_6da7_bee2_0ae0;
const TABLES: [(&str, u8, usize, &str); 4] = [
    ("nls", 16, 5, "NLS"),
    ("cls", 32, 6, "NLS"),
    ("npl", 16, 14, "NPL"),
    ("icpl", 16, 4, "NPL"),
];

/// Generate a header with four type substitutions and an unchanged consumer
/// translation unit. Unknown structure, uses or integer domains abort the build.
pub fn generate(native: &Path, out: &Path) -> PathBuf {
    let source = fs::read_to_string(native.join("swephlib.c")).expect("read nutation consumer");
    let original = fs::read_to_string(native.join("swenut2000a.h")).expect("read nutation tables");
    let header = decoded_header(&original, &source).expect("reviewed exact nutation table layout");
    fs::write(out.join("swenut2000a.h"), header).expect("write generated nutation header");
    let generated = out.join("swephlib.c");
    // A local quoted include selects the generated header. The entire consumer
    // text, including casts, arithmetic, order and licensing, stays identical.
    fs::copy(native.join("swephlib.c"), &generated).expect("copy unchanged nutation consumer");
    generated
}

fn decoded_header(header: &str, source: &str) -> Result<String, String> {
    let body = body_after(source, CONSUMER)?;
    if fingerprint(body.as_bytes()) != CONSUMER_FINGERPRINT {
        return Err("unreviewed nutation promotions or arithmetic".into());
    }
    let clean_source = without_comments(source)?;
    let clean_body = body_after(&clean_source, CONSUMER)?;
    let clean_header = without_comments(header)?;
    let nls = dimension(&clean_header, "NLS")?;
    let b = dimension(&clean_header, "NLS_2000B")?;
    if b == 0 || b > nls {
        return Err("invalid short nutation series length".into());
    }
    let mut generated = header.to_string();
    for (name, bits, width, rows) in TABLES {
        // These are the complete uses: five/14 argument products and six/four
        // coefficient products, all promoted before any arithmetic. A second
        // consumer anywhere in this translation unit requires a fresh review.
        if identifier_count(clean_body, name) != width
            || identifier_count(&clean_source, name) != width
        {
            return Err(format!("unreviewed use of {name}"));
        }
        let declaration = format!("static const int{bits} {name}[] =");
        let values = body_after(&clean_header, &declaration)?;
        validate_values(values, bits, width * dimension(&clean_header, rows)?)?;
        unique_offset(header, &declaration)?;
        generated = generated.replacen(&declaration, &format!("static const double {name}[] ="), 1);
    }
    Ok(generated)
}

fn validate_values(body: &str, bits: u8, expected: usize) -> Result<(), String> {
    let values: Vec<i64> = integers(body)?;
    if values.len() != expected {
        return Err("nutation coefficient count differs".into());
    }
    let bound = 1_i64 << (bits - 1);
    for value in values {
        if !(-bound..bound).contains(&value) {
            return Err("coefficient outside declared signed domain".into());
        }
        if value.unsigned_abs() > (1_u64 << 53) {
            return Err("integer is not exactly representable as double".into());
        }
    }
    Ok(())
}

fn dimension(source: &str, name: &str) -> Result<usize, String> {
    let mut definitions = source.lines().filter_map(|line| {
        let fields: Vec<_> = line.split_whitespace().collect();
        (fields.len() == 3 && fields[0] == "#define" && fields[1] == name).then(|| fields[2])
    });
    let value = definitions
        .next()
        .ok_or_else(|| format!("missing dimension {name}"))?;
    if definitions.next().is_some() {
        return Err(format!("ambiguous dimension {name}"));
    }
    value
        .parse()
        .map_err(|_| format!("invalid dimension {name}"))
}

fn identifier_count(source: &str, name: &str) -> usize {
    source
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .filter(|word| *word == name)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_every_coefficient_token_and_all_other_header_text() {
        let source = include_str!("../swisseph/swephlib.c");
        let original = include_str!("../swisseph/swenut2000a.h");
        let mut generated = decoded_header(original, source).unwrap();
        for (name, bits, _, _) in TABLES {
            generated = generated.replacen(
                &format!("static const double {name}[] ="),
                &format!("static const int{bits} {name}[] ="),
                1,
            );
        }
        assert_eq!(generated, original);
    }

    #[test]
    fn rejects_range_count_and_grammar_changes() {
        assert!(validate_values("-32768,32767,", 16, 2).is_ok());
        assert!(validate_values("-2147483648,2147483647", 32, 2).is_ok());
        for (values, bits, count) in [
            ("32768", 16, 1),
            ("-32769", 16, 1),
            ("2147483648", 32, 1),
            ("-2147483649", 32, 1),
            ("1,,2", 16, 2),
            ("1.0", 16, 1),
            ("1,2", 16, 1),
        ] {
            assert!(validate_values(values, bits, count).is_err());
        }
    }

    #[test]
    fn rejects_unreviewed_consumers_and_changed_dimensions() {
        let source = include_str!("../swisseph/swephlib.c");
        let header = include_str!("../swisseph/swenut2000a.h");
        assert!(
            decoded_header(
                header,
                &source.replacen("(double) nls[j + 0]", "nls[j + 0]", 1)
            )
            .is_err()
        );
        assert!(decoded_header(header, &format!("{source}\nint unexpected = nls[0];")).is_err());
        assert!(
            decoded_header(
                &header.replacen("#define NLS 678", "#define NLS 679", 1),
                source
            )
            .is_err()
        );
        assert!(
            decoded_header(
                &header.replacen("#define NLS_2000B 77", "#define NLS_2000B 999", 1),
                source
            )
            .is_err()
        );
    }
}
