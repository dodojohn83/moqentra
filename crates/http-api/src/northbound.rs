//! Northbound API primitives: problem details, idempotency, cursors and webhooks.

use hmac::{Hmac, Mac};
use moqentra_types::{Error, RequestContext, UtcTimestamp};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

/// RFC 9457 Problem Details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProblemDetails {
    #[serde(rename = "type")]
    pub problem_type: String,
    pub title: String,
    pub status: u16,
    pub code: String,
    pub detail: String,
    pub request_id: String,
    pub timestamp: UtcTimestamp,
}

impl ProblemDetails {
    pub fn from_error(err: &Error, request_id: impl Into<String>) -> Self {
        Self {
            problem_type: "about:blank".to_string(),
            title: err.to_string(),
            status: err.kind.http_status().as_u16(),
            code: err.kind.as_str().to_string(),
            detail: err.safe_message().to_string(),
            request_id: request_id.into(),
            timestamp: UtcTimestamp::now(),
        }
    }
}

/// Idempotency key with stored response digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyRecord {
    pub key: String,
    pub fingerprint: String,
    pub response_digest: String,
    pub completed_at: UtcTimestamp,
}

impl IdempotencyRecord {
    pub fn new(
        key: impl Into<String>,
        fingerprint: impl Into<String>,
        response_digest: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            fingerprint: fingerprint.into(),
            response_digest: response_digest.into(),
            completed_at: UtcTimestamp::now(),
        }
    }

    pub fn matches(&self, key: &str, fingerprint: &str) -> bool {
        self.key == key && self.fingerprint == fingerprint
    }
}

/// Opaque cursor for list pagination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub value: String,
    pub revision: u64,
}

/// Long-running operation reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationRef {
    pub operation_id: String,
    pub status_url: String,
    pub retry_after_seconds: u32,
}

/// Webhook subscription and delivery metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookSubscription {
    pub id: String,
    pub tenant_id: moqentra_types::TenantId,
    pub url: String,
    pub event_types: Vec<String>,
    pub secret_hmac: String,
    pub active: bool,
    pub max_retries: u32,
    pub circuit_open: bool,
}

type HmacSha256 = Hmac<Sha256>;

impl WebhookSubscription {
    pub fn sign_payload(
        &self,
        body: &[u8],
        delivery_id: &str,
        timestamp: i64,
    ) -> Result<String, Error> {
        let mut mac = HmacSha256::new_from_slice(self.secret_hmac.as_bytes())
            .map_err(|_| Error::invalid_argument("invalid hmac key length"))?;
        mac.update(delivery_id.as_bytes());
        mac.update(b":");
        mac.update(timestamp.to_string().as_bytes());
        mac.update(b":");
        mac.update(body);
        let signature = mac.finalize().into_bytes();
        Ok(format!("sha256={}", hex::encode(signature)))
    }

    pub fn validate_url(&self) -> Result<(), Error> {
        let parsed =
            url::Url::parse(&self.url).map_err(|_| Error::invalid_argument("invalid url"))?;
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return Err(Error::invalid_argument("url scheme must be http or https"));
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(Error::invalid_argument("url must not contain credentials"));
        }

        match parsed.host() {
            Some(url::Host::Domain(domain)) => {
                let lower = domain.to_lowercase();
                // Ignore a trailing dot so `metadata.google.internal.` is also blocked.
                let lower = lower.trim_end_matches('.');
                if lower.is_empty()
                    || lower == "localhost"
                    || lower.ends_with(".localhost")
                    || lower == "metadata.google.internal"
                {
                    return Err(Error::invalid_argument(
                        "SSRF: internal address not allowed",
                    ));
                }
                // Reject domain-looking IPs such as 127.1 or 0x7f.0.0.1 that the
                // url crate does not parse as an address literal.
                if lower.parse::<IpAddr>().is_ok()
                    || Ipv4Addr::from_str(lower).is_ok()
                    || parse_ipv4_like(lower).is_some_and(is_internal_ipv4)
                {
                    return Err(Error::invalid_argument(
                        "SSRF: internal address not allowed",
                    ));
                }
            }
            Some(url::Host::Ipv4(ip)) => {
                if is_internal_ipv4(ip) {
                    return Err(Error::invalid_argument(
                        "SSRF: internal address not allowed",
                    ));
                }
            }
            Some(url::Host::Ipv6(ip)) => {
                if is_internal_ipv6(ip) {
                    return Err(Error::invalid_argument(
                        "SSRF: internal address not allowed",
                    ));
                }
            }
            None => return Err(Error::invalid_argument("url missing host")),
        }

        Ok(())
    }
}

fn is_internal_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.is_broadcast()
        || ip.is_unspecified()
        || is_cgnat(ip) // 100.64.0.0/10
        || is_benchmarking(ip) // 198.18.0.0/15
}

fn is_cgnat(ip: Ipv4Addr) -> bool {
    ip.octets()[0] == 100 && (ip.octets()[1] & 0b1100_0000 == 64)
}

fn is_benchmarking(ip: Ipv4Addr) -> bool {
    ip.octets()[0] == 198 && (ip.octets()[1] == 18 || ip.octets()[1] == 19)
}

fn is_internal_ipv6(ip: Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() || ip.is_unique_local() {
        return true;
    }
    // Catch IPv4-mapped loopback/private addresses such as ::ffff:127.0.0.1.
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_internal_ipv4(mapped);
    }
    false
}

fn parse_ipv4_like(domain: &str) -> Option<Ipv4Addr> {
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.is_empty() || parts.len() > 4 {
        return None;
    }
    if parts.iter().any(|p| p.is_empty()) {
        return None;
    }

    let mut values = [0u32; 4];
    let count = parts.len();
    for (i, part) in parts.iter().enumerate() {
        let is_last = i + 1 == count;
        let max = match (count, i == 0, is_last) {
            (1, _, _) => u32::MAX,
            (2, false, true) | (3, false, true) => {
                // The final part of a 2- or 3-part address fills the
                // remaining 24 or 16 bits respectively.
                if count == 2 {
                    0xffffff
                } else {
                    0xffff
                }
            }
            _ => 0xff,
        };
        let value = parse_numeric_literal(part, max)?;
        values[i] = value;
    }

    let addr = match count {
        1 => Ipv4Addr::from(values[0]),
        2 => Ipv4Addr::from((values[0] << 24) | (values[1] & 0xffffff)),
        3 => Ipv4Addr::from((values[0] << 24) | (values[1] << 16) | (values[2] & 0xffff)),
        4 => Ipv4Addr::from((values[0] << 24) | (values[1] << 16) | (values[2] << 8) | values[3]),
        _ => return None,
    };
    Some(addr)
}

fn parse_numeric_literal(part: &str, max: u32) -> Option<u32> {
    let lower = part.to_lowercase();
    let (base, num) = if let Some(stripped) = lower.strip_prefix("0x") {
        (16u32, stripped)
    } else if lower.starts_with('0') && lower.len() > 1 {
        // Treat ambiguous leading-zero forms as octal to avoid bypasses.
        (8u32, &lower[1..])
    } else {
        (10u32, &lower[..])
    };
    let value = u32::from_str_radix(num, base).ok()?;
    if value > max {
        return None;
    }
    Some(value)
}

/// SSE event envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SseEvent {
    pub cursor: String,
    pub event_type: String,
    pub tenant_id: String,
    pub payload: serde_json::Value,
}

/// Request authorization summary for middleware.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedRequest {
    pub context: RequestContext,
    pub idempotency_key: Option<String>,
    pub if_match: Option<u64>,
}

/// Minimal rate-limit window (placeholder for a token-bucket implementation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitWindow {
    pub tenant_id: moqentra_types::TenantId,
    pub remaining: u32,
    pub reset_at: UtcTimestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{RandomIdGenerator, TenantId};

    #[test]
    fn problem_details_hides_stack() {
        let err = Error::invalid_argument("visible message")
            .with_source(std::io::Error::other("internal stack trace"));
        let pd = ProblemDetails::from_error(&err, "req-1");
        assert_eq!(pd.status, 400);
        assert!(pd.detail.contains("visible message"));
        assert!(!pd.detail.contains("internal stack trace"));
    }

    #[test]
    fn idempotency_record_matches_fingerprint() {
        let rec = IdempotencyRecord::new("key-1", "fp-1", "resp-abc");
        assert!(rec.matches("key-1", "fp-1"));
        assert!(!rec.matches("key-1", "fp-2"));
    }

    #[test]
    fn webhook_rejects_internal_addresses() {
        let sub = WebhookSubscription {
            id: "w1".to_string(),
            tenant_id: TenantId::new_v7(&RandomIdGenerator),
            url: "http://localhost:8080/hook".to_string(),
            event_types: vec![],
            secret_hmac: "secret".to_string(),
            active: true,
            max_retries: 3,
            circuit_open: false,
        };
        assert!(sub.validate_url().is_err());
    }

    #[test]
    fn webhook_accepts_external_addresses() {
        let sub = WebhookSubscription {
            id: "w2".to_string(),
            tenant_id: TenantId::new_v7(&RandomIdGenerator),
            url: "https://example.com/hook".to_string(),
            event_types: vec![],
            secret_hmac: "secret".to_string(),
            active: true,
            max_retries: 3,
            circuit_open: false,
        };
        assert!(sub.validate_url().is_ok());
    }

    #[test]
    fn webhook_rejects_internal_and_encoded_addresses() {
        let make = |url: &str| WebhookSubscription {
            id: "w".to_string(),
            tenant_id: TenantId::new_v7(&RandomIdGenerator),
            url: url.to_string(),
            event_types: vec![],
            secret_hmac: "secret".to_string(),
            active: true,
            max_retries: 3,
            circuit_open: false,
        };
        assert!(make("http://127.0.0.2/hook").validate_url().is_err());
        assert!(make("http://169.254.169.254/latest/").validate_url().is_err());
        assert!(make("http://[::1]/hook").validate_url().is_err());
        assert!(make("http://[::ffff:127.0.0.1]/hook").validate_url().is_err());
        assert!(make("http://127.1/hook").validate_url().is_err());
        assert!(make("http://0x7f.0.0.1/hook").validate_url().is_err());
        assert!(make("http://0177.0.0.1/hook").validate_url().is_err());
        assert!(make("http://100.64.0.1/hook").validate_url().is_err());
        assert!(make("http://198.18.0.1/hook").validate_url().is_err());
        assert!(make("https://example.com/hook").validate_url().is_ok());
    }

    #[test]
    fn webhook_signs_payload_with_hmac() {
        let sub = WebhookSubscription {
            id: "w3".to_string(),
            tenant_id: TenantId::new_v7(&RandomIdGenerator),
            url: "https://example.com/hook".to_string(),
            event_types: vec![],
            secret_hmac: "secret".to_string(),
            active: true,
            max_retries: 3,
            circuit_open: false,
        };
        let signature = sub.sign_payload(b"body", "d1", 42).unwrap();
        assert!(signature.starts_with("sha256="));
        assert_eq!(
            signature,
            sub.sign_payload(b"body", "d1", 42).unwrap(),
            "signature is deterministic"
        );
        assert_ne!(
            signature,
            sub.sign_payload(b"body", "d2", 42).unwrap(),
            "different delivery id changes signature"
        );
    }

    #[test]
    fn sse_event_includes_cursor() {
        let ev = SseEvent {
            cursor: "c1".to_string(),
            event_type: "dataset.created".to_string(),
            tenant_id: "tenant-1".to_string(),
            payload: serde_json::json!({}),
        };
        assert_eq!(ev.cursor, "c1");
    }
}
