//! Inference platform: deployment, rollout, cluster and endpoint domain.

use moqentra_types::{
    ApplicationVersionId, ClusterId, DeploymentId, EndpointId, ModelVersionId, ProjectId, TenantId,
    UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A release bundle linking training and inference artifacts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseBundle {
    pub version: String,
    pub model_version_id: ModelVersionId,
    pub application_version_id: ApplicationVersionId,
    pub dyun_bundle_digest: String,
    pub policy: RolloutPolicy,
    pub signature: String,
}

/// Rollout strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RolloutStrategy {
    RollingUpdate,
    BlueGreen,
    Canary { percent: u32 },
    Paused,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RolloutPolicy {
    pub strategy: RolloutStrategy,
    pub auto_rollback: bool,
    pub slo_error_rate: f64,
    pub min_available_percent: u32,
    pub max_surge_percent: u32,
}

impl RolloutPolicy {
    pub fn validate(&self) -> Result<(), moqentra_types::Error> {
        if let RolloutStrategy::Canary { percent } = self.strategy {
            if percent > 100 {
                return Err(moqentra_types::Error::invalid_argument(
                    "canary percent cannot exceed 100",
                ));
            }
        }
        if !(0.0..=1.0).contains(&self.slo_error_rate) {
            return Err(moqentra_types::Error::invalid_argument(
                "slo error rate must be between 0.0 and 1.0",
            ));
        }
        if self.min_available_percent > 100 {
            return Err(moqentra_types::Error::invalid_argument(
                "min available percent cannot exceed 100",
            ));
        }
        if self.max_surge_percent > 100 {
            return Err(moqentra_types::Error::invalid_argument(
                "max surge percent cannot exceed 100",
            ));
        }
        Ok(())
    }
}

/// Inference cluster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cluster {
    pub id: ClusterId,
    pub tenant_id: TenantId,
    pub zone: String,
    pub region: String,
    pub capabilities: Vec<String>,
    pub nodes: BTreeSet<String>,
    pub healthy: bool,
    pub cached_models: BTreeSet<ModelVersionId>,
}

/// Endpoint serving traffic.
#[derive(Debug, Clone, PartialEq)]
pub struct Endpoint {
    pub id: EndpointId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub name: String,
    pub deployment_id: DeploymentId,
    pub traffic_split: BTreeMap<String, u32>,
    pub slo_latency_ms: u64,
    pub slo_error_rate: f64,
}

/// Deployment desired state.
#[derive(Debug, Clone, PartialEq)]
pub struct Deployment {
    pub id: DeploymentId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub release_bundle: ReleaseBundle,
    pub desired_replicas: u32,
    pub observed_generation: u64,
    pub desired_state: DeploymentState,
    pub replicas: BTreeMap<String, ReplicaObservedState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentState {
    Pending,
    Rolling,
    Stable,
    Paused,
    RollingBack,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplicaObservedState {
    pub generation: u64,
    pub state: String,
    pub ready: bool,
    pub metrics: BTreeMap<String, f64>,
    pub last_heartbeat: Option<UtcTimestamp>,
}

impl Deployment {
    pub fn apply_release(
        &mut self,
        bundle: ReleaseBundle,
        generation: u64,
    ) -> Result<(), moqentra_types::Error> {
        bundle.policy.validate()?;
        self.release_bundle = bundle;
        self.observed_generation = generation;
        self.desired_state = DeploymentState::Rolling;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(
            self.desired_state,
            DeploymentState::Rolling | DeploymentState::Stable
        ) {
            return Err(moqentra_types::Error::conflict("cannot pause"));
        }
        self.desired_state = DeploymentState::Paused;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.desired_state, DeploymentState::Paused) {
            return Err(moqentra_types::Error::conflict("not paused"));
        }
        self.desired_state = DeploymentState::Rolling;
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<(), moqentra_types::Error> {
        if !self.release_bundle.policy.auto_rollback {
            return Err(moqentra_types::Error::permission_denied(
                "auto rollback disabled",
            ));
        }
        self.desired_state = DeploymentState::RollingBack;
        Ok(())
    }

    pub fn observe_replica(
        &mut self,
        replica_id: impl Into<String>,
        generation: u64,
        state: impl Into<String>,
        ready: bool,
        metrics: BTreeMap<String, f64>,
    ) -> Result<(), moqentra_types::Error> {
        if generation < self.observed_generation {
            // Old generation reports are ignored (idempotent reconciliation).
            return Ok(());
        }
        self.replicas.insert(
            replica_id.into(),
            ReplicaObservedState {
                generation,
                state: state.into(),
                ready,
                metrics,
                last_heartbeat: Some(UtcTimestamp::now()),
            },
        );
        Ok(())
    }

    pub fn available_ratio(&self) -> f64 {
        if self.replicas.is_empty() {
            return 0.0;
        }
        let ready = self
            .replicas
            .values()
            .filter(|r| r.ready && r.generation >= self.observed_generation)
            .count();
        ready as f64 / self.replicas.len() as f64
    }

    pub fn should_rollback(&self) -> bool {
        if !self.release_bundle.policy.auto_rollback {
            return false;
        }
        let error_rate = self
            .replicas
            .values()
            .filter(|r| r.generation >= self.observed_generation)
            .map(|r| r.metrics.get("error_rate").copied().unwrap_or(0.0))
            .fold(0.0, f64::max);
        error_rate > self.release_bundle.policy.slo_error_rate
    }
}

/// Multi-cluster placement policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementPolicy {
    pub regions: Vec<String>,
    pub data_locality: bool,
    pub fault_domains: BTreeSet<String>,
    pub required_capabilities: Vec<String>,
    pub affinity_labels: BTreeMap<String, String>,
}

impl PlacementPolicy {
    pub fn score(&self, cluster: &Cluster, model: ModelVersionId) -> i64 {
        let mut score = 0i64;
        if self.regions.contains(&cluster.region) {
            score = score.saturating_add(10);
        }
        if self.data_locality && cluster.cached_models.contains(&model) {
            score = score.saturating_add(20);
        }
        for cap in &self.required_capabilities {
            if cluster.capabilities.contains(cap) {
                score = score.saturating_add(5);
            } else {
                score = score.saturating_sub(100);
            }
        }
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{
        ApplicationVersionId, ClusterId, DeploymentId, ModelVersionId, ProjectId,
        RandomIdGenerator, TenantId,
    };

    fn make_bundle() -> ReleaseBundle {
        ReleaseBundle {
            version: "v1".to_string(),
            model_version_id: ModelVersionId::new_v7(&RandomIdGenerator),
            application_version_id: ApplicationVersionId::new_v7(&RandomIdGenerator),
            dyun_bundle_digest: "sha256:dyun".to_string(),
            policy: RolloutPolicy {
                strategy: RolloutStrategy::RollingUpdate,
                auto_rollback: true,
                slo_error_rate: 0.05,
                min_available_percent: 50,
                max_surge_percent: 25,
            },
            signature: "sig".to_string(),
        }
    }

    fn make_deployment() -> Deployment {
        let gen = RandomIdGenerator;
        Deployment {
            id: DeploymentId::new_v7(&gen),
            tenant_id: TenantId::new_v7(&gen),
            project_id: ProjectId::new_v7(&gen),
            release_bundle: make_bundle(),
            desired_replicas: 2,
            observed_generation: 1,
            desired_state: DeploymentState::Pending,
            replicas: BTreeMap::new(),
        }
    }

    #[test]
    fn rollout_lifecycle() {
        let mut d = make_deployment();
        d.apply_release(make_bundle(), 2).unwrap();
        assert!(matches!(d.desired_state, DeploymentState::Rolling));
        d.pause().unwrap();
        d.resume().unwrap();
        d.rollback().unwrap();
        assert!(matches!(d.desired_state, DeploymentState::RollingBack));
    }

    #[test]
    fn rollback_disabled_fails() {
        let _gen = RandomIdGenerator;
        let mut d = make_deployment();
        d.release_bundle.policy.auto_rollback = false;
        assert!(d.rollback().is_err());
    }

    #[test]
    fn observe_old_generation_ignored() {
        let mut d = make_deployment();
        d.observed_generation = 5;
        let mut m = BTreeMap::new();
        m.insert("error_rate".to_string(), 0.99);
        d.observe_replica("r1", 3, "Running", true, m).unwrap();
        assert!(d.replicas.is_empty());
    }

    #[test]
    fn slo_error_triggers_rollback() {
        let mut d = make_deployment();
        d.observed_generation = 1;
        let mut m = BTreeMap::new();
        m.insert("error_rate".to_string(), 0.1);
        d.observe_replica("r1", 1, "Running", true, m).unwrap();
        assert!(d.should_rollback());
    }

    #[test]
    fn placement_prefers_cached_model() {
        let model = ModelVersionId::new_v7(&RandomIdGenerator);
        let mut cluster = Cluster {
            id: ClusterId::new_v7(&RandomIdGenerator),
            tenant_id: TenantId::new_v7(&RandomIdGenerator),
            zone: "z1".to_string(),
            region: "us-east".to_string(),
            capabilities: vec!["gpu".to_string()],
            nodes: BTreeSet::new(),
            healthy: true,
            cached_models: [model].iter().cloned().collect(),
        };
        let policy = PlacementPolicy {
            regions: vec!["us-east".to_string()],
            data_locality: true,
            fault_domains: BTreeSet::new(),
            required_capabilities: vec!["gpu".to_string()],
            affinity_labels: BTreeMap::new(),
        };
        assert!(policy.score(&cluster, model) > 0);
        cluster.cached_models.clear();
        assert!(
            policy.score(&cluster, model)
                < policy.score(
                    &{
                        let mut c = cluster.clone();
                        c.cached_models.insert(model);
                        c
                    },
                    model
                )
        );
    }
}
