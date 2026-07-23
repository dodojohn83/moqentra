//! Dependency health checks and composite readiness reporting.
//!
//! Health endpoints must not expose secrets, DSNs, stack traces or cross-tenant
//! statistics.  Each check reports only its name, whether it passed and an opaque
//! reason string.

use async_trait::async_trait;
use serde::Serialize;
use std::sync::Arc;

/// A single dependency that can be probed for readiness.
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// Unique check name exposed in health output.
    fn name(&self) -> &str;

    /// When `true`, a failure makes the overall readiness fail-closed.
    fn required(&self) -> bool;

    /// Probe the dependency.  `Ok(())` means healthy; `Err(reason)` returns a
    /// safe, non-secret reason string.
    async fn check(&self) -> Result<(), String>;
}

/// Summary returned by the composite health endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    pub status: &'static str,
    pub ready: bool,
    pub checks: Vec<HealthCheckReport>,
}

/// Per-check report.
#[derive(Debug, Clone, Serialize)]
pub struct HealthCheckReport {
    pub name: String,
    pub required: bool,
    pub healthy: bool,
    pub reason: Option<String>,
}

/// Composite readiness probe.
#[derive(Clone, Default)]
pub struct CompositeHealthCheck {
    checks: Vec<Arc<dyn HealthCheck>>,
}

impl CompositeHealthCheck {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn add(&mut self, check: Arc<dyn HealthCheck>) {
        self.checks.push(check);
    }

    /// Run all checks and produce an aggregated report.
    pub async fn ready(&self) -> HealthReport {
        let mut checks = Vec::new();
        for check in &self.checks {
            let result = check.check().await;
            checks.push(HealthCheckReport {
                name: check.name().to_string(),
                required: check.required(),
                healthy: result.is_ok(),
                reason: result.err(),
            });
        }
        let ready = checks.iter().all(|c| !c.required || c.healthy);

        HealthReport {
            status: if ready { "ready" } else { "not_ready" },
            ready,
            checks,
        }
    }
}

/// Simple always-healthy liveness check.
#[derive(Clone, Default)]
pub struct LivenessCheck;

#[async_trait]
impl HealthCheck for LivenessCheck {
    fn name(&self) -> &str {
        "liveness"
    }

    fn required(&self) -> bool {
        false
    }

    async fn check(&self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailingCheck;

    #[async_trait]
    impl HealthCheck for FailingCheck {
        fn name(&self) -> &str {
            "failing"
        }
        fn required(&self) -> bool {
            true
        }
        async fn check(&self) -> Result<(), String> {
            Err("dependency unreachable".into())
        }
    }

    #[tokio::test]
    async fn required_failure_makes_not_ready() {
        let mut composite = CompositeHealthCheck::new();
        composite.add(Arc::new(FailingCheck));
        let report = composite.ready().await;
        assert!(!report.ready);
        assert_eq!(report.checks[0].name, "failing");
        assert_eq!(
            report.checks[0].reason.as_deref(),
            Some("dependency unreachable")
        );
    }
}
