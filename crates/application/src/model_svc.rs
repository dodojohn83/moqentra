//! Model registry application services.

use moqentra_domain::model_registry::{Model, ModelVersion};
use moqentra_types::{Error, ModelId, ModelVersionId, ProjectId, TenantId};
use std::collections::BTreeMap;

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
            annotation_project_id: None,
            base_model_version_id: None,
            code_digest: "sha256:a172cedcae47474b615c54d510a5d84a8dea3032e958587430b413538be3f333"
                .to_string(),
            image_digest: "sha256:eef93e1d14482804277fca0172464032d1a4fdbcc338524059fa1e861454ad4d"
                .to_string(),
            hyperparameter_digest:
                "sha256:b29814cf5792e684cd75d6a7fce7a67a11887e312f87ca2ac2496d81f365ff72"
                    .to_string(),
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
}
