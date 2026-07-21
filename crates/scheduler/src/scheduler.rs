//! Kubernetes/Volcano scheduling policy and execution plan compiler.

use moqentra_domain::training::{ResourceRequest, TrainingJob};
use moqentra_types::{AttemptId, TenantId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Scheduling queue with fairness and tenant/project quotas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulingQueue {
    pub name: String,
    pub tenant_id: TenantId,
    pub max_jobs: usize,
    pub priority_weights: BTreeMap<String, u32>,
    pub entries: VecDeque<QueueEntry>,
    pub tenant_quota: u64,
    pub project_quota: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry {
    pub job_id: String,
    pub project_id: String,
    pub priority: u32,
    pub submitted_at: moqentra_types::UtcTimestamp,
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
            priority_weights: BTreeMap::new(),
            entries: VecDeque::new(),
            tenant_quota,
            project_quota: BTreeMap::new(),
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

    pub fn pop(&mut self) -> Option<QueueEntry> {
        let mut best_idx = 0usize;
        let mut best_score = None;
        for (idx, e) in self.entries.iter().enumerate() {
            let score = (u64::from(e.priority), std::cmp::Reverse(e.submitted_at));
            if best_score.as_ref().is_none_or(|s: &(u64, _)| score > *s) {
                best_score = Some(score);
                best_idx = idx;
            }
        }
        best_score?;
        self.entries.remove(best_idx)
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
        let gang = match spec.distributed {
            moqentra_domain::training::DistributedConfig::Single => GangGroup {
                name: format!("gang-{}", attempt_id),
                min_available: 1,
                total_members: 1,
                topology: "none".to_string(),
            },
            moqentra_domain::training::DistributedConfig::Ddp { world_size } => {
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
        DatasetVersionId, ExperimentId, ProjectId, RandomIdGenerator, TenantId, TrainingJobId,
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
        let e1 = QueueEntry {
            job_id: "job-low".to_string(),
            project_id: "p1".to_string(),
            priority: 1,
            submitted_at: moqentra_types::UtcTimestamp::now(),
        };
        let mut e2 = e1.clone();
        e2.job_id = "job-high".to_string();
        e2.priority = 10;
        queue.enqueue(e1).unwrap();
        queue.enqueue(e2).unwrap();
        assert!(queue
            .enqueue(QueueEntry {
                job_id: "job-full".to_string(),
                project_id: "p1".to_string(),
                priority: 5,
                submitted_at: moqentra_types::UtcTimestamp::now(),
            })
            .is_err());
        let popped = queue.pop().unwrap();
        assert_eq!(popped.job_id, "job-high");
    }

    #[test]
    fn placement_matches_accelerator_and_taints() {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "node-a".to_string(),
            NodeCapacity {
                cpu_cores: 8,
                memory_mib: 32768,
                accelerators: vec![AcceleratorCapability {
                    kind: "nvidia".to_string(),
                    memory_mib: 24576,
                    compute_units: 80,
                    supported_collectives: vec!["nccl".to_string()],
                    runtime_labels: BTreeMap::new(),
                }],
                taints: BTreeSet::new(),
                labels: BTreeMap::new(),
            },
        );
        nodes.insert(
            "node-b".to_string(),
            NodeCapacity {
                cpu_cores: 8,
                memory_mib: 32768,
                accelerators: vec![],
                taints: ["no-nvidia".to_string()].iter().cloned().collect(),
                labels: BTreeMap::new(),
            },
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
}
