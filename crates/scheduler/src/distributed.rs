//! Distributed training launcher, rendezvous and checkpoint recovery.

use moqentra_types::{AttemptId, CheckpointId, RankId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Launcher config for PyTorch DDP/Elastic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub world_size: u32,
    pub master_addr: String,
    pub master_port: u16,
    pub backend: String,
    pub nproc_per_node: u32,
    pub rdzv_backend: String,
    pub rdzv_endpoint: String,
}

/// Rendezvous state for rank discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rendezvous {
    pub attempt_id: AttemptId,
    pub world_size: u32,
    pub members: BTreeMap<RankId, String>,
    pub finalized: bool,
}

impl Rendezvous {
    pub fn new(attempt_id: AttemptId, world_size: u32) -> Result<Self, moqentra_types::Error> {
        if world_size == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "rendezvous world_size must be > 0",
            ));
        }
        Ok(Self {
            attempt_id,
            world_size,
            members: BTreeMap::new(),
            finalized: false,
        })
    }

    pub fn join(
        &mut self,
        rank_id: RankId,
        endpoint: impl Into<String>,
    ) -> Result<u32, moqentra_types::Error> {
        if self.finalized {
            return Err(moqentra_types::Error::conflict(
                "rendezvous already finalized",
            ));
        }
        if self.members.contains_key(&rank_id) {
            return Err(moqentra_types::Error::conflict("rank already joined"));
        }
        let rank_index = u32::try_from(self.members.len())
            .map_err(|_| moqentra_types::Error::internal("rank index overflow"))?;
        self.members.insert(rank_id, endpoint.into());
        Ok(rank_index)
    }

    pub fn finalize(&mut self) -> Result<(), moqentra_types::Error> {
        if self.finalized {
            return Ok(());
        }
        if self.world_size == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "rendezvous world_size must be > 0",
            ));
        }
        if self.members.len() != self.world_size as usize {
            return Err(moqentra_types::Error::unavailable("rendezvous incomplete"));
        }
        self.finalized = true;
        Ok(())
    }
}

/// Checkpoint state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckpointState {
    Uploading,
    Complete,
    Failed,
}

/// Distributed checkpoint with compatibility metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: CheckpointId,
    pub attempt_id: AttemptId,
    pub step: u64,
    pub state: CheckpointState,
    pub digest: String,
    pub manifest_digest: String,
    pub framework_version: String,
    pub world_size: u32,
    pub optimizer_signature: String,
    pub model_signature: String,
    pub metadata: BTreeMap<String, String>,
}

/// Recovery policy and checkpoint selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryPolicy {
    pub max_elastic_restarts: u32,
    pub allowed_world_sizes: Vec<u32>,
    pub compatible_framework_versions: Vec<String>,
    pub preserve_optimizer: bool,
}

impl RecoveryPolicy {
    pub fn can_restore(&self, checkpoint: &Checkpoint) -> Result<(), moqentra_types::Error> {
        if !matches!(checkpoint.state, CheckpointState::Complete) {
            return Err(moqentra_types::Error::invalid_argument(
                "checkpoint not complete",
            ));
        }
        if !self.allowed_world_sizes.contains(&checkpoint.world_size) {
            return Err(moqentra_types::Error::invalid_argument(
                "world size incompatible",
            ));
        }
        if !self.compatible_framework_versions.contains(&checkpoint.framework_version) {
            return Err(moqentra_types::Error::invalid_argument(
                "framework version incompatible",
            ));
        }
        if self.preserve_optimizer && checkpoint.optimizer_signature.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "optimizer signature missing",
            ));
        }
        Ok(())
    }

    pub fn select_latest_compatible(&self, checkpoints: &[Checkpoint]) -> Option<Checkpoint> {
        let mut candidates: Vec<_> =
            checkpoints.iter().filter(|c| self.can_restore(c).is_ok()).collect();
        candidates.sort_by_key(|c| std::cmp::Reverse(c.step));
        candidates.first().cloned().cloned()
    }
}

/// Distributed job state across ranks.
#[derive(Debug, Clone, PartialEq)]
pub struct DistributedJobState {
    pub attempt_id: AttemptId,
    pub world_size: u32,
    pub rank_exit_codes: BTreeMap<RankId, Option<i32>>,
    pub global_cancelled: bool,
}

impl DistributedJobState {
    pub fn new(attempt_id: AttemptId, world_size: u32) -> Result<Self, moqentra_types::Error> {
        if world_size == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "distributed job world_size must be > 0",
            ));
        }
        Ok(Self {
            attempt_id,
            world_size,
            rank_exit_codes: BTreeMap::new(),
            global_cancelled: false,
        })
    }

    pub fn report_exit(&mut self, rank_id: RankId, code: i32) {
        self.rank_exit_codes.insert(rank_id, Some(code));
    }

    pub fn all_finished(&self) -> bool {
        self.world_size > 0
            && self.rank_exit_codes.len() == self.world_size as usize
            && self.rank_exit_codes.values().all(|c| c.is_some())
    }

    pub fn all_succeeded(&self) -> bool {
        self.all_finished() && self.rank_exit_codes.values().all(|c| c == &Some(0))
    }

    pub fn any_failed(&self) -> bool {
        self.rank_exit_codes.values().any(|c| c.is_some_and(|v| v != 0))
    }

    pub fn cancel(&mut self) {
        self.global_cancelled = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{AttemptId, CheckpointId, RandomIdGenerator, RankId};

    fn make_checkpoint(state: CheckpointState, world_size: u32, step: u64) -> Checkpoint {
        let gen = RandomIdGenerator;
        Checkpoint {
            id: CheckpointId::new_v7(&gen),
            attempt_id: AttemptId::new_v7(&gen),
            step,
            state,
            digest: "sha256:ckpt".to_string(),
            manifest_digest: "sha256:manifest".to_string(),
            framework_version: "pytorch-2.4".to_string(),
            world_size,
            optimizer_signature: "adam".to_string(),
            model_signature: "resnet50".to_string(),
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn rendezvous_finalizes_when_full() {
        let gen = RandomIdGenerator;
        let mut r = Rendezvous::new(AttemptId::new_v7(&gen), 2).unwrap();
        let r1 = RankId::new_v7(&gen);
        let r2 = RankId::new_v7(&gen);
        r.join(r1, "10.0.0.1:1111").unwrap();
        r.join(r2, "10.0.0.2:2222").unwrap();
        assert!(r.finalize().is_ok());
        assert!(r.finalized);
    }

    #[test]
    fn recovery_rejects_incomplete_checkpoint() {
        let policy = RecoveryPolicy {
            max_elastic_restarts: 3,
            allowed_world_sizes: vec![2],
            compatible_framework_versions: vec!["pytorch-2.4".to_string()],
            preserve_optimizer: true,
        };
        let ckpt = make_checkpoint(CheckpointState::Uploading, 2, 10);
        assert!(policy.can_restore(&ckpt).is_err());
    }

    #[test]
    fn distributed_job_requires_all_ranks_succeed() {
        let gen = RandomIdGenerator;
        let mut state = DistributedJobState::new(AttemptId::new_v7(&gen), 2).unwrap();
        let r1 = RankId::new_v7(&gen);
        let r2 = RankId::new_v7(&gen);
        state.report_exit(r1, 0);
        assert!(!state.all_succeeded());
        state.report_exit(r2, 1);
        assert!(!state.all_succeeded());
        assert!(state.any_failed());
    }
}
