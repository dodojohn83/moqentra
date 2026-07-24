//! PostgreSQL-backed scheduler polling for queued training jobs.

use moqentra_application::ports::{DatasetRepository, TrainingJobRepository, Versioned};
use moqentra_domain::dataset::DatasetVersionState;
use moqentra_domain::training::{Attempt, Rank, RankState, TrainingJob, TrainingJobState};
use moqentra_scheduler::scheduler::{ClusterTopology, NodeCapacity};
use moqentra_scheduler::AcceleratorCapability;
use moqentra_storage::{PgDatasetRepository, PgTrainingJobRepository};
use moqentra_types::{
    AttemptId, DatasetId, NodeId, Principal, RandomIdGenerator, RankId, RequestContext,
    UtcTimestamp,
};
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Clone, Deserialize, Debug)]
struct NodeCapabilities {
    node_id: NodeId,
    cpu_cores: u32,
    memory_mib: u64,
    disk_mib: u64,
    container_runtime: String,
    devices: Vec<Device>,
    healthy: bool,
    reported_at: UtcTimestamp,
}

#[derive(Clone, Deserialize, Debug)]
struct Device {
    uuid: String,
    #[serde(with = "accel_kind")]
    kind: AcceleratorKind,
    memory_mib: u64,
    driver_version: String,
    runtime: String,
    healthy: bool,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum AcceleratorKind {
    Nvidia,
    Amd,
    Ascend,
    Cpu,
}

impl AcceleratorKind {
    fn as_str(&self) -> &'static str {
        match self {
            AcceleratorKind::Nvidia => "nvidia",
            AcceleratorKind::Amd => "amd",
            AcceleratorKind::Ascend => "ascend",
            AcceleratorKind::Cpu => "cpu",
        }
    }
}

mod accel_kind {
    use super::AcceleratorKind;
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<AcceleratorKind, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "nvidia" => Ok(AcceleratorKind::Nvidia),
            "amd" => Ok(AcceleratorKind::Amd),
            "ascend" => Ok(AcceleratorKind::Ascend),
            "cpu" => Ok(AcceleratorKind::Cpu),
            _ => Err(serde::de::Error::custom(format!(
                "unknown accelerator kind: {s}"
            ))),
        }
    }
}

pub struct PgScheduler {
    pool: PgPool,
    http: Client,
    node_url: Option<String>,
    fencing_counter: Arc<AtomicU64>,
}

impl PgScheduler {
    pub fn new(
        pool: PgPool,
        http: Client,
        node_url: Option<String>,
        fencing_counter: Arc<AtomicU64>,
    ) -> Self {
        Self {
            pool,
            http,
            node_url,
            fencing_counter,
        }
    }

    pub async fn poll(&self, limit: u32) -> u32 {
        let repo = PgTrainingJobRepository::new(self.pool.clone());
        let jobs = match repo.list_queued_for_scheduler(limit).await {
            Ok(jobs) => jobs,
            Err(e) => {
                tracing::warn!(error = %e, "failed to list queued training jobs");
                return 0;
            }
        };

        let mut processed = 0u32;
        for versioned in jobs {
            if let Err(e) = self.process_job(versioned).await {
                tracing::warn!(error = %e, "failed to process queued job");
            } else {
                processed += 1;
            }
        }
        processed
    }

    async fn process_job(
        &self,
        versioned: Versioned<TrainingJob>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let Versioned {
            entity: mut job,
            revision,
            ..
        } = versioned;

        if job.state != TrainingJobState::Queued {
            return Ok(());
        }

        let gen = RandomIdGenerator;
        let ctx = RequestContext::new(
            job.tenant_id,
            Principal::service("moqentra-scheduler"),
            "scheduler-poll",
        )
        .with_project(job.project_id);

        // Validate that the referenced dataset version is published (frozen).
        let dataset_repo = PgDatasetRepository::new(self.pool.clone());
        let dummy_dataset_id = DatasetId::new_v7(&gen);
        let version = dataset_repo
            .get_version(&ctx, dummy_dataset_id, job.spec.dataset_version_id)
            .await?;
        if version.entity.state != DatasetVersionState::Published {
            return Err(format!(
                "dataset version {} is not published (state: {:?})",
                job.spec.dataset_version_id, version.entity.state
            )
            .into());
        }

        // Validate capabilities against the configured node agent, if any.
        if let Some(base) = &self.node_url {
            let caps = self.fetch_capabilities(base).await?;
            let topology = self.build_topology(&caps);
            topology.find_placement(&job.spec.resources).map_err(|e| {
                Box::<dyn std::error::Error + Send + Sync>::from(format!(
                    "no node matches resource request: {e}"
                ))
            })?;
        }

        // Create an attempt and transition the job to Starting.
        let attempt_id = AttemptId::new_v7(&gen);
        let fencing_token = self.fencing_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let world_size = job.spec.world_size();
        if world_size == 0 {
            return Err("distributed world_size must be greater than zero".into());
        }

        let mut ranks = Vec::with_capacity(usize::try_from(world_size).unwrap_or(0));
        for _ in 0..world_size {
            ranks.push(Rank {
                id: RankId::new_v7(&gen),
                attempt_id,
                node_id: None,
                state: RankState::Pending,
                last_heartbeat: None,
                exit_code: None,
            });
        }
        let attempt = Attempt::new(attempt_id, job.id, fencing_token, ranks)?;

        let job_id = job.id;
        let resources = job.spec.resources.clone();
        job.admit()?;
        job.start_attempt(attempt)?;

        let repo = PgTrainingJobRepository::new(self.pool.clone());
        repo.update(&ctx, job.id, revision, job).await?;

        // Request a resource lease from the node agent, if configured.
        if let Some(base) = &self.node_url {
            let url = format!("{}/v1/allocations", base.trim_end_matches('/'));
            let body = serde_json::json!({
                "attempt_id": attempt_id.to_string(),
                "cpu_cores": resources.cpu_milli.div_ceil(1000),
                "memory_mib": resources.memory_mib,
                "device_count": resources.accelerator_count,
                "fencing_token": fencing_token,
            });
            let resp = self.http.post(&url).json(&body).send().await?;
            if !resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(format!("node-agent allocate failed: {text}").into());
            }
        }

        tracing::info!(%job_id, %attempt_id, "admitted training job");
        Ok(())
    }

    async fn fetch_capabilities(
        &self,
        base: &str,
    ) -> Result<NodeCapabilities, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/v1/capabilities", base.trim_end_matches('/'));
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(format!("node-agent capabilities returned {}", resp.status()).into());
        }
        Ok(resp.json().await?)
    }

    fn build_topology(&self, caps: &NodeCapabilities) -> ClusterTopology {
        let accelerators: Vec<AcceleratorCapability> = caps
            .devices
            .iter()
            .filter(|d| d.healthy)
            .map(|d| AcceleratorCapability {
                kind: d.kind.as_str().to_string(),
                memory_mib: d.memory_mib,
                compute_units: 0,
                supported_collectives: vec![],
                runtime_labels: {
                    let mut labels = BTreeMap::new();
                    labels.insert("runtime".to_string(), d.runtime.clone());
                    labels.insert("driver".to_string(), d.driver_version.clone());
                    labels.insert("uuid".to_string(), d.uuid.clone());
                    labels
                },
            })
            .collect();

        let mut nodes = BTreeMap::new();
        nodes.insert(
            caps.node_id.to_string(),
            NodeCapacity {
                cpu_cores: caps.cpu_cores,
                memory_mib: caps.memory_mib,
                accelerators,
                taints: BTreeSet::new(),
                labels: BTreeMap::new(),
                healthy: true,
                observed_at: moqentra_types::UtcTimestamp::now(),
            },
        );
        ClusterTopology { nodes }
    }
}
