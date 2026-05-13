//! Shared best-effort redaction helpers for evidence artifacts.
//!
//! These helpers are intentionally conservative. They reduce obvious accidental
//! secret disclosure in local evidence, but they are not a complete DLP system.

pub const DEFAULT_CAPTURE_BYTES: usize = 262_144;
#[derive(Debug, Clone, Copy)]
pub struct RedactionConfig<'a> {
    pub max_bytes: usize,
    pub redacted_line: &'a str,
    pub truncated_notice: &'a str,
}

pub fn sanitize_bytes(bytes: &[u8], config: RedactionConfig<'_>) -> String {
    let text = String::from_utf8_lossy(bytes);
    sanitize_text(&text, config)
}

pub fn sanitize_text(input: &str, config: RedactionConfig<'_>) -> String {
    let redacted = redact_secret_like_lines(input, config.redacted_line);
    truncate_utf8(&redacted, config.max_bytes, config.truncated_notice)
}

pub fn redact_secret_like_lines(input: &str, redacted_line: &str) -> String {
    let mut out = String::new();
    for line in input.lines() {
        if is_secret_like_line(line) {
            out.push_str(redacted_line);
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

pub fn redact_one_line(input: &str, redacted_line: &str, max_chars: usize) -> String {
    let first = input.lines().next().unwrap_or_default();
    if is_secret_like_line(first) {
        return redacted_line.to_string();
    }
    first.chars().take(max_chars).collect()
}

pub fn truncate_utf8(input: &str, max_bytes: usize, truncated_notice: &str) -> String {
    if input.len() <= max_bytes {
        return input.to_string();
    }
    let mut end = max_bytes;
    while !input.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = input[..end].to_string();
    out.push_str(truncated_notice);
    out
}

pub fn is_secret_like_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let compact: String = lower
        .chars()
        .filter(|ch| !matches!(ch, '-' | '_' | '.'))
        .collect();

    lower.contains(".env")
        || lower.contains("password")
        || lower.contains("passwd")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.contains("authorization")
        || lower.contains("bearer ")
        || lower.contains("credential")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("access_key")
        || lower.contains("secret_key")
        || lower.contains("private_key")
        || lower.contains("client_secret")
        || lower.contains("refresh_token")
        || lower.contains("id_token")
        || lower.contains("session_key")
        || lower.contains("auth_header")
        || lower.contains("x-api-key")
        || lower.contains(".pem")
        || lower.contains(".p12")
        || lower.contains(".pfx")
        || lower.contains(".key")
        || compact.contains("awsaccesskeyid")
        || compact.contains("awssecretaccesskey")
        || lower.contains("ghp_")
        || lower.contains("github_pat_")
        || lower.contains("xoxb-")
        || lower.contains("sk-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_common_secret_like_lines() {
        let input = "ok\npassword=abc\nAuthorization: Bearer token\ndone\n";
        let output = redact_secret_like_lines(input, "[redacted]");
        assert!(output.contains("ok"));
        assert!(output.contains("[redacted]"));
        assert!(!output.contains("password=abc"));
        assert!(!output.contains("Bearer token"));
        assert!(output.contains("done"));
    }

    #[test]
    fn truncates_on_utf8_boundary() {
        let input = "abc\u{00e9}def";
        let output = truncate_utf8(input, 4, "\n[truncated]\n");
        assert!(output.ends_with("[truncated]\n"));
    }

    #[test]
    fn redacts_secret_like_summary_line() {
        let output = redact_one_line("token=abc", "[redacted summary]", 240);
        assert_eq!(output, "[redacted summary]");
    }
}
