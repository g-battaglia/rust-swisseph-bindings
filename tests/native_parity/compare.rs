//! Independent result projectors and strict comparisons; native values never
//! appear in failure messages. Numerical mismatch reports identify only slots.
use super::protocol::{Record, Request};
use super::registry::{Layout, Op};
use super::rust_worker::{INVALID_INPUT, NATIVE_ERROR};

/// Map complete native house buffers to the public arrays without copying the
/// binding's implementation. Exact ABI width is checked before slicing.
pub(super) fn project_houses(
    record: &mut Record,
    cusps: usize,
    speeds: bool,
) -> Result<(), String> {
    let width = cusps + 1 + 10;
    let expected = if speeds { width * 2 } else { width };
    if record.floats.len() != expected {
        return Err("native house allocation width".into());
    }
    let mut result = Vec::with_capacity(if speeds { (cusps + 8) * 2 } else { cusps + 8 });
    for offset in [0, width].into_iter().take(if speeds { 2 } else { 1 }) {
        result.extend_from_slice(&record.floats[offset + 1..offset + cusps + 1]);
        result.extend_from_slice(&record.floats[offset + cusps + 1..offset + cusps + 9]);
    }
    if cusps == 12 {
        // Both twelve-cusp types expose these getters in addition to arrays.
        result.extend_from_slice(&record.floats[cusps + 1..cusps + 3]);
    }
    record.floats = result;
    // These public house types do not expose successful native diagnostics.
    record.texts.clear();
    Ok(())
}

/// Independently map the named getters to their native leading slots. The raw
/// array remains fully checked, including its reserved tail, before duplication.
pub(super) fn project_accessors(
    record: &mut Record,
    width: usize,
    accessors: usize,
) -> Result<(), String> {
    if record.floats.len() != width || accessors > width {
        return Err("native accessor source width".into());
    }
    let leading = record.floats[..accessors].to_vec();
    record.floats.extend(leading);
    Ok(())
}

/// Project non-leading getters by the separately frozen public slot contract.
/// Width/index checks precede every slice/index, and raw tails are retained.
fn project_slots(record: &mut Record, width: usize, indices: &[usize]) -> Result<(), String> {
    if record.floats.len() != width || indices.iter().any(|index| *index >= width) {
        return Err("native named-slot width/index".into());
    }
    let getters = indices
        .iter()
        .map(|index| record.floats[*index])
        .collect::<Vec<_>>();
    record.floats.extend(getters);
    Ok(())
}

/// Validate that a complete diagnostic is retained with recognized Rust
/// context. Native prefixes, punctuation and whitespace are not normalized.
fn native_error(op: Op, c: &Record, rust: &Record) -> Result<usize, String> {
    if rust.code != NATIVE_ERROR {
        return Err("native error classification".into());
    }
    if rust.ints.len() + rust.floats.len() != 0 || rust.texts.len() != 1 {
        return Err("error exposes success-shaped components".into());
    }
    let message = std::str::from_utf8(&rust.texts[0]).map_err(|_| "Rust error encoding")?;
    let prefix = format!("{}: ", op.entry().name);
    let body = message
        .strip_prefix(&prefix)
        .ok_or("missing error operation context")?;
    let diagnostic = c
        .texts
        .last()
        .map(|s| String::from_utf8_lossy(s))
        .unwrap_or_default();
    if diagnostic.is_empty() {
        if body.is_empty() {
            return Err("missing synthesized native error context".into());
        }
    } else if matches!(op, Op::HousesEx2 | Op::HousesArmcEx2 | Op::HousesGauquelin) {
        // House errors put the input context first and the diagnostic last;
        // this is an explicitly different framing, not a substring allowance.
        let framed = body
            .strip_prefix("native computation failed (")
            .and_then(|body| body.split_once("): "))
            .map(|(_, diagnostic)| diagnostic)
            .ok_or("unrecognized house error framing")?;
        if framed != diagnostic {
            return Err("native house diagnostic altered or omitted".into());
        }
    } else if matches!(op, Op::Fixstar2 | Op::Fixstar2Ut | Op::Fixstar2Mag)
        && body.starts_with("fixed-star catalog unavailable; ")
    {
        let framed = body
            .strip_prefix(
                "fixed-star catalog unavailable; configure set_ephe_path or SE_EPHE_PATH: ",
            )
            .ok_or("unrecognized catalog preflight framing")?;
        if framed != diagnostic {
            return Err("catalog preflight diagnostic altered or omitted".into());
        }
    } else {
        let suffix = body
            .strip_prefix(diagnostic.as_ref())
            .ok_or("native diagnostic altered or omitted")?;
        if !suffix.is_empty() && !(suffix.starts_with(" (") && suffix.ends_with(')')) {
            return Err("unrecognized diagnostic framing".into());
        }
    }
    Ok(2)
}

/// Classify raw double-returning crossing sentinels against the input epoch.
/// No raw C status or float is rewritten to manufacture a numerical match.
pub fn operation(
    op: Op,
    input: &Request,
    native: Option<&Record>,
    rust: &Record,
) -> Result<usize, String> {
    if matches!(
        op,
        Op::Solcross
            | Op::SolcrossUt
            | Op::Mooncross
            | Op::MooncrossUt
            | Op::MooncrossNode
            | Op::MooncrossNodeUt
    ) {
        let c = native.ok_or("missing native crossing")?;
        let node = matches!(op, Op::MooncrossNode | Op::MooncrossNodeUt);
        if c.floats.len() != if node { 3 } else { 1 }
            || c.texts.len() != 1
            || c.code != 0
            || !c.ints.is_empty()
        {
            return Err("native double-crossing layout".into());
        }
        if !c.floats[0].is_finite() || c.floats[0] < input.f(if node { 0 } else { 1 }) {
            return native_error(op, c, rust);
        }
    }
    compare(op, native, rust, false)
}

/// Compare one public operation to its C result; `rejected` means the driver
/// intentionally did not dispatch a hazardous request to the C worker.
pub fn compare(
    op: Op,
    native: Option<&Record>,
    rust: &Record,
    rejected: bool,
) -> Result<usize, String> {
    if rejected {
        return if rust.code == INVALID_INPUT
            && rust.texts.len() == 1
            && rust.ints.is_empty()
            && rust.floats.is_empty()
        {
            Ok(1)
        } else {
            Err("safe-boundary rejection".into())
        };
    }
    let c = native.ok_or("missing C response")?;
    if c.op != rust.op || c.id != rust.id {
        return Err("response identity".into());
    }
    if matches!(op, Op::Calc | Op::CalcUt | Op::CalcPctr)
        && c.code >= 0
        && c.floats.iter().any(|value| !value.is_finite())
    {
        // The existing public contract exposes no Position on non-finite C
        // output, even when C returns nonnegative flags. This is an error
        // classification, never NaN-class equality for scientific components.
        if c.floats.len() != 6 || c.texts.len() != 1 || !c.ints.is_empty() {
            return Err("non-finite native position layout".into());
        }
        let expected = format!(
            "{}: non-finite native position output; check observer/sidereal configuration: {}",
            op.entry().name,
            String::from_utf8_lossy(&c.texts[0])
        );
        if rust.code != NATIVE_ERROR
            || !rust.ints.is_empty()
            || !rust.floats.is_empty()
            || rust.texts != [expected.into_bytes()]
        {
            return Err("non-finite position classification/complete diagnostic".into());
        }
        return Ok(2);
    }
    if matches!(op, Op::Refrac | Op::RefracExtended) && c.floats.iter().any(|v| !v.is_finite()) {
        // Version 1.0.0 promises a classified Native error, not non-finite
        // partial refraction data. These two C APIs have no diagnostic buffer.
        let expected = if op == Op::Refrac {
            "refrac: native calculation produced a non-finite converted altitude"
        } else {
            "refrac_extended: native calculation produced a non-finite converted altitude or detail"
        };
        let width = if op == Op::Refrac { 1 } else { 5 };
        if c.code != 0
            || c.floats.len() != width
            || !c.ints.is_empty()
            || !c.texts.is_empty()
            || rust.code != NATIVE_ERROR
            || !rust.ints.is_empty()
            || !rust.floats.is_empty()
            || rust.texts != [expected.as_bytes().to_vec()]
        {
            return Err("non-finite refraction outcome/diagnostic".into());
        }
        return Ok(2);
    }
    let circumpolar = c.code == -2 && matches!(op, Op::RiseTrans | Op::RiseTransTrueHor);
    let below_horizon = c.code == -2 && op == Op::VisLimitMag;
    if c.code < 0 && !circumpolar && !below_horizon {
        return native_error(op, c, rust);
    }
    let mut c = c.clone();
    if circumpolar {
        if c.floats.len() != 1 || c.texts.len() != 1 {
            return Err("circumpolar native layout".into());
        }
        // The native output time has no public meaning on this outcome.
        c.floats.clear();
    }
    match op {
        Op::Calc
        | Op::CalcUt
        | Op::CalcPctr
        | Op::Fixstar
        | Op::FixstarUt
        | Op::Fixstar2
        | Op::Fixstar2Ut => project_accessors(&mut c, 6, 3)?,
        Op::Pheno | Op::PhenoUt => project_accessors(&mut c, 20, 5)?,
        Op::GetOrbitalElements => project_accessors(&mut c, 50, 17)?,
        Op::HeliacalUt => project_accessors(&mut c, 10, 3)?,
        Op::HeliacalPhenoUt => project_accessors(&mut c, 30, 28)?,
        Op::VisLimitMag if !below_horizon => project_slots(&mut c, 8, &[0, 1, 2, 3, 4, 7])?,
        Op::SolEclipseWhenGlob
        | Op::LunEclipseWhen
        | Op::LunOccultWhenGlob
        | Op::LunOccultWhenGlobWithOptions => project_slots(&mut c, 10, &[0, 2, 3, 4, 5, 6, 7])?,
        Op::SolEclipseHow => project_slots(&mut c, 20, &[0, 1, 2, 3, 4, 5, 6, 7, 9, 10])?,
        Op::LunEclipseHow => project_slots(&mut c, 20, &[0, 1, 4, 5, 6, 7, 9, 10])?,
        Op::SolEclipseWhenLoc => project_slots(
            &mut c,
            27,
            &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17],
        )?,
        Op::LunEclipseWhenLoc => project_slots(
            &mut c,
            30,
            &[0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 14, 15, 16, 17, 19, 20],
        )?,
        Op::LunOccultWhenLoc | Op::LunOccultWhenLocWithOptions => project_slots(
            &mut c,
            30,
            &[0, 1, 2, 3, 4, 5, 6, 10, 11, 12, 13, 14, 15, 16, 17],
        )?,
        Op::LunOccultWhere => project_slots(&mut c, 22, &[2, 3, 4, 5, 6, 7, 8, 9])?,
        _ => (),
    }
    match op.entry().layout {
        Layout::Houses => project_houses(&mut c, 12, false)?,
        Layout::HouseSpeeds => project_houses(&mut c, 12, true)?,
        Layout::Sectors => project_houses(&mut c, 36, true)?,
        Layout::JulDays => {
            if c.floats.len() != 2 || c.texts.len() != 1 {
                return Err("native UTC output width".into());
            }
            // JulDays exposes only ET/UT, not the native diagnostic buffer.
            c.texts.clear();
        }
        _ => (),
    }
    same(
        &c,
        rust,
        op.entry().layout == Layout::PureScalar || matches!(op, Op::Cotrans | Op::CotransSp),
    )
}

/// Check a Rust-only strict-source policy using the preceding genuine C
/// position as evidence. Expected acceptance is an input-recipe contract, not
/// a copy of the production diagnostic parser or a fabricated Position.
pub fn source_policy(
    native: &Record,
    rust: &Record,
    accepted: bool,
    primary: i32,
    moshier_component: bool,
) -> Result<usize, String> {
    use swisseph_bindings::{FLG_JPLEPH, FLG_MOSEPH, FLG_SWIEPH};
    let mask = FLG_JPLEPH | FLG_MOSEPH | FLG_SWIEPH;
    if native.code < 0
        || native.code & mask != primary
        || native.floats.len() != 6
        || native.texts.len() != 1
    {
        return Err("source-policy native primary evidence".into());
    }
    if moshier_component
        && !native.texts[0]
            .windows(b"using Moshier eph".len())
            .any(|part| part == b"using Moshier eph")
    {
        return Err("source-policy fixture lacks native Moshier component evidence".into());
    }
    if accepted {
        if rust.code != 0
            || !rust.ints.is_empty()
            || !rust.floats.is_empty()
            || !rust.texts.is_empty()
        {
            return Err("strict-source accepted outcome".into());
        }
    } else {
        if rust.code != NATIVE_ERROR
            || !rust.ints.is_empty()
            || !rust.floats.is_empty()
            || rust.texts.len() != 1
        {
            return Err("strict-source rejected outcome".into());
        }
        let message =
            std::str::from_utf8(&rust.texts[0]).map_err(|_| "source-policy error encoding")?;
        let diagnostic = String::from_utf8_lossy(&native.texts[0]);
        let suffix = format!(
            "unexpected primary/component fallback ({})",
            if diagnostic.is_empty() {
                "no native diagnostic"
            } else {
                &diagnostic
            }
        );
        if !message.starts_with("require_source_flags: requested source bits ")
            || !message.ends_with(&suffix)
        {
            return Err("strict-source complete diagnostic/context".into());
        }
    }
    Ok(3)
}

/// Exact exposed-field comparison, with NaN-class matching restricted to
/// explicitly documented pure floating-point propagation paths.
pub fn same(c: &Record, rust: &Record, allow_nonfinite: bool) -> Result<usize, String> {
    if c.code != rust.code {
        return Err(if c.code >= 0 && rust.code == NATIVE_ERROR {
            "C success / Rust Native error"
        } else if c.code >= 0 && rust.code >= 0 {
            "returned native flag sets differ"
        } else {
            "status/error class differs"
        }
        .into());
    }
    if c.ints.len() != rust.ints.len()
        || c.floats.len() != rust.floats.len()
        || c.texts.len() != rust.texts.len()
    {
        return Err("result field count/truncation".into());
    }
    for (index, (a, b)) in c.ints.iter().zip(&rust.ints).enumerate() {
        if a != b {
            return Err(format!("integer[{index}]"));
        }
    }
    for (index, (a, b)) in c.floats.iter().zip(&rust.floats).enumerate() {
        if !allow_nonfinite && (!a.is_finite() || !b.is_finite()) {
            return Err(format!("unexpected non-finite float[{index}]"));
        }
        if a.to_bits() != b.to_bits() && !(allow_nonfinite && a.is_nan() && b.is_nan()) {
            return Err(format!("float[{index}] finite bits/sign/non-finite class"));
        }
    }
    for (index, (a, b)) in c.texts.iter().zip(&rust.texts).enumerate() {
        if String::from_utf8_lossy(a).as_bytes() != b {
            return Err(format!("text[{index}]"));
        }
    }
    Ok(1 + c.ints.len() + c.floats.len() + c.texts.len())
}

/// Mutation tests use synthetic values only; they prove that altered flags,
/// bytes, signed zero, ordering and tails cannot produce a vacuous pass.
pub fn self_test() -> Result<(), String> {
    let original = Record {
        code: 256,
        ints: vec![1, 2],
        floats: vec![0.0, 1.0, 2.0],
        texts: vec![b"native diagnostic".to_vec(), b"resolved name".to_vec()],
        ..Record::default()
    };
    same(&original, &original, false)?;
    let mut mutations = Vec::new();
    let mut r = original.clone();
    r.floats[1] = f64::from_bits(1.0_f64.to_bits() + 1);
    mutations.push(r);
    let mut r = original.clone();
    r.floats[0] = -0.0;
    mutations.push(r);
    let mut r = original.clone();
    r.code ^= 1;
    mutations.push(r);
    let mut r = original.clone();
    r.floats.swap(1, 2);
    mutations.push(r);
    let mut r = original.clone();
    r.floats.pop();
    mutations.push(r);
    let mut r = original.clone();
    r.texts.pop();
    mutations.push(r);
    let mut r = original.clone();
    r.texts[0].pop();
    mutations.push(r);
    let mut r = original.clone();
    r.ints[0] = -2;
    mutations.push(r);
    for mutation in mutations {
        if same(&original, &mutation, false).is_ok() {
            return Err("comparator mutation accepted".into());
        }
    }
    for (cusps, speeds) in [(12, false), (12, true), (36, true)] {
        let width = cusps + 11;
        let raw: Vec<_> = (0..width * if speeds { 2 } else { 1 })
            .map(|n| n as f64)
            .collect();
        let mut record = Record {
            floats: raw.clone(),
            ..Record::default()
        };
        project_houses(&mut record, cusps, speeds)?;
        let mut expected = Vec::new();
        // Synthetic slot labels independently enumerate the public indices.
        for block in 0..if speeds { 2 } else { 1 } {
            for slot in 1..=cusps {
                expected.push((block * width + slot) as f64);
            }
            for slot in 0..8 {
                expected.push((block * width + cusps + 1 + slot) as f64);
            }
        }
        if cusps == 12 {
            expected.extend([(cusps + 1) as f64, (cusps + 2) as f64]);
        }
        if record.floats != expected {
            return Err("house projector synthetic slots".into());
        }
        if cusps == 12 {
            let mut altered = record.clone();
            let end = altered.floats.len() - 1;
            altered.floats[end] += 1.0;
            if same(&record, &altered, false).is_ok() {
                return Err("altered house accessor accepted".into());
            }
        }
        let mut truncated = Record {
            floats: raw[..raw.len() - 1].to_vec(),
            ..Record::default()
        };
        if project_houses(&mut truncated, cusps, speeds).is_ok() {
            return Err("truncated native house buffer accepted".into());
        }
    }
    for (width, accessors) in [(6, 3), (20, 5), (50, 17)] {
        let raw = (1..=width).map(|slot| slot as f64).collect::<Vec<_>>();
        let mut projected = Record {
            floats: raw.clone(),
            ..Record::default()
        };
        project_accessors(&mut projected, width, accessors)?;
        let expected = raw
            .iter()
            .chain(&raw[..accessors])
            .copied()
            .collect::<Vec<_>>();
        if projected.floats != expected {
            return Err("accessor projector self-test".into());
        }
        let mut changed = projected.clone();
        changed.floats[width] = 0.0;
        if same(&projected, &changed, false).is_ok() {
            return Err("altered accessor accepted".into());
        }
        projected.floats.pop();
        if project_accessors(&mut projected, width, accessors).is_ok() {
            return Err("accessor raw-width mismatch accepted".into());
        }
    }
    // Both recognized error framings must retain the full diagnostic. These
    // synthetic fixtures test framing, not any stored engine error vector.
    for (op, message) in [
        (Op::Calc, "calc: diagnostic (jd_et=0)"),
        (
            Op::HousesEx2,
            "houses_ex2: native computation failed (jd_ut=0): diagnostic",
        ),
    ] {
        let native = Record {
            code: -1,
            texts: vec![b"diagnostic".to_vec()],
            ..Record::default()
        };
        let mut rust = Record {
            code: NATIVE_ERROR,
            texts: vec![message.as_bytes().to_vec()],
            ..Record::default()
        };
        native_error(op, &native, &rust)?;
        rust.texts[0] = message.replace("diagnostic", "diagnosti").into_bytes();
        if native_error(op, &native, &rust).is_ok() {
            return Err("partial error diagnostic accepted".into());
        }
    }
    for op in [Op::Calc, Op::CalcUt, Op::CalcPctr] {
        let native = Record {
            op: op as u32,
            code: 256,
            floats: vec![f64::NAN, 0.0, 0.0, 0.0, 0.0, 0.0],
            texts: vec![b"complete diagnostic".to_vec()],
            ..Record::default()
        };
        let mut error = Record { op: op as u32, code: NATIVE_ERROR, texts: vec![format!("{}: non-finite native position output; check observer/sidereal configuration: complete diagnostic", op.entry().name).into_bytes()], ..Record::default() };
        compare(op, Some(&native), &error, false)?;
        error.texts[0].pop();
        if compare(op, Some(&native), &error, false).is_ok() {
            return Err("non-finite position partial diagnostic accepted".into());
        }
    }
    let encoded = original.encode();
    let decoded = Record::decode(&encoded).map_err(|e| e.to_string())?;
    same(&original, &decoded, false)?;
    for n in 0..encoded.len() {
        if Record::decode(&encoded[..n]).is_ok() {
            return Err("truncated protocol accepted".into());
        }
    }
    let mut trailing = encoded;
    trailing.push(0);
    if Record::decode(&trailing).is_ok() {
        return Err("trailing protocol accepted".into());
    }
    Ok(())
}
