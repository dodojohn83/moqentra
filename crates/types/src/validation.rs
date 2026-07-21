//! Common validation helpers.

/// Validates a content-addressed digest string.
///
/// Accepts `sha256:<64 lower/upper-case hex chars>`, `sha384:<96 hex chars>`,
/// or `sha512:<128 hex chars>`. Returns `false` for any other format.
pub fn valid_content_digest(d: &str) -> bool {
    let Some((algo, hex)) = d.split_once(':') else {
        return false;
    };
    if algo.is_empty() || hex.is_empty() {
        return false;
    }
    let expected_len = match algo {
        "sha256" => 64,
        "sha384" => 96,
        "sha512" => 128,
        _ => return false,
    };
    if hex.len() != expected_len {
        return false;
    }
    hex.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_sha256_hex() {
        let digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
        assert!(valid_content_digest(digest));
    }

    #[test]
    fn rejects_bad_algorithm() {
        assert!(!valid_content_digest("md5:abcd"));
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(!valid_content_digest("sha256:abcd"));
    }

    #[test]
    fn rejects_non_hex() {
        assert!(!valid_content_digest(
            "sha256:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
        ));
    }
}
