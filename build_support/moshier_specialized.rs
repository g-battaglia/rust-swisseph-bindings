//! Specialize immutable term control, retaining the original floating stages.

use super::{Argument, Plan};
use std::fmt::Write;

/// Emit one private function per planet and one shared dispatch table. Only
/// control/addresses become constants; coefficients remain in their arrays.
pub(super) fn generate(plans: &[(&str, Plan)]) -> Result<String, String> {
    let mut output = String::from("/* Generated from fully decoded pinned Moshier terms. */\n");
    for (name, plan) in plans {
        output.push_str(&planet(name, plan)?);
    }
    output.push_str("static void (*const swe_moshier_evaluators[])(double, struct swe_moshier_pair *, double *) = {\n");
    for (name, _) in plans {
        writeln!(output, "  swe_moshier_eval_{name},").unwrap();
    }
    output.push_str("};\n");
    Ok(output)
}

fn planet(name: &str, plan: &Plan) -> Result<String, String> {
    if plan.coefficient_pairs.len() != plan.coefficients {
        return Err("specialized coefficient count differs".into());
    }
    let mut output = format!(
        r#"static void swe_moshier_eval_{name}(double T, struct swe_moshier_pair *out, double *out_radius)
{{
  struct swe_moshier_pair sum = {{0.0, 0.0}};
  double sr = 0.0;
"#
    );
    let mut argument_offset = 0;
    let mut coefficient_offset = 0;
    for (index, term) in plan.terms.iter().enumerate() {
        let count = usize::from(term.arguments);
        let degree = usize::from(term.degree);
        let width = if term.polynomial { 1 } else { 2 };
        let end = coefficient_offset + (degree + 1) * width;
        if term.polynomial != (count == 0) || end > plan.coefficients {
            return Err("specialized term exceeds its native stream".into());
        }
        let arguments = plan
            .arguments
            .get(argument_offset..argument_offset + count)
            .ok_or("specialized argument stream is truncated")?;
        writeln!(output, "  {{ /* Ordered term {index}; degree {degree}. */").unwrap();
        if term.polynomial {
            writeln!(
                output,
                r#"    struct swe_moshier_pair polynomial = {name}_coefficients[{coefficient_offset}];
    double radius = {name}tabr[{coefficient_offset}];"#
            ).unwrap();
            for offset in coefficient_offset + 1..end {
                writeln!(
                    output,
                    r#"    polynomial.longitude = polynomial.longitude * T + {name}_coefficients[{offset}].longitude;
    polynomial.latitude = polynomial.latitude * T + {name}_coefficients[{offset}].latitude;
    radius = radius * T + {name}tabr[{offset}];"#
                ).unwrap();
            }
            output.push_str(
                r#"    sum.longitude += mods3600 (polynomial.longitude);
    sum.latitude += polynomial.latitude;
    sr += radius;
"#,
            );
        } else {
            output.push_str("    double su, cu, sv, cv;\n");
            for (index, argument) in arguments.iter().enumerate() {
                angle(&mut output, argument, index == 0);
            }
            let sine = coefficient_offset + 1;
            writeln!(
                output,
                r#"    struct swe_moshier_pair cosine = {name}_coefficients[{coefficient_offset}];
    struct swe_moshier_pair sine = {name}_coefficients[{sine}];
    double radius_cos = {name}tabr[{coefficient_offset}];
    double radius_sin = {name}tabr[{sine}];"#
            )
            .unwrap();
            for offset in (coefficient_offset + 2..end).step_by(2) {
                let sine = offset + 1;
                for (channel, offset) in [("cosine", offset), ("sine", sine)] {
                    writeln!(
                        output,
                        r#"    {channel}.longitude = {channel}.longitude * T + {name}_coefficients[{offset}].longitude;
    {channel}.latitude = {channel}.latitude * T + {name}_coefficients[{offset}].latitude;"#
                    ).unwrap();
                }
                writeln!(
                    output,
                    r#"    radius_cos = radius_cos * T + {name}tabr[{offset}];
    radius_sin = radius_sin * T + {name}tabr[{sine}];"#
                )
                .unwrap();
            }
            // Shipping C rounds the sine product, fuses the cosine sum, then
            // adds to each ordered accumulator. Separate statements retain it.
            output.push_str(
                r#"    const struct swe_moshier_pair product = {sine.longitude * sv, sine.latitude * sv};
    const double radius_product = radius_sin * sv;
    const struct swe_moshier_pair contribution = {cosine.longitude * cv + product.longitude, cosine.latitude * cv + product.latitude};
    sum.longitude += contribution.longitude;
    sum.latitude += contribution.latitude;
    sr += radius_cos * cv + radius_product;
"#
            );
        }
        output.push_str("  }\n");
        argument_offset += count;
        coefficient_offset = end;
    }
    if argument_offset != plan.arguments.len() || coefficient_offset != plan.coefficients {
        return Err("specialized native streams were not completely consumed".into());
    }
    output.push_str("  *out = sum;\n  *out_radius = sr;\n}\n");
    Ok(output)
}

fn angle(output: &mut String, argument: &Argument, first: bool) {
    let index = argument.index;
    writeln!(
        output,
        "    su = swe_moshier_harmonic_value((const unsigned char *)&ss, {index});"
    )
    .unwrap();
    if argument.negative {
        output.push_str("    su = -su;\n");
    }
    writeln!(
        output,
        "    cu = swe_moshier_harmonic_value((const unsigned char *)&cc, {index});"
    )
    .unwrap();
    if first {
        // An identity multiply would change zero signs. Assign the first angle.
        output.push_str("    sv = su;\n    cv = cu;\n");
    } else {
        // Match the existing FMUL/FNMUL -> two FMA stages, with the negation
        // after its rounded product. Both components still use the old sv/cv.
        output.push_str(
            r#"    {
      double t;
      const double sine_product = cu * sv;
      const double cosine_product = -(su * sv);
      t = su * cv + sine_product;
      cv = cu * cv + cosine_product;
      sv = t;
    }
"#,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::{PLANETS, decode_plan, without_comments};
    use super::*;

    fn native(name: &str) -> Plan {
        decode_plan(
            &without_comments(include_str!("../swisseph/swemptab.h")).unwrap(),
            name,
        )
        .unwrap()
    }

    fn indices(source: &str, marker: &str) -> Vec<usize> {
        source
            .lines()
            .filter_map(|line| {
                line.split_once(marker)
                    .map(|(_, suffix)| suffix.split_once(']').unwrap().0.parse().unwrap())
            })
            .collect()
    }

    #[test]
    fn covers_all_native_terms_addresses_degrees_and_signs_in_order() {
        for name in PLANETS {
            let plan = native(name);
            let source = planet(name, &plan).unwrap();
            assert_eq!(source.matches("/* Ordered term").count(), plan.terms.len());
            assert!(!source.contains("for (") && !source.contains("while ("));
            // Expand a pair initialization into its two field reads. Every
            // later Horner read must use its own field and the exact next
            // offset, so neither a duplicated nor advanced index is hidden.
            let mut coefficient_uses = Vec::new();
            for line in source.lines() {
                if let Some((assignment, suffix)) =
                    line.split_once(&format!("{name}_coefficients["))
                {
                    let (index, field) = suffix.split_once(']').unwrap();
                    let index: usize = index.parse().unwrap();
                    if field == ";" {
                        coefficient_uses.extend([(index, "longitude"), (index, "latitude")]);
                    } else {
                        let field = field.strip_prefix('.').unwrap().strip_suffix(';').unwrap();
                        assert!(assignment.trim_start().starts_with(&format!(
                            "{}.{} = ",
                            assignment.trim_start().split_once('.').unwrap().0,
                            field
                        )));
                        coefficient_uses.push((index, field));
                    }
                }
            }
            let expected: Vec<_> = (0..plan.coefficients)
                .flat_map(|index| [(index, "longitude"), (index, "latitude")])
                .collect();
            assert_eq!(coefficient_uses, expected);
            assert_eq!(
                indices(&source, &format!("{name}tabr[")),
                (0..plan.coefficients).collect::<Vec<_>>()
            );
            let mut harmonics = Vec::new();
            let mut lines = source.lines();
            while let Some(line) = lines.next() {
                if let Some((_, suffix)) = line.split_once("&ss, ") {
                    let index: u8 = suffix.trim_end_matches(");").parse().unwrap();
                    let mut next = lines.next().unwrap();
                    let negative = next.trim() == "su = -su;";
                    if negative {
                        next = lines.next().unwrap();
                    }
                    assert_eq!(
                        next.trim(),
                        format!(
                            "cu = swe_moshier_harmonic_value((const unsigned char *)&cc, {index});"
                        )
                    );
                    harmonics.push((index, negative));
                }
            }
            assert_eq!(
                harmonics,
                plan.arguments
                    .iter()
                    .map(|arg| (arg.index, arg.negative))
                    .collect::<Vec<_>>()
            );
            let periodic = plan.terms.iter().filter(|term| !term.polynomial).count();
            for stage in [
                "sv = su;",
                "cv = cu;",
                "const struct swe_moshier_pair product =",
                "const double radius_product =",
                "const struct swe_moshier_pair contribution =",
                "sum.longitude += contribution.longitude;",
                "sum.latitude += contribution.latitude;",
                "sr += radius_cos * cv + radius_product;",
            ] {
                assert_eq!(source.matches(stage).count(), periodic, "{name}: {stage}");
            }
            assert_eq!(
                source.matches("t = su * cv + sine_product;").count(),
                plan.arguments.len() - periodic
            );
            assert_eq!(
                source.matches("cv = cu * cv + cosine_product;").count(),
                plan.arguments.len() - periodic
            );
        }
    }

    #[test]
    fn rejects_incomplete_native_streams() {
        let mut plan = native("mer");
        plan.coefficients -= 1;
        assert!(planet("mer", &plan).is_err());
        let mut plan = native("mer");
        plan.arguments.pop();
        assert!(planet("mer", &plan).is_err());
        let mut plan = native("mer");
        plan.terms.pop();
        assert!(planet("mer", &plan).is_err());
    }
}
