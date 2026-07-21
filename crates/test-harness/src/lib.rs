//! Moqentra test harness: fake workers, agents, S3 fault proxy and OIDC issuer.

#![allow(missing_docs)]

use moqentra_types::{RequestContext, UtcTimestamp};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// Simulates a worker runtime without external dependencies.
#[derive(Debug, Clone, Default)]
pub struct FakeWorkerRuntime {
    pub attempts_started: Vec<String>,
    pub checkpoints: Vec<String>,
    pub metrics_reported: Vec<(String, f64)>,
    pub cancelled: bool,
}

impl FakeWorkerRuntime {
    pub fn start(&mut self, attempt_id: impl Into<String>) {
        self.attempts_started.push(attempt_id.into());
    }

    pub fn report_metric(&mut self, name: impl Into<String>, value: f64) {
        self.metrics_reported.push((name.into(), value));
    }

    pub fn save_checkpoint(&mut self, path: impl Into<String>) {
        self.checkpoints.push(path.into());
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
}

/// Simulates a dyun-agent capable of replaying events out of order.
#[derive(Debug, Clone, Default)]
pub struct FakeDyunAgent {
    pub events: VecDeque<(u64, String)>,
    pub signed: bool,
    pub capabilities: Vec<String>,
}

impl FakeDyunAgent {
    pub fn emit_event(&mut self, sequence: u64, payload: impl Into<String>) {
        self.events.push_back((sequence, payload.into()));
    }

    pub fn drain_ordered(&mut self) -> Vec<(u64, String)> {
        let mut v: Vec<_> = self.events.drain(..).collect();
        v.sort_by_key(|(seq, _)| *seq);
        v
    }
}

/// In-memory S3-compatible fault proxy.
#[derive(Debug, Clone, Default)]
pub struct FakeS3Proxy {
    objects: BTreeMap<String, Vec<u8>>,
    pub fault_rate: f64,
    pub requests: u64,
}

impl FakeS3Proxy {
    pub fn put_object(&mut self, key: impl Into<String>, value: Vec<u8>) {
        self.objects.insert(key.into(), value);
    }

    pub fn get_object(&mut self, key: &str) -> Result<Vec<u8>, moqentra_types::Error> {
        self.requests += 1;
        if self.should_fault() {
            return Err(moqentra_types::Error::unavailable("injected s3 fault"));
        }
        self.objects
            .get(key)
            .cloned()
            .ok_or_else(|| moqentra_types::Error::not_found("object"))
    }

    fn should_fault(&mut self) -> bool {
        if self.fault_rate <= 0.0 || self.fault_rate.is_nan() {
            return false;
        }
        if self.fault_rate >= 1.0 {
            return true;
        }
        #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
        let period = ((1.0 / self.fault_rate) as u64).max(1);
        self.requests.is_multiple_of(period)
    }
}

/// Simulates Kubernetes API events.
#[derive(Debug, Clone, Default)]
pub struct FakeKubernetesApi {
    pub pods: BTreeMap<String, PodStatus>,
    pub events: Vec<String>,
    pub resource_version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PodStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

impl FakeKubernetesApi {
    pub fn apply_pod(&mut self, name: impl Into<String>, status: PodStatus) {
        self.pods.insert(name.into(), status);
        self.resource_version += 1;
    }

    pub fn delete_pod(&mut self, name: &str) {
        self.pods.remove(name);
        self.resource_version += 1;
    }

    pub fn emit_event(&mut self, message: impl Into<String>) {
        self.events.push(message.into());
    }
}

/// Fake OIDC issuer for tests.
#[derive(Debug, Clone, Default)]
pub struct FakeOidcIssuer {
    pub issuer_url: String,
    pub jwks: BTreeMap<String, String>,
    pub tokens: BTreeMap<String, RequestContext>,
}

impl FakeOidcIssuer {
    pub fn new(issuer_url: impl Into<String>) -> Self {
        Self {
            issuer_url: issuer_url.into(),
            jwks: BTreeMap::new(),
            tokens: BTreeMap::new(),
        }
    }

    pub fn issue_token(&mut self, token: impl Into<String>, ctx: RequestContext) {
        self.tokens.insert(token.into(), ctx);
    }

    pub fn validate(&self, token: &str) -> Option<&RequestContext> {
        self.tokens.get(token)
    }
}

/// Deterministic test clock.
#[derive(Debug, Clone, Copy)]
pub struct TestClock {
    pub now: UtcTimestamp,
}

impl TestClock {
    pub fn new(now: UtcTimestamp) -> Self {
        Self { now }
    }

    pub fn advance(&mut self, seconds: u64) {
        if let Some(next) = self.now.add_std_duration(std::time::Duration::from_secs(seconds)) {
            self.now = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{Principal, RandomIdGenerator, TenantId};

    #[test]
    fn fake_worker_lifecycle() {
        let mut w = FakeWorkerRuntime::default();
        w.start("a1");
        w.report_metric("loss", 0.5);
        w.save_checkpoint("/tmp/c1");
        assert_eq!(w.attempts_started.len(), 1);
        assert_eq!(w.metrics_reported.len(), 1);
        assert_eq!(w.checkpoints.len(), 1);
    }

    #[test]
    fn fake_dyun_agent_orders_events() {
        let mut a = FakeDyunAgent::default();
        a.emit_event(3, "c");
        a.emit_event(1, "a");
        a.emit_event(2, "b");
        let ordered = a.drain_ordered();
        assert_eq!(ordered[0].0, 1);
        assert_eq!(ordered[2].0, 3);
    }

    #[test]
    fn fake_s3_proxy_injects_faults() {
        let mut s3 = FakeS3Proxy {
            fault_rate: 0.5,
            ..Default::default()
        };
        s3.put_object("k1", vec![1, 2, 3]);
        let mut ok = 0;
        let mut err = 0;
        for _ in 0..4 {
            match s3.get_object("k1") {
                Ok(_) => ok += 1,
                Err(_) => err += 1,
            }
        }
        assert_eq!(ok + err, 4);
        assert!(err > 0);
    }

    #[test]
    fn fake_kubernetes_api_tracks_pods() {
        let mut k = FakeKubernetesApi::default();
        k.apply_pod("pod-1", PodStatus::Pending);
        k.apply_pod("pod-1", PodStatus::Running);
        assert_eq!(k.pods.len(), 1);
        assert_eq!(k.resource_version, 2);
    }

    #[test]
    fn fake_oidc_issuer_validates_token() {
        let gen = RandomIdGenerator;
        let mut issuer = FakeOidcIssuer::new("http://test");
        let ctx = RequestContext::new(TenantId::new_v7(&gen), Principal::service("sa"), "req-1");
        issuer.issue_token("abc", ctx);
        assert!(issuer.validate("abc").is_some());
    }

    #[test]
    fn test_clock_advances() {
        let now = UtcTimestamp::now();
        let mut clock = TestClock::new(now);
        clock.advance(10);
        assert!(clock.now > now);
    }
}
