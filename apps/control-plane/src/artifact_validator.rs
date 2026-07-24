//! Artifact validation Operation triggered by a worker Result frame.
//!
//! On a successful worker Result we parse the returned artifact manifest,
//! validate its digests, finalize the training job, and create a unique
//! ModelVersion under a newly-created Model family.

use async_trait::async_trait;
use moqentra_application::{InMemoryModelRegistry, InMemoryTrainingRegistry};
use moqentra_domain::model_registry::{Artifact, Model, ModelLineage, ModelVersion};
use moqentra_domain::training::{OutputManifest, TrainingJobState};
use moqentra_object_store::ObjectKey;
use moqentra_types::{AssetId, AttemptId, ModelId, ModelVersionId, RandomIdGenerator};
use moqentra_worker_control::ArtifactValidator;
use sha2::Digest;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppArtifactValidator {
    pub training: Arc<Mutex<InMemoryTrainingRegistry>>,
    pub models: Arc<Mutex<InMemoryModelRegistry>>,
}

impl AppArtifactValidator {
    fn compute_sha256(data: &[u8]) -> String {
        let mut hasher = sha2::Sha256::new();
        hasher.update(data);
        format!("sha256:{:x}", hasher.finalize())
    }

    fn compute_hyperparameter_digest(
        hp: &moqentra_domain::training::ParameterSchema,
    ) -> Result<String, moqentra_types::Error> {
        let json = serde_json::to_vec(hp).map_err(|e| {
            moqentra_types::Error::internal(format!("failed to serialize hyperparameters: {e}"))
        })?;
        Ok(Self::compute_sha256(&json))
    }
}

#[async_trait]
impl ArtifactValidator for AppArtifactValidator {
    async fn validate(
        &self,
        command_id: &str,
        payload: &[u8],
    ) -> Result<(), moqentra_types::Error> {
        let attempt_id = AttemptId::try_from(command_id)
            .map_err(|_| moqentra_types::Error::invalid_argument("invalid attempt id"))?;

        // 1. Parse the worker Result payload and extract the artifact manifest.
        let envelope: serde_json::Value = serde_json::from_slice(payload).map_err(|e| {
            moqentra_types::Error::invalid_argument(format!("invalid result json: {e}"))
        })?;
        let artifact_manifest = envelope
            .get("artifact_manifest")
            .ok_or_else(|| moqentra_types::Error::invalid_argument("missing artifact_manifest"))?
            .clone();
        let spec = artifact_manifest.get("spec").ok_or_else(|| {
            moqentra_types::Error::invalid_argument("missing artifact_manifest.spec")
        })?;
        let metadata = artifact_manifest.get("metadata").ok_or_else(|| {
            moqentra_types::Error::invalid_argument("missing artifact_manifest.metadata")
        })?;

        // 2. Verify the manifest references the expected attempt.
        let manifest_attempt = metadata
            .get("labels")
            .and_then(|l| l.get("attemptId"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if manifest_attempt != command_id {
            return Err(moqentra_types::Error::invalid_argument(
                "artifact manifest attemptId does not match result command_id",
            ));
        }

        // 3. Compute content digests for the returned manifest.
        let manifest_json = serde_json::to_vec(&artifact_manifest).map_err(|e| {
            moqentra_types::Error::internal(format!("manifest serialization failed: {e}"))
        })?;
        let model_artifact_digest = Self::compute_sha256(&manifest_json);

        let metrics_value = spec.get("metrics").cloned().unwrap_or(serde_json::Value::Null);
        let metrics_json = serde_json::to_vec(&metrics_value).map_err(|e| {
            moqentra_types::Error::internal(format!("metrics serialization failed: {e}"))
        })?;
        let metric_digest = Self::compute_sha256(&metrics_json);

        let mut checkpoint_digests = Vec::new();
        if let Some(signature) = spec.get("signature") {
            if let Some(ckpt) = signature.get("checkpointDigest").and_then(|v| v.as_str()) {
                checkpoint_digests.push(ckpt.to_string());
            }
        }

        let output_manifest = OutputManifest {
            model_artifact_digest: Some(model_artifact_digest.clone()),
            checkpoint_digests: checkpoint_digests.clone(),
            metric_digest: Some(metric_digest.clone()),
            log_digest: None,
        };
        // Validate the manifest before mutating job state so a malformed payload
        // does not leave the job stuck in Finalizing.
        output_manifest.validate()?;

        // 4. Locate the training job and finalize it atomically.
        let gen = RandomIdGenerator;
        let (job_id, tenant_id, project_id, lineage) = {
            let mut training = self.training.lock().unwrap_or_else(|e| e.into_inner());
            let job = training
                .find_by_attempt_id_mut(attempt_id)
                .ok_or_else(|| moqentra_types::Error::not_found("training job for attempt"))?;

            if !matches!(
                job.state,
                TrainingJobState::Starting | TrainingJobState::Running
            ) {
                return Err(moqentra_types::Error::conflict(
                    "training job is not in a state that can be finalized",
                ));
            }

            let fencing_token = job
                .current_attempt
                .as_ref()
                .map(|a| a.fencing_token)
                .ok_or_else(|| moqentra_types::Error::not_found("current attempt"))?;

            // Move Starting -> Running -> Finalizing -> Succeeded in one validation step.
            if job.state == TrainingJobState::Starting {
                job.mark_running(fencing_token)?;
            }
            job.mark_finalizing(fencing_token)?;
            job.finalize(fencing_token, output_manifest)?;

            let attempt_id = job.current_attempt.as_ref().map(|a| a.id);
            let lineage = ModelLineage {
                training_job_id: Some(job.id),
                experiment_id: Some(job.experiment_id),
                dataset_version_id: job.spec.dataset_version_id,
                attempt_id,
                annotation_project_id: None,
                base_model_version_id: None,
                code_digest: job.spec.code_digest.clone(),
                image_digest: job.spec.image_digest.clone(),
                hyperparameter_digest: Self::compute_hyperparameter_digest(
                    &job.spec.hyperparameters,
                )?,
                dataset_manifest_digest: None,
                framework: None,
                template: None,
                template_version: None,
                parameters_digest: None,
                metrics_digest: Some(metric_digest),
                checkpoint_digests,
                hardware_environment: None,
            };
            (job.id, job.tenant_id, job.project_id, lineage)
        };

        // 5. Check for an existing ModelVersion from the same (tenant, attempt, artifact digest)
        //    before creating a new one.
        let mut models = self.models.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) =
            models.find_duplicate_version(tenant_id, attempt_id, &model_artifact_digest)
        {
            tracing::info!(
                job_id = %job_id,
                version_id = %existing.id,
                "duplicate model version detected; skipping creation"
            );
            return Ok(());
        }

        // 6. Create a unique ModelVersion for the validated artifact.
        let model_id = ModelId::new_v7(&gen);
        let model = Model::new(model_id, tenant_id, project_id, format!("model-{job_id}"))?;
        models.create_model(model)?;

        let version_id = ModelVersionId::new_v7(&gen);
        let mut version =
            ModelVersion::new(version_id, model_id, tenant_id, project_id, "v1", lineage)?;

        // Build metrics from the manifest.
        let mut metrics = BTreeMap::new();
        if let Some(map) = spec.get("metrics").and_then(|m| m.as_object()) {
            for (name, value) in map {
                if let Some(v) = value.as_f64() {
                    metrics.insert(name.clone(), v);
                } else if let Some(arr) = value.as_array() {
                    if let Some(last) =
                        arr.last().and_then(|o| o.get("value")).and_then(|v| v.as_f64())
                    {
                        metrics.insert(name.clone(), last);
                    }
                }
            }
        }
        version.metrics = metrics;

        // Add the validated artifact. Record the controlled object key so the
        // artifact is protected from GC while the ModelVersion exists.
        let asset_id = AssetId::new_v7(&gen);
        let object_key = ObjectKey::asset(
            tenant_id,
            project_id,
            "models",
            &model_id.to_string(),
            &version_id.to_string(),
            &format!("artifact-{asset_id}.json"),
        )?;
        version.artifacts.push(Artifact {
            asset_id,
            digest: model_artifact_digest,
            size_bytes: u64::try_from(manifest_json.len())
                .map_err(|_| moqentra_types::Error::internal("manifest size exceeds u64 range"))?,
            media_type: "application/json".to_string(),
            scan_status: "clean".to_string(),
            object_key: Some(object_key.to_string()),
        });

        // Run the ModelVersion validation state machine.
        version.validate()?;
        version.mark_ready()?;

        models.create_version(version)?;

        tracing::info!(
            job_id = %job_id,
            model_id = %model_id,
            version_id = %version_id,
            "artifact validation succeeded; model version created"
        );
        Ok(())
    }
}
