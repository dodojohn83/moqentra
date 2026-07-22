//! Moqentra dyun-agent: replica lifecycle + health.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use moqentra_dyun_adapter::dyun::{
    AgentCapabilities, DyunGraphBundle, Replica, ReplicaState, ResourceLimits,
};
use moqentra_types::{DeploymentId, NodeId, RandomIdGenerator, ReplicaId, UtcTimestamp};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AgentState {
    capabilities: Arc<AgentCapabilities>,
    replicas: Arc<Mutex<HashMap<ReplicaId, Replica>>>,
    sandbox_root: String,
    production_keys: Arc<Vec<String>>,
    dev_keys: Arc<Vec<String>>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    node_id: String,
    replica_count: usize,
    running: usize,
    sandbox_root: String,
}

#[derive(Deserialize)]
struct CreateReplicaRequest {
    deployment_id: String,
    generation: u64,
    fencing_token: u64,
    application_digest: String,
    graph_spec_digest: String,
    signature: String,
}

#[derive(Deserialize)]
struct TransitionRequest {
    generation: u64,
    fencing_token: u64,
    state: String,
    #[serde(default)]
    observed_generation: u64,
}

async fn healthz(State(state): State<AgentState>) -> Json<HealthResponse> {
    let replicas = state.replicas.lock().unwrap_or_else(|e| e.into_inner());
    let running = replicas.values().filter(|r| matches!(r.state, ReplicaState::Running)).count();
    Json(HealthResponse {
        status: "ok",
        service: "moqentra-dyun-agent",
        node_id: state.capabilities.node_id.to_string(),
        replica_count: replicas.len(),
        running,
        sandbox_root: state.sandbox_root.clone(),
    })
}

async fn capabilities(State(state): State<AgentState>) -> Json<AgentCapabilities> {
    Json(state.capabilities.as_ref().clone())
}

async fn list_replicas(State(state): State<AgentState>) -> Json<Vec<serde_json::Value>> {
    let replicas = state.replicas.lock().unwrap_or_else(|e| e.into_inner());
    let list = replicas
        .values()
        .map(|r| {
            serde_json::json!({
                "id": r.id.to_string(),
                "deployment_id": r.deployment_id.to_string(),
                "generation": r.generation,
                "state": format!("{:?}", r.state),
                "desired_state": format!("{:?}", r.desired_state),
                "node_id": r.node_id.to_string(),
            })
        })
        .collect();
    Json(list)
}

async fn create_replica(
    State(state): State<AgentState>,
    Json(req): Json<CreateReplicaRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    let gen = RandomIdGenerator;
    let deployment_id = DeploymentId::try_from(req.deployment_id.as_str()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    let bundle = DyunGraphBundle {
        version: "DyunGraphBundle/v1".to_string(),
        application_digest: req.application_digest,
        graph_spec_digest: req.graph_spec_digest,
        artifact_bindings: BTreeMap::new(),
        runtime_profile: "rtsp-track-rtmp".to_string(),
        resource_limits: ResourceLimits {
            cpu_milli: 2000,
            memory_mib: 4096,
            gpu_count: 0,
        },
        signature: req.signature,
        compatible_dg_versions: vec!["dg-1.0".to_string()],
    };
    let mut replica = Replica::new(
        ReplicaId::new_v7(&gen),
        deployment_id,
        req.generation,
        req.fencing_token,
        bundle,
        state.capabilities.node_id,
    );
    replica.verify_bundle(&state.production_keys, &state.dev_keys).map_err(|e| {
        (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    replica
        .transition(req.generation, req.fencing_token, ReplicaState::Preparing)
        .map_err(|e| {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;
    let id = replica.id;
    state.replicas.lock().unwrap_or_else(|e| e.into_inner()).insert(id, replica);
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id.to_string(),
            "state": "Preparing",
        })),
    ))
}

fn parse_state(s: &str) -> Option<ReplicaState> {
    match s {
        "Pending" => Some(ReplicaState::Pending),
        "Preparing" => Some(ReplicaState::Preparing),
        "Starting" => Some(ReplicaState::Starting),
        "Running" => Some(ReplicaState::Running),
        "Draining" => Some(ReplicaState::Draining),
        "Stopped" => Some(ReplicaState::Stopped),
        "Failed" => Some(ReplicaState::Failed),
        _ => None,
    }
}

async fn transition_replica(
    State(state): State<AgentState>,
    Path(id): Path<String>,
    Json(req): Json<TransitionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let replica_id = ReplicaId::try_from(id.as_str()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    let target = parse_state(&req.state).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid state"})),
        )
    })?;
    let drain_timeout = std::env::var("MOQENTRA_DYUN_DRAIN_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let mut replicas = state.replicas.lock().unwrap_or_else(|e| e.into_inner());
    let replica = replicas.get_mut(&replica_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "replica not found"})),
        )
    })?;
    if target == ReplicaState::Draining {
        replica.drain(std::time::Duration::from_secs(drain_timeout));
    } else {
        replica.transition(req.generation, req.fencing_token, target).map_err(|e| {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;
    }
    Ok(Json(serde_json::json!({
        "id": replica.id.to_string(),
        "state": format!("{:?}", replica.state),
    })))
}

async fn heartbeat_replica(
    State(state): State<AgentState>,
    Path(id): Path<String>,
    Json(req): Json<TransitionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let replica_id = ReplicaId::try_from(id.as_str()).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;
    let mut replicas = state.replicas.lock().unwrap_or_else(|e| e.into_inner());
    let replica = replicas.get_mut(&replica_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "replica not found"})),
        )
    })?;
    replica
        .heartbeat(req.generation, req.fencing_token, req.observed_generation)
        .map_err(|e| {
            (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;
    Ok(Json(serde_json::json!({
        "id": replica.id.to_string(),
        "last_heartbeat": replica.last_heartbeat.map(|t| t.to_string()),
    })))
}

fn comma_list(env_var: &str) -> Vec<String> {
    std::env::var(env_var)
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn discover_capabilities() -> AgentCapabilities {
    let gen = RandomIdGenerator;
    // Capabilities are determined at startup from the dg build environment,
    // not hard-coded. If no probe data is provided, the agent advertises no
    // accelerators, forcing explicit configuration.
    AgentCapabilities {
        node_id: NodeId::new_v7(&gen),
        agent_version: std::env::var("MOQENTRA_DYUN_AGENT_VERSION")
            .unwrap_or_else(|_| "0.1.0".to_string()),
        dg_versions: comma_list("MOQENTRA_DG_VERSIONS"),
        codecs: comma_list("MOQENTRA_DG_CODECS"),
        accelerators: comma_list("MOQENTRA_DG_ACCELERATORS"),
        element_schemas: comma_list("MOQENTRA_DG_ELEMENT_SCHEMAS"),
        backends: comma_list("MOQENTRA_DG_BACKENDS"),
        build_features: comma_list("MOQENTRA_DG_BUILD_FEATURES"),
        max_replicas: std::env::var("MOQENTRA_DYUN_MAX_REPLICAS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let caps = discover_capabilities();
    let sandbox_root =
        std::env::var("MOQENTRA_DYUN_SANDBOX").unwrap_or_else(|_| "/tmp/moqentra-dyun".to_string());
    std::fs::create_dir_all(&sandbox_root)?;
    let production_keys = std::env::var("MOQENTRA_DYUN_PRODUCTION_KEYS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let dev_keys = std::env::var("MOQENTRA_DYUN_DEV_KEYS")
        .unwrap_or_else(|_| "trusted:".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let state = AgentState {
        capabilities: Arc::new(caps),
        replicas: Arc::new(Mutex::new(HashMap::new())),
        sandbox_root,
        production_keys: Arc::new(production_keys),
        dev_keys: Arc::new(dev_keys),
    };

    let listen =
        std::env::var("MOQENTRA_DYUN_AGENT_ADDR").unwrap_or_else(|_| "0.0.0.0:8083".to_string());
    let addr: SocketAddr = listen.parse()?;
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/capabilities", get(capabilities))
        .route("/v1/replicas", get(list_replicas).post(create_replica))
        .route("/v1/replicas/{id}/transition", post(transition_replica))
        .route("/v1/replicas/{id}/heartbeat", post(heartbeat_replica))
        .with_state(state.clone());

    let heartbeat_timeout = std::env::var("MOQENTRA_DYUN_HEARTBEAT_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    tokio::spawn(reconcile_replicas(
        state,
        std::time::Duration::from_secs(heartbeat_timeout),
    ));

    tracing::info!(%addr, "moqentra-dyun-agent listening");
    let _ = UtcTimestamp::now();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn reconcile_replicas(state: AgentState, heartbeat_timeout: std::time::Duration) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        interval.tick().await;
        let mut replicas = state.replicas.lock().unwrap_or_else(|e| e.into_inner());
        let now = UtcTimestamp::now();
        for replica in replicas.values_mut() {
            replica.reconcile(now, heartbeat_timeout);
        }
    }
}
