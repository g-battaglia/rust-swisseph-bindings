//! Pair immutable periodic coefficients for independent precession coordinates.
//!
//! Original C tokens are transposed, never parsed as Rust floating point.
//! Every period, row and per-coordinate operation retains its native order.

use super::c_source::{body_after, fingerprint, tokens, unique_offset, without_comments};
use std::{fs, path::Path};

const HELPER: &str = "src/native/precession_pairs.h";
const SOURCE_FINGERPRINT: u64 = 0xe642_4aa4_7d15_66b1;
const PRECEDING_FINGERPRINT: u64 = 0xad74_3f33_1c0d_1304;
const HELPER_FINGERPRINT: u64 = 0x0e3165c8fdd97df0;
const TABLES: [(&str, &str, usize, &str, &str); 2] = [
    ("pqper", "NPER_PECL", 8, "p", "q"),
    ("xyper", "NPER_PEQU", 14, "x", "y"),
];

/// Owned inputs declared to Cargo and the native-input manifest.
pub const INPUTS: [&str; 2] = ["build_support/precession_pairs.rs", HELPER];

/// Compose after the unchanged nutation and angle-normalizer overlays.
/// Complete source identities prevent additional consumers or preceding edits
/// from invalidating the token, arithmetic and storage-ownership proof.
pub fn apply(native: &Path, generated: &Path) {
    let source = fs::read_to_string(native.join("swephlib.c")).expect("read precession source");
    let preceding = fs::read_to_string(generated).expect("read preceding native overlays");
    let helper = fs::read_to_string(HELPER).expect("read paired precession helper");
    let paired =
        transformed(&source, &preceding, &helper).expect("reviewed paired precession terms");
    fs::write(generated, paired).expect("write paired precession source");
}

fn columns(body: &str, count: usize) -> Result<Vec<Vec<String>>, String> {
    let clean = without_comments(body)?;
    let mut remaining = clean.trim();
    let mut columns = Vec::with_capacity(5);
    for column in 0..5 {
        let row = remaining.strip_prefix('{').ok_or("missing periodic row")?;
        let (values, rest) = row.split_once('}').ok_or("unterminated periodic row")?;
        let values = tokens(values)?;
        if values.len() != count {
            return Err("periodic coefficient count differs".into());
        }
        columns.push(values);
        remaining = rest.trim();
        if column < 4 {
            remaining = remaining
                .strip_prefix(',')
                .ok_or("missing row separator")?
                .trim();
        } else if let Some(rest) = remaining.strip_prefix(',') {
            remaining = rest.trim();
        }
    }
    if !remaining.is_empty() {
        return Err("unconsumed periodic table input".into());
    }
    Ok(columns)
}

fn paired_table(name: &str, dimension: &str, body: &str, count: usize) -> Result<String, String> {
    let columns = columns(body, count)?;
    let mut paired = format!("static const swe_precession_term {name}_pairs[{dimension}] = {{\n");
    if name == "pqper" {
        // Keep the original publication/erratum notes, including its commented
        // historical row. Only live coefficient tokens enter compiled storage.
        let first = "  // original publication    A&A 534, A22 (2011):\n";
        let last = "  // typo fixed according to A&A 541, C1 (2012)\n";
        let start = unique_offset(body, first)?;
        let end = unique_offset(body, last)? + last.len();
        paired.push_str(&body[start..end]);
    }
    for (index, period) in columns[0].iter().enumerate() {
        paired.push_str(&format!(
            "  {{ {}, {{ {}, {} }}, {{ {}, {} }} }},\n",
            period, columns[1][index], columns[2][index], columns[3][index], columns[4][index]
        ));
    }
    paired.push_str("};");
    // Independently recover all five column streams from the emitted records.
    // This binds field placement and multiplicity, not merely token membership.
    let declaration = format!("static const swe_precession_term {name}_pairs[{dimension}] =");
    let clean = without_comments(&paired)?;
    let fields: String = body_after(&clean, &declaration)?
        .chars()
        .filter(|ch| *ch != '{' && *ch != '}')
        .collect();
    let fields = tokens(&fields)?;
    if fields.len() != count * 5 {
        return Err("paired record width differs".into());
    }
    for column in 0..5 {
        for index in 0..count {
            if fields[index * 5 + column] != columns[column][index] {
                return Err("paired coefficient token correspondence differs".into());
            }
        }
    }
    Ok(paired)
}

fn periodic_loop(name: &str, first: &str, second: &str) -> String {
    format!(
        r#"  for (i = 0; i < nper; i++) {{
    w = D2PI * t;
    a = w / {name}[0][i];
    s = sin(a);
    c = cos(a);
    {first} += c * {name}[1][i] + s * {name}[3][i];
    {second} += c * {name}[2][i] + s * {name}[4][i];
  }}"#
    )
}

fn paired_loop(name: &str, first: &str, second: &str) -> String {
    format!(
        r#"  swe_precession_pair total = {{ {first}, {second} }};
  for (i = 0; i < nper; i++) {{
    w = D2PI * t;
    a = w / {name}_pairs[i].period;
    s = sin(a);
    c = cos(a);
    total = swe_precession_accumulate(total, c, s, &{name}_pairs[i]);
  }}
  {first} = total.first;
  {second} = total.second;"#
    )
}

fn transformed(source: &str, preceding: &str, helper: &str) -> Result<String, String> {
    if fingerprint(source.as_bytes()) != SOURCE_FINGERPRINT
        || fingerprint(preceding.as_bytes()) != PRECEDING_FINGERPRINT
        || fingerprint(helper.as_bytes()) != HELPER_FINGERPRINT
    {
        return Err("unreviewed source, preceding overlays or paired helper".into());
    }
    let edits = source_edits(source)?;
    let mut generated = preceding.to_owned();
    for (old, new) in &edits {
        let start = unique_offset(&generated, old)?;
        generated.replace_range(start..start + old.len(), new);
    }
    // The old arrays are replaced, not retained as a second coefficient copy.
    // Undo exactly these four edits: every other source byte, including the
    // normalizers, all precession branches and projection, must be unchanged.
    let restored = undo(&generated, &edits)?;
    if restored != preceding {
        return Err("paired precession transformation is not byte-reversible".into());
    }
    Ok(generated)
}

fn source_edits(source: &str) -> Result<Vec<(String, String)>, String> {
    let mut edits = Vec::with_capacity(4);
    for (name, dimension, count, first, second) in TABLES {
        let declaration = format!("static const double {name}[5][{dimension}] =");
        let body = body_after(source, &declaration)?;
        let old = format!("{declaration} {{{body}}};");
        let mut replacement = paired_table(name, dimension, body, count)?;
        if name == "pqper" {
            replacement.insert_str(0, "#include \"precession_pairs.h\"\n");
        }
        edits.push((old, replacement));
        edits.push((
            periodic_loop(name, first, second),
            paired_loop(name, first, second),
        ));
    }
    Ok(edits)
}

fn undo(generated: &str, edits: &[(String, String)]) -> Result<String, String> {
    let mut restored = generated.to_owned();
    for (old, new) in edits.iter().rev() {
        let start = unique_offset(&restored, new)?;
        restored.replace_range(start..start + new.len(), old);
    }
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;
    const NATIVE: &str = include_str!("../swisseph/swephlib.c");

    #[test]
    fn recovers_the_complete_preceding_source_and_reapplies_identically() {
        // Remove the local preparation first, then the filename presentation
        // stage. Keep both exact inverses and require the independently
        // qualified complete precession output before testing our own inverse.
        let generated = super::super::precession_frame::restore_lib(include_str!(concat!(
            env!("OUT_DIR"),
            "/swephlib.c"
        )))
        .expect("restore final local precession preparation")
        .replacen("#include \"file_name_suffix.h\"\n", "", 1)
        .replacen(
            "  swe_append_file_century(fname + strlen(fname), (unsigned int) icty);",
            "  sprintf(fname + strlen(fname), \"%02d.%s\", icty, SE_FILE_SUFFIX);",
            1,
        );
        assert_eq!(fingerprint(generated.as_bytes()), 0x2e8a_8b17_e026_807e);
        let helper = include_str!("../src/native/precession_pairs.h");
        let preceding = undo(&generated, &source_edits(NATIVE).unwrap()).unwrap();
        assert_eq!(fingerprint(preceding.as_bytes()), PRECEDING_FINGERPRINT);
        assert_eq!(transformed(NATIVE, &preceding, helper).unwrap(), generated);
        assert!(transformed(NATIVE, &format!("{preceding}\n"), helper).is_err());
        assert!(transformed(NATIVE, &preceding, &format!("{helper}\n")).is_err());
    }

    #[test]
    fn preserves_all_period_and_coefficient_tokens_in_both_tables() {
        for (name, dimension, count, _, _) in TABLES {
            let declaration = format!("static const double {name}[5][{dimension}] =");
            let original = body_after(NATIVE, &declaration).unwrap();
            let paired = paired_table(name, dimension, original, count).unwrap();
            assert_eq!(
                paired.matches("static const swe_precession_term").count(),
                1
            );
            assert!(!paired.contains("static const double"));
            if name == "pqper" {
                assert!(paired.contains("198.296071")); // Original commented erratum row.
                assert!(paired.contains("198.296701")); // Live corrected token.
            }
        }
    }

    #[test]
    fn rejects_incomplete_or_extra_table_tokens() {
        for input in [
            "{1},{2},{3},{4}",
            "{1},{2},{3},{4},{5},{6}",
            "{1,,2},{2},{3},{4},{5}",
            "{1},{2},{3},{4},{5} unexpected",
        ] {
            assert!(columns(input, 1).is_err());
        }
        assert_eq!(columns("{1},{2},{3},{4},{5},", 1).unwrap()[4], ["5"]);
    }

    #[test]
    fn rejects_changed_complete_source_and_preceding_overlays() {
        let helper = include_str!("../src/native/precession_pairs.h");
        assert!(transformed(&format!("{NATIVE}\n"), NATIVE, helper).is_err());
        assert!(transformed(NATIVE, NATIVE, helper).is_err());
        assert!(transformed(NATIVE, "unreviewed preceding overlay", helper).is_err());
    }
}
