//! Experiment and training domain aggregates and state machines.

use moqentra_types::{
    AttemptId, CheckpointId, DatasetVersionId, ExperimentId, ModelVersionId, NodeId, ProjectId,
    RankId, TenantId, TrainingJobId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::str::FromStr;

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

impl OutputManifest {
    /// Validates any present digests are valid content-addressed digests.
    pub fn validate(&self) -> Result<(), moqentra_types::Error> {
        if let Some(d) = &self.model_artifact_digest {
            if !moqentra_types::valid_content_digest(d) {
                return Err(moqentra_types::Error::invalid_argument(
                    "model_artifact_digest must be a valid content digest",
                ));
            }
        }
        if let Some(d) = &self.metric_digest {
            if !moqentra_types::valid_content_digest(d) {
                return Err(moqentra_types::Error::invalid_argument(
                    "metric_digest must be a valid content digest",
                ));
            }
        }
        if let Some(d) = &self.log_digest {
            if !moqentra_types::valid_content_digest(d) {
                return Err(moqentra_types::Error::invalid_argument(
                    "log_digest must be a valid content digest",
                ));
            }
        }
        for d in &self.checkpoint_digests {
            if !moqentra_types::valid_content_digest(d) {
                return Err(moqentra_types::Error::invalid_argument(
                    "checkpoint digests must be valid content digests",
                ));
            }
        }
        Ok(())
    }
}

/// R2 checkpoint retention policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointPolicy {
    Disabled,
    #[default]
    Full,
    Sharded,
}

/// R2 preemption behaviour for a training job.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreemptionPolicy {
    #[default]
    Never,
    LowPriority,
    Any,
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
    #[serde(default = "default_processes_per_replica")]
    pub processes_per_replica: u32,
    pub max_attempts: u32,
    pub deadline_seconds: u64,
    #[serde(default)]
    pub checkpoint_policy: CheckpointPolicy,
    pub queue_ref: Option<String>,
    pub priority_class_ref: Option<String>,
    #[serde(default)]
    pub preemption_policy: PreemptionPolicy,
    pub resource_class_ref: Option<String>,
}

fn default_processes_per_replica() -> u32 {
    1
}

impl TrainingJobSpec {
    /// Validates that the spec defines a runnable, non-degenerate job.
    pub fn validate(&self) -> Result<(), moqentra_types::Error> {
        if !moqentra_types::valid_content_digest(&self.code_digest)
            || !moqentra_types::valid_content_digest(&self.image_digest)
        {
            return Err(moqentra_types::Error::invalid_argument(
                "code and image digests must be valid content digests",
            ));
        }
        if self.max_attempts == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "max_attempts must be greater than zero",
            ));
        }
        if self.deadline_seconds == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "deadline_seconds must be greater than zero",
            ));
        }
        if self.resources.ephemeral_storage_mib == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "ephemeral_storage_mib must be greater than zero",
            ));
        }
        if self.resources.replicas == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "replicas must be greater than zero",
            ));
        }
        if self.processes_per_replica == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "processes_per_replica must be greater than zero",
            ));
        }
        if self.resources.cpu_milli == 0 || self.resources.memory_mib == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "cpu_milli and memory_mib must be greater than zero",
            ));
        }
        if self.resources.accelerator_count > 0
            && self.resources.accelerator_kind.as_deref().is_none_or(|k| k.trim().is_empty())
        {
            return Err(moqentra_types::Error::invalid_argument(
                "accelerator_kind must be set when accelerator_count is > 0",
            ));
        }
        if self.hyperparameters.argv.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "hyperparameters.argv must not be empty",
            ));
        }

        let computed_world_size =
            self.resources
                .replicas
                .checked_mul(self.processes_per_replica)
                .ok_or_else(|| moqentra_types::Error::invalid_argument("world size overflow"))?;

        if matches!(self.distributed, DistributedConfig::Single)
            && (self.resources.replicas != 1 || self.processes_per_replica != 1)
        {
            return Err(moqentra_types::Error::invalid_argument(
                "single-node training requires exactly one replica and one process per replica",
            ));
        }

        const MAX_WORLD_SIZE: u32 = 10_000;
        if computed_world_size == 0 || computed_world_size > MAX_WORLD_SIZE {
            return Err(moqentra_types::Error::invalid_argument(
                "computed world_size must be between 1 and 10000",
            ));
        }

        if let DistributedConfig::Ddp { world_size: stored } = self.distributed {
            if stored != computed_world_size {
                return Err(moqentra_types::Error::invalid_argument(
                    "distributed.world_size must equal resources.replicas * processes_per_replica",
                ));
            }
        }
        Ok(())
    }

    /// Total number of training ranks: `replicas * processes_per_replica` for DDP, or `1` for single.
    pub fn world_size(&self) -> u32 {
        match self.distributed {
            DistributedConfig::Single => 1,
            DistributedConfig::Ddp { world_size } => world_size,
        }
    }

    /// R1-compatible canonicalization: missing R2 fields default to single replica, single process,
    /// no preemption, and the default checkpoint policy.
    pub fn canonicalize(&mut self) {
        if self.processes_per_replica == 0 {
            self.processes_per_replica = 1;
        }
        if matches!(self.distributed, DistributedConfig::Single) {
            self.processes_per_replica = 1;
            self.resources.replicas = 1;
        }
        match self.distributed {
            DistributedConfig::Single => {}
            DistributedConfig::Ddp { ref mut world_size } => {
                *world_size = self.resources.replicas.saturating_mul(self.processes_per_replica);
            }
        }
        if self.max_attempts == 0 {
            self.max_attempts = 1;
        }
        if self.deadline_seconds == 0 {
            self.deadline_seconds = 3600;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistributedConfig {
    Single,
    Ddp { world_size: u32 },
}

/// State of a training job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingJobState {
    Draft,
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

impl FromStr for TrainingJobState {
    type Err = moqentra_types::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(Self::Draft),
            "Queued" => Ok(Self::Queued),
            "Admitted" => Ok(Self::Admitted),
            "Starting" => Ok(Self::Starting),
            "Running" => Ok(Self::Running),
            "Finalizing" => Ok(Self::Finalizing),
            "Succeeded" => Ok(Self::Succeeded),
            "Cancelling" => Ok(Self::Cancelling),
            "Cancelled" => Ok(Self::Cancelled),
            "Failed" => Ok(Self::Failed),
            "TimedOut" => Ok(Self::TimedOut),
            _ => Err(moqentra_types::Error::invalid_argument(format!(
                "unknown training job state: {s}"
            ))),
        }
    }
}

/// A single rank in an attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RankState {
    Pending,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rank {
    pub id: RankId,
    pub attempt_id: AttemptId,
    pub node_id: Option<NodeId>,
    pub state: RankState,
    pub last_heartbeat: Option<UtcTimestamp>,
    pub exit_code: Option<i32>,
}

/// A training attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub fn new(
        id: AttemptId,
        job_id: TrainingJobId,
        fencing_token: u64,
        ranks: Vec<Rank>,
    ) -> Result<Self, moqentra_types::Error> {
        if ranks.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "attempt must contain at least one rank",
            ));
        }
        let now = UtcTimestamp::now();
        Ok(Self {
            id,
            job_id,
            fencing_token,
            state: TrainingJobState::Starting,
            ranks,
            checkpoint_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        })
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    ) -> Result<Self, moqentra_types::Error> {
        spec.validate()?;
        let now = UtcTimestamp::now();
        Ok(Self {
            id,
            experiment_id,
            tenant_id,
            project_id,
            spec,
            state: TrainingJobState::Draft,
            current_attempt: None,
            attempts: Vec::new(),
            metrics: Vec::new(),
            output_manifest: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn submit(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, TrainingJobState::Draft) {
            return Err(moqentra_types::Error::conflict("job is not draft"));
        }
        self.state = TrainingJobState::Queued;
        self.updated_at = UtcTimestamp::now();
        Ok(())
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
        let max_attempts = usize::try_from(self.spec.max_attempts)
            .map_err(|_| moqentra_types::Error::internal("max_attempts too large"))?;
        if self.attempts.len() >= max_attempts {
            return Err(moqentra_types::Error::unavailable("max attempts reached"));
        }
        let world_size = self.spec.world_size();
        let expected_ranks = usize::try_from(world_size)
            .map_err(|_| moqentra_types::Error::internal("world_size too large"))?;
        if attempt.ranks.len() != expected_ranks {
            return Err(moqentra_types::Error::invalid_argument(
                "attempt rank count does not match distributed configuration",
            ));
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
        manifest.validate()?;
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    pub fn add_job(&mut self, job_id: TrainingJobId) -> Result<(), moqentra_types::Error> {
        if self.job_ids.contains(&job_id) {
            return Err(moqentra_types::Error::conflict("job already in experiment"));
        }
        self.job_ids.push(job_id);
        self.updated_at = UtcTimestamp::now();
        Ok(())
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
            code_digest: "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                .to_string(),
            image_digest: "sha256:6105d6cc76af400325e94d588ce511be5bfdbb73b437dc51eca43917d7a43e3d"
                .to_string(),
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
            processes_per_replica: 1,
            checkpoint_policy: Default::default(),
            queue_ref: None,
            priority_class_ref: None,
            preemption_policy: Default::default(),
            resource_class_ref: None,
            max_attempts: 3,
            deadline_seconds: 3600,
        };
        let mut job = TrainingJob::new(
            TrainingJobId::new_v7(&gen),
            ExperimentId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            spec,
        )
        .unwrap();
        job.submit().unwrap();
        job
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
        let attempt = Attempt::new(AttemptId::new_v7(&gen), job.id, 7, vec![rank]).unwrap();
        job.start_attempt(attempt).unwrap();
        job.mark_running(7).unwrap();
        job.mark_finalizing(7).unwrap();
        let manifest = OutputManifest {
            model_artifact_digest: Some(
                "sha256:9372c470eeadd5ecd9c3c74c2b3cb633f8e2f2fad799250a0f70d652b6b825e4"
                    .to_string(),
            ),
            checkpoint_digests: vec![
                "sha256:be4e4dc1f1e4907ebc4040e2a6c2ebcba6bf79cc8211367a3aceedb760503840"
                    .to_string(),
            ],
            metric_digest: Some(
                "sha256:177a7ea3611fe6b138557deaf44941614b66c0ece1766308044c8b65c8ba2123"
                    .to_string(),
            ),
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
        let attempt = Attempt::new(AttemptId::new_v7(&gen), job.id, 5, vec![rank]).unwrap();
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
        let attempt = Attempt::new(AttemptId::new_v7(&gen), job.id, 1, vec![rank]).unwrap();
        job.start_attempt(attempt).unwrap();
        job.mark_running(1).unwrap();
        job.mark_finalizing(1).unwrap();
        let manifest = OutputManifest {
            model_artifact_digest: None,
            checkpoint_digests: vec![],
            metric_digest: Some(
                "sha256:177a7ea3611fe6b138557deaf44941614b66c0ece1766308044c8b65c8ba2123"
                    .to_string(),
            ),
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
