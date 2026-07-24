//! Log/message sanitization helpers for safe health, error and metric outputs.

use serde_json::Value;

const SECRET_KEYS: &[&str] = &[
    "password",
    "secret",
    "token",
    "api_key",
    "private_key",
    "credential",
    "access_key",
];

const SECRET_PATTERNS: &[&str] = &[
    "password=",
    "secret=",
    "token=",
    "api_key=",
    "private_key=",
    "access_key_id=",
    "secret_access_key=",
    "authorization: bearer ",
    "basic ",
];

/// Recursively redact common secret keys inside a JSON value.
pub fn sanitize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    if is_secret_key(k) {
                        (k.clone(), Value::String("[REDACTED]".into()))
                    } else {
                        (k.clone(), sanitize_json(v))
                    }
                })
                .collect(),
        ),
        Value::Array(arr) => Value::Array(arr.iter().map(sanitize_json).collect()),
        other => other.clone(),
    }
}

/// Returns true when `key` contains any secret token as a distinct word.
fn is_secret_key(key: &str) -> bool {
    let lower = key.to_lowercase();
    SECRET_KEYS.iter().any(|pat| {
        lower.match_indices(pat).any(|(pos, _)| {
            let before_ok = pos == 0
                || lower
                    .as_bytes()
                    .get(pos.saturating_sub(1))
                    .is_some_and(|b| !b.is_ascii_alphanumeric());
            let after_pos = pos.saturating_add(pat.len());
            let after_ok = after_pos >= lower.len()
                || lower.as_bytes().get(after_pos).is_some_and(|b| !b.is_ascii_alphanumeric());
            before_ok && after_ok
        })
    })
}

/// Redact secret-like substrings from a free-text message.  This is a fallback
/// for external strings that are not already structured logs.
pub fn sanitize_message(message: &str) -> String {
    let mut out = message.to_string();
    for pat in SECRET_PATTERNS {
        if let Some(idx) = out.to_lowercase().find(pat) {
            let start = idx.saturating_add(pat.len());
            let end = out[start..]
                .find(|c: char| c.is_whitespace() || c == '&' || c == '\n')
                .map(|n| start.saturating_add(n))
                .unwrap_or(out.len());
            out.replace_range(start..end, "[REDACTED]");
        }
    }
    out
}

/// Returns a safe representation of a URL with userinfo stripped and only
/// whitelisted schemes.  Non-conforming URLs return `None`.
pub fn safe_url(url_str: &str) -> Option<String> {
    let url = url::Url::parse(url_str).ok()?;
    match url.scheme() {
        "http" | "https" | "s3" | "s3a" | "minio" => {}
        _ => return None,
    }
    let host = url.host_str()?;
    if host.is_empty() || host == "localhost" || host == "127.0.0.1" || host == "::1" {
        return None;
    }
    let mut safe = url.clone();
    if safe.password().is_some() || safe.username() != "" {
        let _ = safe.set_username("");
        let _ = safe.set_password(None);
    }
    Some(safe.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secret_json() {
        let v = serde_json::json!({"password": "abc", "tenant": "t1"});
        let safe = sanitize_json(&v);
        assert_eq!(safe["password"], "[REDACTED]");
        assert_eq!(safe["tenant"], "t1");
    }

    #[test]
    fn strips_url_credentials() {
        let s = safe_url("https://user:pass@example.com/path").unwrap();
        assert!(!s.contains("pass"));
    }

    #[test]
    fn rejects_localhost_url() {
        assert!(safe_url("http://localhost:9000/bucket").is_none());
    }
}
