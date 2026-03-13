fn split_wrapped_passthrough_token(token: &str) -> Vec<String> {
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

pub fn parse_passthrough_flags(flags: &str) -> Result<Vec<String>, String> {
    let parsed = shlex::split(flags)
        .ok_or_else(|| "--trippy-flags contains invalid shell quoting".to_string())?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wrapped_passthrough_token() {
        let parsed = parse_passthrough_flags("\"--mode tui --dns-ttl 5s\"").unwrap();
        assert_eq!(parsed, vec!["--mode", "tui", "--dns-ttl", "5s"]);
    }

    #[test]
    fn rejects_invalid_shell_quoting() {
        assert!(parse_passthrough_flags("\"--foo").is_err());
    }

    #[test]
    fn preserves_inner_quoted_segments_when_splitting_wrapped_token() {
        let parsed = parse_passthrough_flags("\"--label 'hello world' --mode tui\"").unwrap();
        assert_eq!(parsed, vec!["--label", "hello world", "--mode", "tui"]);
    }

    #[test]
    fn keeps_unclosed_quote_literal_for_follow_up_validation() {
        let parsed = split_wrapped_passthrough_token("--flag \"unterminated value");
        assert_eq!(parsed, vec!["--flag", "\"unterminated value"]);
    }
}
