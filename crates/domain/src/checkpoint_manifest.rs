//! Distributed checkpoint manifest and shard domain model.

use moqentra_types::{
    CheckpointManifestId, DatasetVersionId, ModelId, ProjectId, TenantId, TrainingJobId,
    UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// State of a checkpoint manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointState {
    Uploading,
    Validating,
    Complete,
    Failed,
}

/// A single checkpoint shard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointShard {
    pub rank: u32,
    pub object_key: String,
    pub size: u64,
    pub digest: String,
    pub tensor_layout: BTreeMap<String, String>,
}

/// Manifest describing a distributed checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointManifest {
    pub id: CheckpointManifestId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub training_job_id: TrainingJobId,
    pub attempt_id: String,
    pub step: u64,
    pub epoch: u64,
    pub world_size: u32,
    pub framework: String,
    pub template: String,
    pub code_digest: String,
    pub image_digest: String,
    pub dataset_version_id: DatasetVersionId,
    pub model_id: Option<ModelId>,
    pub shards: Vec<CheckpointShard>,
    pub compatibility: BTreeMap<String, String>,
    pub state: CheckpointState,
    pub created_at: UtcTimestamp,
    pub completed_at: Option<UtcTimestamp>,
}

impl CheckpointManifest {
    pub fn validate(&self) -> Result<(), moqentra_types::Error> {
        if self.world_size == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "world_size must be > 0",
            ));
        }
        if self.step == 0 {
            return Err(moqentra_types::Error::invalid_argument("step must be > 0"));
        }
        if !moqentra_types::valid_content_digest(&self.code_digest)
            || !moqentra_types::valid_content_digest(&self.image_digest)
        {
            return Err(moqentra_types::Error::invalid_argument(
                "code and image digests must be valid content digests",
            ));
        }
        let expected: std::collections::HashSet<u32> = (0..self.world_size).collect();
        let actual: std::collections::HashSet<u32> = self.shards.iter().map(|s| s.rank).collect();
        if expected != actual {
            return Err(moqentra_types::Error::invalid_argument(
                "checkpoint shards must contain exactly one entry per rank",
            ));
        }
        for shard in &self.shards {
            if !moqentra_types::valid_content_digest(&shard.digest) {
                return Err(moqentra_types::Error::invalid_argument(
                    "shard digest must be a valid content digest",
                ));
            }
        }
        Ok(())
    }

    pub fn mark_complete(&mut self) {
        self.state = CheckpointState::Complete;
        self.completed_at = Some(UtcTimestamp::now());
    }
}

/// A hold preventing checkpoint garbage collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointHold {
    pub manifest_id: CheckpointManifestId,
    pub holder: String,
    pub reason: String,
    pub created_at: UtcTimestamp,
    pub expires_at: Option<UtcTimestamp>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::RandomIdGenerator;

    fn digest() -> String {
        "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string()
    }

    #[test]
    fn manifest_requires_full_shard_set() {
        let g = RandomIdGenerator;
        let mut m = CheckpointManifest {
            id: CheckpointManifestId::new_v7(&g),
            tenant_id: TenantId::new_v7(&g),
            project_id: ProjectId::new_v7(&g),
            training_job_id: TrainingJobId::new_v7(&g),
            attempt_id: "attempt-1".to_string(),
            step: 100,
            epoch: 1,
            world_size: 2,
            framework: "pytorch".to_string(),
            template: "resnet18".to_string(),
            code_digest: digest(),
            image_digest: digest(),
            dataset_version_id: DatasetVersionId::new_v7(&g),
            model_id: None,
            shards: vec![],
            compatibility: BTreeMap::new(),
            state: CheckpointState::Uploading,
            created_at: UtcTimestamp::now(),
            completed_at: None,
        };
        assert!(m.validate().is_err());
        m.shards.push(CheckpointShard {
            rank: 0,
            object_key: "s0".to_string(),
            size: 1024,
            digest: digest(),
            tensor_layout: BTreeMap::new(),
        });
        m.shards.push(CheckpointShard {
            rank: 1,
            object_key: "s1".to_string(),
            size: 1024,
            digest: digest(),
            tensor_layout: BTreeMap::new(),
        });
        assert!(m.validate().is_ok());
    }
}
