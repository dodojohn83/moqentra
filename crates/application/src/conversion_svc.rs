//! Conversion and evaluation application services.

use moqentra_domain::conversion::{ConversionJob, EvaluationRun};
use moqentra_types::{ConversionJobId, Error, EvaluationRunId, ProjectId, TenantId};
use std::collections::BTreeMap;

/// In-memory conversion job registry for unit tests and local dev.
#[derive(Debug, Default)]
pub struct InMemoryConversionRegistry {
    jobs: BTreeMap<ConversionJobId, ConversionJob>,
}

impl InMemoryConversionRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a conversion job with tenant isolation.
    pub fn create_job(&mut self, job: ConversionJob) -> Result<ConversionJob, Error> {
        if self.jobs.contains_key(&job.id) {
            return Err(Error::conflict("conversion job already exists"));
        }
        self.jobs.insert(job.id, job.clone());
        Ok(job)
    }

    /// Get a conversion job with tenant/project isolation.
    pub fn get_job(
        &self,
        tenant_id: TenantId,
        project_id: ProjectId,
        id: ConversionJobId,
    ) -> Result<&ConversionJob, Error> {
        let job = self.jobs.get(&id).ok_or_else(|| Error::not_found("conversion job"))?;
        if job.tenant_id != tenant_id || job.project_id != project_id {
            return Err(Error::not_found("conversion job"));
        }
        Ok(job)
    }

    /// List conversion jobs for a tenant/project.
    pub fn list_jobs(&self, tenant_id: TenantId, project_id: ProjectId) -> Vec<&ConversionJob> {
        self.jobs
            .values()
            .filter(|j| j.tenant_id == tenant_id && j.project_id == project_id)
            .collect()
    }

    /// Update a conversion job after tenant/project check.
    pub fn update_job(
        &mut self,
        tenant_id: TenantId,
        project_id: ProjectId,
        id: ConversionJobId,
        f: impl FnOnce(&mut ConversionJob) -> Result<(), Error>,
    ) -> Result<ConversionJob, Error> {
        let job = self.jobs.get_mut(&id).ok_or_else(|| Error::not_found("conversion job"))?;
        if job.tenant_id != tenant_id || job.project_id != project_id {
            return Err(Error::not_found("conversion job"));
        }
        f(job)?;
        Ok(job.clone())
    }

    /// Delete a conversion job.
    pub fn delete_job(
        &mut self,
        tenant_id: TenantId,
        project_id: ProjectId,
        id: ConversionJobId,
    ) -> Result<(), Error> {
        let job = self.jobs.get(&id).ok_or_else(|| Error::not_found("conversion job"))?;
        if job.tenant_id != tenant_id || job.project_id != project_id {
            return Err(Error::not_found("conversion job"));
        }
        self.jobs.remove(&id);
        Ok(())
    }
}

/// In-memory evaluation run registry.
#[derive(Debug, Default)]
pub struct InMemoryEvaluationRegistry {
    runs: BTreeMap<EvaluationRunId, EvaluationRun>,
}

impl InMemoryEvaluationRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an evaluation run with tenant/project isolation.
    pub fn create_run(&mut self, run: EvaluationRun) -> Result<EvaluationRun, Error> {
        if self.runs.contains_key(&run.id) {
            return Err(Error::conflict("evaluation run already exists"));
        }
        self.runs.insert(run.id, run.clone());
        Ok(run)
    }

    /// Get an evaluation run.
    pub fn get_run(
        &self,
        tenant_id: TenantId,
        project_id: ProjectId,
        id: EvaluationRunId,
    ) -> Result<&EvaluationRun, Error> {
        let run = self.runs.get(&id).ok_or_else(|| Error::not_found("evaluation run"))?;
        if run.tenant_id != tenant_id || run.project_id != project_id {
            return Err(Error::not_found("evaluation run"));
        }
        Ok(run)
    }

    /// List evaluation runs.
    pub fn list_runs(&self, tenant_id: TenantId, project_id: ProjectId) -> Vec<&EvaluationRun> {
        self.runs
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.project_id == project_id)
            .collect()
    }

    /// Update an evaluation run.
    pub fn update_run(
        &mut self,
        tenant_id: TenantId,
        project_id: ProjectId,
        id: EvaluationRunId,
        f: impl FnOnce(&mut EvaluationRun) -> Result<(), Error>,
    ) -> Result<EvaluationRun, Error> {
        let run = self.runs.get_mut(&id).ok_or_else(|| Error::not_found("evaluation run"))?;
        if run.tenant_id != tenant_id || run.project_id != project_id {
            return Err(Error::not_found("evaluation run"));
        }
        f(run)?;
        Ok(run.clone())
    }

    /// Delete an evaluation run.
    pub fn delete_run(
        &mut self,
        tenant_id: TenantId,
        project_id: ProjectId,
        id: EvaluationRunId,
    ) -> Result<(), Error> {
        let run = self.runs.get(&id).ok_or_else(|| Error::not_found("evaluation run"))?;
        if run.tenant_id != tenant_id || run.project_id != project_id {
            return Err(Error::not_found("evaluation run"));
        }
        self.runs.remove(&id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::conversion::{ConversionJobState, ConversionProfile, ConversionTarget};
    use moqentra_types::{ConversionJobId, ModelVersionId, ProjectId, TenantId};

    fn ids() -> (TenantId, ProjectId, ModelVersionId, ConversionJobId) {
        let g = moqentra_types::RandomIdGenerator;
        (
            TenantId::new_v7(&g),
            ProjectId::new_v7(&g),
            ModelVersionId::new_v7(&g),
            ConversionJobId::new_v7(&g),
        )
    }

    fn profile() -> ConversionProfile {
        ConversionProfile {
            target: ConversionTarget::Onnx,
            sdk_version: "1.16.3".to_string(),
            toolchain_image_digest:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            target_chip: "generic".to_string(),
            precision: "fp32".to_string(),
            dynamic_shapes: false,
            capabilities: vec![],
        }
    }

    fn job(
        id: ConversionJobId,
        tenant_id: TenantId,
        project_id: ProjectId,
        model_version_id: ModelVersionId,
    ) -> ConversionJob {
        ConversionJob::new(
            id,
            tenant_id,
            project_id,
            model_version_id,
            ConversionTarget::Onnx,
            profile(),
            BTreeMap::new(),
        )
        .unwrap()
    }

    #[test]
    fn in_memory_conversion_registry_lifecycle() {
        let (t, p, m, id) = ids();
        let mut reg = InMemoryConversionRegistry::new();
        let j = job(id, t, p, m);
        reg.create_job(j.clone()).unwrap();
        assert_eq!(reg.get_job(t, p, id).unwrap().id, id);
        reg.update_job(t, p, id, |j| j.start()).unwrap();
        assert_eq!(
            reg.get_job(t, p, id).unwrap().state,
            ConversionJobState::Running
        );
        reg.delete_job(t, p, id).unwrap();
        assert!(reg.get_job(t, p, id).is_err());
    }
}
