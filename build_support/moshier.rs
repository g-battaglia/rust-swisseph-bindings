//! Decode immutable Moshier term structure before compiling the C evaluator.
//!
//! Only signed-byte argument metadata is transformed. Coefficients remain in
//! the pinned arrays, and the evaluator retains their order and floating-point
//! expressions. Unsupported table changes fail closed rather than silently
//! changing the native calculation.

use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

const PLANETS: [&str; 9] = [
    "mer", "ven", "ear", "mar", "jup", "sat", "ura", "nep", "plu",
];
const EVALUATOR: &str = "src/native/moshier_metadata_eval.h";
// FNV-1a identifies the exact reviewed source fragment, not a security boundary.
const PINNED_EVALUATOR_FINGERPRINT: u64 = 0x485d_2085_3d01_0d66;

/// Generated translation unit and header, both confined to Cargo OUT_DIR.
pub struct Generated {
    pub source: PathBuf,
    pub header: PathBuf,
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

struct Plan {
    terms: Vec<Term>,
    arguments: Vec<Argument>,
    coefficients: usize,
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

    let mut header = String::from(
        "/* Generated from pinned swemptab.h; immutable structure only. */\n\
         struct swe_moshier_term { unsigned char count, degree, polynomial; };\n\
         struct swe_moshier_argument { unsigned char index, negative; };\n\
         struct swe_moshier_plan {\n\
           const struct swe_moshier_term *terms;\n\
           const struct swe_moshier_argument *arguments;\n\
           unsigned int count;\n\
         };\n",
    );
    for name in PLANETS {
        let plan = decode_plan(&clean, name).unwrap_or_else(|error| panic!("{name}: {error}"));
        writeln!(
            header,
            "static const struct swe_moshier_term {name}_terms[] = {{"
        )
        .unwrap();
        for term in &plan.terms {
            writeln!(
                header,
                "  {{{}, {}, {}}},",
                term.arguments,
                term.degree,
                u8::from(term.polynomial)
            )
            .unwrap();
        }
        writeln!(
            header,
            "}};\nstatic const struct swe_moshier_argument {name}_arguments[] = {{"
        )
        .unwrap();
        for arg in &plan.arguments {
            writeln!(header, "  {{{}, {}}},", arg.index, u8::from(arg.negative)).unwrap();
        }
        writeln!(
            header,
            "}}; /* {} terms; {} original coefficients per axis. */",
            plan.terms.len(),
            plan.coefficients
        )
        .unwrap();
    }
    header.push_str("static const struct swe_moshier_plan swe_moshier_plans[] = {\n");
    for name in PLANETS {
        writeln!(
            header,
            "  {{{name}_terms, {name}_arguments, sizeof({name}_terms) / sizeof({name}_terms[0])}},"
        )
        .unwrap();
    }
    header.push_str("};\n");

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
    };
    fs::write(&generated.source, overlay).expect("write generated Moshier source");
    fs::write(&generated.header, header).expect("write generated Moshier metadata");
    generated
}

/// Inputs declared to Cargo and the native-input manifest.
pub const INPUTS: [&str; 2] = ["build_support/moshier.rs", EVALUATOR];

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
    let plan = decode_arguments(&args, &harmonics, max_degree)?;
    for suffix in ["l", "b", "r"] {
        let coefficients = tokens(body_after(
            source,
            &format!("static double {name}tab{suffix}[] ="),
        )?)?;
        if coefficients.len() != plan.coefficients {
            return Err(format!(
                "{suffix} coefficient count {} differs from consumed {}",
                coefficients.len(),
                plan.coefficients
            ));
        }
    }
    Ok(plan)
}

fn decode_arguments(args: &[i16], harmonics: &[i16], max_degree: i16) -> Result<Plan, String> {
    let mut stream = args.iter().copied();
    let mut plan = Plan {
        terms: Vec::new(),
        arguments: Vec::new(),
        coefficients: 0,
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

fn tokens(body: &str) -> Result<Vec<String>, String> {
    let mut values: Vec<_> = body.split(',').map(str::trim).map(str::to_string).collect();
    if values.last().is_some_and(String::is_empty) {
        values.pop();
    }
    if values.iter().any(String::is_empty) {
        return Err("empty table element".into());
    }
    Ok(values)
}

fn integers(body: &str) -> Result<Vec<i16>, String> {
    tokens(body)?
        .into_iter()
        .map(|value| {
            value
                .parse()
                .map_err(|_| format!("invalid integer {value}"))
        })
        .collect()
}

/// Stable content identity for generated overlays; never a numerical value.
pub fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
    })
}

fn unique_offset(source: &str, marker: &str) -> Result<usize, String> {
    let mut matches = source.match_indices(marker);
    let (offset, _) = matches
        .next()
        .ok_or_else(|| format!("missing declaration {marker}"))?;
    if matches.next().is_some() {
        return Err(format!("ambiguous declaration {marker}"));
    }
    Ok(offset)
}

fn body_after<'a>(source: &'a str, marker: &str) -> Result<&'a str, String> {
    let start = unique_offset(source, marker)? + marker.len();
    let rest = source[start..]
        .trim_start()
        .strip_prefix('{')
        .ok_or("missing opening brace")?;
    let mut depth = 1;
    for (index, byte) in rest.bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return Ok(&rest[..index]);
        }
    }
    Err("missing closing brace".into())
}

fn without_comments(source: &str) -> Result<String, String> {
    let mut clean = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut closed = false;
            while let Some(ch) = chars.next() {
                if ch == '*' && chars.next_if_eq(&'/').is_some() {
                    closed = true;
                    break;
                }
            }
            if !closed {
                return Err("unterminated block comment".into());
            }
            clean.push(' ');
        } else if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            for ch in chars.by_ref() {
                if ch == '\n' {
                    break;
                }
            }
            clean.push('\n');
        } else {
            clean.push(ch);
        }
    }
    Ok(clean)
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
        assert_eq!(integers(" 1, -128, 127, ").unwrap(), [1, -128, 127]);
        assert!(integers("1,,nan").is_err());
        assert!(integers("1,,2").is_err());
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
}
