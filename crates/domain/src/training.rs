//! Experiment and training domain aggregates and state machines.

use moqentra_types::{
    AttemptId, CheckpointId, DatasetVersionId, ExperimentId, ModelVersionId, NodeId, ProjectId,
    RankId, TenantId, TrainingJobId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Resource request for a training job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRequest {
    pub replicas: u32,
    pub cpu_milli: u64,
    pub memory_mib: u64,
    pub ephemeral_storage_mib: u64,
    pub accelerator_kind: Option<String>,
    pub accelerator_count: u32,
    pub topology: Option<String>,
}

/// Hyperparameter / parameter schema with argv array; no shell concatenation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParameterSchema {
    pub argv: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub config_files: BTreeMap<String, Value>,
}

/// Output manifest expected from a training run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputManifest {
    pub model_artifact_digest: Option<String>,
    pub checkpoint_digests: Vec<String>,
    pub metric_digest: Option<String>,
    pub log_digest: Option<String>,
}

/// Training job specification; immutable after creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainingJobSpec {
    pub code_digest: String,
    pub image_digest: String,
    pub dataset_version_id: DatasetVersionId,
    pub hyperparameters: ParameterSchema,
    pub seed: u64,
    pub resources: ResourceRequest,
    pub distributed: DistributedConfig,
    pub max_attempts: u32,
    pub deadline_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistributedConfig {
    Single,
    Ddp { world_size: u32 },
}

/// State of a training job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingJobState {
    Queued,
    Admitted,
    Starting,
    Running,
    Finalizing,
    Succeeded,
    Cancelling,
    Cancelled,
    Failed,
    TimedOut,
}

/// A single rank in an attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RankState {
    Pending,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rank {
    pub id: RankId,
    pub attempt_id: AttemptId,
    pub node_id: Option<NodeId>,
    pub state: RankState,
    pub last_heartbeat: Option<UtcTimestamp>,
    pub exit_code: Option<i32>,
}

/// A training attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    pub id: AttemptId,
    pub job_id: TrainingJobId,
    pub fencing_token: u64,
    pub state: TrainingJobState,
    pub ranks: Vec<Rank>,
    pub checkpoint_ids: Vec<CheckpointId>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl Attempt {
    pub fn new(id: AttemptId, job_id: TrainingJobId, fencing_token: u64, ranks: Vec<Rank>) -> Self {
        let now = UtcTimestamp::now();
        Self {
            id,
            job_id,
            fencing_token,
            state: TrainingJobState::Starting,
            ranks,
            checkpoint_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn rank_states_ok(&self) -> bool {
        !self.ranks.is_empty() && self.ranks.iter().all(|r| matches!(r.state, RankState::Succeeded))
    }

    pub fn any_rank_failed(&self) -> bool {
        self.ranks.iter().any(|r| matches!(r.state, RankState::Failed))
    }

    pub fn update_rank(
        &mut self,
        rank_id: RankId,
        state: RankState,
    ) -> Result<(), moqentra_types::Error> {
        let rank = self
            .ranks
            .iter_mut()
            .find(|r| r.id == rank_id)
            .ok_or_else(|| moqentra_types::Error::not_found("rank"))?;
        rank.state = state;
        rank.last_heartbeat = Some(UtcTimestamp::now());
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }
}

/// A training metric point.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricPoint {
    pub step: u64,
    pub timestamp: UtcTimestamp,
    pub name: String,
    pub value: f64,
    pub tags: BTreeMap<String, String>,
}

/// A training checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    pub id: CheckpointId,
    pub attempt_id: AttemptId,
    pub path: String,
    pub digest: String,
    pub step: u64,
    pub created_at: UtcTimestamp,
}

/// A training job.
#[derive(Debug, Clone, PartialEq)]
pub struct TrainingJob {
    pub id: TrainingJobId,
    pub experiment_id: ExperimentId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub spec: TrainingJobSpec,
    pub state: TrainingJobState,
    pub current_attempt: Option<Attempt>,
    pub attempts: Vec<AttemptId>,
    pub metrics: Vec<MetricPoint>,
    pub output_manifest: Option<OutputManifest>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl TrainingJob {
    pub fn new(
        id: TrainingJobId,
        experiment_id: ExperimentId,
        tenant_id: TenantId,
        project_id: ProjectId,
        spec: TrainingJobSpec,
    ) -> Self {
        let now = UtcTimestamp::now();
        Self {
            id,
            experiment_id,
            tenant_id,
            project_id,
            spec,
            state: TrainingJobState::Queued,
            current_attempt: None,
            attempts: Vec::new(),
            metrics: Vec::new(),
            output_manifest: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn admit(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, TrainingJobState::Queued) {
            return Err(moqentra_types::Error::conflict("job is not queued"));
        }
        self.state = TrainingJobState::Admitted;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn start_attempt(&mut self, attempt: Attempt) -> Result<(), moqentra_types::Error> {
        if !matches!(
            self.state,
            TrainingJobState::Admitted | TrainingJobState::Failed | TrainingJobState::TimedOut
        ) {
            return Err(moqentra_types::Error::conflict("job cannot start attempt"));
        }
        if self.attempts.len() >= self.spec.max_attempts as usize {
            return Err(moqentra_types::Error::unavailable("max attempts reached"));
        }
        self.attempts.push(attempt.id);
        self.current_attempt = Some(attempt);
        self.state = TrainingJobState::Starting;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn mark_running(&mut self, fencing_token: u64) -> Result<(), moqentra_types::Error> {
        self.check_fencing(fencing_token)?;
        if !matches!(self.state, TrainingJobState::Starting) {
            return Err(moqentra_types::Error::conflict("job is not starting"));
        }
        self.state = TrainingJobState::Running;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn mark_finalizing(&mut self, fencing_token: u64) -> Result<(), moqentra_types::Error> {
        self.check_fencing(fencing_token)?;
        if !matches!(self.state, TrainingJobState::Running) {
            return Err(moqentra_types::Error::conflict("job is not running"));
        }
        self.state = TrainingJobState::Finalizing;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn finalize(
        &mut self,
        fencing_token: u64,
        manifest: OutputManifest,
    ) -> Result<(), moqentra_types::Error> {
        self.check_fencing(fencing_token)?;
        if !matches!(self.state, TrainingJobState::Finalizing) {
            return Err(moqentra_types::Error::conflict("job is not finalizing"));
        }
        if manifest.model_artifact_digest.is_none() || manifest.metric_digest.is_none() {
            return Err(moqentra_types::Error::invalid_argument(
                "incomplete output manifest",
            ));
        }
        self.output_manifest = Some(manifest);
        self.state = TrainingJobState::Succeeded;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), moqentra_types::Error> {
        if matches!(
            self.state,
            TrainingJobState::Succeeded
                | TrainingJobState::Failed
                | TrainingJobState::TimedOut
                | TrainingJobState::Cancelled
        ) {
            return Err(moqentra_types::Error::conflict("job is already terminal"));
        }
        self.state = TrainingJobState::Cancelling;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn mark_cancelled(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, TrainingJobState::Cancelling) {
            return Err(moqentra_types::Error::conflict("job is not cancelling"));
        }
        self.state = TrainingJobState::Cancelled;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn fail(&mut self) -> Result<(), moqentra_types::Error> {
        if matches!(
            self.state,
            TrainingJobState::Succeeded | TrainingJobState::Cancelled
        ) {
            return Err(moqentra_types::Error::conflict("job is already terminal"));
        }
        self.state = TrainingJobState::Failed;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn timeout(&mut self) -> Result<(), moqentra_types::Error> {
        if matches!(
            self.state,
            TrainingJobState::Succeeded | TrainingJobState::Cancelled | TrainingJobState::Failed
        ) {
            return Err(moqentra_types::Error::conflict("job is already terminal"));
        }
        self.state = TrainingJobState::TimedOut;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn append_metrics(
        &mut self,
        points: Vec<MetricPoint>,
        max_cardinality: usize,
    ) -> Result<(), moqentra_types::Error> {
        if points.iter().any(|p| !p.value.is_finite()) {
            return Err(moqentra_types::Error::invalid_argument(
                "metric value must be finite",
            ));
        }
        let names: BTreeMap<String, usize> = points.iter().fold(BTreeMap::new(), |mut acc, p| {
            *acc.entry(p.name.clone()).or_insert(0) += 1;
            acc
        });
        if names.len() > max_cardinality {
            return Err(moqentra_types::Error::invalid_argument(
                "metric cardinality exceeded",
            ));
        }
        self.metrics.extend(points);
        Ok(())
    }

    fn check_fencing(&self, fencing_token: u64) -> Result<(), moqentra_types::Error> {
        let attempt = self
            .current_attempt
            .as_ref()
            .ok_or_else(|| moqentra_types::Error::conflict("no active attempt"))?;
        if attempt.fencing_token != fencing_token {
            return Err(moqentra_types::Error::conflict("stale fencing token"));
        }
        Ok(())
    }
}

/// Experiment grouping training jobs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Experiment {
    pub id: ExperimentId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub name: String,
    pub dataset_version_id: DatasetVersionId,
    pub target_metric: String,
    pub job_ids: Vec<TrainingJobId>,
    pub best_model_version_id: Option<ModelVersionId>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl Experiment {
    pub fn new(
        id: ExperimentId,
        tenant_id: TenantId,
        project_id: ProjectId,
        name: impl Into<String>,
        dataset_version_id: DatasetVersionId,
        target_metric: impl Into<String>,
    ) -> Result<Self, moqentra_types::Error> {
        let name = name.into();
        let target_metric = target_metric.into();
        if name.trim().is_empty() || name.len() > 128 {
            return Err(moqentra_types::Error::invalid_argument(
                "experiment name must be non-empty and at most 128 characters",
            ));
        }
        if target_metric.trim().is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "experiment target_metric must be non-empty",
            ));
        }
        let now = UtcTimestamp::now();
        Ok(Self {
            id,
            tenant_id,
            project_id,
            name,
            dataset_version_id,
            target_metric,
            job_ids: Vec::new(),
            best_model_version_id: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn add_job(&mut self, job_id: TrainingJobId) {
        self.job_ids.push(job_id);
        self.updated_at = UtcTimestamp::now();
    }

    pub fn set_best(&mut self, model_version_id: ModelVersionId) {
        self.best_model_version_id = Some(model_version_id);
        self.updated_at = UtcTimestamp::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{ExperimentId, ProjectId, RandomIdGenerator, RankId, TenantId};

    fn make_job() -> TrainingJob {
        let gen = RandomIdGenerator;
        let spec = TrainingJobSpec {
            code_digest: "sha256:abc".to_string(),
            image_digest: "sha256:image".to_string(),
            dataset_version_id: DatasetVersionId::new_v7(&gen),
            hyperparameters: ParameterSchema {
                argv: vec![
                    "train.py".to_string(),
                    "--seed".to_string(),
                    "42".to_string(),
                ],
                env: BTreeMap::new(),
                config_files: BTreeMap::new(),
            },
            seed: 42,
            resources: ResourceRequest {
                replicas: 1,
                cpu_milli: 1000,
                memory_mib: 4096,
                ephemeral_storage_mib: 10240,
                accelerator_kind: None,
                accelerator_count: 0,
                topology: None,
            },
            distributed: DistributedConfig::Single,
            max_attempts: 3,
            deadline_seconds: 3600,
        };
        TrainingJob::new(
            TrainingJobId::new_v7(&gen),
            ExperimentId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            spec,
        )
    }

    #[test]
    fn job_state_machine_success() {
        let gen = RandomIdGenerator;
        let mut job = make_job();
        job.admit().unwrap();
        let rank = Rank {
            id: RankId::new_v7(&gen),
            attempt_id: AttemptId::new_v7(&gen),
            node_id: None,
            state: RankState::Pending,
            last_heartbeat: None,
            exit_code: None,
        };
        let attempt = Attempt::new(AttemptId::new_v7(&gen), job.id, 7, vec![rank]);
        job.start_attempt(attempt).unwrap();
        job.mark_running(7).unwrap();
        job.mark_finalizing(7).unwrap();
        let manifest = OutputManifest {
            model_artifact_digest: Some("sha256:model".to_string()),
            checkpoint_digests: vec!["sha256:ckpt".to_string()],
            metric_digest: Some("sha256:metrics".to_string()),
            log_digest: None,
        };
        job.finalize(7, manifest).unwrap();
        assert!(matches!(job.state, TrainingJobState::Succeeded));
        assert!(job.output_manifest.is_some());
    }

    #[test]
    fn stale_fencing_token_rejected() {
        let gen = RandomIdGenerator;
        let mut job = make_job();
        job.admit().unwrap();
        let rank = Rank {
            id: RankId::new_v7(&gen),
            attempt_id: AttemptId::new_v7(&gen),
            node_id: None,
            state: RankState::Pending,
            last_heartbeat: None,
            exit_code: None,
        };
        let attempt = Attempt::new(AttemptId::new_v7(&gen), job.id, 5, vec![rank]);
        job.start_attempt(attempt).unwrap();
        assert!(job.mark_running(99).is_err());
    }

    #[test]
    fn incomplete_manifest_rejected() {
        let gen = RandomIdGenerator;
        let mut job = make_job();
        job.admit().unwrap();
        let rank = Rank {
            id: RankId::new_v7(&gen),
            attempt_id: AttemptId::new_v7(&gen),
            node_id: None,
            state: RankState::Pending,
            last_heartbeat: None,
            exit_code: None,
        };
        let attempt = Attempt::new(AttemptId::new_v7(&gen), job.id, 1, vec![rank]);
        job.start_attempt(attempt).unwrap();
        job.mark_running(1).unwrap();
        job.mark_finalizing(1).unwrap();
        let manifest = OutputManifest {
            model_artifact_digest: None,
            checkpoint_digests: vec![],
            metric_digest: Some("sha256:metrics".to_string()),
            log_digest: None,
        };
        assert!(job.finalize(1, manifest).is_err());
    }

    #[test]
    fn metric_cardinality_enforced() {
        let mut job = make_job();
        let points = vec![
            MetricPoint {
                step: 1,
                timestamp: UtcTimestamp::now(),
                name: "loss".to_string(),
                value: 0.5,
                tags: BTreeMap::new(),
            },
            MetricPoint {
                step: 1,
                timestamp: UtcTimestamp::now(),
                name: "acc".to_string(),
                value: 0.9,
                tags: BTreeMap::new(),
            },
            MetricPoint {
                step: 1,
                timestamp: UtcTimestamp::now(),
                name: "lr".to_string(),
                value: 0.01,
                tags: BTreeMap::new(),
            },
        ];
        assert!(job.append_metrics(points, 2).is_err());
    }

    #[test]
    fn non_finite_metric_rejected() {
        let mut job = make_job();
        let nan = MetricPoint {
            step: 1,
            timestamp: UtcTimestamp::now(),
            name: "loss".to_string(),
            value: f64::NAN,
            tags: BTreeMap::new(),
        };
        let inf = MetricPoint {
            step: 1,
            timestamp: UtcTimestamp::now(),
            name: "loss".to_string(),
            value: f64::INFINITY,
            tags: BTreeMap::new(),
        };
        assert!(job.append_metrics(vec![nan], 10).is_err());
        assert!(job.append_metrics(vec![inf], 10).is_err());
    }

    #[test]
    fn experiment_tracks_best_model() {
        let gen = RandomIdGenerator;
        let mut exp = Experiment::new(
            ExperimentId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            "exp-1",
            DatasetVersionId::new_v7(&gen),
            "accuracy",
        )
        .unwrap();
        let model = ModelVersionId::new_v7(&gen);
        exp.set_best(model);
        assert_eq!(exp.best_model_version_id, Some(model));
    }
}
