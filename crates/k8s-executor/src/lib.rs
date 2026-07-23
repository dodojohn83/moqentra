//! Kubernetes and Volcano execution adapter for Moqentra training jobs.
//!
//! This crate compiles `TrainingJobSpec` into Kubernetes Job/VolcanoJob objects,
//! generates per-tenant namespace security manifests, and provides an in-memory
//! executor for unit testing. A real `kube`-based client can later implement the
//! same `K8sExecutor` trait.

use async_trait::async_trait;
use moqentra_domain::training::{Attempt, TrainingJob};
use moqentra_types::{AttemptId, ProjectId, TenantId, TrainingJobId};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::sleep;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct K8sCluster {
    pub server_version: String,
    pub supported_versions: Vec<String>,
    pub namespace: String,
    pub gpu_resources: Vec<String>,
    pub volcano_installed: bool,
    pub storage_classes: Vec<String>,
    pub healthy: bool,
    pub diagnostics: Vec<String>,
}

impl K8sCluster {
    pub fn baseline_ok(&self) -> bool {
        self.healthy
            && self.supported_versions.contains(&"v1.30".to_string())
            && !self.storage_classes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterCapability {
    pub version: String,
    pub gpu_available: bool,
    pub volcano_available: bool,
    pub storage_ready: bool,
}

/// Result of a discovery pass.
pub async fn discover_cluster() -> K8sCluster {
    // In a real implementation this would query /version, list CRDs, etc.
    K8sCluster {
        server_version: "v1.30.0+k3s1".to_string(),
        supported_versions: vec![
            "v1.30".to_string(),
            "v1.31".to_string(),
            "v1.32".to_string(),
        ],
        namespace: "moqentra".to_string(),
        gpu_resources: vec!["nvidia.com/gpu".to_string()],
        volcano_installed: true,
        storage_classes: vec!["local-path".to_string()],
        healthy: true,
        diagnostics: Vec::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct K8sWorkload {
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub training_job_id: TrainingJobId,
    pub attempt_id: AttemptId,
    pub namespace: String,
    pub name: String,
    pub kind: String,
    pub manifest: serde_json::Value,
    pub desired_state: String,
    pub observed_state: String,
    pub resource_version: u64,
    pub generation: u64,
    #[serde(skip)]
    pub lease_until: Option<Instant>,
    pub diagnostics: Vec<String>,
    pub owner_instance: String,
}

/// Executor interface for a Kubernetes-backed runtime.
#[async_trait]
pub trait K8sExecutor: Send + Sync {
    async fn discover(&self) -> K8sCluster;
    async fn apply(&self, workload: K8sWorkload) -> Result<(), moqentra_types::Error>;
    async fn delete(&self, namespace: &str, name: &str) -> Result<(), moqentra_types::Error>;
    async fn get(&self, namespace: &str, name: &str) -> Result<K8sWorkload, moqentra_types::Error>;
    async fn list(
        &self,
        namespace: &str,
        label_selector: &str,
    ) -> Result<Vec<K8sWorkload>, moqentra_types::Error>;
    async fn watch(
        &self,
        namespace: &str,
        label_selector: &str,
        resource_version: u64,
        timeout: Duration,
    ) -> Result<(Vec<K8sWorkload>, u64), moqentra_types::Error>;
    fn instance_id(&self) -> String;
}

/// In-memory executor used for unit testing.
pub struct InMemoryK8sExecutor {
    instance_id: String,
    workloads: Arc<RwLock<HashMap<String, K8sWorkload>>>,
    counter: AtomicU64,
}

impl Default for InMemoryK8sExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryK8sExecutor {
    pub fn new() -> Self {
        Self {
            instance_id: uuid::Uuid::new_v4().to_string(),
            workloads: Arc::new(RwLock::new(HashMap::new())),
            counter: AtomicU64::new(1),
        }
    }

    fn key(namespace: &str, name: &str) -> String {
        format!("{}/{}", namespace, name)
    }
}

#[async_trait]
impl K8sExecutor for InMemoryK8sExecutor {
    async fn discover(&self) -> K8sCluster {
        discover_cluster().await
    }

    async fn apply(&self, mut workload: K8sWorkload) -> Result<(), moqentra_types::Error> {
        let key = Self::key(&workload.namespace, &workload.name);
        let mut map = self.workloads.write().await;
        workload.resource_version = self.counter.fetch_add(1, Ordering::SeqCst);
        workload.generation += 1;
        if workload.lease_until.is_none() {
            workload.lease_until = Some(Instant::now() + Duration::from_secs(600));
        }
        map.insert(key, workload);
        Ok(())
    }

    async fn delete(&self, namespace: &str, name: &str) -> Result<(), moqentra_types::Error> {
        let key = Self::key(namespace, name);
        let mut map = self.workloads.write().await;
        let work = map.get(&key).ok_or_else(|| moqentra_types::Error::not_found("workload"))?;
        if work.lease_until.map(|t| t > Instant::now()).unwrap_or(false)
            && work.owner_instance != self.instance_id
        {
            return Err(moqentra_types::Error::permission_denied(
                "cannot delete workload owned by another instance with a valid lease",
            ));
        }
        map.remove(&key);
        Ok(())
    }

    async fn get(&self, namespace: &str, name: &str) -> Result<K8sWorkload, moqentra_types::Error> {
        let key = Self::key(namespace, name);
        let map = self.workloads.read().await;
        map.get(&key)
            .cloned()
            .ok_or_else(|| moqentra_types::Error::not_found("workload"))
    }

    async fn list(
        &self,
        namespace: &str,
        label_selector: &str,
    ) -> Result<Vec<K8sWorkload>, moqentra_types::Error> {
        let map = self.workloads.read().await;
        Ok(map
            .values()
            .filter(|w| {
                w.namespace == namespace && label_selector_match(&w.manifest, label_selector)
            })
            .cloned()
            .collect())
    }

    async fn watch(
        &self,
        namespace: &str,
        label_selector: &str,
        resource_version: u64,
        timeout: Duration,
    ) -> Result<(Vec<K8sWorkload>, u64), moqentra_types::Error> {
        let start = Instant::now();
        loop {
            let events = self.list(namespace, label_selector).await?;
            let rv = self.counter.load(Ordering::SeqCst);
            if rv > resource_version || start.elapsed() > timeout {
                return Ok((events, rv));
            }
            sleep(Duration::from_millis(50)).await;
        }
    }

    fn instance_id(&self) -> String {
        self.instance_id.clone()
    }
}

fn label_selector_match(manifest: &serde_json::Value, selector: &str) -> bool {
    let labels = manifest
        .get("metadata")
        .and_then(|m| m.get("labels"))
        .and_then(|l| l.as_object())
        .cloned()
        .unwrap_or_default();
    for part in selector.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((k, v)) = part.split_once('=') {
            if labels.get(k).and_then(|x| x.as_str()) != Some(v) {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

/// Compiler for Kubernetes Job and VolcanoJob objects.
pub struct JobCompiler;

impl JobCompiler {
    /// Compile a single-replica job into a Kubernetes Job manifest.
    pub fn compile_k8s_job(
        job: &TrainingJob,
        attempt: &Attempt,
        namespace: &str,
    ) -> Result<serde_json::Value, moqentra_types::Error> {
        Self::compile(job, attempt, namespace, "Job", false)
    }

    /// Compile a distributed job into a VolcanoJob manifest.
    pub fn compile_volcano_job(
        job: &TrainingJob,
        attempt: &Attempt,
        namespace: &str,
    ) -> Result<serde_json::Value, moqentra_types::Error> {
        Self::compile(job, attempt, namespace, "VolcanoJob", true)
    }

    pub fn compile(
        job: &TrainingJob,
        attempt: &Attempt,
        namespace: &str,
        _kind: &str,
        is_volcano: bool,
    ) -> Result<serde_json::Value, moqentra_types::Error> {
        let spec = &job.spec;
        let image = format!("moqentra/executor@{}", &spec.image_digest);
        let labels = labels(job, attempt);
        let deadline_seconds = spec.deadline_seconds as i64;
        let replicas = spec.resources.replicas.max(1);
        let world_size = spec.world_size();
        let processes_per_replica = spec.processes_per_replica.max(1);

        let mut env: Vec<serde_json::Value> = spec
            .hyperparameters
            .env
            .iter()
            .map(|(k, v)| json!({"name": k, "value": v}))
            .collect();
        env.push(json!({"name": "MOQENTRA_ATTEMPT_ID", "value": attempt.id.to_string()}));
        env.push(
            json!({"name": "MOQENTRA_FENCING_TOKEN", "value": attempt.fencing_token.to_string()}),
        );
        env.push(json!({"name": "MOQENTRA_TRAINING_JOB_ID", "value": job.id.to_string()}));
        env.push(json!({"name": "MOQENTRA_TENANT_ID", "value": job.tenant_id.to_string()}));
        env.push(json!({"name": "MOQENTRA_PROJECT_ID", "value": job.project_id.to_string()}));
        env.push(json!({"name": "MOQENTRA_SEED", "value": spec.seed.to_string()}));
        env.push(json!({"name": "MOQENTRA_WORLD_SIZE", "value": world_size.to_string()}));
        env.push(json!({"name": "MOQENTRA_NNODES", "value": replicas.to_string()}));
        env.push(
            json!({"name": "MOQENTRA_NPROC_PER_NODE", "value": processes_per_replica.to_string()}),
        );

        let command;
        if is_volcano && world_size > 1 {
            // DDP/Elastic: compile to torchrun argv and environment. Each pod receives the same
            // launcher env; torchrun assigns per-process RANK/LOCAL_RANK at runtime.
            let rdzv_endpoint = format!("moqentra-{}-rdzv:29500", sanitize(&job.id.to_string()));
            let launcher =
                moqentra_scheduler::distributed::DdpLauncher::new(job, attempt, &rdzv_endpoint, 0)
                    .map_err(|e| {
                        moqentra_types::Error::invalid_argument(format!("ddp launcher: {e}"))
                    })?;
            command = launcher.torchrun_argv();
            env = launcher
                .environment_for_rank(job, attempt, 0, 0, "node-0", &spec.hyperparameters.env)
                .into_iter()
                .filter(|(k, _)| k != "RANK" && k != "LOCAL_RANK")
                .map(|(k, v)| json!({"name": k, "value": v}))
                .collect();
        } else {
            command = if spec.hyperparameters.argv.is_empty() {
                vec![
                    "python".to_string(),
                    "-m".to_string(),
                    "moqentra_worker".to_string(),
                ]
            } else {
                spec.hyperparameters.argv.clone()
            };
        }

        let mut requests = serde_json::Map::new();
        requests.insert(
            "cpu".to_string(),
            json!(format!("{}m", spec.resources.cpu_milli)),
        );
        requests.insert(
            "memory".to_string(),
            json!(format!("{}Mi", spec.resources.memory_mib)),
        );
        requests.insert(
            "ephemeral-storage".to_string(),
            json!(format!("{}Mi", spec.resources.ephemeral_storage_mib)),
        );
        if spec.resources.accelerator_count > 0 {
            let kind = spec
                .resources
                .accelerator_kind
                .clone()
                .unwrap_or_else(|| "nvidia.com/gpu".to_string());
            requests.insert(kind, json!(spec.resources.accelerator_count));
        }

        let resources = json!({ "requests": requests });

        let container = json!({
            "name": "trainer",
            "image": image,
            "imagePullPolicy": "IfNotPresent",
            "command": command,
            "env": env,
            "resources": resources,
            "securityContext": {
                "readOnlyRootFilesystem": true,
                "allowPrivilegeEscalation": false,
                "runAsNonRoot": true,
                "runAsUser": 1000,
                "capabilities": { "drop": ["ALL"] },
                "seccompProfile": { "type": "RuntimeDefault" }
            },
            "volumeMounts": [
                { "name": "workspace", "mountPath": "/workspace" }
            ]
        });

        let pod_spec = json!({
            "restartPolicy": "Never",
            "terminationGracePeriodSeconds": 30,
            "securityContext": {
                "fsGroup": 1000,
                "runAsNonRoot": true,
                "seccompProfile": { "type": "RuntimeDefault" }
            },
            "containers": [container],
            "volumes": [
                { "name": "workspace", "emptyDir": { "sizeLimit": format!("{}Mi", spec.resources.ephemeral_storage_mib) } }
            ]
        });

        let manifest = if is_volcano {
            json!({
                "apiVersion": "batch.volcano.sh/v1alpha1",
                "kind": "Job",
                "metadata": {
                    "name": workload_name(job, attempt),
                    "namespace": namespace,
                    "labels": labels,
                    "ownerReferences": [owner_reference(job)]
                },
                "spec": {
                    "minAvailable": replicas,
                    "schedulerName": "volcano",
                    "tasks": [{
                        "name": "trainer",
                        "replicas": replicas,
                        "policies": [{"event": "TaskCompleted", "action": "CompleteJob"}],
                        "template": {
                            "spec": pod_spec
                        }
                    }],
                    "policies": [{"event": "JobFailed", "action": "AbortJob"}],
                    "maxRetry": 0,
                    "ttlSecondsAfterFinished": 3600,
                    "activeDeadlineSeconds": deadline_seconds
                }
            })
        } else {
            json!({
                "apiVersion": "batch/v1",
                "kind": "Job",
                "metadata": {
                    "name": workload_name(job, attempt),
                    "namespace": namespace,
                    "labels": labels,
                    "ownerReferences": [owner_reference(job)]
                },
                "spec": {
                    "activeDeadlineSeconds": deadline_seconds,
                    "backoffLimit": 0,
                    "ttlSecondsAfterFinished": 3600,
                    "template": {
                        "metadata": { "labels": labels },
                        "spec": pod_spec
                    }
                }
            })
        };
        Ok(manifest)
    }
}

fn workload_name(job: &TrainingJob, attempt: &Attempt) -> String {
    format!(
        "moqentra-{}-{}",
        sanitize(&job.id.to_string()),
        sanitize(&attempt.id.to_string())
    )
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase()
        .chars()
        .take(52)
        .collect()
}

fn owner_reference(job: &TrainingJob) -> serde_json::Value {
    json!({
        "apiVersion": "moqentra.io/v1",
        "kind": "TrainingJob",
        "name": job.id.to_string(),
        "uid": job.id.to_string(),
        "blockOwnerDeletion": true,
        "controller": true
    })
}

fn labels(job: &TrainingJob, attempt: &Attempt) -> serde_json::Value {
    json!({
        "app": "moqentra",
        "moqentra.io/tenant-id": job.tenant_id.to_string(),
        "moqentra.io/project-id": job.project_id.to_string(),
        "moqentra.io/training-job-id": job.id.to_string(),
        "moqentra.io/attempt-id": attempt.id.to_string(),
        "moqentra.io/managed-by": "moqentra-k8s-executor"
    })
}

/// Per-tenant security profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantSecurityProfile {
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub namespace: String,
    pub pod_security_level: PodSecurityLevel,
    pub network_ingress: bool,
    pub network_egress: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PodSecurityLevel {
    Restricted,
    Baseline,
    Privileged,
}

impl TenantSecurityProfile {
    pub fn namespace_manifest(&self) -> serde_json::Value {
        json!({
            "apiVersion": "v1",
            "kind": "Namespace",
            "metadata": {
                "name": self.namespace,
                "labels": {
                    "moqentra.io/tenant-id": self.tenant_id.to_string(),
                    "moqentra.io/project-id": self.project_id.to_string(),
                    "pod-security.kubernetes.io/enforce": match self.pod_security_level {
                        PodSecurityLevel::Restricted => "restricted",
                        PodSecurityLevel::Baseline => "baseline",
                        PodSecurityLevel::Privileged => "privileged",
                    },
                    "pod-security.kubernetes.io/audit": "restricted",
                    "pod-security.kubernetes.io/warn": "restricted"
                }
            }
        })
    }

    pub fn service_account_manifest(&self) -> serde_json::Value {
        json!({
            "apiVersion": "v1",
            "kind": "ServiceAccount",
            "metadata": {
                "name": "moqentra-runner",
                "namespace": self.namespace,
                "labels": {
                    "moqentra.io/tenant-id": self.tenant_id.to_string(),
                    "moqentra.io/project-id": self.project_id.to_string()
                }
            },
            "automountServiceAccountToken": false
        })
    }

    pub fn role_manifest(&self) -> serde_json::Value {
        json!({
            "apiVersion": "rbac.authorization.k8s.io/v1",
            "kind": "Role",
            "metadata": {
                "name": "moqentra-runner",
                "namespace": self.namespace
            },
            "rules": [
                { "apiGroups": [""], "resources": ["configmaps"], "verbs": ["get", "list"] },
                { "apiGroups": [""], "resources": ["pods"], "verbs": ["get", "list", "watch"] }
            ]
        })
    }

    pub fn role_binding_manifest(&self) -> serde_json::Value {
        json!({
            "apiVersion": "rbac.authorization.k8s.io/v1",
            "kind": "RoleBinding",
            "metadata": {
                "name": "moqentra-runner",
                "namespace": self.namespace
            },
            "subjects": [{
                "kind": "ServiceAccount",
                "name": "moqentra-runner",
                "namespace": self.namespace
            }],
            "roleRef": {
                "apiGroup": "rbac.authorization.k8s.io",
                "kind": "Role",
                "name": "moqentra-runner"
            }
        })
    }

    pub fn network_policy_manifest(&self) -> serde_json::Value {
        let mut spec = json!({
            "podSelector": { "matchLabels": { "app": "moqentra" } },
            "policyTypes": ["Ingress", "Egress"]
        });
        if !self.network_ingress {
            spec["ingress"] = json!([]);
        }
        if !self.network_egress {
            spec["egress"] = json!([]);
        }
        json!({
            "apiVersion": "networking.k8s.io/v1",
            "kind": "NetworkPolicy",
            "metadata": {
                "name": "moqentra-runner",
                "namespace": self.namespace
            },
            "spec": spec
        })
    }
}

/// Reconciler that removes orphaned workloads owned by this instance.
pub struct Reconciler<E: K8sExecutor> {
    executor: Arc<E>,
}

impl<E: K8sExecutor> Reconciler<E> {
    pub fn new(executor: Arc<E>) -> Self {
        Self { executor }
    }

    pub async fn reconcile(&self, namespace: &str) -> Result<u32, moqentra_types::Error> {
        let instance = self.executor.instance_id();
        let workloads = self.executor.list(namespace, "app=moqentra").await?;
        let mut removed = 0u32;
        for w in workloads {
            if w.owner_instance != instance {
                continue;
            }
            if w.lease_until.map(|t| t > Instant::now()).unwrap_or(false) {
                continue;
            }
            self.executor.delete(&w.namespace, &w.name).await?;
            removed += 1;
        }
        Ok(removed)
    }
}

/// Watch loop that maps Pod/Job states to attempt states and stays resilient.
pub async fn watch_workloads<E: K8sExecutor>(
    executor: Arc<E>,
    namespace: &str,
    label_selector: &str,
    mut resource_version: u64,
) -> Result<(), moqentra_types::Error> {
    let mut backoff = Duration::from_millis(100);
    loop {
        match executor
            .watch(
                namespace,
                label_selector,
                resource_version,
                Duration::from_secs(30),
            )
            .await
        {
            Ok((events, new_rv)) => {
                resource_version = new_rv;
                backoff = Duration::from_millis(100);
                for event in events {
                    tracing::info!(
                        name = %event.name,
                        observed_state = %event.observed_state,
                        "workload event"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "watch failed; relisting");
                sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(30));
                // Simulate 410 Gone recovery by relisting from zero.
                resource_version = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::training::{
        DistributedConfig, ParameterSchema, Rank, RankState, ResourceRequest, TrainingJobSpec,
    };
    use moqentra_types::{
        AttemptId, DatasetVersionId, ExperimentId, ProjectId, RandomIdGenerator, RankId, TenantId,
        TrainingJobId,
    };
    use std::collections::BTreeMap;

    fn gen_ids() -> (
        TrainingJobId,
        ExperimentId,
        TenantId,
        ProjectId,
        DatasetVersionId,
        AttemptId,
        RankId,
    ) {
        let g = RandomIdGenerator;
        (
            TrainingJobId::new_v7(&g),
            ExperimentId::new_v7(&g),
            TenantId::new_v7(&g),
            ProjectId::new_v7(&g),
            DatasetVersionId::new_v7(&g),
            AttemptId::new_v7(&g),
            RankId::new_v7(&g),
        )
    }

    fn make_attempt(job: &TrainingJob, token: u64) -> Attempt {
        let (_, _, _, _, _, id, _) = gen_ids();
        let world_size = job.spec.world_size();
        let mut ranks = Vec::with_capacity(world_size as usize);
        for _ in 0..world_size {
            let (_, _, _, _, _, _, rank_id) = gen_ids();
            ranks.push(Rank {
                id: rank_id,
                attempt_id: id,
                node_id: None,
                state: RankState::Pending,
                last_heartbeat: None,
                exit_code: None,
            });
        }
        Attempt::new(id, job.id, token, ranks).unwrap()
    }

    fn test_job() -> TrainingJob {
        let (job_id, exp_id, tenant_id, project_id, dv_id, _, _) = gen_ids();
        TrainingJob::new(
            job_id,
            exp_id,
            tenant_id,
            project_id,
            TrainingJobSpec {
                code_digest:
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                        .to_string(),
                image_digest:
                    "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                        .to_string(),
                dataset_version_id: dv_id,
                hyperparameters: ParameterSchema {
                    argv: vec!["train.py".to_string()],
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
            },
        )
        .unwrap()
    }

    #[test]
    fn compile_k8s_job_is_batch_v1() {
        let mut job = test_job();
        job.submit().unwrap();
        job.admit().unwrap();
        let attempt = make_attempt(&job, 1);
        job.start_attempt(attempt.clone()).unwrap();
        let manifest = JobCompiler::compile_k8s_job(&job, &attempt, "moqentra").unwrap();
        assert_eq!(manifest["apiVersion"], "batch/v1");
        assert_eq!(manifest["kind"], "Job");
        assert_eq!(manifest["metadata"]["namespace"], "moqentra");
        assert!(manifest["metadata"]["labels"]["moqentra.io/attempt-id"].is_string());
        assert_eq!(manifest["spec"]["backoffLimit"], 0);
        assert_eq!(manifest["spec"]["activeDeadlineSeconds"], 3600);
        let env_vars =
            manifest["spec"]["template"]["spec"]["containers"][0]["env"].as_array().unwrap();
        assert!(env_vars.iter().any(|e| e["name"] == "MOQENTRA_FENCING_TOKEN"));
    }

    #[test]
    fn compile_volcano_job_for_ddp() {
        let mut job = test_job();
        job.spec.resources.replicas = 2;
        job.spec.distributed = DistributedConfig::Ddp { world_size: 2 };
        job.submit().unwrap();
        job.admit().unwrap();
        let attempt = make_attempt(&job, 1);
        job.start_attempt(attempt.clone()).unwrap();
        let manifest = JobCompiler::compile_volcano_job(&job, &attempt, "moqentra").unwrap();
        assert_eq!(manifest["apiVersion"], "batch.volcano.sh/v1alpha1");
        assert_eq!(manifest["kind"], "Job");
        let tasks = manifest["spec"]["tasks"].as_array().unwrap();
        assert_eq!(tasks[0]["replicas"], 2);
        assert_eq!(tasks[0]["name"], "trainer");
    }

    #[test]
    fn namespace_manifest_enforces_psa() {
        let (_, _, tenant_id, project_id, _, _, _) = gen_ids();
        let profile = TenantSecurityProfile {
            tenant_id,
            project_id,
            namespace: "moqentra-t1-p1".to_string(),
            pod_security_level: PodSecurityLevel::Restricted,
            network_ingress: false,
            network_egress: false,
        };
        let ns = profile.namespace_manifest();
        assert_eq!(
            ns["metadata"]["labels"]["pod-security.kubernetes.io/enforce"],
            "restricted"
        );
    }

    #[tokio::test]
    async fn in_memory_executor_lifecycle() {
        let exec = Arc::new(InMemoryK8sExecutor::new());
        let mut job = test_job();
        job.submit().unwrap();
        job.admit().unwrap();
        let attempt = make_attempt(&job, 1);
        job.start_attempt(attempt.clone()).unwrap();
        let manifest = JobCompiler::compile_k8s_job(&job, &attempt, "moqentra").unwrap();
        let workload = K8sWorkload {
            tenant_id: job.tenant_id,
            project_id: job.project_id,
            training_job_id: job.id,
            attempt_id: attempt.id,
            namespace: "moqentra".to_string(),
            name: manifest["metadata"]["name"].as_str().unwrap().to_string(),
            kind: "Job".to_string(),
            manifest,
            desired_state: "Running".to_string(),
            observed_state: "Pending".to_string(),
            resource_version: 0,
            generation: 0,
            lease_until: None,
            diagnostics: Vec::new(),
            owner_instance: exec.instance_id(),
        };
        exec.apply(workload).await.unwrap();
        let listed = exec.list("moqentra", "app=moqentra").await.unwrap();
        assert_eq!(listed.len(), 1);

        let reconciler = Reconciler::new(exec.clone());
        // Lease is fresh, nothing should be removed.
        assert_eq!(reconciler.reconcile("moqentra").await.unwrap(), 0);
    }
}
