//! dyun-agent runtime, bundle and replica state machine.

use moqentra_types::{DeploymentId, NodeId, ReplicaId, UtcTimestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use moqentra_domain::application::ArtifactBinding;

/// dyun graph bundle version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DyunGraphBundle {
    pub version: String,
    pub application_digest: String,
    pub graph_spec: moqentra_domain::application::GraphSpec,
    pub graph_spec_digest: String,
    pub artifact_bindings: BTreeMap<String, ArtifactBinding>,
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
    pub agent_version: String,
    pub dg_versions: Vec<String>,
    pub codecs: Vec<String>,
    pub accelerators: Vec<String>,
    pub element_schemas: Vec<String>,
    pub backends: Vec<String>,
    pub build_features: Vec<String>,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replica {
    pub id: ReplicaId,
    pub deployment_id: DeploymentId,
    pub generation: u64,
    pub fencing_token: u64,
    pub state: ReplicaState,
    pub bundle: DyunGraphBundle,
    pub node_id: NodeId,
    pub runner_pid: Option<u32>,
    pub runner_id: Option<String>,
    pub last_heartbeat: Option<UtcTimestamp>,
    pub desired_state: ReplicaState,
    /// Last generation reported by the runner (may lag behind desired).
    pub observed_generation: u64,
    /// Recent error messages (bounded in practice by agent truncation).
    pub errors: Vec<String>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
    pub drain_deadline: Option<UtcTimestamp>,
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
            runner_id: None,
            last_heartbeat: None,
            desired_state: ReplicaState::Running,
            observed_generation: generation,
            errors: Vec::new(),
            created_at: now,
            updated_at: now,
            drain_deadline: None,
        }
    }

    pub fn verify_bundle(
        &self,
        production_keys: &[String],
        dev_keys: &[String],
    ) -> Result<(), moqentra_types::Error> {
        let prod: Vec<&String> = production_keys.iter().filter(|k| !k.is_empty()).collect();
        let dev: Vec<&String> = dev_keys.iter().filter(|k| !k.is_empty()).collect();

        // Production signatures must match a trusted key exactly.
        let prod_trusted = prod.iter().any(|k| self.bundle.signature == k.as_str());
        // Development signatures may use a namespace prefix (e.g. "trusted:") but
        // only when a development key explicitly allows that namespace.
        let dev_trusted = dev.iter().any(|k| {
            k.ends_with(':')
                && self.bundle.signature.starts_with(k.as_str())
                && self.bundle.signature.len() > k.len()
        });

        if prod.is_empty() && dev.is_empty() {
            return Err(moqentra_types::Error::permission_denied(
                "no trusted keys configured",
            ));
        }
        if !prod_trusted && !dev_trusted {
            return Err(moqentra_types::Error::permission_denied(
                "bundle signature not trusted",
            ));
        }
        if !moqentra_types::valid_content_digest(&self.bundle.application_digest)
            || !moqentra_types::valid_content_digest(&self.bundle.graph_spec_digest)
        {
            return Err(moqentra_types::Error::invalid_argument(
                "bundle digests must be valid content digests",
            ));
        }
        let computed = self.bundle.graph_spec.canonical_digest()?;
        if computed != self.bundle.graph_spec_digest {
            return Err(moqentra_types::Error::invalid_argument(
                "graph spec digest mismatch",
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

    pub fn set_runner(&mut self, runner_id: impl Into<String>, pid: u32) {
        self.runner_id = Some(runner_id.into());
        self.runner_pid = Some(pid);
        self.updated_at = UtcTimestamp::now();
    }

    pub fn record_error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
        if self.errors.len() > 64 {
            self.errors.remove(0);
        }
        self.updated_at = UtcTimestamp::now();
    }

    pub fn observe_generation(&mut self, observed: u64) {
        if observed > self.observed_generation {
            self.observed_generation = observed;
            self.updated_at = UtcTimestamp::now();
        }
    }

    pub fn heartbeat(
        &mut self,
        generation: u64,
        fencing_token: u64,
        observed_generation: u64,
    ) -> Result<(), moqentra_types::Error> {
        if generation != self.generation || fencing_token != self.fencing_token {
            return Err(moqentra_types::Error::conflict("stale heartbeat"));
        }
        self.last_heartbeat = Some(UtcTimestamp::now());
        self.observe_generation(observed_generation);
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn drain(&mut self, timeout: std::time::Duration) {
        self.desired_state = ReplicaState::Stopped;
        self.state = ReplicaState::Draining;
        self.drain_deadline = UtcTimestamp::now().add_std_duration(timeout);
        self.updated_at = UtcTimestamp::now();
    }

    pub fn reconcile(&mut self, now: UtcTimestamp, heartbeat_timeout: std::time::Duration) {
        if self.state == ReplicaState::Draining {
            if let Some(deadline) = self.drain_deadline {
                if now >= deadline {
                    self.state = ReplicaState::Stopped;
                    self.updated_at = now;
                }
            }
        }
        if matches!(self.state, ReplicaState::Running | ReplicaState::Starting) {
            if let Some(last) = self.last_heartbeat {
                if now.unix_millis().saturating_sub(last.unix_millis())
                    > heartbeat_timeout.as_millis() as i64
                {
                    self.state = ReplicaState::Failed;
                    self.errors.push("heartbeat timed out".to_string());
                    self.updated_at = now;
                }
            }
        }
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
    pub fn validate_sandbox(&self, root: impl AsRef<Path>) -> Result<(), moqentra_types::Error> {
        if self.sandbox_path.is_empty() || self.sandbox_path.contains('\0') {
            return Err(moqentra_types::Error::invalid_argument(
                "invalid sandbox path",
            ));
        }
        if !is_within_root(&self.sandbox_path, root)? {
            return Err(moqentra_types::Error::permission_denied(
                "sandbox path outside workspace",
            ));
        }
        Ok(())
    }
}

/// Verifies `path` is confined to `root`, resolving symlinks on every
/// existing ancestor so that a directory symlink cannot escape the workspace.
fn is_within_root(
    path: impl AsRef<Path>,
    root: impl AsRef<Path>,
) -> Result<bool, moqentra_types::Error> {
    let root = root.as_ref();
    std::fs::create_dir_all(root).map_err(|e| {
        moqentra_types::Error::invalid_argument(format!("sandbox root not creatable: {e}"))
    })?;
    let root_canonical = std::fs::canonicalize(root)
        .map_err(|e| moqentra_types::Error::internal(format!("canonicalize root: {e}")))?;
    let abs = std::path::absolute(path.as_ref())
        .map_err(|e| moqentra_types::Error::invalid_argument(format!("invalid path: {e}")))?;

    for ancestor in abs.ancestors() {
        if ancestor.exists() {
            let canonical = std::fs::canonicalize(ancestor)
                .map_err(|e| moqentra_types::Error::internal(format!("canonicalize path: {e}")))?;
            if !canonical.starts_with(&root_canonical) {
                return Ok(false);
            }
            let suffix = abs
                .strip_prefix(ancestor)
                .map_err(|_| moqentra_types::Error::internal("sandbox path prefix mismatch"))?;
            for comp in suffix.components() {
                if comp.as_os_str() == ".." || comp.as_os_str().is_empty() {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
    }
    Ok(false)
}

/// Returns the default sandbox workspace root.
pub fn default_sandbox_root() -> PathBuf {
    PathBuf::from("/tmp/moqentra-dyun")
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    fn make_bundle() -> DyunGraphBundle {
        let graph_spec = moqentra_domain::application::GraphSpec {
            version: "dg/v1".to_string(),
            elements: BTreeMap::new(),
            connections: Vec::new(),
        };
        let graph_spec_digest = graph_spec.canonical_digest().unwrap();
        DyunGraphBundle {
            version: "DyunGraphBundle/v1".to_string(),
            application_digest:
                "sha256:a172cedcae47474b615c54d510a5d84a8dea3032e958587430b413538be3f333"
                    .to_string(),
            graph_spec,
            graph_spec_digest,
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
        assert!(replica.verify_bundle(&["other".to_string()], &[]).is_err());
        assert!(replica.verify_bundle(&[], &["trusted:".to_string()]).is_ok());
        // Production keys require an exact signature match.
        assert!(replica.verify_bundle(&["trusted:abc".to_string()], &[]).is_ok());
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

        // Heartbeat advances observed generation but never moves backwards.
        assert!(replica.heartbeat(1, 7, 1).is_ok());
        assert_eq!(replica.observed_generation, 1);
        replica.observe_generation(3);
        assert_eq!(replica.observed_generation, 3);
        replica.observe_generation(2);
        assert_eq!(replica.observed_generation, 3);
    }

    #[test]
    fn sandbox_path_rejected_if_not_absolute() {
        let runner = Runner {
            id: "r1".to_string(),
            replica_id: ReplicaId::new_v7(&RandomIdGenerator),
            state: RunnerState::Created,
            image_digest: "sha256:b29814cf5792e684cd75d6a7fce7a67a11887e312f87ca2ac2496d81f365ff72"
                .to_string(),
            sandbox_path: "../etc".to_string(),
            exit_code: None,
        };
        assert!(runner.validate_sandbox(default_sandbox_root()).is_err());
    }

    #[test]
    fn sandbox_path_confined_to_workspace() {
        let runner = Runner {
            id: "r2".to_string(),
            replica_id: ReplicaId::new_v7(&RandomIdGenerator),
            state: RunnerState::Created,
            image_digest: "sha256:b29814cf5792e684cd75d6a7fce7a67a11887e312f87ca2ac2496d81f365ff72"
                .to_string(),
            sandbox_path: "/etc".to_string(),
            exit_code: None,
        };
        assert!(runner.validate_sandbox(default_sandbox_root()).is_err());

        let valid = Runner {
            id: "r3".to_string(),
            replica_id: ReplicaId::new_v7(&RandomIdGenerator),
            state: RunnerState::Created,
            image_digest: "sha256:b29814cf5792e684cd75d6a7fce7a67a11887e312f87ca2ac2496d81f365ff72"
                .to_string(),
            sandbox_path: "/tmp/moqentra-dyun/run-1".to_string(),
            exit_code: None,
        };
        assert!(valid.validate_sandbox(default_sandbox_root()).is_ok());
    }
}
