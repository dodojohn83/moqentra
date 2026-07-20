//! dyun-agent runtime, bundle and replica state machine.

use moqentra_types::{AssetId, DeploymentId, NodeId, ReplicaId, UtcTimestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// dyun graph bundle version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DyunGraphBundle {
    pub version: String,
    pub application_digest: String,
    pub graph_spec_digest: String,
    pub artifact_bindings: BTreeMap<String, AssetId>,
    pub runtime_profile: String,
    pub resource_limits: ResourceLimits,
    pub signature: String,
    pub compatible_dg_versions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub cpu_milli: u64,
    pub memory_mib: u64,
    pub gpu_count: u32,
}

/// Agent capability advertisement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCapabilities {
    pub node_id: NodeId,
    pub dg_versions: Vec<String>,
    pub codecs: Vec<String>,
    pub accelerators: Vec<String>,
    pub max_replicas: u32,
}

/// Replica state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicaState {
    Pending,
    Preparing,
    Starting,
    Running,
    Draining,
    Stopped,
    Failed,
}

/// A running replica managed by dyun-agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replica {
    pub id: ReplicaId,
    pub deployment_id: DeploymentId,
    pub generation: u64,
    pub fencing_token: u64,
    pub state: ReplicaState,
    pub bundle: DyunGraphBundle,
    pub node_id: NodeId,
    pub runner_pid: Option<u32>,
    pub last_heartbeat: Option<UtcTimestamp>,
    pub desired_state: ReplicaState,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl Replica {
    pub fn new(
        id: ReplicaId,
        deployment_id: DeploymentId,
        generation: u64,
        fencing_token: u64,
        bundle: DyunGraphBundle,
        node_id: NodeId,
    ) -> Self {
        let now = UtcTimestamp::now();
        Self {
            id,
            deployment_id,
            generation,
            fencing_token,
            state: ReplicaState::Pending,
            bundle,
            node_id,
            runner_pid: None,
            last_heartbeat: None,
            desired_state: ReplicaState::Running,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn verify_bundle(&self, trusted_keys: &[String]) -> Result<(), moqentra_types::Error> {
        let valid_keys: Vec<&String> = trusted_keys.iter().filter(|k| !k.is_empty()).collect();
        let trusted = valid_keys.iter().any(|k| {
            self.bundle.signature == k.as_str()
                || (k.ends_with(':')
                    && self.bundle.signature.starts_with(k.as_str())
                    && self.bundle.signature.len() > k.len())
        });
        if valid_keys.is_empty() || !trusted {
            return Err(moqentra_types::Error::permission_denied(
                "bundle signature not trusted",
            ));
        }
        if self.bundle.application_digest.is_empty() || self.bundle.graph_spec_digest.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "bundle missing digests",
            ));
        }
        Ok(())
    }

    pub fn transition(
        &mut self,
        generation: u64,
        fencing_token: u64,
        state: ReplicaState,
    ) -> Result<(), moqentra_types::Error> {
        if generation < self.generation {
            return Err(moqentra_types::Error::conflict("stale generation"));
        }
        if generation == self.generation && fencing_token != self.fencing_token {
            return Err(moqentra_types::Error::conflict("stale fencing token"));
        }
        self.generation = generation;
        self.fencing_token = fencing_token;
        self.state = state;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn heartbeat(
        &mut self,
        generation: u64,
        fencing_token: u64,
    ) -> Result<(), moqentra_types::Error> {
        if generation != self.generation || fencing_token != self.fencing_token {
            return Err(moqentra_types::Error::conflict("stale heartbeat"));
        }
        self.last_heartbeat = Some(UtcTimestamp::now());
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn drain(&mut self) {
        self.desired_state = ReplicaState::Stopped;
        self.state = ReplicaState::Draining;
        self.updated_at = UtcTimestamp::now();
    }
}

/// Runner supervisor state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runner {
    pub id: String,
    pub replica_id: ReplicaId,
    pub state: RunnerState,
    pub image_digest: String,
    pub sandbox_path: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunnerState {
    Created,
    Starting,
    Running,
    Exited,
    Orphaned,
}

impl Runner {
    pub fn validate_sandbox(&self) -> Result<(), moqentra_types::Error> {
        if self.sandbox_path.is_empty()
            || self.sandbox_path.contains('\0')
            || !self.sandbox_path.starts_with('/')
            || self.sandbox_path.split('/').any(|c| c == "..")
        {
            return Err(moqentra_types::Error::invalid_argument(
                "invalid sandbox path",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    fn make_bundle() -> DyunGraphBundle {
        DyunGraphBundle {
            version: "DyunGraphBundle/v1".to_string(),
            application_digest: "sha256:app".to_string(),
            graph_spec_digest: "sha256:graph".to_string(),
            artifact_bindings: BTreeMap::new(),
            runtime_profile: "rtsp-track-rtmp".to_string(),
            resource_limits: ResourceLimits {
                cpu_milli: 2000,
                memory_mib: 4096,
                gpu_count: 1,
            },
            signature: "trusted:abc".to_string(),
            compatible_dg_versions: vec!["dg-1.0".to_string()],
        }
    }

    #[test]
    fn verify_bundle_rejects_untrusted() {
        let gen = RandomIdGenerator;
        let bundle = make_bundle();
        let replica = Replica::new(
            ReplicaId::new_v7(&gen),
            DeploymentId::new_v7(&gen),
            1,
            7,
            bundle,
            NodeId::new_v7(&gen),
        );
        assert!(replica.verify_bundle(&["other".to_string()]).is_err());
        assert!(replica.verify_bundle(&["trusted:".to_string()]).is_ok());
    }

    #[test]
    fn stale_generation_rejected() {
        let gen = RandomIdGenerator;
        let mut replica = Replica::new(
            ReplicaId::new_v7(&gen),
            DeploymentId::new_v7(&gen),
            1,
            7,
            make_bundle(),
            NodeId::new_v7(&gen),
        );
        replica.transition(1, 7, ReplicaState::Running).unwrap();
        assert!(replica.transition(0, 7, ReplicaState::Running).is_err());
        assert!(replica.transition(1, 99, ReplicaState::Running).is_err());
    }

    #[test]
    fn sandbox_path_rejected_if_not_absolute() {
        let runner = Runner {
            id: "r1".to_string(),
            replica_id: ReplicaId::new_v7(&RandomIdGenerator),
            state: RunnerState::Created,
            image_digest: "sha256:img".to_string(),
            sandbox_path: "../etc".to_string(),
            exit_code: None,
        };
        assert!(runner.validate_sandbox().is_err());
    }
}
