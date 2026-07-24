//! Distributed checkpoint orchestration service.
//!
//! Implements the two-phase finalize protocol: ranks upload shards under a
//! controlled key prefix, the coordinator validates the full set and writes a
//! content-addressed manifest followed by a complete marker. Idempotency is
//! guaranteed by the marker: re-running `finalize` with the same manifest
//! returns the same digest without writing duplicate objects.

use moqentra_domain::checkpoint_manifest::{CheckpointManifest, CheckpointShard, CheckpointState};
use moqentra_domain::training::Attempt;
use moqentra_object_store::{ObjectKey, ObjectStorage};
use moqentra_types::{
    CheckpointManifestId, Error, ProjectId, TenantId, TrainingJobId, UtcTimestamp,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Checkpoint builder and coordinator.
pub struct CheckpointService;

impl CheckpointService {
    /// Controlled object key for the checkpoint manifest.
    pub fn manifest_key(
        tenant_id: TenantId,
        project_id: ProjectId,
        job_id: TrainingJobId,
        attempt_id: &str,
        step: u64,
    ) -> Result<ObjectKey, Error> {
        ObjectKey::asset(
            tenant_id,
            project_id,
            "training-jobs",
            &job_id.to_string(),
            attempt_id,
            &format!("steps/{step}/manifest.json"),
        )
    }

    /// Controlled object key for a single shard.
    pub fn shard_key(
        tenant_id: TenantId,
        project_id: ProjectId,
        job_id: TrainingJobId,
        attempt_id: &str,
        step: u64,
        rank: u32,
    ) -> Result<ObjectKey, Error> {
        ObjectKey::asset(
            tenant_id,
            project_id,
            "training-jobs",
            &job_id.to_string(),
            attempt_id,
            &format!("steps/{step}/shard-{rank}.pt"),
        )
    }

    /// Content-addressed complete marker key.
    pub fn complete_marker_key(
        tenant_id: TenantId,
        project_id: ProjectId,
        job_id: TrainingJobId,
        attempt_id: &str,
        manifest_digest: &str,
    ) -> Result<ObjectKey, Error> {
        ObjectKey::asset(
            tenant_id,
            project_id,
            "training-jobs",
            &job_id.to_string(),
            attempt_id,
            &format!("completed/{manifest_digest}"),
        )
    }

    /// Build an empty manifest for a training step.
    #[allow(clippy::too_many_arguments)]
    pub fn build_manifest(
        id: CheckpointManifestId,
        tenant_id: TenantId,
        project_id: ProjectId,
        job_id: TrainingJobId,
        attempt: &Attempt,
        fencing_token: u64,
        step: u64,
        epoch: u64,
        framework: &str,
        template: &str,
        code_digest: &str,
        image_digest: &str,
        dataset_version_id: moqentra_types::DatasetVersionId,
        world_size: u32,
        compatibility: std::collections::BTreeMap<String, String>,
    ) -> Result<CheckpointManifest, Error> {
        if world_size == 0 {
            return Err(Error::invalid_argument("world_size must be > 0"));
        }
        if step == 0 {
            return Err(Error::invalid_argument("step must be > 0"));
        }
        let mut compatibility = compatibility;
        compatibility.insert("fencing_token".to_string(), fencing_token.to_string());
        Ok(CheckpointManifest {
            id,
            tenant_id,
            project_id,
            training_job_id: job_id,
            attempt_id: attempt.id.to_string(),
            step,
            epoch,
            world_size,
            framework: framework.to_string(),
            template: template.to_string(),
            code_digest: code_digest.to_string(),
            image_digest: image_digest.to_string(),
            dataset_version_id,
            model_id: None,
            shards: Vec::new(),
            compatibility,
            state: CheckpointState::Uploading,
            created_at: UtcTimestamp::now(),
            completed_at: None,
        })
    }

    /// Report a shard from a rank. Validates the shard belongs to the attempt,
    /// uses the controlled object key, and has a valid digest/size.
    pub fn report_shard(
        manifest: &mut CheckpointManifest,
        shard: CheckpointShard,
    ) -> Result<(), Error> {
        if manifest.state != CheckpointState::Uploading {
            return Err(Error::conflict("checkpoint is not in uploading state"));
        }
        if shard.rank >= manifest.world_size {
            return Err(Error::invalid_argument("shard rank out of range"));
        }
        if manifest.shards.iter().any(|s| s.rank == shard.rank) {
            return Err(Error::conflict("shard for rank already reported"));
        }
        if !moqentra_types::valid_content_digest(&shard.digest) {
            return Err(Error::invalid_argument(
                "shard digest is not a valid content digest",
            ));
        }
        if shard.size == 0 {
            return Err(Error::invalid_argument(
                "shard size must be greater than zero",
            ));
        }
        let expected = Self::shard_key(
            manifest.tenant_id,
            manifest.project_id,
            manifest.training_job_id,
            &manifest.attempt_id,
            manifest.step,
            shard.rank,
        )?
        .to_string();
        if shard.object_key != expected {
            return Err(Error::invalid_argument(
                "shard object key is not under the controlled attempt/step path",
            ));
        }
        manifest.shards.push(shard);
        Ok(())
    }

    /// Finalize a checkpoint once all shards are reported.
    ///
    /// 1. Validates the manifest and full shard set.
    /// 2. Verifies the caller holds the current attempt's fencing token.
    /// 3. Computes a stable content digest over the canonical manifest.
    /// 4. Writes the manifest object and a content-addressed complete marker.
    /// 5. Idempotent: if the marker already exists with the same digest, the
    ///    manifest is returned as complete without duplicate writes.
    pub async fn finalize(
        manifest: &mut CheckpointManifest,
        store: &dyn ObjectStorage,
        fencing_token: u64,
    ) -> Result<String, Error> {
        let stored_token = manifest
            .compatibility
            .get("fencing_token")
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| Error::invalid_argument("manifest missing fencing_token"))?;
        if stored_token != fencing_token {
            return Err(Error::conflict("fencing token mismatch; stale attempt"));
        }

        if manifest.state == CheckpointState::Complete {
            // Recompute the canonical digest to detect whether a marker already
            // exists for this exact manifest.
            let digest = Self::manifest_digest(manifest)?;
            let marker_key = Self::complete_marker_key(
                manifest.tenant_id,
                manifest.project_id,
                manifest.training_job_id,
                &manifest.attempt_id,
                &digest,
            )?;
            if let Ok((bytes, _)) = store.get_object(marker_key.as_str()).await {
                if String::from_utf8_lossy(&bytes).trim() == digest {
                    return Ok(digest);
                }
            }
            return Err(Error::conflict(
                "checkpoint marked complete but marker is missing or mismatched",
            ));
        }
        if !matches!(
            manifest.state,
            CheckpointState::Uploading | CheckpointState::Validating
        ) {
            return Err(Error::conflict(
                "checkpoint cannot be finalized from its current state",
            ));
        }
        manifest.validate()?;

        let expected: BTreeSet<u32> = (0..manifest.world_size).collect();
        let actual: BTreeSet<u32> = manifest.shards.iter().map(|s| s.rank).collect();
        if expected != actual {
            return Err(Error::invalid_argument(
                "checkpoint is missing shards or contains duplicate/invalid ranks",
            ));
        }

        manifest.shards.sort_by_key(|s| s.rank);

        // Compute the canonical digest before attaching the wall-clock completion
        // time so that re-finalization is deterministic.
        let digest = Self::manifest_digest(manifest)?;

        let manifest_key = Self::manifest_key(
            manifest.tenant_id,
            manifest.project_id,
            manifest.training_job_id,
            &manifest.attempt_id,
            manifest.step,
        )?;
        let marker_key = Self::complete_marker_key(
            manifest.tenant_id,
            manifest.project_id,
            manifest.training_job_id,
            &manifest.attempt_id,
            &digest,
        )?;

        // Idempotency check: if the marker exists and matches, do not write again.
        if let Ok((bytes, _)) = store.get_object(marker_key.as_str()).await {
            let existing = String::from_utf8_lossy(&bytes);
            if existing.trim() == digest {
                manifest.state = CheckpointState::Complete;
                manifest.completed_at = Some(UtcTimestamp::now());
                return Ok(digest);
            }
            return Err(Error::conflict(
                "complete marker exists with a different digest",
            ));
        }

        manifest.state = CheckpointState::Complete;
        manifest.completed_at = Some(UtcTimestamp::now());

        let bytes = serde_json::to_vec(manifest)
            .map_err(|e| Error::internal(format!("manifest serialization failed: {e}")))?;
        store
            .put_object(
                manifest_key.as_str(),
                bytes.into(),
                Some("application/json"),
            )
            .await?;
        store
            .put_object(
                marker_key.as_str(),
                digest.clone().into(),
                Some("text/plain"),
            )
            .await?;

        Ok(digest)
    }

    /// Compute a stable content digest for `manifest`.
    ///
    /// The digest treats the manifest as Complete and clears `completed_at` so
    /// that re-finalization is deterministic and idempotent.
    fn manifest_digest(manifest: &CheckpointManifest) -> Result<String, Error> {
        let mut canonical = manifest.clone();
        canonical.state = CheckpointState::Complete;
        canonical.completed_at = None;
        let bytes = serde_json::to_vec(&canonical).map_err(|e| {
            Error::internal(format!("canonical manifest serialization failed: {e}"))
        })?;
        Ok(format!("sha256:{:x}", Sha256::digest(&bytes)))
    }

    /// Select the latest compatible complete checkpoint for recovery.
    pub fn select_for_recovery(
        manifests: &[CheckpointManifest],
        world_size: u32,
        framework: &str,
        template: &str,
        code_digest: &str,
        image_digest: &str,
        dataset_version_id: moqentra_types::DatasetVersionId,
    ) -> Option<CheckpointManifest> {
        let mut candidates: Vec<_> = manifests
            .iter()
            .filter(|m| m.state == CheckpointState::Complete)
            .filter(|m| m.world_size == world_size)
            .filter(|m| m.framework == framework)
            .filter(|m| m.template == template)
            .filter(|m| m.code_digest == code_digest)
            .filter(|m| m.image_digest == image_digest)
            .filter(|m| m.dataset_version_id == dataset_version_id)
            .cloned()
            .collect();
        candidates.sort_by_key(|m| std::cmp::Reverse(m.step));
        candidates.into_iter().next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::training::{Attempt, Rank, RankState};
    use moqentra_object_store::InMemoryObjectStore;
    use moqentra_types::{
        AttemptId, DatasetVersionId, ProjectId, RandomIdGenerator, RankId, TenantId, TrainingJobId,
    };

    fn make_manifest(world_size: u32) -> (CheckpointManifest, Attempt) {
        let g = RandomIdGenerator;
        let tenant_id = TenantId::new_v7(&g);
        let project_id = ProjectId::new_v7(&g);
        let job_id = TrainingJobId::new_v7(&g);
        let dataset_version_id = DatasetVersionId::new_v7(&g);
        let attempt_id = AttemptId::new_v7(&g);
        let ranks: Vec<_> = (0..world_size)
            .map(|_| Rank {
                id: RankId::new_v7(&g),
                attempt_id,
                node_id: None,
                state: RankState::Pending,
                last_heartbeat: None,
                exit_code: None,
            })
            .collect();
        let attempt = Attempt::new(attempt_id, job_id, 7, ranks).unwrap();
        let manifest = CheckpointService::build_manifest(
            CheckpointManifestId::new_v7(&g),
            tenant_id,
            project_id,
            job_id,
            &attempt,
            7,
            10,
            1,
            "pytorch",
            "resnet18",
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "sha256:1111111111111111111111111111111111111111111111111111111111111111",
            dataset_version_id,
            world_size,
            std::collections::BTreeMap::new(),
        )
        .unwrap();
        (manifest, attempt)
    }

    fn digest() -> String {
        "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string()
    }

    fn make_shard(manifest: &CheckpointManifest, rank: u32) -> CheckpointShard {
        let object_key = CheckpointService::shard_key(
            manifest.tenant_id,
            manifest.project_id,
            manifest.training_job_id,
            &manifest.attempt_id,
            manifest.step,
            rank,
        )
        .unwrap()
        .to_string();
        CheckpointShard {
            rank,
            object_key,
            size: 1024,
            digest: digest(),
            tensor_layout: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn build_manifest_rejects_zero_world_size() {
        let g = RandomIdGenerator;
        let attempt = make_manifest(1).1;
        assert!(CheckpointService::build_manifest(
            CheckpointManifestId::new_v7(&g),
            TenantId::new_v7(&g),
            ProjectId::new_v7(&g),
            TrainingJobId::new_v7(&g),
            &attempt,
            7,
            1,
            1,
            "pytorch",
            "resnet18",
            &digest(),
            &digest(),
            DatasetVersionId::new_v7(&g),
            0,
            std::collections::BTreeMap::new(),
        )
        .is_err());
    }

    #[test]
    fn report_shard_rejects_duplicate_and_out_of_range_ranks() {
        let (mut manifest, _) = make_manifest(2);
        let s0 = make_shard(&manifest, 0);
        CheckpointService::report_shard(&mut manifest, s0.clone()).unwrap();
        assert!(CheckpointService::report_shard(&mut manifest, s0).is_err());
        let s2 = make_shard(&manifest, 2);
        assert!(CheckpointService::report_shard(&mut manifest, s2).is_err());
    }

    #[test]
    fn report_shard_rejects_uncontrolled_object_key() {
        let (mut manifest, _) = make_manifest(1);
        let mut shard = make_shard(&manifest, 0);
        shard.object_key = "uncontrolled/key".to_string();
        assert!(CheckpointService::report_shard(&mut manifest, shard).is_err());
    }

    #[tokio::test]
    async fn finalize_writes_manifest_and_complete_marker() {
        let (mut manifest, _) = make_manifest(2);
        let s0 = make_shard(&manifest, 0);
        let s1 = make_shard(&manifest, 1);
        CheckpointService::report_shard(&mut manifest, s0).unwrap();
        CheckpointService::report_shard(&mut manifest, s1).unwrap();

        let store = InMemoryObjectStore::new();
        let digest = CheckpointService::finalize(&mut manifest, &store, 7).await.unwrap();
        assert!(digest.starts_with("sha256:"));
        assert_eq!(manifest.state, CheckpointState::Complete);
        assert!(manifest.completed_at.is_some());

        let marker_key = CheckpointService::complete_marker_key(
            manifest.tenant_id,
            manifest.project_id,
            manifest.training_job_id,
            &manifest.attempt_id,
            &digest,
        )
        .unwrap();
        let (bytes, _) = store.get_object(marker_key.as_str()).await.unwrap();
        assert_eq!(String::from_utf8_lossy(&bytes).trim(), digest);
    }

    #[tokio::test]
    async fn finalize_is_idempotent() {
        let (mut manifest, _) = make_manifest(2);
        let s0 = make_shard(&manifest, 0);
        let s1 = make_shard(&manifest, 1);
        CheckpointService::report_shard(&mut manifest, s0).unwrap();
        CheckpointService::report_shard(&mut manifest, s1).unwrap();

        let store = InMemoryObjectStore::new();
        let d1 = CheckpointService::finalize(&mut manifest, &store, 7).await.unwrap();
        let d2 = CheckpointService::finalize(&mut manifest, &store, 7).await.unwrap();
        assert_eq!(d1, d2);

        let manifest_key = CheckpointService::manifest_key(
            manifest.tenant_id,
            manifest.project_id,
            manifest.training_job_id,
            &manifest.attempt_id,
            manifest.step,
        )
        .unwrap();
        // A second manifest object should not have been written; the marker
        // short-circuits the second call.
        assert!(store.get_object(manifest_key.as_str()).await.is_ok());
    }

    #[test]
    fn finalize_rejects_incomplete_shard_set() {
        let (mut manifest, _) = make_manifest(2);
        let s0 = make_shard(&manifest, 0);
        CheckpointService::report_shard(&mut manifest, s0).unwrap();
        // world_size=2 but only one shard reported.
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let store = InMemoryObjectStore::new();
            assert!(CheckpointService::finalize(&mut manifest, &store, 7).await.is_err());
        });
    }

    #[test]
    fn select_for_recovery_chooses_latest_complete_compatible() {
        let (mut m1, _) = make_manifest(2);
        m1.state = CheckpointState::Complete;
        m1.step = 5;
        let (mut m2, _) = make_manifest(2);
        m2.state = CheckpointState::Complete;
        m2.step = 10;
        let (mut m3, _) = make_manifest(2);
        m3.state = CheckpointState::Uploading;
        m3.step = 20;
        let selected = CheckpointService::select_for_recovery(
            &[m1, m2.clone(), m3],
            2,
            "pytorch",
            "resnet18",
            &m2.code_digest,
            &m2.image_digest,
            m2.dataset_version_id,
        )
        .unwrap();
        assert_eq!(selected.step, 10);
    }
}
