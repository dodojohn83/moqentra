//! Training experiment and job application services.

use moqentra_domain::training::{Experiment, TrainingJob, TrainingJobSpec, TrainingJobState};
use moqentra_types::{AttemptId, Error, ExperimentId, ProjectId, TenantId, TrainingJobId};
use std::collections::BTreeMap;

/// In-memory training registry for single-process control plane.
#[derive(Debug, Default)]
pub struct InMemoryTrainingRegistry {
    experiments: BTreeMap<ExperimentId, Experiment>,
    jobs: BTreeMap<TrainingJobId, TrainingJob>,
}

impl InMemoryTrainingRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace in-memory state with durable rows (control-plane restart recovery).
    pub fn hydrate(&mut self, experiments: Vec<Experiment>, jobs: Vec<TrainingJob>) {
        self.experiments.clear();
        self.jobs.clear();
        for exp in experiments {
            self.experiments.insert(exp.id, exp);
        }
        for job in jobs {
            self.jobs.insert(job.id, job);
        }
    }

    /// Create an experiment under a tenant/project.
    pub fn create_experiment(&mut self, experiment: Experiment) -> Result<Experiment, Error> {
        if self.experiments.contains_key(&experiment.id) {
            return Err(Error::conflict("experiment already exists"));
        }
        self.experiments.insert(experiment.id, experiment.clone());
        Ok(experiment)
    }

    /// Get experiment with tenant isolation.
    pub fn get_experiment(
        &self,
        tenant_id: TenantId,
        id: ExperimentId,
    ) -> Result<&Experiment, Error> {
        let exp = self.experiments.get(&id).ok_or_else(|| Error::not_found("experiment"))?;
        if exp.tenant_id != tenant_id {
            return Err(Error::not_found("experiment"));
        }
        Ok(exp)
    }

    /// List experiments for a tenant.
    pub fn list_experiments(
        &self,
        tenant_id: TenantId,
        project_id: Option<ProjectId>,
    ) -> Vec<&Experiment> {
        self.experiments
            .values()
            .filter(|e| e.tenant_id == tenant_id)
            .filter(|e| project_id.is_none_or(|p| e.project_id == p))
            .collect()
    }

    /// Create a training job bound to an existing experiment.
    pub fn create_job(&mut self, mut job: TrainingJob) -> Result<TrainingJob, Error> {
        let exp = self
            .experiments
            .get_mut(&job.experiment_id)
            .ok_or_else(|| Error::not_found("experiment"))?;
        if exp.tenant_id != job.tenant_id || exp.project_id != job.project_id {
            return Err(Error::permission_denied(
                "job tenant/project does not match experiment",
            ));
        }
        if self.jobs.contains_key(&job.id) {
            return Err(Error::conflict("training job already exists"));
        }
        // Jobs are submitted to the queue at creation time; Draft is used for
        // programmatic construction before submission.
        job.submit()?;
        exp.add_job(job.id)?;
        self.jobs.insert(job.id, job.clone());
        Ok(job)
    }

    /// Get a training job with tenant isolation.
    pub fn get_job(&self, tenant_id: TenantId, id: TrainingJobId) -> Result<&TrainingJob, Error> {
        let job = self.jobs.get(&id).ok_or_else(|| Error::not_found("training job"))?;
        if job.tenant_id != tenant_id {
            return Err(Error::not_found("training job"));
        }
        Ok(job)
    }

    /// List jobs for a tenant, optionally filtered by experiment.
    pub fn list_jobs(
        &self,
        tenant_id: TenantId,
        experiment_id: Option<ExperimentId>,
    ) -> Vec<&TrainingJob> {
        self.jobs
            .values()
            .filter(|j| j.tenant_id == tenant_id)
            .filter(|j| experiment_id.is_none_or(|e| j.experiment_id == e))
            .collect()
    }

    /// Admit a queued job (scheduler hand-off).
    pub fn admit_job(
        &mut self,
        tenant_id: TenantId,
        id: TrainingJobId,
    ) -> Result<&TrainingJob, Error> {
        let job = self.jobs.get_mut(&id).ok_or_else(|| Error::not_found("training job"))?;
        if job.tenant_id != tenant_id {
            return Err(Error::not_found("training job"));
        }
        job.admit()?;
        Ok(job)
    }

    /// Cancel a job.
    pub fn cancel_job(
        &mut self,
        tenant_id: TenantId,
        id: TrainingJobId,
    ) -> Result<&TrainingJob, Error> {
        let job = self.jobs.get_mut(&id).ok_or_else(|| Error::not_found("training job"))?;
        if job.tenant_id != tenant_id {
            return Err(Error::not_found("training job"));
        }
        job.cancel()?;
        Ok(job)
    }

    /// Pop next queued job id for a tenant (FIFO by created_at).
    pub fn next_queued_job_id(&self, tenant_id: TenantId) -> Option<TrainingJobId> {
        self.jobs
            .values()
            .filter(|j| j.tenant_id == tenant_id && matches!(j.state, TrainingJobState::Queued))
            .min_by_key(|j| j.created_at)
            .map(|j| j.id)
    }

    /// Find a mutable training job by its current attempt id.
    pub fn find_by_attempt_id_mut(&mut self, attempt_id: AttemptId) -> Option<&mut TrainingJob> {
        self.jobs
            .values_mut()
            .find(|j| j.current_attempt.as_ref().map(|a| a.id) == Some(attempt_id))
    }

    /// Helper to build a job from a validated spec.
    pub fn build_job(
        id: TrainingJobId,
        experiment_id: ExperimentId,
        tenant_id: TenantId,
        project_id: ProjectId,
        spec: TrainingJobSpec,
    ) -> Result<TrainingJob, Error> {
        TrainingJob::new(id, experiment_id, tenant_id, project_id, spec)
    }

    /// Snapshot jobs in any of the given states (desired/observed reconciler recovery).
    pub fn jobs_in_states(&self, states: &[TrainingJobState], limit: usize) -> Vec<TrainingJob> {
        self.jobs
            .values()
            .filter(|j| states.contains(&j.state))
            .take(limit)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::training::{
        DistributedConfig, ParameterSchema, ResourceRequest, TrainingJobSpec,
    };
    use moqentra_types::{DatasetVersionId, RandomIdGenerator};

    fn sample_spec(gen: &RandomIdGenerator) -> TrainingJobSpec {
        TrainingJobSpec {
            code_digest: "sha256:a172cedcae47474b615c54d510a5d84a8dea3032e958587430b413538be3f333"
                .to_string(),
            image_digest: "sha256:eef93e1d14482804277fca0172464032d1a4fdbcc338524059fa1e861454ad4d"
                .to_string(),
            dataset_version_id: DatasetVersionId::new_v7(gen),
            hyperparameters: ParameterSchema {
                argv: vec!["train".to_string()],
                env: Default::default(),
                config_files: Default::default(),
            },
            seed: 1,
            resources: ResourceRequest {
                replicas: 1,
                cpu_milli: 1000,
                memory_mib: 2048,
                ephemeral_storage_mib: 1024,
                accelerator_kind: None,
                accelerator_count: 0,
                topology: None,
            },
            distributed: DistributedConfig::Single,
            max_attempts: 1,
            deadline_seconds: 3600,
        }
    }

    #[test]
    fn create_admit_cancel_job() {
        let gen = RandomIdGenerator;
        let mut reg = InMemoryTrainingRegistry::new();
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let spec = sample_spec(&gen);
        let exp = Experiment::new(
            ExperimentId::new_v7(&gen),
            tenant,
            project,
            "exp-1",
            spec.dataset_version_id,
            "loss",
        )
        .unwrap();
        let exp_id = exp.id;
        reg.create_experiment(exp).unwrap();

        let job =
            TrainingJob::new(TrainingJobId::new_v7(&gen), exp_id, tenant, project, spec).unwrap();
        let job_id = job.id;
        reg.create_job(job).unwrap();
        reg.admit_job(tenant, job_id).unwrap();
        assert!(matches!(
            reg.get_job(tenant, job_id).unwrap().state,
            TrainingJobState::Admitted
        ));
    }
}
