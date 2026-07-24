//! Kubernetes/Volcano scheduling policy and execution plan compiler.

use moqentra_domain::queue::{PriorityClass, QueueDecision, QueuePolicy};
use moqentra_domain::training::{ResourceRequest, TrainingJob};
use moqentra_types::{AttemptId, QueuePolicyId, TenantId};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Scheduling queue with fairness and tenant/project quotas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulingQueue {
    pub name: String,
    pub tenant_id: TenantId,
    pub max_jobs: usize,
    pub priority_classes: BTreeMap<String, PriorityClass>,
    pub entries: VecDeque<QueueEntry>,
    pub tenant_quota: u64,
    pub project_quota: BTreeMap<String, u64>,
    pub queue_policy: Option<QueuePolicy>,
    pub resource_snapshot: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry {
    pub job_id: String,
    /// Tenant that owns the job (opaque string form of TenantId).
    pub tenant_id: String,
    pub project_id: String,
    /// Numeric priority used when no priority class is set.
    pub priority: u32,
    /// Optional named priority class.
    pub priority_class_name: Option<String>,
    /// Resource request used for dominant-resource fair sharing.
    pub resource_request: Option<ResourceRequest>,
    pub submitted_at: moqentra_types::UtcTimestamp,
    /// Number of failed admit/dispatch attempts.
    pub retry_count: u32,
    /// Earliest instant at which this entry may be popped again after a failure.
    pub next_attempt_at: Option<moqentra_types::UtcTimestamp>,
}

impl QueueEntry {
    pub fn new(
        job_id: impl Into<String>,
        tenant_id: impl Into<String>,
        project_id: impl Into<String>,
        priority: u32,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            tenant_id: tenant_id.into(),
            project_id: project_id.into(),
            priority,
            priority_class_name: None,
            resource_request: None,
            submitted_at: moqentra_types::UtcTimestamp::now(),
            retry_count: 0,
            next_attempt_at: None,
        }
    }

    pub fn with_priority_class(mut self, name: impl Into<String>) -> Self {
        self.priority_class_name = Some(name.into());
        self
    }

    pub fn with_resource_request(mut self, request: ResourceRequest) -> Self {
        self.resource_request = Some(request);
        self
    }
}

impl SchedulingQueue {
    pub fn new(
        name: impl Into<String>,
        tenant_id: TenantId,
        max_jobs: usize,
        tenant_quota: u64,
    ) -> Self {
        Self {
            name: name.into(),
            tenant_id,
            max_jobs,
            priority_classes: BTreeMap::new(),
            entries: VecDeque::new(),
            tenant_quota,
            project_quota: BTreeMap::new(),
            queue_policy: None,
            resource_snapshot: None,
        }
    }

    pub fn enqueue(&mut self, entry: QueueEntry) -> Result<(), moqentra_types::Error> {
        if self.entries.len() >= self.max_jobs {
            return Err(moqentra_types::Error::unavailable("queue full"));
        }
        let tenant_count = u64::try_from(self.entries.len()).unwrap_or(u64::MAX);
        if tenant_count >= self.tenant_quota {
            return Err(moqentra_types::Error::unavailable("tenant quota exceeded"));
        }
        let project_count =
            u64::try_from(self.entries.iter().filter(|e| e.project_id == entry.project_id).count())
                .unwrap_or(u64::MAX);
        let quota = self.project_quota.get(&entry.project_id).copied().unwrap_or(u64::MAX);
        if project_count >= quota {
            return Err(moqentra_types::Error::unavailable("project quota exceeded"));
        }
        self.entries.push_back(entry);
        Ok(())
    }

    /// Select and remove the best entry using priority class + aging + stable tiebreakers.
    pub fn pop(&mut self) -> Option<QueueEntry> {
        let now = moqentra_types::UtcTimestamp::now();
        let mut best_idx = 0usize;
        let mut best_score = None;
        for (idx, e) in self.entries.iter().enumerate() {
            if e.next_attempt_at.is_some_and(|t| t > now) {
                continue;
            }
            let score = self.score(e, now);
            if best_score.as_ref().is_none_or(|s| score > *s) {
                best_score = Some(score);
                best_idx = idx;
            }
        }
        best_score?;
        self.entries.remove(best_idx)
    }

    /// Select the best entry and return an auditable [QueueDecision].
    pub fn pop_decision(
        &mut self,
        queue_id: QueuePolicyId,
        policy_revision: u64,
    ) -> Option<(QueueEntry, QueueDecision)> {
        let candidate_ids: Vec<_> = self.entries.iter().map(|e| e.job_id.clone()).collect();
        let selected = self.pop()?;
        let decision = QueueDecision {
            queue_id,
            policy_revision,
            resource_snapshot: self.resource_snapshot.clone().unwrap_or_default(),
            candidate_ids,
            selected_id: selected.job_id.clone(),
            explanation: self.explain_selection(&selected),
            created_at: moqentra_types::UtcTimestamp::now(),
        };
        Some((selected, decision))
    }

    fn score(
        &self,
        entry: &QueueEntry,
        now: moqentra_types::UtcTimestamp,
    ) -> (i32, u64, Reverse<moqentra_types::UtcTimestamp>, String) {
        let class = entry.priority_class_name.as_ref().and_then(|n| self.priority_classes.get(n));
        let class_priority = class.map_or(i32::try_from(entry.priority).unwrap_or(i32::MAX), |c| {
            c.priority
        });
        let max_wait = class.map_or(3600u64, |c| u64::from(c.max_wait_seconds));
        let wait_millis = now.unix_millis().saturating_sub(entry.submitted_at.unix_millis());
        let wait_secs = u64::try_from(wait_millis / 1000).unwrap_or(0);
        let aging = wait_secs.min(max_wait);
        (
            class_priority,
            aging,
            Reverse(entry.submitted_at),
            entry.job_id.clone(),
        )
    }

    fn explain_selection(&self, entry: &QueueEntry) -> String {
        let class = entry.priority_class_name.as_ref().and_then(|n| self.priority_classes.get(n));
        if let Some(c) = class {
            format!(
                "selected job {} with priority class {} (priority {}, preemptible {})",
                entry.job_id, c.name, c.priority, c.preemptible
            )
        } else {
            format!(
                "selected job {} with numeric priority {}",
                entry.job_id, entry.priority
            )
        }
    }

    /// Maximum admit retries before the entry is dropped (dead-letter).
    pub const MAX_ADMIT_RETRIES: u32 = 8;

    /// Re-enqueue after a failed admit with exponential backoff, or drop when exhausted.
    pub fn requeue_with_backoff(
        &mut self,
        mut entry: QueueEntry,
    ) -> Result<bool, moqentra_types::Error> {
        entry.retry_count = entry.retry_count.saturating_add(1);
        if entry.retry_count > Self::MAX_ADMIT_RETRIES {
            return Ok(false);
        }
        let backoff_secs = 1i64 << entry.retry_count.min(6);
        entry.next_attempt_at =
            moqentra_types::UtcTimestamp::now().add_duration(time::Duration::seconds(backoff_secs));
        self.enqueue(entry)?;
        Ok(true)
    }
}

/// Gang scheduling group (Volcano PodGroup abstraction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GangGroup {
    pub name: String,
    pub min_available: u32,
    pub total_members: u32,
    pub topology: String,
}

/// Accelerator capability normalized across NVIDIA/AMD/Ascend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceleratorCapability {
    pub kind: String,
    pub memory_mib: u64,
    pub compute_units: u32,
    pub supported_collectives: Vec<String>,
    pub runtime_labels: BTreeMap<String, String>,
}

/// Execution plan compiled from a TrainingJobSpec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub attempt_id: AttemptId,
    pub namespace: String,
    pub labels: BTreeMap<String, String>,
    pub replicas: u32,
    pub resource_request: ResourceRequest,
    pub gang_group: GangGroup,
    pub volumes: Vec<VolumeSpec>,
    pub network_policy: NetworkPolicySpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeSpec {
    pub name: String,
    pub mount_path: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkPolicySpec {
    pub allowed_namespaces: Vec<String>,
}

/// Scheduler compiler.
pub struct PlanCompiler;

impl PlanCompiler {
    pub fn compile(
        job: &TrainingJob,
        attempt_id: AttemptId,
    ) -> Result<ExecutionPlan, moqentra_types::Error> {
        job.spec.validate()?;
        let spec = &job.spec;
        if spec.resources.replicas == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "replicas must be > 0",
            ));
        }
        if spec.resources.cpu_milli == 0 || spec.resources.memory_mib == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "cpu_milli and memory_mib must be greater than zero",
            ));
        }
        let mut labels = BTreeMap::new();
        labels.insert("tenant".to_string(), job.tenant_id.to_string());
        labels.insert("project".to_string(), job.project_id.to_string());
        labels.insert("job".to_string(), job.id.to_string());
        labels.insert("attempt".to_string(), attempt_id.to_string());

        let replicas = spec.resources.replicas;
        let world_size = spec.world_size();
        let gang = match spec.distributed {
            moqentra_domain::training::DistributedConfig::Single => GangGroup {
                name: format!("gang-{}", attempt_id),
                min_available: 1,
                total_members: 1,
                topology: "none".to_string(),
            },
            moqentra_domain::training::DistributedConfig::Ddp { .. } => {
                if world_size == 0 {
                    return Err(moqentra_types::Error::invalid_argument(
                        "ddp world_size must be > 0",
                    ));
                }
                GangGroup {
                    name: format!("gang-{}", attempt_id),
                    min_available: world_size,
                    total_members: world_size,
                    topology: spec.resources.topology.clone().unwrap_or_default(),
                }
            }
        };

        Ok(ExecutionPlan {
            attempt_id,
            namespace: format!("tenant-{}", job.tenant_id),
            labels,
            replicas,
            resource_request: spec.resources.clone(),
            gang_group: gang,
            volumes: vec![VolumeSpec {
                name: "dataset".to_string(),
                mount_path: "/data".to_string(),
                source: spec.dataset_version_id.to_string(),
            }],
            network_policy: NetworkPolicySpec {
                allowed_namespaces: vec!["control-plane".to_string()],
            },
        })
    }
}

/// Cluster topology for matching capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterTopology {
    pub nodes: BTreeMap<String, NodeCapacity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeCapacity {
    pub cpu_cores: u32,
    pub memory_mib: u64,
    pub accelerators: Vec<AcceleratorCapability>,
    pub taints: BTreeSet<String>,
    pub labels: BTreeMap<String, String>,
    pub healthy: bool,
    pub observed_at: moqentra_types::UtcTimestamp,
}

impl NodeCapacity {
    pub fn new(cpu_cores: u32, memory_mib: u64, accelerators: Vec<AcceleratorCapability>) -> Self {
        Self {
            cpu_cores,
            memory_mib,
            accelerators,
            taints: BTreeSet::new(),
            labels: BTreeMap::new(),
            healthy: true,
            observed_at: moqentra_types::UtcTimestamp::now(),
        }
    }

    pub fn with_taint(mut self, taint: impl Into<String>) -> Self {
        self.taints.insert(taint.into());
        self
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    pub fn unhealthy(mut self) -> Self {
        self.healthy = false;
        self
    }
}

impl ClusterTopology {
    pub fn find_placement(
        &self,
        request: &ResourceRequest,
    ) -> Result<String, moqentra_types::Error> {
        let total_cpu_milli = u64::from(request.replicas)
            .checked_mul(request.cpu_milli)
            .ok_or_else(|| moqentra_types::Error::invalid_argument("cpu request overflow"))?;
        let total_memory_mib = u64::from(request.replicas)
            .checked_mul(request.memory_mib)
            .ok_or_else(|| moqentra_types::Error::invalid_argument("memory request overflow"))?;
        for (name, node) in &self.nodes {
            if !node.healthy {
                continue;
            }
            if u64::from(node.cpu_cores).checked_mul(1000).is_none_or(|c| c < total_cpu_milli) {
                continue;
            }
            if node.memory_mib < total_memory_mib {
                continue;
            }
            if request.accelerator_count > 0 {
                let available = u64::try_from(
                    node.accelerators
                        .iter()
                        .filter(|a| {
                            a.kind == request.accelerator_kind.as_deref().unwrap_or("")
                                && !node.taints.contains(&format!("no-{}", a.kind))
                        })
                        .count(),
                )
                .map_err(|_| moqentra_types::Error::internal("accelerator count overflow"))?;
                let needed = u64::from(request.accelerator_count)
                    .checked_mul(u64::from(request.replicas))
                    .ok_or_else(|| {
                        moqentra_types::Error::invalid_argument("accelerator request overflow")
                    })?;
                if available < needed {
                    continue;
                }
            }
            return Ok(name.clone());
        }
        Err(moqentra_types::Error::unavailable("no matching node"))
    }

    /// Total cluster capacity across healthy nodes, dimension order: CPU milli, memory MiB,
    /// accelerator count.
    pub fn total_capacity(&self) -> (u64, u64, u64) {
        let mut cpu: u64 = 0;
        let mut mem: u64 = 0;
        let mut accel: u64 = 0;
        for node in self.nodes.values().filter(|n| n.healthy) {
            cpu = cpu.saturating_add(u64::from(node.cpu_cores).saturating_mul(1000));
            mem = mem.saturating_add(node.memory_mib);
            accel = accel.saturating_add(u64::try_from(node.accelerators.len()).unwrap_or(0));
        }
        (cpu, mem, accel)
    }
}

/// Weighted dominant-resource fair-share selection across tenants.
///
/// `tenant_usage` maps each tenant to a vector of used resources with the same
/// dimension order as `capacity`. `weights` maps a tenant to a scheduling weight
/// (default 1). The tenant with the smallest weighted maximum share is returned,
/// preventing a single tenant from monopolizing a scarce resource.
pub fn pick_tenant_drf<T: Ord + Clone>(
    tenants: &[T],
    tenant_usage: &BTreeMap<T, Vec<u64>>,
    capacity: &[u64],
    weights: &BTreeMap<T, u32>,
) -> Option<T> {
    if capacity.is_empty() || capacity.contains(&0) {
        return None;
    }
    let mut best: Option<(u64, T)> = None;
    for t in tenants {
        let usage = tenant_usage.get(t).cloned().unwrap_or_else(|| vec![0u64; capacity.len()]);
        if usage.len() != capacity.len() {
            continue;
        }
        let weight = u64::from(weights.get(t).copied().unwrap_or(1).max(1));
        let mut dominant = 0u64;
        for (u, c) in usage.iter().zip(capacity.iter()) {
            let share = u.saturating_mul(weight) / c;
            dominant = dominant.max(share);
        }
        if best.as_ref().is_none_or(|(score, _)| dominant < *score) {
            best = Some((dominant, t.clone()));
        }
    }
    best.map(|(_, t)| t)
}

/// Watcher event with resourceVersion for resume and revision idempotency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchEvent {
    pub resource_version: String,
    pub revision: u64,
    pub kind: String,
    pub name: String,
    pub payload: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::training::{DistributedConfig, TrainingJob, TrainingJobSpec};
    use moqentra_types::{
        DatasetVersionId, ExperimentId, ProjectId, QueuePolicyId, RandomIdGenerator, TenantId,
        TrainingJobId,
    };

    fn make_job(replicas: u32, kind: Option<&str>) -> TrainingJob {
        let gen = RandomIdGenerator;
        let spec = TrainingJobSpec {
            code_digest: "sha256:5694d08a2e53ffcae0c3103e5ad6f6076abd960eb1f8a56577040bc1028f702b"
                .to_string(),
            image_digest: "sha256:6105d6cc76af400325e94d588ce511be5bfdbb73b437dc51eca43917d7a43e3d"
                .to_string(),
            dataset_version_id: DatasetVersionId::new_v7(&gen),
            hyperparameters: moqentra_domain::training::ParameterSchema {
                argv: vec!["train.py".to_string()],
                env: BTreeMap::new(),
                config_files: BTreeMap::new(),
            },
            seed: 1,
            resources: ResourceRequest {
                replicas,
                cpu_milli: 1000,
                memory_mib: 8192,
                ephemeral_storage_mib: 1024,
                accelerator_kind: kind.map(|s| s.to_string()),
                accelerator_count: if kind.is_some() { 1 } else { 0 },
                topology: None,
            },
            distributed: if replicas > 1 {
                DistributedConfig::Ddp {
                    world_size: replicas,
                }
            } else {
                DistributedConfig::Single
            },
            processes_per_replica: 1,
            checkpoint_policy: Default::default(),
            queue_ref: None,
            priority_class_ref: None,
            preemption_policy: Default::default(),
            resource_class_ref: None,
            max_attempts: 1,
            deadline_seconds: 3600,
        };
        TrainingJob::new(
            TrainingJobId::new_v7(&gen),
            ExperimentId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            spec,
        )
        .unwrap()
    }

    #[test]
    fn compile_rejects_zero_replicas() {
        let mut job = make_job(1, None);
        job.spec.resources.replicas = 0;
        let gen = RandomIdGenerator;
        let plan = PlanCompiler::compile(&job, AttemptId::new_v7(&gen));
        assert!(plan.is_err());
    }

    #[test]
    fn compile_ddp_gang() {
        let job = make_job(4, Some("nvidia"));
        let gen = RandomIdGenerator;
        let plan = PlanCompiler::compile(&job, AttemptId::new_v7(&gen)).unwrap();
        assert_eq!(plan.gang_group.total_members, 4);
        assert_eq!(plan.gang_group.min_available, 4);
        assert!(plan.labels.contains_key("tenant"));
    }

    #[test]
    fn queue_priority_and_quota() {
        let gen = RandomIdGenerator;
        let mut queue = SchedulingQueue::new("default", TenantId::new_v7(&gen), 2, 10);
        let e1 = QueueEntry::new("job-low", "tenant-1", "p1", 1);
        let mut e2 = e1.clone();
        e2.job_id = "job-high".to_string();
        e2.priority = 10;
        queue.enqueue(e1).unwrap();
        queue.enqueue(e2).unwrap();
        assert!(queue.enqueue(QueueEntry::new("job-full", "tenant-1", "p1", 5)).is_err());
        let popped = queue.pop().unwrap();
        assert_eq!(popped.job_id, "job-high");
    }

    #[test]
    fn queue_backoff_skips_not_ready_and_dead_letters() {
        let gen = RandomIdGenerator;
        let mut queue = SchedulingQueue::new("default", TenantId::new_v7(&gen), 10, 10);
        let mut entry = QueueEntry::new("job-1", "tenant-1", "p1", 1);
        entry.next_attempt_at =
            moqentra_types::UtcTimestamp::now().add_duration(time::Duration::seconds(3600));
        queue.enqueue(entry).unwrap();
        assert!(queue.pop().is_none());

        let mut failed = QueueEntry::new("job-2", "tenant-1", "p1", 1);
        failed.retry_count = SchedulingQueue::MAX_ADMIT_RETRIES;
        let kept = queue.requeue_with_backoff(failed).unwrap();
        assert!(!kept);
        assert!(queue.entries.iter().all(|e| e.job_id != "job-2"));
    }

    #[test]
    fn placement_matches_accelerator_and_taints() {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "node-a".to_string(),
            NodeCapacity::new(
                8,
                32768,
                vec![AcceleratorCapability {
                    kind: "nvidia".to_string(),
                    memory_mib: 24576,
                    compute_units: 80,
                    supported_collectives: vec!["nccl".to_string()],
                    runtime_labels: BTreeMap::new(),
                }],
            ),
        );
        nodes.insert(
            "node-b".to_string(),
            NodeCapacity::new(8, 32768, vec![]).with_taint("no-nvidia"),
        );
        let topology = ClusterTopology { nodes };
        let request = ResourceRequest {
            replicas: 1,
            cpu_milli: 1000,
            memory_mib: 8192,
            ephemeral_storage_mib: 1024,
            accelerator_kind: Some("nvidia".to_string()),
            accelerator_count: 1,
            topology: None,
        };
        assert_eq!(topology.find_placement(&request).unwrap(), "node-a");
    }

    #[test]
    fn unhealthy_node_is_skipped() {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "node-healthy".to_string(),
            NodeCapacity::new(8, 32768, vec![]),
        );
        nodes.insert(
            "node-unhealthy".to_string(),
            NodeCapacity::new(16, 65536, vec![]).unhealthy(),
        );
        let topology = ClusterTopology { nodes };
        let request = ResourceRequest {
            replicas: 1,
            cpu_milli: 1000,
            memory_mib: 8192,
            ephemeral_storage_mib: 1024,
            accelerator_kind: None,
            accelerator_count: 0,
            topology: None,
        };
        assert_eq!(topology.find_placement(&request).unwrap(), "node-healthy");
    }

    #[test]
    fn queue_priority_class_outranks_numeric_priority() {
        let gen = RandomIdGenerator;
        let _queue_id = QueuePolicyId::new_v7(&gen);
        let tenant = TenantId::new_v7(&gen);
        let mut queue = SchedulingQueue::new("default", tenant, 10, 10);
        queue.priority_classes.insert(
            "production".to_string(),
            PriorityClass {
                id: moqentra_types::PriorityClassId::new_v7(&gen),
                tenant_id: tenant,
                project_id: None,
                name: "production".to_string(),
                priority: 100,
                preemptible: false,
                max_wait_seconds: 3600,
                revision: 1,
                created_at: moqentra_types::UtcTimestamp::now(),
            },
        );
        let low = QueueEntry::new("job-low", "t1", "p1", 1);
        let high = QueueEntry::new("job-high", "t1", "p1", 1).with_priority_class("production");
        queue.enqueue(low).unwrap();
        queue.enqueue(high).unwrap();
        let popped = queue.pop().unwrap();
        assert_eq!(popped.job_id, "job-high");
    }

    #[test]
    fn pop_decision_records_explanation() {
        let gen = RandomIdGenerator;
        let queue_id = QueuePolicyId::new_v7(&gen);
        let tenant = TenantId::new_v7(&gen);
        let mut queue = SchedulingQueue::new("default", tenant, 10, 10);
        queue.resource_snapshot = Some("cpu=8000m,mem=64Gi".to_string());
        queue.enqueue(QueueEntry::new("job-1", "t1", "p1", 1)).unwrap();
        let (entry, decision) = queue.pop_decision(queue_id, 1).unwrap();
        assert_eq!(entry.job_id, "job-1");
        assert_eq!(decision.selected_id, "job-1");
        assert!(!decision.explanation.is_empty());
        assert_eq!(decision.resource_snapshot, "cpu=8000m,mem=64Gi");
    }

    #[test]
    fn drf_selects_lowest_max_share_tenant() {
        let tenants = vec!["tenant-a".to_string(), "tenant-b".to_string()];
        let mut usage = BTreeMap::new();
        // Dimension order: cpu, memory, accel.
        usage.insert("tenant-a".to_string(), vec![1000u64, 8192, 0]);
        usage.insert("tenant-b".to_string(), vec![1000u64, 8192, 1]);
        let capacity = vec![2000u64, 16384, 1];
        let mut weights = BTreeMap::new();
        weights.insert("tenant-a".to_string(), 1u32);
        weights.insert("tenant-b".to_string(), 1u32);
        let selected = pick_tenant_drf(&tenants, &usage, &capacity, &weights);
        // tenant-b dominates the single accelerator and has higher max share.
        assert_eq!(selected, Some("tenant-a".to_string()));
    }
}
