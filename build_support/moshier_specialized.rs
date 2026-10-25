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
    // Preserve the original ascending preparation and full-expression double
    // assignment. Even n=1 calls the original sscc and writes harmonic two;
    // neither its recurrence nor any floating constant is duplicated here.
    let mut preparation = String::from("  /* Native harmonic preparation. */\n");
    for (index, count) in plan.max_harmonic.iter().enumerate() {
        if *count > 0 {
            writeln!(
                preparation,
                r#"  {{
    const double argument = (mods3600 (freqs[{index}] * T) + phases[{index}]) * STR;
    sscc ({index}, argument, {count});
  }}"#
            )
            .unwrap();
        }
    }
    preparation.push_str("  /* End native harmonic preparation. */\n");
    let mut output = format!(
        r#"static void swe_moshier_eval_{name}(double T, struct swe_moshier_pair *out, double *out_radius)
{{
{preparation}  swe_moshier_coordinate sum = swe_moshier_coordinate_zero();
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
                r#"    swe_moshier_coordinate polynomial = swe_moshier_coordinate_load(&{name}_coefficients[{coefficient_offset}]);
    double radius = {name}tabr[{coefficient_offset}];"#
            ).unwrap();
            for offset in coefficient_offset + 1..end {
                writeln!(
                    output,
                    r#"    polynomial = swe_moshier_coordinate_horner(polynomial, T, &{name}_coefficients[{offset}]);
    radius = radius * T + {name}tabr[{offset}];"#
                ).unwrap();
            }
            output.push_str(
                r#"    polynomial = swe_moshier_coordinate_set_longitude(
        polynomial, mods3600 (swe_moshier_coordinate_longitude(polynomial)));
    sum = swe_moshier_coordinate_add(sum, polynomial);
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
                r#"    swe_moshier_coordinate cosine = swe_moshier_coordinate_load(&{name}_coefficients[{coefficient_offset}]);
    swe_moshier_coordinate sine = swe_moshier_coordinate_load(&{name}_coefficients[{sine}]);
    double radius_cos = {name}tabr[{coefficient_offset}];
    double radius_sin = {name}tabr[{sine}];"#
            )
            .unwrap();
            for offset in (coefficient_offset + 2..end).step_by(2) {
                let sine = offset + 1;
                for (channel, offset) in [("cosine", offset), ("sine", sine)] {
                    writeln!(
                        output,
                        r#"    {channel} = swe_moshier_coordinate_horner({channel}, T, &{name}_coefficients[{offset}]);"#
                    ).unwrap();
                }
                writeln!(
                    output,
                    r#"    radius_cos = radius_cos * T + {name}tabr[{offset}];
    radius_sin = radius_sin * T + {name}tabr[{sine}];"#
                )
                .unwrap();
            }
            // Keep both coordinates in one private arithmetic value. The
            // rounded sine product, cosine FMA and incoming-total ADD remain
            // separate full expressions, with radius in its original place.
            output.push_str(
                r#"    const swe_moshier_coordinate product = swe_moshier_coordinate_product(sine, sv);
    const double radius_product = radius_sin * sv;
    const swe_moshier_coordinate contribution = swe_moshier_coordinate_contribution(cosine, cv, product);
    sum = swe_moshier_coordinate_add(sum, contribution);
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
    output.push_str("  swe_moshier_coordinate_store(out, sum);\n  *out_radius = sr;\n}\n");
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

    /// Recover the predecessor source only from an exact reviewed generated
    /// header/body. Whole-input and whole-output fingerprints prevent a parser
    /// from hiding altered coefficients, radius, harmonics or arithmetic.
    fn scalar_coordinates(source: &str) -> Result<String, String> {
        let accepted = [
            (0x1425298f77f94529, 0x924f3a1cb2161961),
            (0xa5bdd5a9690e64b9, 0x221b7ecf5e3a0644),
            (0xf70616eec855b9b2, 0x81b87ee7edc4b88e),
            (0x0720aaf095dfbe17, 0xc82cd36da37dc6fb),
            (0x306b3ed5a0ca8676, 0xf219b1f42cab4fde),
            (0x6d8850e3c4092b07, 0xdc1fd81e5ffb0bbf),
            (0xa52de68871152007, 0x47b60271f66cf1f1),
            (0x8038d3ed4b05c7d1, 0xae9e0f510d294025),
            (0x0255871dced94062, 0xbcb949d023d2bd3a),
            (0xe28da02022bbef86, 0x2b1e0c824cb0dc13),
        ];
        let input = super::super::fingerprint(source.as_bytes());
        let expected = accepted
            .iter()
            .find_map(|(pair, scalar)| (*pair == input).then_some(*scalar))
            .ok_or("unreviewed coordinate source")?;
        let mut output = String::new();
        let mut lines = source.lines();
        while let Some(line) = lines.next() {
            match line {
                "  swe_moshier_coordinate sum = swe_moshier_coordinate_zero();" => {
                    output.push_str("  struct swe_moshier_pair sum = {0.0, 0.0};\n");
                }
                "    polynomial = swe_moshier_coordinate_set_longitude(" => {
                    if lines.next()
                        != Some(
                            "        polynomial, mods3600 (swe_moshier_coordinate_longitude(polynomial)));",
                        )
                        || lines.next()
                            != Some("    sum = swe_moshier_coordinate_add(sum, polynomial);")
                    {
                        return Err("incomplete polynomial normalization".into());
                    }
                    output.push_str("    sum.longitude += mods3600 (polynomial.longitude);\n    sum.latitude += polynomial.latitude;\n");
                }
                "    const swe_moshier_coordinate product = swe_moshier_coordinate_product(sine, sv);" =>
                {
                    output.push_str("    const struct swe_moshier_pair product = {sine.longitude * sv, sine.latitude * sv};\n");
                }
                "    const swe_moshier_coordinate contribution = swe_moshier_coordinate_contribution(cosine, cv, product);" =>
                {
                    output.push_str("    const struct swe_moshier_pair contribution = {cosine.longitude * cv + product.longitude, cosine.latitude * cv + product.latitude};\n");
                }
                "    sum = swe_moshier_coordinate_add(sum, contribution);" => {
                    output.push_str("    sum.longitude += contribution.longitude;\n    sum.latitude += contribution.latitude;\n");
                }
                "  swe_moshier_coordinate_store(out, sum);" => {
                    output.push_str("  *out = sum;\n");
                }
                _ => {
                    if let Some(rest) = line.strip_prefix("    swe_moshier_coordinate ") {
                        let (channel, coefficient) = rest
                            .split_once(" = swe_moshier_coordinate_load(&")
                            .ok_or("unmatched coordinate declaration")?;
                        if !["polynomial", "cosine", "sine"].contains(&channel) {
                            return Err("unknown coordinate declaration".into());
                        }
                        let coefficient = coefficient
                            .strip_suffix(");")
                            .ok_or("truncated coefficient record")?;
                        writeln!(
                            output,
                            "    struct swe_moshier_pair {channel} = {coefficient};"
                        )
                        .unwrap();
                    } else if let Some((assignment, rest)) =
                        line.split_once(" = swe_moshier_coordinate_horner(")
                    {
                        let channel = assignment.trim();
                        if !["polynomial", "cosine", "sine"].contains(&channel) {
                            return Err("unknown Horner channel".into());
                        }
                        let prefix = format!("{channel}, T, &");
                        let coefficient = rest
                            .strip_prefix(prefix.as_str())
                            .and_then(|tail| tail.strip_suffix(");"))
                            .ok_or("unmatched Horner stage")?;
                        for field in ["longitude", "latitude"] {
                            writeln!(output, "    {channel}.{field} = {channel}.{field} * T + {coefficient}.{field};").unwrap();
                        }
                    } else {
                        if line.contains("swe_moshier_coordinate") {
                            return Err("unmatched coordinate stage".into());
                        }
                        writeln!(output, "{line}").unwrap();
                    }
                }
            }
        }
        if super::super::fingerprint(output.as_bytes()) != expected {
            return Err("coordinate inverse differs from complete predecessor".into());
        }
        Ok(output)
    }

    #[test]
    fn coordinate_source_keeps_every_radius_and_angular_stage_in_order() {
        for name in PLANETS {
            let plan = native(name);
            let source = planet(name, &plan).unwrap();
            let scalar = scalar_coordinates(&source).unwrap();
            let preserved = |text: &str| -> Vec<String> {
                text.lines()
                    .filter(|line| {
                        line.contains("radius")
                            || line.contains("sr +=")
                            || line.contains("sscc (")
                            || line.contains("harmonic_value")
                            || line.contains("const double argument =")
                            || line.contains("su = -su;")
                            || line.contains("sv = su;")
                            || line.contains("cv = cu;")
                            || line.contains("sine_product")
                            || line.contains("cosine_product")
                            || line.contains("sv = t;")
                    })
                    .map(str::to_owned)
                    .collect()
            };
            assert_eq!(preserved(&source), preserved(&scalar));
            assert_eq!(
                source
                    .matches("swe_moshier_coordinate_set_longitude(")
                    .count(),
                plan.terms.iter().filter(|term| term.polynomial).count()
            );
            for term in source.split("/* Ordered term").skip(1) {
                if let Some(product) = term.find("const swe_moshier_coordinate product =") {
                    let radius = term.find("const double radius_product =").unwrap();
                    let contribution = term
                        .find("const swe_moshier_coordinate contribution =")
                        .unwrap();
                    let sum = term
                        .find("sum = swe_moshier_coordinate_add(sum, contribution);")
                        .unwrap();
                    let radius_sum = term
                        .find("sr += radius_cos * cv + radius_product;")
                        .unwrap();
                    assert!(
                        product < radius
                            && radius < contribution
                            && contribution < sum
                            && sum < radius_sum
                    );
                }
            }
            assert!(!source.contains(".longitude") && !source.contains(".latitude"));
        }
    }

    #[test]
    fn inverse_rejects_unmatched_truncated_or_reordered_stages() {
        let source = planet("mer", &native("mer")).unwrap();
        for altered in [
            source.replace("_horner(polynomial, T,", "_horner(polynomial, -T,"),
            source.replace("radius = radius * T", "radius = -radius * T"),
            source.replace("sum, contribution", "contribution, sum"),
            source.trim_end_matches("}\n").to_owned(),
            format!("{source}\n"),
        ] {
            assert!(scalar_coordinates(&altered).is_err());
        }
    }

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
    fn prepares_the_native_harmonics_in_original_index_order() {
        let tables = without_comments(include_str!("../swisseph/swemptab.h")).unwrap();
        for name in PLANETS {
            let plan = native(name);
            let definition =
                super::super::body_after(&tables, &format!("static struct plantbl {name}404 ="))
                    .unwrap();
            let end = definition.find('}').unwrap();
            let expected =
                super::super::integers::<i16>(definition[..end].trim().strip_prefix('{').unwrap())
                    .unwrap();
            assert_eq!(plan.max_harmonic.as_slice(), expected.as_slice());
            let source = planet(name, &plan).unwrap();
            let calls: Vec<_> = source
                .lines()
                .filter_map(|line| {
                    let call = line.trim().strip_prefix("sscc (")?.strip_suffix(");")?;
                    let (index, count) = call.split_once(", argument, ")?;
                    Some((
                        index.parse::<usize>().unwrap(),
                        count.parse::<i16>().unwrap(),
                    ))
                })
                .collect();
            assert_eq!(
                calls,
                expected
                    .into_iter()
                    .enumerate()
                    .filter(|(_, count)| *count > 0)
                    .collect::<Vec<_>>()
            );
            for (index, _) in calls {
                assert!(source.contains(&format!(
                    "const double argument = (mods3600 (freqs[{index}] * T) + phases[{index}]) * STR;"
                )));
            }
        }
    }

    #[test]
    fn a_single_harmonic_still_calls_the_original_recurrence() {
        let mut plan = native("mer");
        plan.max_harmonic = [0; 9];
        plan.max_harmonic[7] = 1;
        let source = planet("mer", &plan).unwrap();
        assert_eq!(source.matches("sscc (").count(), 1);
        assert!(source.contains("sscc (7, argument, 1);"));
        // The original helper writes index 1 before testing its loop. Keeping
        // that helper/call preserves this otherwise easy-to-miss native write.
        let native = include_str!("../swisseph/swemplan.c");
        let helper =
            super::super::body_after(native, "static void sscc (int k, double arg, int n)\n")
                .unwrap();
        assert!(
            helper.find("ss[k][1] = sv;").unwrap()
                < helper.find("for (i = 2; i < n; i++)").unwrap()
        );
    }

    #[test]
    fn removing_only_preparation_recovers_the_entire_reviewed_evaluator_header() {
        let plans: Vec<_> = PLANETS
            .into_iter()
            .map(|name| (name, native(name)))
            .collect();
        let source = scalar_coordinates(&generate(&plans).unwrap()).unwrap();
        let mut original = String::new();
        let mut preparation = false;
        for line in source.lines() {
            match line.trim() {
                "/* Native harmonic preparation. */" => preparation = true,
                "/* End native harmonic preparation. */" => preparation = false,
                _ if !preparation => writeln!(original, "{line}").unwrap(),
                _ => {}
            }
        }
        assert!(!preparation);
        // Complete immutable generated source, including every coefficient
        // access, angular stage, ordered sum and store; no result vector.
        assert_eq!(
            super::super::fingerprint(original.as_bytes()),
            0x53fff8e0e3b26837
        );
    }

    #[test]
    fn covers_all_native_terms_addresses_degrees_and_signs_in_order() {
        for name in PLANETS {
            let plan = native(name);
            let source = scalar_coordinates(&planet(name, &plan).unwrap()).unwrap();
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
        plan.coefficient_pairs.pop();
        assert!(planet("mer", &plan).is_err());
        let mut plan = native("mer");
        plan.arguments.pop();
        assert!(planet("mer", &plan).is_err());
        let mut plan = native("mer");
        plan.terms.pop();
        assert!(planet("mer", &plan).is_err());
    }
}
