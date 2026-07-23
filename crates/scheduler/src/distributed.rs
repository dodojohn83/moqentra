//! Distributed training launcher, rendezvous and checkpoint recovery.

use moqentra_domain::training::{Attempt, TrainingJob};
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
        let expected = usize::try_from(self.world_size)
            .map_err(|_| moqentra_types::Error::internal("world_size too large"))?;
        if self.members.len() != expected {
            return Err(moqentra_types::Error::unavailable("rendezvous incomplete"));
        }
        self.finalized = true;
        Ok(())
    }
}

/// Finalize rendezvous and transition the training job to `Running`.
///
/// This is the safe coordinator path: the job only becomes `Running` once the
/// full gang has checked in, and the fencing token prevents stale controllers
/// from driving the state machine.
pub fn coordinate_rendezvous(
    rendezvous: &mut Rendezvous,
    job: &mut TrainingJob,
    fencing_token: u64,
) -> Result<(), moqentra_types::Error> {
    rendezvous.finalize()?;
    job.mark_running(fencing_token)?;
    Ok(())
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

    /// Exponential backoff for the `n`-th recovery attempt (capped at 64s).
    pub fn backoff_seconds(&self, n: u32) -> u64 {
        1u64 << n.min(6)
    }
}

/// Classified reason for a gang failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    UserCode,
    Data,
    Oom,
    Device,
    Node,
    Network,
    Scheduler,
    Platform,
    Unknown,
}

impl FailureClass {
    /// Whether the failure is eligible for automatic retry.
    pub fn retry_eligible(self) -> bool {
        !matches!(self, Self::UserCode | Self::Data | Self::Unknown)
    }
}

/// Classify an exit code (and optional signal) into a `FailureClass`.
pub struct FailureClassifier;

impl FailureClassifier {
    fn diagnose(lower: &[String]) -> Option<FailureClass> {
        let mentions = |needle: &str| lower.iter().any(|s| s.contains(needle));
        if mentions("oom") || mentions("out of memory") || mentions("killed") {
            Some(FailureClass::Oom)
        } else if mentions("cuda") || mentions("device") {
            Some(FailureClass::Device)
        } else if mentions("network") || mentions("connection") {
            Some(FailureClass::Network)
        } else if mentions("node") {
            Some(FailureClass::Node)
        } else {
            None
        }
    }

    pub fn classify(exit_code: i32, diagnostics: &[String]) -> FailureClass {
        let lower: Vec<String> = diagnostics.iter().map(|s| s.to_lowercase()).collect();

        if exit_code == 0 {
            return FailureClass::Unknown; // should not happen for failures
        }

        // Diagnostics override generic signal/platform codes, but not clean user-code exits.
        if !matches!(exit_code, 1 | 2) {
            if let Some(class) = Self::diagnose(&lower) {
                return class;
            }
        }

        // Negative values represent raw Unix signals.
        if exit_code < 0 {
            return match -exit_code {
                6 => FailureClass::Oom,        // SIGABRT
                9 => FailureClass::Oom,        // SIGKILL (OOM killer / node loss)
                11 => FailureClass::Device,    // SIGSEGV
                15 => FailureClass::Scheduler, // SIGTERM (preemption / cancel)
                _ => FailureClass::Platform,
            };
        }

        // Exit codes 128+N represent signal N.
        if exit_code >= 128 {
            return match exit_code - 128 {
                6 => FailureClass::Oom,
                9 => FailureClass::Oom,
                11 => FailureClass::Device,
                15 => FailureClass::Scheduler,
                _ => FailureClass::Platform,
            };
        }

        match exit_code {
            1 | 2 | 126 | 127 => FailureClass::UserCode,
            124 => FailureClass::Scheduler,      // timeout
            125 | 64 | 65 => FailureClass::Data, // data / assertion errors
            134 => FailureClass::Oom,            // SIGABRT without 128 offset
            137 => FailureClass::Oom,
            139 => FailureClass::Device,
            143 => FailureClass::Scheduler,
            _ => FailureClass::Unknown,
        }
    }
}

/// Outcome of a recovery planning decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryDecision {
    RetryFromStart,
    RetryFromCheckpoint(Checkpoint),
    Fail(String),
    Cancelled,
}

/// Decide whether and how to recover a training job.
pub struct RecoveryPlanner;

impl RecoveryPlanner {
    pub fn decide(
        job: &TrainingJob,
        checkpoints: &[Checkpoint],
        policy: &RecoveryPolicy,
        failure: Option<FailureClass>,
    ) -> Result<RecoveryDecision, moqentra_types::Error> {
        if matches!(
            job.state,
            moqentra_domain::training::TrainingJobState::Cancelled
                | moqentra_domain::training::TrainingJobState::Cancelling
        ) {
            return Ok(RecoveryDecision::Cancelled);
        }

        if let Some(class) = failure {
            if !class.retry_eligible() {
                return Ok(RecoveryDecision::Fail(format!(
                    "{class:?} is not retry eligible"
                )));
            }
        }

        let max_attempts = usize::try_from(job.spec.max_attempts)
            .map_err(|_| moqentra_types::Error::internal("max_attempts too large"))?;
        if job.attempts.len() >= max_attempts {
            return Ok(RecoveryDecision::Fail("max attempts reached".to_string()));
        }

        if let Some(checkpoint) = policy.select_latest_compatible(checkpoints) {
            return Ok(RecoveryDecision::RetryFromCheckpoint(checkpoint));
        }

        Ok(RecoveryDecision::RetryFromStart)
    }
}

/// Distributed job state across ranks.
#[derive(Debug, Clone, PartialEq)]
pub struct DistributedJobState {
    pub attempt_id: AttemptId,
    pub fencing_token: u64,
    pub world_size: u32,
    pub rank_exit_codes: BTreeMap<RankId, Option<i32>>,
    pub global_cancelled: bool,
}

impl DistributedJobState {
    pub fn new(
        attempt_id: AttemptId,
        fencing_token: u64,
        world_size: u32,
    ) -> Result<Self, moqentra_types::Error> {
        if world_size == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "distributed job world_size must be > 0",
            ));
        }
        Ok(Self {
            attempt_id,
            fencing_token,
            world_size,
            rank_exit_codes: BTreeMap::new(),
            global_cancelled: false,
        })
    }

    /// Record a rank exit code. Rejects late results from an old fencing token.
    pub fn report_exit(
        &mut self,
        rank_id: RankId,
        fencing_token: u64,
        code: i32,
    ) -> Result<(), moqentra_types::Error> {
        if fencing_token != self.fencing_token {
            return Err(moqentra_types::Error::conflict(
                "stale fencing token; result belongs to a previous attempt",
            ));
        }
        self.rank_exit_codes.insert(rank_id, Some(code));
        Ok(())
    }

    pub fn all_finished(&self) -> bool {
        self.world_size > 0
            && usize::try_from(self.world_size).is_ok_and(|ws| {
                self.rank_exit_codes.len() == ws
                    && self.rank_exit_codes.values().all(|c| c.is_some())
            })
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

/// PyTorch DDP/Elastic launcher builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DdpLauncher {
    pub nnodes: u32,
    pub nproc_per_node: u32,
    pub world_size: u32,
    pub rdzv_id: String,
    pub rdzv_backend: String,
    pub rdzv_endpoint: String,
    pub master_addr: String,
    pub master_port: u16,
    pub backend: String,
    pub max_restarts: u32,
    pub script_argv: Vec<String>,
}

impl DdpLauncher {
    /// Build a launcher from a canonical job spec and attempt.
    ///
    /// `rdzv_endpoint` must be a stable `<host>:<port>` the gang can reach; for
    /// Kubernetes this is a headless service fronting the coordinator pod.
    pub fn new(
        job: &TrainingJob,
        attempt: &Attempt,
        rdzv_endpoint: impl Into<String>,
        max_restarts: u32,
    ) -> Result<Self, moqentra_types::Error> {
        let spec = &job.spec;
        let nnodes = spec.resources.replicas.max(1);
        let nproc_per_node = spec.processes_per_replica.max(1);
        let computed_world_size = nnodes
            .checked_mul(nproc_per_node)
            .ok_or_else(|| moqentra_types::Error::invalid_argument("world size overflow"))?;

        let backend = if spec.resources.accelerator_count > 0 {
            "nccl".to_string()
        } else {
            "gloo".to_string()
        };

        let rdzv_endpoint = rdzv_endpoint.into();
        let (host, port_str) = rdzv_endpoint.rsplit_once(':').unwrap_or((&rdzv_endpoint, "29500"));
        let master_port = port_str
            .parse::<u16>()
            .map_err(|_| moqentra_types::Error::invalid_argument("invalid rdzv port"))?;

        Ok(Self {
            nnodes,
            nproc_per_node,
            world_size: computed_world_size,
            rdzv_id: attempt.id.to_string(),
            rdzv_backend: "c10d".to_string(),
            rdzv_endpoint: rdzv_endpoint.clone(),
            master_addr: host.to_string(),
            master_port,
            backend,
            max_restarts,
            script_argv: spec.hyperparameters.argv.clone(),
        })
    }

    /// Produce a `torchrun` argv list. No shell concatenation is used: each token
    /// is a discrete argument.
    pub fn torchrun_argv(&self) -> Vec<String> {
        let mut args = vec![
            "torchrun".to_string(),
            "--nnodes".to_string(),
            self.nnodes.to_string(),
            "--nproc_per_node".to_string(),
            self.nproc_per_node.to_string(),
            "--rdzv_id".to_string(),
            self.rdzv_id.clone(),
            "--rdzv_backend".to_string(),
            self.rdzv_backend.clone(),
            "--rdzv_endpoint".to_string(),
            self.rdzv_endpoint.clone(),
            "--max_restarts".to_string(),
            self.max_restarts.to_string(),
        ];
        args.extend(self.script_argv.iter().cloned());
        args
    }

    /// Environment for a single rank. Reserved keys always override the user's
    /// env so the launcher cannot be spoofed.
    pub fn environment_for_rank(
        &self,
        job: &TrainingJob,
        attempt: &Attempt,
        rank: u32,
        local_rank: u32,
        node_id: impl Into<String>,
        user_env: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        let mut env = user_env.clone();
        env.insert("MOQENTRA_ATTEMPT_ID".to_string(), attempt.id.to_string());
        env.insert(
            "MOQENTRA_FENCING_TOKEN".to_string(),
            attempt.fencing_token.to_string(),
        );
        env.insert("MOQENTRA_TRAINING_JOB_ID".to_string(), job.id.to_string());
        env.insert("MOQENTRA_TENANT_ID".to_string(), job.tenant_id.to_string());
        env.insert(
            "MOQENTRA_PROJECT_ID".to_string(),
            job.project_id.to_string(),
        );
        env.insert("MOQENTRA_SEED".to_string(), job.spec.seed.to_string());
        env.insert(
            "MOQENTRA_WORLD_SIZE".to_string(),
            self.world_size.to_string(),
        );
        env.insert("MOQENTRA_NNODES".to_string(), self.nnodes.to_string());
        env.insert(
            "MOQENTRA_NPROC_PER_NODE".to_string(),
            self.nproc_per_node.to_string(),
        );
        env.insert("MOQENTRA_NODE_ID".to_string(), node_id.into());
        env.insert("MOQENTRA_BACKEND".to_string(), self.backend.clone());
        env.insert("RANK".to_string(), rank.to_string());
        env.insert("LOCAL_RANK".to_string(), local_rank.to_string());
        env.insert("WORLD_SIZE".to_string(), self.world_size.to_string());
        env.insert("MASTER_ADDR".to_string(), self.master_addr.clone());
        env.insert("MASTER_PORT".to_string(), self.master_port.to_string());
        env
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::training::{
        DistributedConfig, ParameterSchema, Rank, RankState, ResourceRequest, TrainingJobSpec,
    };
    use moqentra_types::{
        AttemptId, CheckpointId, DatasetVersionId, ExperimentId, NodeId, ProjectId,
        RandomIdGenerator, RankId, TenantId, TrainingJobId,
    };

    fn make_ddp_job(world_size: u32) -> (TrainingJob, Attempt) {
        let gen = RandomIdGenerator;
        let job_id = TrainingJobId::new_v7(&gen);
        let exp_id = ExperimentId::new_v7(&gen);
        let tenant_id = TenantId::new_v7(&gen);
        let project_id = ProjectId::new_v7(&gen);
        let dataset_version_id = DatasetVersionId::new_v7(&gen);
        let attempt_id = AttemptId::new_v7(&gen);

        let nproc = 2;
        let replicas = world_size / nproc;
        let spec = TrainingJobSpec {
            code_digest: "sha256:5694d08a2e53ffcae0c3103e5ad6f6076abd960eb1f8a56577040bc1028f702b"
                .to_string(),
            image_digest: "sha256:6105d6cc76af400325e94d588ce511be5bfdbb73b437dc51eca43917d7a43e3d"
                .to_string(),
            dataset_version_id,
            hyperparameters: ParameterSchema {
                argv: vec![
                    "train.py".to_string(),
                    "--lr".to_string(),
                    "0.01".to_string(),
                ],
                env: BTreeMap::new(),
                config_files: BTreeMap::new(),
            },
            seed: 42,
            resources: ResourceRequest {
                replicas,
                cpu_milli: 1000,
                memory_mib: 8192,
                ephemeral_storage_mib: 1024,
                accelerator_kind: Some("nvidia.com/gpu".to_string()),
                accelerator_count: 1,
                topology: None,
            },
            distributed: DistributedConfig::Ddp { world_size },
            processes_per_replica: nproc,
            checkpoint_policy: Default::default(),
            queue_ref: None,
            priority_class_ref: None,
            preemption_policy: Default::default(),
            resource_class_ref: None,
            max_attempts: 3,
            deadline_seconds: 3600,
        };

        let mut job = TrainingJob::new(job_id, exp_id, tenant_id, project_id, spec).unwrap();
        job.submit().unwrap();
        job.admit().unwrap();

        let mut ranks = Vec::with_capacity(world_size as usize);
        for _ in 0..world_size {
            ranks.push(Rank {
                id: RankId::new_v7(&gen),
                attempt_id,
                node_id: Some(NodeId::new_v7(&gen)),
                state: RankState::Pending,
                last_heartbeat: None,
                exit_code: None,
            });
        }
        let attempt = Attempt::new(attempt_id, job.id, 7, ranks).unwrap();
        job.start_attempt(attempt.clone()).unwrap();
        (job, attempt)
    }

    fn make_checkpoint(state: CheckpointState, world_size: u32, step: u64) -> Checkpoint {
        let gen = RandomIdGenerator;
        Checkpoint {
            id: CheckpointId::new_v7(&gen),
            attempt_id: AttemptId::new_v7(&gen),
            step,
            state,
            digest: "sha256:be4e4dc1f1e4907ebc4040e2a6c2ebcba6bf79cc8211367a3aceedb760503840"
                .to_string(),
            manifest_digest:
                "sha256:05b3abf2579a5eb66403cd78be557fd860633a1fe2103c7642030defe32c657f"
                    .to_string(),
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
    fn coordinate_rendezvous_marks_job_running() {
        let (mut job, attempt) = make_ddp_job(4);
        let mut r = Rendezvous::new(attempt.id, 4).unwrap();
        for rank in &attempt.ranks {
            r.join(rank.id, "10.0.0.1:1111").unwrap();
        }
        coordinate_rendezvous(&mut r, &mut job, attempt.fencing_token).unwrap();
        assert!(matches!(
            job.state,
            moqentra_domain::training::TrainingJobState::Running
        ));
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
    fn recovery_planner_selects_latest_compatible() {
        let policy = RecoveryPolicy {
            max_elastic_restarts: 3,
            allowed_world_sizes: vec![2, 4],
            compatible_framework_versions: vec!["pytorch-2.4".to_string()],
            preserve_optimizer: true,
        };
        let old = make_checkpoint(CheckpointState::Complete, 4, 5);
        let latest = make_checkpoint(CheckpointState::Complete, 4, 10);
        let selected = policy.select_latest_compatible(&[old, latest.clone()]).unwrap();
        assert_eq!(selected.id, latest.id);
    }

    #[test]
    fn distributed_job_requires_all_ranks_succeed_and_rejects_stale_token() {
        let gen = RandomIdGenerator;
        let attempt_id = AttemptId::new_v7(&gen);
        let mut state = DistributedJobState::new(attempt_id, 7, 2).unwrap();
        let r1 = RankId::new_v7(&gen);
        let r2 = RankId::new_v7(&gen);
        state.report_exit(r1, 7, 0).unwrap();
        assert!(!state.all_succeeded());
        assert!(state.report_exit(r2, 99, 1).is_err()); // stale token
        state.report_exit(r2, 7, 1).unwrap();
        assert!(!state.all_succeeded());
        assert!(state.any_failed());
    }

    #[test]
    fn ddp_launcher_produces_torchrun_argv() {
        let (job, attempt) = make_ddp_job(4);
        let launcher = DdpLauncher::new(&job, &attempt, "moqentra-rdzv:29500", 3).unwrap();
        assert_eq!(launcher.nnodes, 2);
        assert_eq!(launcher.nproc_per_node, 2);
        assert_eq!(launcher.world_size, 4);
        assert_eq!(launcher.backend, "nccl");

        let argv = launcher.torchrun_argv();
        assert_eq!(argv[0], "torchrun");
        assert!(argv.windows(2).any(|w| w == ["--nnodes", "2"]));
        assert!(argv.windows(2).any(|w| w == ["--nproc_per_node", "2"]));
        assert!(argv.windows(2).any(|w| w[0] == "--rdzv_id"));
        assert!(argv.windows(2).any(|w| w == ["--rdzv_backend", "c10d"]));
        assert!(argv.windows(2).any(|w| w == ["--rdzv_endpoint", "moqentra-rdzv:29500"]));
        assert!(argv.windows(2).any(|w| w == ["--max_restarts", "3"]));
        assert!(argv.ends_with(&[
            "train.py".to_string(),
            "--lr".to_string(),
            "0.01".to_string()
        ]));
    }

    #[test]
    fn ddp_environment_reserves_keys_and_overrides_user_env() {
        let (job, attempt) = make_ddp_job(4);
        let launcher = DdpLauncher::new(&job, &attempt, "headless:29500", 0).unwrap();
        let mut user_env = BTreeMap::new();
        user_env.insert("RANK".to_string(), "99".to_string());
        user_env.insert("USER_KEY".to_string(), "keep".to_string());

        let env = launcher.environment_for_rank(&job, &attempt, 3, 1, "node-a", &user_env);
        assert_eq!(env.get("RANK").unwrap(), "3");
        assert_eq!(env.get("LOCAL_RANK").unwrap(), "1");
        assert_eq!(env.get("WORLD_SIZE").unwrap(), "4");
        assert_eq!(env.get("MOQENTRA_FENCING_TOKEN").unwrap(), "7");
        assert_eq!(env.get("MASTER_ADDR").unwrap(), "headless");
        assert_eq!(env.get("MASTER_PORT").unwrap(), "29500");
        assert_eq!(env.get("USER_KEY").unwrap(), "keep");
    }

    #[test]
    fn failure_classifier_detects_oom_device_and_user_code() {
        assert_eq!(FailureClassifier::classify(137, &[]), FailureClass::Oom);
        assert_eq!(FailureClassifier::classify(139, &[]), FailureClass::Device);
        assert_eq!(FailureClassifier::classify(1, &[]), FailureClass::UserCode);
        assert_eq!(
            FailureClassifier::classify(126, &[]),
            FailureClass::UserCode
        );
        assert_eq!(
            FailureClassifier::classify(124, &[]),
            FailureClass::Scheduler
        );
        assert_eq!(FailureClassifier::classify(-9, &[]), FailureClass::Oom);
        assert_eq!(FailureClassifier::classify(-11, &[]), FailureClass::Device);
        assert_eq!(
            FailureClassifier::classify(1, &["OOM".to_string()]),
            FailureClass::UserCode
        );
        assert_eq!(
            FailureClassifier::classify(255, &["CUDA out of memory".to_string()]),
            FailureClass::Oom
        );
    }

    #[test]
    fn recovery_planner_limits_attempts() {
        let (mut job, attempt) = make_ddp_job(4);
        // Consume all attempts.
        let _ = attempt;
        for _ in 0..job.spec.max_attempts {
            job.attempts.push(AttemptId::new_v7(&RandomIdGenerator));
        }

        let policy = RecoveryPolicy {
            max_elastic_restarts: 3,
            allowed_world_sizes: vec![4],
            compatible_framework_versions: vec!["pytorch-2.4".to_string()],
            preserve_optimizer: true,
        };
        let decision =
            RecoveryPlanner::decide(&job, &[], &policy, Some(FailureClass::Oom)).unwrap();
        assert!(matches!(decision, RecoveryDecision::Fail(_)));
    }

    #[test]
    fn recovery_planner_warms_start_from_latest_checkpoint() {
        let (job, _) = make_ddp_job(4);
        let ckpt = make_checkpoint(CheckpointState::Complete, 4, 7);
        let policy = RecoveryPolicy {
            max_elastic_restarts: 3,
            allowed_world_sizes: vec![4],
            compatible_framework_versions: vec!["pytorch-2.4".to_string()],
            preserve_optimizer: true,
        };
        let decision = RecoveryPlanner::decide(
            &job,
            std::slice::from_ref(&ckpt),
            &policy,
            Some(FailureClass::Oom),
        )
        .unwrap();
        assert!(matches!(decision, RecoveryDecision::RetryFromCheckpoint(c) if c.id == ckpt.id));
    }
}
