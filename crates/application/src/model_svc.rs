//! Model registry application services.

use moqentra_domain::model_registry::{Artifact, Model, ModelVersion, ModelVersionState};
use moqentra_types::{AttemptId, Error, ModelId, ModelVersionId, ProjectId, TenantId};
use std::collections::BTreeMap;

/// Reconciles a model version's artifacts and transitions it to ready.
pub struct ArtifactReconciler;

impl ArtifactReconciler {
    /// Validate every artifact on the model version, then move the version
    /// through `Draft -> Validating -> Ready` if all checks pass.
    ///
    /// Checks performed:
    /// - artifact digest is a valid content digest,
    /// - artifact size is non-zero,
    /// - media type and scan status are non-empty,
    /// - scan status is `clean`.
    pub fn reconcile(version: &mut ModelVersion) -> Result<(), Error> {
        if !matches!(version.state, ModelVersionState::Draft) {
            return Err(Error::conflict("model version is not in draft"));
        }
        for artifact in &version.artifacts {
            Self::validate_artifact(artifact)?;
        }
        version
            .validate()
            .map_err(|e| Error::invalid_argument(format!("validation failed: {e}")))?;
        version
            .mark_ready()
            .map_err(|e| Error::invalid_argument(format!("mark ready failed: {e}")))?;
        Ok(())
    }

    fn validate_artifact(artifact: &Artifact) -> Result<(), Error> {
        if artifact.size_bytes == 0 {
            return Err(Error::invalid_argument(
                "artifact size must be greater than zero",
            ));
        }
        if artifact.media_type.trim().is_empty() {
            return Err(Error::invalid_argument("artifact media_type is empty"));
        }
        if !moqentra_types::valid_content_digest(&artifact.digest) {
            return Err(Error::invalid_argument("artifact digest is invalid"));
        }
        if artifact.scan_status.trim().is_empty() {
            return Err(Error::invalid_argument("artifact scan_status is empty"));
        }
        if artifact.scan_status != "clean" {
            return Err(Error::invalid_argument(
                "artifact scan_status must be clean",
            ));
        }
        Ok(())
    }
}

/// In-memory model registry.
#[derive(Debug, Default)]
pub struct InMemoryModelRegistry {
    models: BTreeMap<ModelId, Model>,
    versions: BTreeMap<ModelVersionId, ModelVersion>,
}

impl InMemoryModelRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace in-memory state with durable rows (control-plane restart recovery).
    pub fn hydrate(&mut self, models: Vec<Model>, versions: Vec<ModelVersion>) {
        self.models.clear();
        self.versions.clear();
        for m in models {
            self.models.insert(m.id, m);
        }
        for v in versions {
            self.versions.insert(v.id, v);
        }
    }

    /// Create a model family.
    pub fn create_model(&mut self, model: Model) -> Result<Model, Error> {
        if self.models.contains_key(&model.id) {
            return Err(Error::conflict("model already exists"));
        }
        self.models.insert(model.id, model.clone());
        Ok(model)
    }

    /// Get model with tenant isolation.
    pub fn get_model(&self, tenant_id: TenantId, id: ModelId) -> Result<&Model, Error> {
        let m = self.models.get(&id).ok_or_else(|| Error::not_found("model"))?;
        if m.tenant_id != tenant_id {
            return Err(Error::not_found("model"));
        }
        Ok(m)
    }

    /// List models for a tenant.
    pub fn list_models(&self, tenant_id: TenantId, project_id: Option<ProjectId>) -> Vec<&Model> {
        self.models
            .values()
            .filter(|m| m.tenant_id == tenant_id)
            .filter(|m| project_id.is_none_or(|p| m.project_id == p))
            .collect()
    }

    /// Create a draft model version under an existing model.
    pub fn create_version(&mut self, version: ModelVersion) -> Result<ModelVersion, Error> {
        let model = self
            .models
            .get_mut(&version.model_id)
            .ok_or_else(|| Error::not_found("model"))?;
        if model.tenant_id != version.tenant_id || model.project_id != version.project_id {
            return Err(Error::permission_denied(
                "version tenant/project does not match model",
            ));
        }
        if self.versions.contains_key(&version.id) {
            return Err(Error::conflict("model version already exists"));
        }
        model.register_version(version.id)?;
        self.versions.insert(version.id, version.clone());
        Ok(version)
    }

    /// Get a version with tenant isolation.
    pub fn get_version(
        &self,
        tenant_id: TenantId,
        id: ModelVersionId,
    ) -> Result<&ModelVersion, Error> {
        let v = self.versions.get(&id).ok_or_else(|| Error::not_found("model version"))?;
        if v.tenant_id != tenant_id {
            return Err(Error::not_found("model version"));
        }
        Ok(v)
    }

    /// Look up an existing model version produced from the same tenant,
    /// training attempt and artifact digest to avoid creating a duplicate.
    pub fn find_duplicate_version(
        &self,
        tenant_id: TenantId,
        attempt_id: AttemptId,
        artifact_digest: &str,
    ) -> Option<&ModelVersion> {
        self.versions.values().find(|v| {
            v.tenant_id == tenant_id
                && v.lineage.attempt_id == Some(attempt_id)
                && v.artifacts.iter().any(|a| a.digest == artifact_digest)
        })
    }

    /// Mutable access for state transitions after tenant check.
    pub fn with_version_mut<R>(
        &mut self,
        tenant_id: TenantId,
        id: ModelVersionId,
        f: impl FnOnce(&mut ModelVersion) -> Result<R, Error>,
    ) -> Result<R, Error> {
        let v = self.versions.get_mut(&id).ok_or_else(|| Error::not_found("model version"))?;
        if v.tenant_id != tenant_id {
            return Err(Error::not_found("model version"));
        }
        f(v)
    }

    /// Return all object keys referenced by model version artifacts. GC uses
    /// this set to protect live artifacts from deletion.
    pub fn referenced_object_keys(&self) -> std::collections::BTreeSet<String> {
        let mut keys = std::collections::BTreeSet::new();
        for v in self.versions.values() {
            for a in &v.artifacts {
                if let Some(k) = &a.object_key {
                    keys.insert(k.clone());
                }
            }
        }
        keys
    }

    /// List up to `limit` draft model versions (for artifact reconciler recovery).
    pub fn draft_versions(&self, limit: usize) -> Vec<ModelVersion> {
        self.versions
            .values()
            .filter(|v| matches!(v.state, ModelVersionState::Draft))
            .take(limit)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::model_registry::ModelLineage;
    use moqentra_types::{DatasetVersionId, RandomIdGenerator};

    #[test]
    fn model_version_tenant_isolation() {
        let gen = RandomIdGenerator;
        let mut reg = InMemoryModelRegistry::new();
        let tenant = TenantId::new_v7(&gen);
        let other = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let model = Model::new(ModelId::new_v7(&gen), tenant, project, "m1").unwrap();
        let model_id = model.id;
        reg.create_model(model).unwrap();
        let lineage = ModelLineage {
            training_job_id: None,
            experiment_id: None,
            dataset_version_id: DatasetVersionId::new_v7(&gen),
            attempt_id: None,
            annotation_project_id: None,
            base_model_version_id: None,
            code_digest: "sha256:a172cedcae47474b615c54d510a5d84a8dea3032e958587430b413538be3f333"
                .to_string(),
            image_digest: "sha256:eef93e1d14482804277fca0172464032d1a4fdbcc338524059fa1e861454ad4d"
                .to_string(),
            hyperparameter_digest:
                "sha256:b29814cf5792e684cd75d6a7fce7a67a11887e312f87ca2ac2496d81f365ff72"
                    .to_string(),
            dataset_manifest_digest: None,
            framework: None,
            template: None,
            template_version: None,
            parameters_digest: None,
            metrics_digest: None,
            checkpoint_digests: Vec::new(),
            hardware_environment: None,
        };
        let version = ModelVersion::new(
            ModelVersionId::new_v7(&gen),
            model_id,
            tenant,
            project,
            "v1",
            lineage,
        )
        .unwrap();
        let vid = version.id;
        reg.create_version(version).unwrap();
        assert!(reg.get_version(tenant, vid).is_ok());
        assert!(reg.get_version(other, vid).is_err());
    }

    #[test]
    fn artifact_reconciler_refuses_unclean_or_empty_artifact() {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let mut reg = InMemoryModelRegistry::new();
        let model = Model::new(ModelId::new_v7(&gen), tenant, project, "m1").unwrap();
        let model_id = model.id;
        reg.create_model(model).unwrap();
        let mut version = ModelVersion::new(
            ModelVersionId::new_v7(&gen),
            model_id,
            tenant,
            project,
            "v1",
            ModelLineage {
                training_job_id: None,
                experiment_id: None,
                dataset_version_id: DatasetVersionId::new_v7(&gen),
                attempt_id: None,
                annotation_project_id: None,
                base_model_version_id: None,
                code_digest:
                    "sha256:a172cedcae47474b615c54d510a5d84a8dea3032e958587430b413538be3f333"
                        .to_string(),
                image_digest:
                    "sha256:eef93e1d14482804277fca0172464032d1a4fdbcc338524059fa1e861454ad4d"
                        .to_string(),
                hyperparameter_digest:
                    "sha256:b29814cf5792e684cd75d6a7fce7a67a11887e312f87ca2ac2496d81f365ff72"
                        .to_string(),
                dataset_manifest_digest: None,
                framework: None,
                template: None,
                template_version: None,
                parameters_digest: None,
                metrics_digest: None,
                checkpoint_digests: Vec::new(),
                hardware_environment: None,
            },
        )
        .unwrap();
        let asset_id = moqentra_types::AssetId::new_v7(&gen);
        version.artifacts.push(Artifact {
            asset_id,
            digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            size_bytes: 123,
            media_type: "application/octet-stream".to_string(),
            scan_status: "infected".to_string(),
            object_key: None,
        });
        assert!(ArtifactReconciler::reconcile(&mut version).is_err());

        // fix the artifact and verify it becomes ready
        version.artifacts[0].scan_status = "clean".to_string();
        ArtifactReconciler::reconcile(&mut version).unwrap();
        assert_eq!(version.state, ModelVersionState::Ready);
    }
}
