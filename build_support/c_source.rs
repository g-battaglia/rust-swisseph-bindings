//! Small fail-closed readers for the pinned C table grammar, not a general C parser.

pub(super) fn tokens(body: &str) -> Result<Vec<String>, String> {
    let mut values: Vec<_> = body.split(',').map(str::trim).map(str::to_string).collect();
    if values.last().is_some_and(String::is_empty) {
        values.pop();
    }
    if values.iter().any(String::is_empty) {
        return Err("empty table element".into());
    }
    Ok(values)
}

pub(super) fn integers<T: std::str::FromStr>(body: &str) -> Result<Vec<T>, String> {
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
pub(super) fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
    })
}

pub(super) fn unique_offset(source: &str, marker: &str) -> Result<usize, String> {
    let mut matches = source.match_indices(marker);
    let (offset, _) = matches
        .next()
        .ok_or_else(|| format!("missing declaration {marker}"))?;
    if matches.next().is_some() {
        return Err(format!("ambiguous declaration {marker}"));
    }
    Ok(offset)
}

pub(super) fn body_after<'a>(source: &'a str, marker: &str) -> Result<&'a str, String> {
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

pub(super) fn without_comments(source: &str) -> Result<String, String> {
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
