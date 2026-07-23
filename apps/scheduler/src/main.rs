//! Moqentra scheduler agent: leader election + queue + health.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use moqentra_scheduler::{LeaderElection, QueueEntry, Reconciler, SchedulingQueue};
use moqentra_types::{RandomIdGenerator, TenantId, UtcTimestamp};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing_subscriber::EnvFilter;

mod pg;

#[derive(Clone)]
struct SchedulerState {
    instance_id: String,
    ready: Arc<AtomicBool>,
    is_leader: Arc<AtomicBool>,
    term: Arc<AtomicU64>,
    cycles: Arc<AtomicU64>,
    admitted: Arc<AtomicU64>,
    fencing_counter: Arc<AtomicU64>,
    election: Arc<Mutex<LeaderElection>>,
    reconciler: Arc<Reconciler>,
    queue: Arc<Mutex<SchedulingQueue>>,
    db_pool: Option<PgPool>,
    http: reqwest::Client,
    node_url: Option<String>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    instance_id: String,
    leader: bool,
    term: u64,
    reconcile_cycles: u64,
    admitted_jobs: u64,
    queue_depth: usize,
}

#[derive(Deserialize)]
struct EnqueueRequest {
    job_id: String,
    #[serde(default)]
    tenant_id: String,
    project_id: String,
    #[serde(default)]
    priority: u32,
}

#[derive(Serialize)]
struct EnqueueResponse {
    job_id: String,
    queue_depth: usize,
}

async fn healthz(State(state): State<SchedulerState>) -> Json<HealthResponse> {
    let queue_depth = state.queue.lock().map(|q| q.entries.len()).unwrap_or(0);
    Json(HealthResponse {
        status: "ok",
        service: "moqentra-scheduler",
        instance_id: state.instance_id.clone(),
        leader: state.is_leader.load(Ordering::Relaxed),
        term: state.term.load(Ordering::Relaxed),
        reconcile_cycles: state.cycles.load(Ordering::Relaxed),
        admitted_jobs: state.admitted.load(Ordering::Relaxed),
        queue_depth,
    })
}

async fn readyz(State(state): State<SchedulerState>) -> impl IntoResponse {
    let ready = state.ready.load(Ordering::Relaxed);
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(serde_json::json!({
            "ready": ready,
            "leader": state.is_leader.load(Ordering::Relaxed),
        })),
    )
}

async fn enqueue(
    State(state): State<SchedulerState>,
    Json(req): Json<EnqueueRequest>,
) -> Result<(StatusCode, Json<EnqueueResponse>), (StatusCode, Json<serde_json::Value>)> {
    if req.job_id.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "job_id required"})),
        ));
    }
    let entry = QueueEntry {
        job_id: req.job_id.clone(),
        tenant_id: req.tenant_id,
        project_id: req.project_id,
        priority: req.priority,
        submitted_at: UtcTimestamp::now(),
    };
    let mut queue = state.queue.lock().unwrap_or_else(|e| e.into_inner());
    queue.enqueue(entry).map_err(|e| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    let depth = queue.entries.len();
    Ok((
        StatusCode::ACCEPTED,
        Json(EnqueueResponse {
            job_id: req.job_id,
            queue_depth: depth,
        }),
    ))
}

async fn list_queue(State(state): State<SchedulerState>) -> Json<Vec<serde_json::Value>> {
    let queue = state.queue.lock().unwrap_or_else(|e| e.into_inner());
    let items = queue
        .entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "job_id": e.job_id,
                "tenant_id": e.tenant_id,
                "project_id": e.project_id,
                "priority": e.priority,
            })
        })
        .collect();
    Json(items)
}

fn spawn_control_loop(state: SchedulerState) {
    let control_plane_url = std::env::var("MOQENTRA_CONTROL_PLANE_URL").ok();
    let service_token = std::env::var("MOQENTRA_SERVICE_TOKEN").ok();

    tokio::spawn(async move {
        let mut term = 1u64;
        loop {
            {
                let mut election = state.election.lock().unwrap_or_else(|e| e.into_inner());
                if election.campaign(term).is_ok() {
                    state.is_leader.store(true, Ordering::Relaxed);
                    state.term.store(term, Ordering::Relaxed);
                    state.ready.store(true, Ordering::Relaxed);
                }
            }

            if state.is_leader.load(Ordering::Relaxed) {
                if let Some(pool) = state.db_pool.clone() {
                    // PostgreSQL-backed scheduling: poll, validate, and create attempts/leases.
                    let pg = pg::PgScheduler::new(
                        pool,
                        state.http.clone(),
                        state.node_url.clone(),
                        state.fencing_counter.clone(),
                    );
                    let processed = pg.poll(16).await;
                    state.admitted.fetch_add(processed as u64, Ordering::Relaxed);
                } else {
                    // In-memory fallback for local-only demos.
                    process_in_memory_job(&state, &control_plane_url, &service_token).await;
                }

                let mut resources: BTreeMap<String, moqentra_scheduler::DesiredObserved<String>> =
                    BTreeMap::new();
                let processed = state.reconciler.run_cycle(&mut resources, |_| None);
                state.cycles.fetch_add(1, Ordering::Relaxed);
                tracing::debug!(processed, "reconcile cycle complete");
            }

            term = term.saturating_add(1);
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

async fn process_in_memory_job(
    state: &SchedulerState,
    control_plane_url: &Option<String>,
    service_token: &Option<String>,
) {
    let popped = {
        let mut queue = state.queue.lock().unwrap_or_else(|e| e.into_inner());
        queue.pop()
    };
    if let Some(entry) = popped {
        state.admitted.fetch_add(1, Ordering::Relaxed);
        tracing::info!(job_id = %entry.job_id, tenant = %entry.tenant_id, "popped job from queue");

        if let Some(base) = control_plane_url {
            let url = format!(
                "{}/v1/training-jobs/{}/admit",
                base.trim_end_matches('/'),
                entry.job_id
            );
            let mut req = state.http.post(&url).header("x-tenant-id", &entry.tenant_id);
            if let Some(tok) = service_token {
                req = req.header("authorization", format!("Bearer {tok}"));
            }
            if !entry.project_id.is_empty() {
                req = req.header("x-project-id", &entry.project_id);
            }
            match req.send().await {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!(job_id = %entry.job_id, "control-plane admit ok");
                }
                Ok(resp) => {
                    tracing::warn!(
                        job_id = %entry.job_id,
                        status = %resp.status(),
                        "control-plane admit failed"
                    );
                }
                Err(e) => {
                    tracing::warn!(error = %e, "control-plane admit request error")
                }
            }
        }

        if let Some(base) = &state.node_url {
            let url = format!("{}/v1/allocations", base.trim_end_matches('/'));
            let body = serde_json::json!({
                "attempt_id": entry.job_id,
                "cpu_cores": 2,
                "memory_mib": 4096,
                "device_count": 0,
                "fencing_token": 1
            });
            match state.http.post(&url).json(&body).send().await {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!(job_id = %entry.job_id, "node-agent allocate ok");
                }
                Ok(resp) => {
                    tracing::warn!(
                        job_id = %entry.job_id,
                        status = %resp.status(),
                        "node-agent allocate failed"
                    );
                }
                Err(e) => {
                    tracing::warn!(error = %e, "node-agent allocate request error")
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let instance_id = std::env::var("MOQENTRA_INSTANCE_ID")
        .unwrap_or_else(|_| format!("scheduler-{}", UtcTimestamp::now()));
    let gen = RandomIdGenerator;
    let tenant = TenantId::new_v7(&gen);

    let db_pool = if let Ok(dsn) = std::env::var("MOQENTRA_DATABASE_DSN") {
        if dsn.is_empty() {
            None
        } else {
            Some(
                PgPool::connect(&dsn)
                    .await
                    .map_err(|e| anyhow::anyhow!("MOQENTRA_DATABASE_DSN connection failed: {e}"))?,
            )
        }
    } else {
        None
    };

    let state = SchedulerState {
        instance_id: instance_id.clone(),
        ready: Arc::new(AtomicBool::new(false)),
        is_leader: Arc::new(AtomicBool::new(false)),
        term: Arc::new(AtomicU64::new(0)),
        cycles: Arc::new(AtomicU64::new(0)),
        admitted: Arc::new(AtomicU64::new(0)),
        fencing_counter: Arc::new(AtomicU64::new(1)),
        election: Arc::new(Mutex::new(LeaderElection::new(instance_id.clone()))),
        reconciler: Arc::new(Reconciler::new(instance_id, 32)),
        queue: Arc::new(Mutex::new(SchedulingQueue::new(
            "default", tenant, 1024, 256,
        ))),
        db_pool,
        http: reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build http client: {e}"))?,
        node_url: std::env::var("MOQENTRA_NODE_AGENT_URL").ok().filter(|s| !s.is_empty()),
    };

    spawn_control_loop(state.clone());

    let listen =
        std::env::var("MOQENTRA_SCHEDULER_ADDR").unwrap_or_else(|_| "0.0.0.0:8082".to_string());
    let addr: SocketAddr = listen.parse()?;
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/v1/queue", get(list_queue).post(enqueue))
        .with_state(state);

    tracing::info!(%addr, "moqentra-scheduler listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
