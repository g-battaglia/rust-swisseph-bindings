//! Prepare independent lunar mean-angle cubics before their unchanged readers.
//!
//! Only MP/D and F/M polynomial evaluation is paired. The original coefficient
//! tokens and three native per-channel FMA stages remain; normalization, trig,
//! the separate LP polynomial and every subsequent correction stay in place.

use super::c_source::{fingerprint, unique_offset};
use std::{fs, path::Path};

const HELPER: &str = "src/native/embofs_angles.h";
const SIGNATURE: &str = "static void embofs_mosh(double tjd, double *xemb)";
const SOURCE_FINGERPRINT: u64 = 0x7d88a869e2c78a0c;
const PRECEDING_FINGERPRINT: u64 = 0x77efa6f23abad5b7;
const HELPER_FINGERPRINT: u64 = 0xbfbcb19574ed275f;
const INCLUDE: &str = "#include \"embofs_angles.h\"\n";
const CHANGES: [(&str, &str); 5] = [
    (
        r#"  T = (tjd-J1900)/36525.0;"#,
        r#"  T = (tjd-J1900)/36525.0;
  /* Raw mean angles only: preserve every normalization and trig site below. */
  const swe_embofs_pair mp_d = swe_embofs_cubic_pair(
      T, (swe_embofs_pair){1.44e-5, 1.9e-6},
      (swe_embofs_pair){0.009192, -0.001436},
      (swe_embofs_pair){477198.8491, 445267.1142},
      (swe_embofs_pair){296.104608, 350.737486});
  const swe_embofs_pair f_m = swe_embofs_cubic_pair(
      T, (swe_embofs_pair){-3.e-7, -3.3e-6},
      (swe_embofs_pair){-0.003211, -1.50e-4},
      (swe_embofs_pair){483202.0251, 35999.0498},
      (swe_embofs_pair){11.250889, 358.475833});"#,
    ),
    (
        r#"  a = swe_degnorm(((1.44e-5*T + 0.009192)*T + 477198.8491)*T + 296.104608);"#,
        r#"  a = swe_degnorm(mp_d.first);"#,
    ),
    (
        r#"  a = swe_degnorm(((1.9e-6*T - 0.001436)*T + 445267.1142)*T + 350.737486);"#,
        r#"  a = swe_degnorm(mp_d.second);"#,
    ),
    (
        r#"  a = swe_degnorm((( -3.e-7*T - 0.003211)*T + 483202.0251)*T + 11.250889);"#,
        r#"  a = swe_degnorm(f_m.first);"#,
    ),
    (
        r#"  M = swe_degnorm((( -3.3e-6*T - 1.50e-4)*T + 35999.0498)*T + 358.475833);"#,
        r#"  M = swe_degnorm(f_m.second);"#,
    ),
];

/// Owned inputs declared to Cargo and the linked native-input manifest.
pub const INPUTS: [&str; 2] = ["build_support/embofs_angles.rs", HELPER];

/// Compose only after the reviewed Moshier metadata overlay. Full source,
/// preceding-unit and helper identities constrain the transformation before C
/// compilation. Only Cargo's generated file is written; vendor input is intact.
pub fn apply(native: &Path, generated: &Path) {
    let original = fs::read_to_string(native.join("swemplan.c")).expect("read lunar offset source");
    let preceding = fs::read_to_string(generated).expect("read Moshier metadata overlay");
    let helper = fs::read_to_string(HELPER).expect("read paired mean-angle helper");
    let output = transformed(&original, &preceding, &helper)
        .expect("reviewed independent mean-angle preparation");
    fs::write(generated, output).expect("write paired mean-angle source");
}

fn transformed(original: &str, preceding: &str, helper: &str) -> Result<String, String> {
    if fingerprint(original.as_bytes()) != SOURCE_FINGERPRINT
        || fingerprint(preceding.as_bytes()) != PRECEDING_FINGERPRINT
        || fingerprint(helper.as_bytes()) != HELPER_FINGERPRINT
    {
        return Err("unreviewed mean-angle source, composition or helper".into());
    }
    let mut output = preceding.to_owned();
    for (old, new) in CHANGES {
        unique_offset(original, old)?;
        let offset = unique_offset(&output, old)?;
        output.replace_range(offset..offset + old.len(), new);
    }
    let offset = unique_offset(&output, SIGNATURE)?;
    output.insert_str(offset, INCLUDE);
    if undo(&output)? != preceding {
        return Err("mean-angle preparation is not byte-reversible".into());
    }
    Ok(output)
}

fn undo(generated: &str) -> Result<String, String> {
    let mut restored = generated.to_owned();
    let include = unique_offset(&restored, INCLUDE)?;
    restored.replace_range(include..include + INCLUDE.len(), "");
    for (old, new) in CHANGES.into_iter().rev() {
        let offset = unique_offset(&restored, new)?;
        restored.replace_range(offset..offset + new.len(), old);
    }
    if fingerprint(restored.as_bytes()) != PRECEDING_FINGERPRINT {
        return Err("unreviewed source outside the mean-angle preparation".into());
    }
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;
    const NATIVE: &str = include_str!("../swisseph/swemplan.c");
    const HELPER_SOURCE: &str = include_str!("../src/native/embofs_angles.h");

    #[test]
    fn actual_generated_unit_has_a_complete_inverse_and_identical_reapplication() {
        let generated = include_str!(concat!(env!("OUT_DIR"), "/swemplan.c"));
        let preceding = undo(generated).unwrap();
        assert_eq!(fingerprint(preceding.as_bytes()), PRECEDING_FINGERPRINT);
        assert_eq!(
            transformed(NATIVE, &preceding, HELPER_SOURCE).unwrap(),
            generated
        );
        assert_eq!(generated.matches(INCLUDE).count(), 1);
        for (old, new) in CHANGES {
            assert_eq!(preceding.matches(old).count(), 1);
            assert_eq!(generated.matches(new).count(), 1);
        }
    }

    #[test]
    fn rejects_changed_whole_inputs_helper_and_unrelated_source_edits() {
        let generated = include_str!(concat!(env!("OUT_DIR"), "/swemplan.c"));
        let preceding = undo(generated).unwrap();
        assert!(transformed(&format!("{NATIVE}\n"), &preceding, HELPER_SOURCE).is_err());
        assert!(transformed(NATIVE, &format!("{preceding}\n"), HELPER_SOURCE).is_err());
        assert!(transformed(NATIVE, &preceding, &format!("{HELPER_SOURCE}\n")).is_err());
        assert!(undo(&format!("{generated}\n")).is_err());
        assert!(undo(&generated.replacen("481267.8831", "481267.8832", 1)).is_err());
    }
}
