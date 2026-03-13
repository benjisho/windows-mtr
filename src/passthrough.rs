const INVALID_SHELL_QUOTING_MSG: &str = "--trippy-flags contains invalid shell quoting";

pub fn split_wrapped_passthrough_token(token: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut active_quote: Option<char> = None;

    for ch in token.chars() {
        if let Some(quote) = active_quote {
            if ch == quote {
                active_quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }

        if ch.is_whitespace() {
            if !current.is_empty() {
                result.push(std::mem::take(&mut current));
            }
            continue;
        }

        if (ch == '\'' || ch == '"') && current.is_empty() {
            active_quote = Some(ch);
            continue;
        }

        current.push(ch);
    }

    if let Some(quote) = active_quote {
        current.insert(0, quote);
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

pub fn parse_passthrough_flags(flags: &str) -> std::result::Result<Vec<String>, &'static str> {
    let parsed = shlex::split(flags).ok_or(INVALID_SHELL_QUOTING_MSG)?;

    // Windows shells sometimes preserve wrapping quotes around the entire passthrough
    // string, which can produce a single token like "--flag value". Split that token
    // into distinct argv entries while preserving embedded quoted segments.
    if parsed.len() == 1 {
        let token = &parsed[0];
        if token.starts_with("--") && token.contains(' ') {
            return Ok(split_wrapped_passthrough_token(token));
        }
    }

    Ok(parsed)
}
