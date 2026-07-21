//! Moqentra node-agent: local resource admission and health endpoints.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use moqentra_types::{AttemptId, NodeId, RandomIdGenerator, UtcTimestamp};
use moqentra_worker_control::local_executor::{
    AcceleratorKind, AllocationRequest, Device, LocalExecutor, NodeCapabilities,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AgentState {
    capabilities: Arc<NodeCapabilities>,
    executor: Arc<Mutex<LocalExecutor>>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    node_id: String,
    healthy: bool,
}

#[derive(Deserialize)]
struct AllocateRequest {
    attempt_id: String,
    cpu_cores: u32,
    memory_mib: u64,
    #[serde(default)]
    device_count: u32,
    #[serde(default)]
    fencing_token: u64,
}

fn discover_capabilities() -> NodeCapabilities {
    let gen = RandomIdGenerator;
    NodeCapabilities {
        node_id: NodeId::new_v7(&gen),
        cpu_cores: std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1),
        memory_mib: 8192,
        disk_mib: 102_400,
        container_runtime: std::env::var("MOQENTRA_CONTAINER_RUNTIME")
            .unwrap_or_else(|_| "podman".to_string()),
        devices: vec![Device {
            uuid: "cpu-0".to_string(),
            kind: AcceleratorKind::Cpu,
            memory_mib: 0,
            driver_version: "n/a".to_string(),
            runtime: "host".to_string(),
            healthy: true,
        }],
        healthy: true,
        reported_at: UtcTimestamp::now(),
    }
}

async fn healthz(State(state): State<AgentState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "moqentra-node-agent",
        node_id: state.capabilities.node_id.to_string(),
        healthy: state.capabilities.healthy,
    })
}

async fn capabilities(State(state): State<AgentState>) -> impl IntoResponse {
    (StatusCode::OK, Json(state.capabilities.as_ref().clone()))
}

async fn allocations(State(state): State<AgentState>) -> impl IntoResponse {
    let executor = state.executor.lock().unwrap_or_else(|e| e.into_inner());
    let summary: Vec<_> = executor
        .allocations_snapshot()
        .into_iter()
        .map(|(id, released)| serde_json::json!({"id": id, "released": released}))
        .collect();
    (StatusCode::OK, Json(summary))
}

async fn allocate(
    State(state): State<AgentState>,
    Json(req): Json<AllocateRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    let attempt_id = AttemptId::try_from(req.attempt_id.as_str()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    let request = AllocationRequest {
        attempt_id,
        cpu_cores: req.cpu_cores,
        memory_mib: req.memory_mib,
        devices: if req.device_count > 0 {
            vec![AcceleratorKind::Cpu]
        } else {
            vec![]
        },
        device_count: req.device_count,
    };
    let mut executor = state.executor.lock().unwrap_or_else(|e| e.into_inner());
    let alloc = executor
        .allocate(
            &request,
            state.capabilities.node_id,
            &state.capabilities,
            req.fencing_token.max(1),
        )
        .map_err(|e| {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": alloc.id,
            "cpu_cores": alloc.cpu_cores,
            "memory_mib": alloc.memory_mib,
            "device_uuids": alloc.device_uuids,
        })),
    ))
}

async fn release(
    State(state): State<AgentState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let mut executor = state.executor.lock().unwrap_or_else(|e| e.into_inner());
    executor.release(&id).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    Ok(StatusCode::NO_CONTENT)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let caps = discover_capabilities();
    let workspace =
        std::env::var("MOQENTRA_WORKSPACE_ROOT").unwrap_or_else(|_| "/tmp/moqentra".to_string());
    let executor = LocalExecutor::new().with_workspace_root(workspace);
    let state = AgentState {
        capabilities: Arc::new(caps),
        executor: Arc::new(Mutex::new(executor)),
    };

    let listen =
        std::env::var("MOQENTRA_NODE_AGENT_ADDR").unwrap_or_else(|_| "0.0.0.0:8081".to_string());
    let addr: SocketAddr = listen.parse()?;

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/capabilities", get(capabilities))
        .route("/v1/allocations", get(allocations).post(allocate))
        .route("/v1/allocations/{id}", delete(release))
        .with_state(state);

    tracing::info!(%addr, "moqentra-node-agent listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
