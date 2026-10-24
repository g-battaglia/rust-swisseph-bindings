//! Decode immutable Moshier term structure before compiling the C evaluator.
//!
//! Argument metadata and immutable longitude/latitude coefficient layout are
//! transformed. Original C coefficient tokens are zipped without Rust floating
//! parsing; the evaluator retains each channel's order and expressions.
//! Unsupported table changes fail closed rather than changing the calculation.

use super::c_source::{body_after, fingerprint, integers, tokens, unique_offset, without_comments};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "moshier_specialized.rs"]
mod specialized;

const PLANETS: [&str; 9] = [
    "mer", "ven", "ear", "mar", "jup", "sat", "ura", "nep", "plu",
];
const EVALUATOR: &str = "src/native/moshier_metadata_eval.h";
const SPECIALIZED: &str = "build_support/moshier_specialized.rs";
const SPECIALIZED_FINGERPRINT: u64 = 0x06fdd669aac06262;
// FNV-1a identifies the exact reviewed source fragment, not a security boundary.
const PINNED_EVALUATOR_FINGERPRINT: u64 = 0x485d_2085_3d01_0d66;

/// Generated translation unit and headers, all confined to Cargo OUT_DIR.
pub struct Generated {
    pub source: PathBuf,
    pub header: PathBuf,
    pub tables: PathBuf,
    pub evaluators: PathBuf,
}

#[derive(Debug, PartialEq)]
struct Term {
    arguments: u8,
    degree: u8,
    polynomial: bool,
}

#[derive(Debug, PartialEq)]
struct Argument {
    index: u8,
    negative: bool,
}

#[derive(Debug, PartialEq)]
struct CoefficientPair {
    longitude: String,
    latitude: String,
}

struct Plan {
    terms: Vec<Term>,
    arguments: Vec<Argument>,
    coefficients: usize,
    coefficient_pairs: Vec<CoefficientPair>,
}

/// Build the deterministic metadata header and an otherwise unchanged native
/// translation unit. A malformed or unreviewed pinned input aborts the build.
pub fn generate(native: &Path, out: &Path) -> Generated {
    let table = fs::read_to_string(native.join("swemptab.h")).expect("read Moshier tables");
    let source = fs::read_to_string(native.join("swemplan.c")).expect("read Moshier evaluator");
    let clean = without_comments(&table).expect("valid Moshier table comments");
    let order = body_after(&source, "static const struct plantbl *planets[] =")
        .expect("pinned Moshier planet order");
    let expected = PLANETS.map(|name| format!("&{name}404"));
    assert_eq!(
        tokens(order).expect("planet list"),
        expected,
        "Moshier planet order differs"
    );

    let emitter = fs::read_to_string(SPECIALIZED).expect("read specialized Moshier emitter");
    assert_eq!(
        fingerprint(emitter.as_bytes()),
        SPECIALIZED_FINGERPRINT,
        "specialized Moshier expressions require review"
    );
    let plans: Vec<_> = PLANETS
        .into_iter()
        .map(|name| {
            (
                name,
                decode_plan(&clean, name).unwrap_or_else(|error| panic!("{name}: {error}")),
            )
        })
        .collect();
    let mut header = String::from(
        "/* Generated from pinned swemptab.h; immutable coefficient tokens preserved. */\n\
         struct swe_moshier_pair { double longitude, latitude; };\n",
    );
    for (name, plan) in &plans {
        writeln!(
            header,
            "static const struct swe_moshier_pair {name}_coefficients[] = {{"
        )
        .unwrap();
        for pair in &plan.coefficient_pairs {
            // Preserve C spelling; never parse or format a Rust float.
            writeln!(header, "  {{{}, {}}},", pair.longitude, pair.latitude).unwrap();
        }
        header.push_str("};\n");
    }
    let evaluators = specialized::generate(&plans).expect("complete specialized Moshier streams");

    let start = unique_offset(
        &source,
        "int swi_moshplan2 (double J, int iplm, double *pobj)",
    )
    .expect("pinned Moshier evaluator signature");
    let end = source[start..]
        .find("\n/* Moshier ephemeris.")
        .expect("pinned evaluator boundary")
        + start;
    assert_eq!(
        fingerprint(&source.as_bytes()[start..end]),
        PINNED_EVALUATOR_FINGERPRINT,
        "pinned Moshier evaluator differs; refusing an unreviewed transformation"
    );
    let mut overlay = source.clone();
    overlay.replace_range(
        start..end,
        "#include \"moshier_terms.generated.h\"\n#include \"moshier_metadata_eval.h\"\n",
    );
    let generated = Generated {
        source: out.join("swemplan.c"),
        header: out.join("moshier_terms.generated.h"),
        tables: out.join("swemptab.h"),
        evaluators: out.join("moshier_evaluators.generated.h"),
    };
    let evaluator = fs::read_to_string(EVALUATOR).expect("read Moshier replacement evaluator");
    let detached = detach_unused_streams(&table, &source, &evaluator)
        .expect("reviewed Moshier coefficient ownership");
    fs::write(&generated.source, overlay).expect("write generated Moshier source");
    fs::write(&generated.header, header).expect("write generated Moshier metadata");
    fs::write(&generated.tables, detached).expect("write generated Moshier table declarations");
    fs::write(&generated.evaluators, evaluators).expect("write specialized Moshier terms");
    generated
}

/// Inputs declared to Cargo and the native-input manifest.
pub const INPUTS: [&str; 3] = ["build_support/moshier.rs", SPECIALIZED, EVALUATOR];

/// Disconnect only the unused longitude/latitude pointers, letting the C
/// compiler discard their original arrays after the lossless pair copy.
/// Complete input fingerprints bind the ownership proof: the static planets
/// never escape, and the replacement has no reader of either disconnected
/// field. New reads, aliases or escapes require review, not a guessed C parse.
fn detach_unused_streams(table: &str, source: &str, evaluator: &str) -> Result<String, String> {
    for (label, text, expected) in [
        ("native consumer", source, 0x7d88_a869_e2c7_8a0c_u64),
        ("native tables", table, 0x3ff2_b63a_9c16_9938_u64),
        ("replacement consumer", evaluator, 0x870e1f3041aea34d_u64),
    ] {
        if fingerprint(text.as_bytes()) != expected {
            return Err(format!(
                "unreviewed {label}; unused-pointer ownership requires review"
            ));
        }
    }
    let mut detached = table.to_owned();
    for name in PLANETS {
        replace_stream_pointers(
            &mut detached,
            name,
            &format!("\n {name}tabl,\n {name}tabb,"),
            "\n 0,\n 0,",
        )?;
    }
    Ok(detached)
}

/// Replace one exact pointer pair inside its unique native initializer. All
/// other bytes, including coefficient tokens and license text, remain intact.
fn replace_stream_pointers(
    table: &mut String,
    name: &str,
    original: &str,
    replacement: &str,
) -> Result<(), String> {
    let body = body_after(table, &format!("static struct plantbl {name}404 ="))?;
    let offset = unique_offset(table, body)? + unique_offset(body, original)?;
    table.replace_range(offset..offset + original.len(), replacement);
    Ok(())
}

fn decode_plan(source: &str, name: &str) -> Result<Plan, String> {
    let args = integers(body_after(
        source,
        &format!("static signed char {name}args[] ="),
    )?)?;
    if args.iter().any(|value| !(-128..=127).contains(value)) {
        return Err("argument outside signed-byte domain".into());
    }
    let definition = body_after(source, &format!("static struct plantbl {name}404 ="))?;
    let harmonics_end = definition.find('}').ok_or("missing max-harmonic array")?;
    let harmonics = integers(
        definition[..harmonics_end]
            .trim()
            .strip_prefix('{')
            .ok_or("missing max-harmonic array")?,
    )?;
    if harmonics.len() != 9 || harmonics.iter().any(|value| !(0..=24).contains(value)) {
        return Err("invalid harmonic dimensions".into());
    }
    let fields = tokens(
        definition[harmonics_end + 1..]
            .trim_start()
            .strip_prefix(',')
            .ok_or("missing harmonic separator")?,
    )?;
    if fields.len() != 6
        || fields[1..5]
            != [
                format!("{name}args"),
                format!("{name}tabl"),
                format!("{name}tabb"),
                format!("{name}tabr"),
            ]
    {
        return Err("unexpected coefficient-table mapping".into());
    }
    let max_degree: i16 = fields[0].parse().map_err(|_| "invalid maximum degree")?;
    let mut plan = decode_arguments(&args, &harmonics, max_degree)?;
    let longitude = coefficient_tokens(source, name, "l", plan.coefficients)?;
    let latitude = coefficient_tokens(source, name, "b", plan.coefficients)?;
    coefficient_tokens(source, name, "r", plan.coefficients)?;
    plan.coefficient_pairs = zip_coefficients(longitude, latitude, &plan.terms)?;
    Ok(plan)
}

fn coefficient_tokens(
    source: &str,
    name: &str,
    suffix: &str,
    expected: usize,
) -> Result<Vec<String>, String> {
    let values = tokens(body_after(
        source,
        &format!("static double {name}tab{suffix}[] ="),
    )?)?;
    if values.len() != expected {
        return Err(format!(
            "{suffix} coefficient count {} differs from consumed {expected}",
            values.len()
        ));
    }
    Ok(values)
}

/// Pair coefficient i with coefficient i, preserving its original C token.
/// Prove that every polynomial/periodic term consumes its complete contiguous
/// range, with no truncated zip, skipped coefficient or trailing data.
fn zip_coefficients(
    longitude: Vec<String>,
    latitude: Vec<String>,
    terms: &[Term],
) -> Result<Vec<CoefficientPair>, String> {
    if longitude.len() != latitude.len() {
        return Err("longitude/latitude coefficient counts differ".into());
    }
    let mut offset = 0_usize;
    for term in terms {
        let width = if term.polynomial { 1 } else { 2 };
        offset = offset
            .checked_add((usize::from(term.degree) + 1) * width)
            .ok_or("coefficient offset overflow")?;
        if offset > longitude.len() {
            return Err("term exceeds its coefficient stream".into());
        }
    }
    if offset != longitude.len() {
        return Err("unconsumed coefficient tokens".into());
    }
    Ok(longitude
        .into_iter()
        .zip(latitude)
        .map(|(longitude, latitude)| CoefficientPair {
            longitude,
            latitude,
        })
        .collect())
}

fn decode_arguments(args: &[i16], harmonics: &[i16], max_degree: i16) -> Result<Plan, String> {
    let mut stream = args.iter().copied();
    let mut plan = Plan {
        terms: Vec::new(),
        arguments: Vec::new(),
        coefficients: 0,
        coefficient_pairs: Vec::new(),
    };
    loop {
        let count = stream.next().ok_or("missing final sentinel")?;
        if count == -1 {
            if stream.next().is_some() {
                return Err("unconsumed argument bytes after sentinel".into());
            }
            return Ok(plan);
        }
        let count = u8::try_from(count).map_err(|_| "invalid argument count")?;
        for _ in 0..count {
            let harmonic = stream.next().ok_or("truncated harmonic")?;
            let planet = stream.next().ok_or("truncated planet")? - 1;
            // The evaluator removes the original if(j). Prove this for every
            // table at build time; an upstream zero requires a fresh review.
            if harmonic == 0 {
                return Err("zero harmonic requires the original dispatch".into());
            }
            if !(0..9).contains(&planet)
                || harmonic.unsigned_abs() > harmonics[planet as usize] as u16
            {
                return Err("harmonic outside the declared planet table".into());
            }
            plan.arguments.push(Argument {
                index: harmonic_index(planet as u8, (harmonic.unsigned_abs() - 1) as u8)?,
                negative: harmonic < 0,
            });
        }
        let degree = stream.next().ok_or("missing polynomial degree")?;
        if !(0..=max_degree).contains(&degree) {
            return Err("degree outside the declared polynomial domain".into());
        }
        let degree = u8::try_from(degree).map_err(|_| "degree does not fit metadata")?;
        let polynomial = count == 0;
        plan.coefficients += (usize::from(degree) + 1) * if polynomial { 1 } else { 2 };
        plan.terms.push(Term {
            arguments: count,
            degree,
            polynomial,
        });
    }
}

/// Byte-sized index into the complete 9-by-24 harmonic object. Native code
/// uses its byte representation, never arithmetic beyond the first subarray.
fn harmonic_index(planet: u8, harmonic: u8) -> Result<u8, String> {
    if planet >= 9 || harmonic >= 24 {
        return Err("harmonic outside the complete native object".into());
    }
    u8::try_from(u16::from(planet) * 24 + u16::from(harmonic))
        .map_err(|_| "harmonic index does not fit metadata".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_polynomials_and_signed_arguments_in_order() {
        let plan = decode_arguments(&[0, 2, 2, -3, 2, 1, 1, 1, -1], &[3; 9], 2).unwrap();
        assert_eq!(
            plan.terms,
            [
                Term {
                    arguments: 0,
                    degree: 2,
                    polynomial: true
                },
                Term {
                    arguments: 2,
                    degree: 1,
                    polynomial: false
                }
            ]
        );
        assert_eq!(
            plan.arguments,
            [
                Argument {
                    index: 26,
                    negative: true
                },
                Argument {
                    index: 0,
                    negative: false
                }
            ]
        );
        assert_eq!(plan.coefficients, 7);
    }

    #[test]
    fn linear_indices_round_trip_every_planet_harmonic_and_sign() {
        for planet in 0..9 {
            for harmonic in 1..=24 {
                for sign in [-1, 1] {
                    let plan =
                        decode_arguments(&[1, sign * harmonic, planet + 1, 0, -1], &[24; 9], 0)
                            .unwrap();
                    let argument = &plan.arguments[0];
                    assert_eq!(i16::from(argument.index / 24), planet);
                    assert_eq!(i16::from(argument.index % 24), harmonic - 1);
                    assert_eq!(argument.negative, sign < 0);
                }
            }
        }
        assert_eq!(harmonic_index(0, 0).unwrap(), 0);
        assert_eq!(harmonic_index(8, 23).unwrap(), 215);
        assert!(harmonic_index(9, 0).is_err());
        assert!(harmonic_index(0, 24).is_err());
    }

    #[test]
    fn rejects_truncation_zero_harmonics_and_invalid_domains() {
        for args in [
            &[][..],
            &[0],
            &[1, 1],
            &[1, 0, 1, 0, -1],
            &[1, 1, 0, 0, -1],
            &[1, 4, 1, 0, -1],
            &[0, 4, -1],
            &[-2],
            &[-1, 0],
        ] {
            assert!(
                decode_arguments(args, &[3; 9], 3).is_err(),
                "accepted {args:?}"
            );
        }
    }

    #[test]
    fn strips_comments_without_joining_tokens() {
        assert_eq!(
            without_comments("1,/* -1 */-3,// x\n2").unwrap(),
            "1, -3,\n2"
        );
        assert!(without_comments("/* unterminated").is_err());
        assert_eq!(integers::<i16>(" 1, -128, 127, ").unwrap(), [1, -128, 127]);
        assert!(integers::<i16>("1,,nan").is_err());
        assert!(integers::<i16>("1,,2").is_err());
    }

    #[test]
    fn validates_pinned_tables_and_complete_coefficient_consumption() {
        let source = without_comments(include_str!("../swisseph/swemptab.h")).unwrap();
        let plans: Vec<_> = PLANETS
            .iter()
            .map(|name| decode_plan(&source, name).unwrap())
            .collect();
        assert_eq!(
            plans.iter().map(|plan| plan.terms.len()).sum::<usize>(),
            1340
        );
        assert_eq!(
            plans.iter().map(|plan| plan.arguments.len()).sum::<usize>(),
            2894
        );
        assert!(decode_plan(&source.replacen("0,  3,", "0,  7,", 1), "mer").is_err());
        assert!(
            decode_plan(
                &source.replacen("  3,  1,  1,-10", "  3,  0,  1,-10", 1),
                "mer"
            )
            .is_err()
        );
        assert!(decode_plan(&source.replacen("merargs,", "venargs,", 1), "mer").is_err());
    }

    #[test]
    fn preserves_every_paired_coefficient_and_native_term_offset() {
        let source = without_comments(include_str!("../swisseph/swemptab.h")).unwrap();
        for name in PLANETS {
            let plan = decode_plan(&source, name).unwrap();
            let longitude = coefficient_tokens(&source, name, "l", plan.coefficients).unwrap();
            let latitude = coefficient_tokens(&source, name, "b", plan.coefficients).unwrap();
            assert_eq!(plan.coefficient_pairs.len(), longitude.len());
            let mut offset = 0;
            for term in &plan.terms {
                let width = if term.polynomial { 1 } else { 2 };
                for step in 0..=usize::from(term.degree) {
                    for component in 0..width {
                        // Polynomial reads one pair per Horner step; periodic
                        // reads cosine then sine, exactly as the native stream.
                        let index = offset + step * width + component;
                        assert_eq!(plan.coefficient_pairs[index].longitude, longitude[index]);
                        assert_eq!(plan.coefficient_pairs[index].latitude, latitude[index]);
                    }
                }
                offset += (usize::from(term.degree) + 1) * width;
            }
            assert_eq!(offset, longitude.len());
        }
    }

    #[test]
    fn zips_literal_spelling_and_rejects_truncation_or_trailing_tokens() {
        let polynomial = [Term {
            arguments: 0,
            degree: 2,
            polynomial: true,
        }];
        let longitude = vec!["-0.0", "1.2300e-4", "- 9.00"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let latitude = vec!["0.0", "-5E+2", "7.000"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let pairs = zip_coefficients(longitude, latitude, &polynomial).unwrap();
        assert_eq!(pairs[0].longitude, "-0.0");
        assert_eq!(pairs[1].longitude, "1.2300e-4");
        assert_eq!(pairs[1].latitude, "-5E+2");
        assert_eq!(pairs[2].longitude, "- 9.00");
        assert!(zip_coefficients(vec!["1".into()], Vec::new(), &polynomial).is_err());
        assert!(zip_coefficients(vec!["1".into()], vec!["2".into()], &polynomial).is_err());
        assert!(zip_coefficients(vec!["1".into()], vec!["2".into()], &[]).is_err());
    }

    #[test]
    fn disconnects_only_eighteen_unused_pointer_tokens() {
        let table = include_str!("../swisseph/swemptab.h");
        let mut detached = detach_unused_streams(
            table,
            include_str!("../swisseph/swemplan.c"),
            include_str!("../src/native/moshier_metadata_eval.h"),
        )
        .unwrap();
        assert_eq!(detached.matches("\n 0,\n 0,").count(), PLANETS.len());
        for name in PLANETS {
            replace_stream_pointers(
                &mut detached,
                name,
                "\n 0,\n 0,",
                &format!("\n {name}tabl,\n {name}tabb,"),
            )
            .unwrap();
        }
        // Restoring only the pointers recovers every original byte: all
        // coefficients, radius references, harmonics, distances and notices.
        assert_eq!(detached, table);
    }

    #[test]
    fn rejects_unreviewed_coefficient_ownership_and_ambiguous_pointers() {
        let table = include_str!("../swisseph/swemptab.h");
        let source = include_str!("../swisseph/swemplan.c");
        let evaluator = include_str!("../src/native/moshier_metadata_eval.h");
        for (table, source, evaluator) in [
            (
                table.to_owned(),
                format!("{source}\nvoid *escape = planets;"),
                evaluator.to_owned(),
            ),
            (
                table.replacen("mertabl,", "ventabl,", 1),
                source.to_owned(),
                evaluator.to_owned(),
            ),
            (
                table.to_owned(),
                source.to_owned(),
                format!("{evaluator}\ndouble *read = planets[0]->lon_tbl;"),
            ),
        ] {
            assert!(detach_unused_streams(&table, &source, &evaluator).is_err());
        }
        assert!(replace_stream_pointers(&mut table.to_owned(), "mer", "missing", "0").is_err());
        assert!(
            replace_stream_pointers(
                &mut format!("{table}\nstatic struct plantbl mer404 = {{0}};"),
                "mer",
                "\n mertabl,\n mertabb,",
                "\n 0,\n 0,",
            )
            .is_err()
        );
    }
}
