//! Secret provider, certificate lifecycle and security controls.

use moqentra_types::UtcTimestamp;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// How a secret value is supplied to the runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretProvider {
    File {
        path: String,
    },
    Env {
        name: String,
    },
    External {
        manager: String,
        key: String,
        version: String,
    },
}

impl SecretProvider {
    /// Resolves a secret value. For `File` variants the resolved path must be
    /// absolute and confined to `allowed_root`.
    pub fn resolve(&self, allowed_root: impl AsRef<Path>) -> Option<String> {
        match self {
            SecretProvider::File { path } => {
                if Self::is_dangerous_path(path) {
                    return None;
                }
                if !Self::is_path_within_root(path, allowed_root.as_ref()) {
                    return None;
                }
                let meta = std::fs::symlink_metadata(path).ok()?;
                if meta.file_type().is_symlink() {
                    return None;
                }
                std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
            }
            SecretProvider::Env { name } => std::env::var(name).ok(),
            SecretProvider::External { .. } => {
                // Placeholder for external secret manager integration.
                None
            }
        }
    }

    fn is_dangerous_path(path: &str) -> bool {
        path.is_empty()
            || path.contains('\0')
            || !path.starts_with('/')
            || path.split('/').any(|c| c == "..")
    }

    fn is_path_within_root(path: &str, root: &Path) -> bool {
        let root = match root.canonicalize() {
            Ok(p) => p,
            Err(_) => return false,
        };
        let abs = match std::fs::canonicalize(path) {
            Ok(p) => p,
            Err(_) => return false,
        };
        abs.starts_with(root)
    }
}

/// X.509 certificate with rotation support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Certificate {
    pub subject: String,
    pub thumbprint: String,
    pub not_before: UtcTimestamp,
    pub not_after: UtcTimestamp,
    pub active: bool,
    pub previous_thumbprint: Option<String>,
}

impl Certificate {
    pub fn is_valid(&self, now: UtcTimestamp) -> bool {
        self.active && now >= self.not_before && now < self.not_after
    }

    pub fn should_rotate(&self, now: UtcTimestamp, lookahead_seconds: u64) -> bool {
        if let Some(rotate_deadline) =
            now.add_std_duration(std::time::Duration::from_secs(lookahead_seconds))
        {
            rotate_deadline >= self.not_after
        } else {
            false
        }
    }
}

/// Redacts sensitive patterns from strings.
#[derive(Debug, Clone, Default)]
pub struct SecretRedactor {
    patterns: Vec<String>,
}

impl SecretRedactor {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                "password".to_string(),
                "secret".to_string(),
                "token".to_string(),
                "api_key".to_string(),
                "private_key".to_string(),
            ],
        }
    }

    pub fn redact(&self, input: &str) -> String {
        let mut output = input.to_string();
        for pat in &self.patterns {
            output = Self::redact_pattern(&output, pat);
        }
        output
    }

    fn redact_pattern(input: &str, pat: &str) -> String {
        let pat_lower = pat.to_lowercase();
        let mut output = String::with_capacity(input.len());
        let mut rest = input;
        while let Some(pos) = Self::find_ci(rest, &pat_lower) {
            let match_end = pos + pat.len();
            // The match is only a secret key if the character before it (if any)
            // is not an ASCII alphanumeric character and the match is followed by
            // an '=' or ':' separator (with optional whitespace).
            let before_ok = pos == 0 || {
                let b = rest.as_bytes()[pos - 1];
                !b.is_ascii_alphanumeric()
            };
            let (sep_len, has_sep) = Self::skip_value_separator(&rest[match_end..]);
            if !before_ok || !has_sep {
                // Not a key/value pair; copy the matched text and continue.
                output.push_str(&rest[..match_end]);
                rest = &rest[match_end..];
                continue;
            }
            let value_start = match_end + sep_len;
            let value_str = &rest[value_start..];
            let consumed = if let Some(quote) =
                value_str.chars().next().filter(|c| matches!(c, '"' | '\''))
            {
                let mut escaped = false;
                value_str
                    .char_indices()
                    .skip(1)
                    .find(|(_, c)| {
                        if escaped {
                            escaped = false;
                            return false;
                        }
                        if *c == '\\' {
                            escaped = true;
                            return false;
                        }
                        *c == quote
                    })
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(value_str.len())
            } else {
                value_str
                    .char_indices()
                    .find(|(_, c)| c.is_whitespace() || matches!(c, '&' | ',' | ';' | '}' | ']'))
                    .map(|(i, _)| i)
                    .unwrap_or(value_str.len())
            };
            output.push_str(&rest[..pos]);
            output.push_str("[REDACTED]");
            rest = &rest[value_start + consumed..];
        }
        output.push_str(rest);
        output
    }

    /// Case-insensitive search for an ASCII pattern in `haystack`.
    /// Returns the byte index of the first match, or `None` if not found.
    fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
        if needle.is_empty() {
            return None;
        }
        let hay = haystack.as_bytes();
        let n = needle.len();
        hay.windows(n).position(|w| {
            w.iter().zip(needle.as_bytes()).all(|(a, b)| a.to_ascii_lowercase() == *b)
        })
    }

    /// If `s` starts with optional whitespace then '=' or ':', returns the
    /// total byte length of that prefix and `true`. Otherwise returns `(0, false)`.
    fn skip_value_separator(s: &str) -> (usize, bool) {
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < bytes.len() && (bytes[i] == b'=' || bytes[i] == b':') {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            (i, true)
        } else {
            (0, false)
        }
    }
}

/// Layered resource limits for uploads/archives/models/URLs/proto/JSON/logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityLimits {
    pub max_upload_size: u64,
    pub max_archive_depth: u32,
    pub max_archive_files: u32,
    pub max_proto_message_size: u64,
    pub max_json_depth: u32,
    pub max_json_size: u64,
    pub max_log_line_length: u32,
    pub max_url_length: u32,
}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_upload_size: 5 * 1024 * 1024 * 1024,
            max_archive_depth: 5,
            max_archive_files: 1000,
            max_proto_message_size: 64 * 1024 * 1024,
            max_json_depth: 32,
            max_json_size: 16 * 1024 * 1024,
            max_log_line_length: 8192,
            max_url_length: 2048,
        }
    }
}

impl SecurityLimits {
    pub fn check_upload_size(&self, size: u64) -> Result<(), moqentra_types::Error> {
        if size > self.max_upload_size {
            Err(moqentra_types::Error::permission_denied(
                "upload exceeds size limit",
            ))
        } else {
            Ok(())
        }
    }

    pub fn check_url_length(&self, len: usize) -> Result<(), moqentra_types::Error> {
        if len as u64 > self.max_url_length as u64 {
            Err(moqentra_types::Error::invalid_argument("url too long"))
        } else {
            Ok(())
        }
    }

    pub fn check_json_size(&self, size: usize) -> Result<(), moqentra_types::Error> {
        if size as u64 > self.max_json_size {
            Err(moqentra_types::Error::invalid_argument("json too large"))
        } else {
            Ok(())
        }
    }

    pub fn check_proto_message_size(&self, size: u64) -> Result<(), moqentra_types::Error> {
        if size > self.max_proto_message_size {
            Err(moqentra_types::Error::invalid_argument(
                "protobuf message too large",
            ))
        } else {
            Ok(())
        }
    }

    pub fn check_log_line_length(&self, len: usize) -> Result<(), moqentra_types::Error> {
        if len as u64 > self.max_log_line_length as u64 {
            Err(moqentra_types::Error::invalid_argument("log line too long"))
        } else {
            Ok(())
        }
    }

    pub fn check_json_depth(
        &self,
        value: &serde_json::Value,
        depth: u32,
    ) -> Result<(), moqentra_types::Error> {
        if depth > self.max_json_depth {
            return Err(moqentra_types::Error::invalid_argument("json too deep"));
        }
        match value {
            serde_json::Value::Array(arr) => {
                for v in arr {
                    self.check_json_depth(v, depth + 1)?;
                }
            }
            serde_json::Value::Object(obj) => {
                for v in obj.values() {
                    self.check_json_depth(v, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Signed artifact manifest for release bundles and images.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedArtifact {
    pub digest: String,
    pub signature: String,
    pub sbom_reference: String,
    pub provenance_reference: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_validity_and_rotation() {
        let now = UtcTimestamp::now();
        let cert = Certificate {
            subject: "svc".to_string(),
            thumbprint: "t1".to_string(),
            not_before: now,
            not_after: now.add_std_duration(std::time::Duration::from_secs(3600)).unwrap(),
            active: true,
            previous_thumbprint: None,
        };
        assert!(cert.is_valid(now));
        assert!(cert.should_rotate(now, 3600));
        assert!(!cert.should_rotate(now, 300));
    }

    #[test]
    fn secret_redaction() {
        let redactor = SecretRedactor::new();
        let out = redactor.redact("password=foo token=bar");
        assert!(!out.contains("password"));
        assert!(!out.contains("token"));
    }

    #[test]
    fn secret_redaction_handles_quotes() {
        let redactor = SecretRedactor::new();
        let out = redactor.redact("password=\"my secret\" api_key='another key'");
        assert!(!out.contains("my secret"));
        assert!(!out.contains("another key"));
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn secret_provider_file_confined_to_root() {
        let root = format!("/tmp/moqentra-secret-test-{}", std::process::id());
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).unwrap();
        let path = format!("{}/secret.txt", root);
        std::fs::write(&path, "hunter2").unwrap();

        let provider = SecretProvider::File { path: path.clone() };
        assert_eq!(provider.resolve(&root).unwrap(), "hunter2");

        let outside = SecretProvider::File {
            path: "/etc/passwd".to_string(),
        };
        assert!(outside.resolve(&root).is_none());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn security_limits_enforced() {
        let limits = SecurityLimits::default();
        assert!(limits.check_upload_size(1024).is_ok());
        assert!(limits.check_upload_size(u64::MAX).is_err());
        assert!(limits.check_url_length(2049).is_err());

        let deep = (0..40).fold(serde_json::json!("leaf"), |acc, _| serde_json::json!([acc]));
        assert!(limits.check_json_depth(&deep, 0).is_err());

        assert!(limits.check_json_size(1024).is_ok());
        assert!(limits.check_json_size(usize::MAX).is_err());
        assert!(limits.check_proto_message_size(1024).is_ok());
        assert!(limits.check_proto_message_size(u64::MAX).is_err());
        assert!(limits.check_log_line_length(1024).is_ok());
        assert!(limits.check_log_line_length(usize::MAX).is_err());
    }
}
