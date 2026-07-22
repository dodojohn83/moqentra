//! Moqentra node-agent: local resource admission, health endpoints, and worker stream.

mod client;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get};
use axum::{Json, Router};
use moqentra_types::{AttemptId, NodeId, RandomIdGenerator, UtcTimestamp};
use moqentra_worker_control::local_executor::{
    AcceleratorKind, AllocationRequest, Device, LocalExecutor, NodeCapabilities,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
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

fn runtime_available(name: &str) -> bool {
    std::process::Command::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn detect_container_runtime() -> String {
    if let Ok(preferred) = std::env::var("MOQENTRA_CONTAINER_RUNTIME") {
        if !preferred.is_empty() && runtime_available(&preferred) {
            return preferred;
        }
    }
    for runtime in ["podman", "docker"] {
        if runtime_available(runtime) {
            return runtime.to_string();
        }
    }
    "none".to_string()
}

fn parse_mib(value: &str) -> u64 {
    value.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0)
}

fn detect_nvidia_devices() -> Vec<Device> {
    let output = match std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=uuid,name,memory.total,driver_version",
            "--format=csv,noheader",
        ])
        .output()
    {
        Ok(out) if out.status.success() => out.stdout,
        _ => return vec![],
    };

    let mut devices = Vec::new();
    for line in String::from_utf8_lossy(&output).lines() {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() < 4 {
            continue;
        }
        devices.push(Device {
            uuid: parts[0].to_string(),
            kind: AcceleratorKind::Nvidia,
            memory_mib: parse_mib(parts[2]),
            driver_version: parts[3].to_string(),
            runtime: "cuda".to_string(),
            healthy: true,
        });
    }
    devices
}

fn discover_capabilities() -> NodeCapabilities {
    let gen = RandomIdGenerator;
    let mut devices = detect_nvidia_devices();
    devices.push(Device {
        uuid: "cpu-0".to_string(),
        kind: AcceleratorKind::Cpu,
        memory_mib: 0,
        driver_version: "n/a".to_string(),
        runtime: "host".to_string(),
        healthy: true,
    });

    NodeCapabilities {
        node_id: NodeId::new_v7(&gen),
        cpu_cores: std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1),
        memory_mib: 8192,
        disk_mib: 102_400,
        container_runtime: detect_container_runtime(),
        devices,
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
    let executor = state.executor.lock().await;
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
    let mut executor = state.executor.lock().await;
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
    let mut executor = state.executor.lock().await;
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

    let caps = Arc::new(discover_capabilities());
    let workspace =
        std::env::var("MOQENTRA_WORKSPACE_ROOT").unwrap_or_else(|_| "/tmp/moqentra".to_string());
    let executor = Arc::new(Mutex::new(
        LocalExecutor::new()
            .with_workspace_root(workspace)
            .with_container_runtime(caps.container_runtime.clone()),
    ));
    let state = AgentState {
        capabilities: Arc::clone(&caps),
        executor: Arc::clone(&executor),
    };

    if let Ok(grpc_addr) = std::env::var("MOQENTRA_CONTROL_PLANE_GRPC_ADDR") {
        if !grpc_addr.is_empty() {
            let caps = Arc::clone(&caps);
            tokio::spawn(async move {
                loop {
                    if let Err(e) = client::run_worker_stream(
                        &grpc_addr,
                        (*caps).clone(),
                        Arc::clone(&executor),
                    )
                    .await
                    {
                        tracing::error!(error = %e, "worker stream failed; reconnecting in 5s");
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            });
        }
    }

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
